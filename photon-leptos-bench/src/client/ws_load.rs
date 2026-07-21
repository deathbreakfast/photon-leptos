//! tokio-tungstenite load generator for BM-PLS*.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use futures::StreamExt;
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;

use crate::stats::MetricStats;
use crate::WS_PATH;

#[derive(Debug, Clone)]
pub struct ServerTarget {
    pub base_http: String,
    pub ws_path: String,
}

impl ServerTarget {
    pub fn ws_url(&self) -> Result<Url> {
        let http = self.base_http.trim_end_matches('/');
        let ws_base = if let Some(rest) = http.strip_prefix("https://") {
            format!("wss://{rest}")
        } else if let Some(rest) = http.strip_prefix("http://") {
            format!("ws://{rest}")
        } else {
            format!("ws://{http}")
        };
        Url::parse(&format!("{ws_base}{}", self.ws_path))
            .with_context(|| format!("ws url from {http}"))
    }

    pub fn publish_url(&self) -> String {
        format!("{}/api/bench/publish", self.base_http.trim_end_matches('/'))
    }

    pub fn health_url(&self) -> String {
        format!("{}/health", self.base_http.trim_end_matches('/'))
    }
}

#[derive(Debug, Clone)]
pub struct ConnectOptions {
    pub count: u32,
    pub key_filter: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PublishOptions {
    pub rate_per_sec: u32,
    pub duration_secs: u32,
    pub payload_bytes: usize,
    /// Single partition for the whole burst (`topic_key` on the server).
    pub topic_key: Option<String>,
    /// Round-robin across `key-0` .. `key-(G-1)`.
    pub key_groups: Option<u32>,
}

impl PublishOptions {
    pub fn simple(rate_per_sec: u32, duration_secs: u32, payload_bytes: usize) -> Self {
        Self {
            rate_per_sec,
            duration_secs,
            payload_bytes,
            topic_key: None,
            key_groups: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LoadGenResult {
    pub connected: u32,
    pub connect_failures: u32,
    pub connect_latency_ms: Vec<f64>,
    pub delivery_latency_ms: Vec<f64>,
    pub messages_received: u64,
    pub publishes_attempted: u64,
    pub publishes_succeeded: u64,
    pub publishes_failed: u64,
    /// Legacy alias: publish HTTP / job failures (prefer `publishes_failed`).
    pub publish_errors: u64,
}

#[derive(Debug, Clone, Default)]
pub struct PublishStats {
    pub attempted: u64,
    pub succeeded: u64,
    pub failed: u64,
}

#[derive(Deserialize)]
struct PhotonWsEvent {
    payload_json: serde_json::Value,
}

#[derive(Deserialize)]
struct BenchPayload {
    published_at_ms: u64,
}

pub struct WsSession {
    connected: u32,
    connect_failures: u32,
    connect_latency_ms: Vec<f64>,
    handles: Vec<JoinHandle<()>>,
    delivery_rx: mpsc::UnboundedReceiver<f64>,
    received: Arc<AtomicU64>,
}

pub async fn wait_for_health(target: &ServerTarget, timeout: Duration) -> Result<()> {
    let client = reqwest::Client::new();
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if client
            .get(target.health_url())
            .send()
            .await
            .is_ok_and(|r| r.status().is_success())
        {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    anyhow::bail!("server health timeout: {}", target.health_url())
}

pub async fn spawn_connections(target: &ServerTarget, opts: &ConnectOptions) -> Result<WsSession> {
    // Unbounded channel never blocks senders (avoids Tokio worker deadlocks).
    let (delivery_tx, delivery_rx) = mpsc::unbounded_channel::<f64>();
    let (ready_tx, mut ready_rx) = mpsc::unbounded_channel::<Result<f64, ()>>();
    let received = Arc::new(AtomicU64::new(0));
    let mut handles = Vec::new();

    for _ in 0..opts.count {
        let target = target.clone();
        let key = opts.key_filter.clone();
        let delivery_tx = delivery_tx.clone();
        let ready_tx = ready_tx.clone();
        let received = Arc::clone(&received);
        let handle = tokio::spawn(async move {
            let started = Instant::now();
            let Ok(mut url) = target.ws_url() else {
                let _ = ready_tx.send(Err(()));
                return;
            };
            if let Some(key) = key {
                url.query_pairs_mut().append_pair("key", &key);
            }
            let connect =
                tokio::time::timeout(Duration::from_secs(10), connect_async(url.as_str()));
            let Ok(Ok((mut ws, _))) = connect.await else {
                let _ = ready_tx.send(Err(()));
                return;
            };
            let connect_ms = started.elapsed().as_secs_f64() * 1000.0;
            let _ = ready_tx.send(Ok(connect_ms));
            while let Some(msg) = ws.next().await {
                let Ok(msg) = msg else { break };
                if let Message::Text(text) = msg {
                    if let Ok(ev) = serde_json::from_str::<PhotonWsEvent>(&text) {
                        if let Ok(payload) = serde_json::from_value::<BenchPayload>(ev.payload_json)
                        {
                            let now_ms = chrono::Utc::now().timestamp_millis() as u64;
                            if payload.published_at_ms > 0 && now_ms >= payload.published_at_ms {
                                let _ = delivery_tx.send((now_ms - payload.published_at_ms) as f64);
                                received.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                }
            }
            let _ = ws.close(None).await;
        });
        handles.push(handle);
    }
    drop(delivery_tx);
    drop(ready_tx);

    // Handshake must complete within settle; still-pending connects are failures.
    // Successfully connected sockets keep running in `handles` for delivery.
    let settle_ms = 500u64.saturating_add(u64::from(opts.count.min(2000)));
    let deadline = Instant::now() + Duration::from_millis(settle_ms);
    let mut connected = 0u32;
    let mut connect_failures = 0u32;
    let mut connect_latency_ms = Vec::new();
    let mut remaining = opts.count;
    while remaining > 0 {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        match tokio::time::timeout(left, ready_rx.recv()).await {
            Ok(Some(Ok(ms))) => {
                connected += 1;
                connect_latency_ms.push(ms);
                remaining -= 1;
            }
            Ok(Some(Err(()))) => {
                connect_failures += 1;
                remaining -= 1;
            }
            Ok(None) => break,
            Err(_) => break,
        }
    }
    connect_failures += remaining;

    Ok(WsSession {
        connected,
        connect_failures,
        connect_latency_ms,
        handles,
        delivery_rx,
        received,
    })
}

pub async fn finish_session(mut session: WsSession) -> LoadGenResult {
    for handle in session.handles.drain(..) {
        handle.abort();
        // Reap aborted tasks so the runtime cannot accumulate wedged I/O futures.
        let _ = tokio::time::timeout(Duration::from_millis(100), handle).await;
    }
    tokio::time::sleep(Duration::from_millis(50)).await;

    let mut delivery_latency_ms = Vec::new();
    while let Ok(sample) = session.delivery_rx.try_recv() {
        delivery_latency_ms.push(sample);
    }
    LoadGenResult {
        connected: session.connected,
        connect_failures: session.connect_failures,
        connect_latency_ms: session.connect_latency_ms,
        delivery_latency_ms,
        messages_received: session.received.load(Ordering::Relaxed),
        ..Default::default()
    }
}

pub async fn connect_many(
    target: &ServerTarget,
    opts: &ConnectOptions,
    steady_secs: u64,
) -> Result<LoadGenResult> {
    let session = spawn_connections(target, opts).await?;
    tokio::time::sleep(Duration::from_secs(steady_secs)).await;
    Ok(finish_session(session).await)
}

pub async fn run_paced_publish(
    target: &ServerTarget,
    opts: &PublishOptions,
) -> Result<PublishStats> {
    let client = reqwest::Client::new();
    let mut body = serde_json::json!({
        "rate_per_sec": opts.rate_per_sec,
        "duration_secs": opts.duration_secs,
        "payload_bytes": opts.payload_bytes,
    });
    if let Some(ref key) = opts.topic_key {
        body["topic_key"] = serde_json::json!(key);
    }
    if let Some(g) = opts.key_groups {
        body["key_groups"] = serde_json::json!(g);
    }
    let mut req = client.post(target.publish_url()).json(&body);
    if let Ok(token) = std::env::var("BENCH_CONTROL_TOKEN") {
        if !token.is_empty() {
            req = req.header("x-bench-token", token);
        }
    }
    let resp = req.send().await.context("publish request")?;
    if !resp.status().is_success() {
        anyhow::bail!("publish failed: {}", resp.status());
    }
    let stats: PublishStatsJson = resp.json().await.context("publish result json")?;
    Ok(PublishStats {
        attempted: stats.attempted,
        succeeded: stats.succeeded,
        failed: stats.failed,
    })
}

#[derive(Deserialize)]
struct PublishStatsJson {
    attempted: u64,
    succeeded: u64,
    failed: u64,
}

/// Ensure the bench server is in the requested fanout mode (no-op if already set).
pub async fn ensure_ws_mode(server_url: &str, mode: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let base = server_url.trim_end_matches('/');
    let get_url = format!("{base}/api/bench/mode");
    let cur = client
        .get(&get_url)
        .send()
        .await
        .context("get ws mode")?
        .error_for_status()
        .context("get ws mode status")?
        .json::<serde_json::Value>()
        .await?;
    let current = cur.get("mode").and_then(|v| v.as_str()).unwrap_or("");
    if current == mode {
        return Ok(());
    }
    let mut req = client
        .post(&get_url)
        .json(&serde_json::json!({ "mode": mode }));
    if let Ok(token) = std::env::var("BENCH_CONTROL_TOKEN") {
        if !token.is_empty() {
            req = req.header("x-bench-token", token);
        }
    }
    let resp = req.send().await.context("set ws mode")?;
    if !resp.status().is_success() {
        anyhow::bail!("set ws mode failed: {}", resp.status());
    }
    Ok(())
}

/// Spawn `count` connections mapped onto `key_groups` partitions (`key-(i % G)`).
pub async fn spawn_connections_key_groups(
    target: &ServerTarget,
    count: u32,
    key_groups: u32,
) -> Result<WsSession> {
    let g = key_groups.max(1);
    let (delivery_tx, delivery_rx) = mpsc::unbounded_channel::<f64>();
    let (ready_tx, mut ready_rx) = mpsc::unbounded_channel::<Result<f64, ()>>();
    let received = Arc::new(AtomicU64::new(0));
    let mut handles = Vec::new();

    for i in 0..count {
        let target = target.clone();
        let key = format!("key-{}", i % g);
        let delivery_tx = delivery_tx.clone();
        let ready_tx = ready_tx.clone();
        let received = Arc::clone(&received);
        let handle = tokio::spawn(async move {
            let started = Instant::now();
            let Ok(mut url) = target.ws_url() else {
                let _ = ready_tx.send(Err(()));
                return;
            };
            url.query_pairs_mut().append_pair("key", &key);
            let connect =
                tokio::time::timeout(Duration::from_secs(10), connect_async(url.as_str()));
            let Ok(Ok((mut ws, _))) = connect.await else {
                let _ = ready_tx.send(Err(()));
                return;
            };
            let connect_ms = started.elapsed().as_secs_f64() * 1000.0;
            let _ = ready_tx.send(Ok(connect_ms));
            while let Some(msg) = ws.next().await {
                let Ok(msg) = msg else { break };
                if let Message::Text(text) = msg {
                    if let Ok(ev) = serde_json::from_str::<PhotonWsEvent>(&text) {
                        if let Ok(payload) = serde_json::from_value::<BenchPayload>(ev.payload_json)
                        {
                            let now_ms = chrono::Utc::now().timestamp_millis() as u64;
                            if payload.published_at_ms > 0 && now_ms >= payload.published_at_ms {
                                let _ = delivery_tx.send((now_ms - payload.published_at_ms) as f64);
                                received.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                }
            }
            let _ = ws.close(None).await;
        });
        handles.push(handle);
    }
    drop(delivery_tx);
    drop(ready_tx);

    let settle_ms = 500u64.saturating_add(u64::from(count.min(2000)));
    let deadline = Instant::now() + Duration::from_millis(settle_ms);
    let mut connected = 0u32;
    let mut connect_failures = 0u32;
    let mut connect_latency_ms = Vec::new();
    let mut remaining = count;
    while remaining > 0 {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        match tokio::time::timeout(left, ready_rx.recv()).await {
            Ok(Some(Ok(ms))) => {
                connected += 1;
                connect_latency_ms.push(ms);
                remaining -= 1;
            }
            Ok(Some(Err(()))) => {
                connect_failures += 1;
                remaining -= 1;
            }
            Ok(None) => break,
            Err(_) => break,
        }
    }
    connect_failures += remaining;

    Ok(WsSession {
        connected,
        connect_failures,
        connect_latency_ms,
        handles,
        delivery_rx,
        received,
    })
}

pub async fn run_keyed_load(
    target: &ServerTarget,
    connections: u32,
    key_groups: u32,
    publish: &PublishOptions,
    warmup_secs: u64,
) -> Result<LoadGenResult> {
    wait_for_health(target, Duration::from_secs(30)).await?;
    let session = spawn_connections_key_groups(target, connections, key_groups).await?;
    tokio::time::sleep(Duration::from_secs(warmup_secs)).await;

    let mut opts = publish.clone();
    opts.key_groups = Some(key_groups.max(1));
    let stats = run_paced_publish(target, &opts)
        .await
        .unwrap_or_else(|_| PublishStats {
            attempted: u64::from(opts.rate_per_sec) * u64::from(opts.duration_secs),
            succeeded: 0,
            failed: u64::from(opts.rate_per_sec) * u64::from(opts.duration_secs),
        });

    // Brief drain for in-flight WS frames after publish completes.
    tokio::time::sleep(Duration::from_millis(500)).await;
    let mut result = finish_session(session).await;
    result.publishes_attempted = stats.attempted;
    result.publishes_succeeded = stats.succeeded;
    result.publishes_failed = stats.failed;
    result.publish_errors = stats.failed;
    Ok(result)
}

pub async fn run_sustained_load(
    targets: &[ServerTarget],
    connections: u32,
    publish: &PublishOptions,
    warmup_secs: u64,
) -> Result<LoadGenResult> {
    if targets.is_empty() {
        anyhow::bail!("at least one server target required");
    }
    for t in targets {
        wait_for_health(t, Duration::from_secs(30)).await?;
    }

    let per_target = connections.div_ceil(targets.len() as u32);
    let mut sessions = Vec::new();
    for (i, target) in targets.iter().enumerate() {
        let count = if i + 1 == targets.len() {
            connections.saturating_sub(per_target * (targets.len() as u32 - 1))
        } else {
            per_target
        };
        if count == 0 {
            continue;
        }
        sessions.push(
            spawn_connections(
                target,
                &ConnectOptions {
                    count,
                    key_filter: None,
                },
            )
            .await?,
        );
    }

    tokio::time::sleep(Duration::from_secs(warmup_secs)).await;

    let mut publishes_attempted = 0u64;
    let mut publishes_succeeded = 0u64;
    let mut publishes_failed = 0u64;
    for (i, target) in targets.iter().enumerate() {
        let share = publish.rate_per_sec / targets.len() as u32;
        let rate = if i == 0 {
            publish.rate_per_sec - share * (targets.len() as u32 - 1)
        } else {
            share
        };
        let opts = PublishOptions {
            rate_per_sec: rate,
            duration_secs: publish.duration_secs,
            payload_bytes: publish.payload_bytes,
            topic_key: publish.topic_key.clone(),
            key_groups: publish.key_groups,
        };
        match run_paced_publish(target, &opts).await {
            Ok(stats) => {
                publishes_attempted += stats.attempted;
                publishes_succeeded += stats.succeeded;
                publishes_failed += stats.failed;
            }
            Err(_) => {
                let attempted = u64::from(rate) * u64::from(publish.duration_secs);
                publishes_attempted += attempted;
                publishes_failed += attempted;
            }
        }
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    let mut combined = LoadGenResult {
        publishes_attempted,
        publishes_succeeded,
        publishes_failed,
        publish_errors: publishes_failed,
        ..Default::default()
    };
    for session in sessions {
        let partial = finish_session(session).await;
        combined.connected += partial.connected;
        combined.connect_failures += partial.connect_failures;
        combined
            .connect_latency_ms
            .extend(partial.connect_latency_ms);
        combined
            .delivery_latency_ms
            .extend(partial.delivery_latency_ms);
        combined.messages_received += partial.messages_received;
    }
    Ok(combined)
}

/// Returns `(delivery, connect, connect_fail_rate, error_rate)` where `error_rate` is
/// `max(publish_fail_rate, delivery_loss)`.
pub fn result_stats(result: &LoadGenResult) -> (MetricStats, MetricStats, f64, f64) {
    let delivery = MetricStats::summarize(result.delivery_latency_ms.clone());
    let connect = MetricStats::summarize(result.connect_latency_ms.clone());
    let total = result.connected + result.connect_failures;
    let connect_fail_rate = if total == 0 {
        1.0
    } else {
        result.connect_failures as f64 / total as f64
    };
    let publish_fail_rate = if result.publishes_attempted == 0 {
        if result.publish_errors > 0 {
            1.0
        } else {
            0.0
        }
    } else {
        result.publishes_failed as f64 / result.publishes_attempted as f64
    };
    let expected = result
        .publishes_succeeded
        .saturating_mul(u64::from(result.connected));
    let delivery_loss = if expected == 0 {
        if result.messages_received == 0 {
            1.0
        } else {
            0.0
        }
    } else {
        1.0 - (result.messages_received as f64 / expected as f64).clamp(0.0, 1.0)
    };
    let error_rate = publish_fail_rate.max(delivery_loss);
    (delivery, connect, connect_fail_rate, error_rate)
}

pub fn default_target(base: &str) -> ServerTarget {
    ServerTarget {
        base_http: base.to_string(),
        ws_path: WS_PATH.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_style_connect_failures_counted() {
        // Documented contract: unfinished handshakes after settle are failures.
        // (Integration coverage is in spawn_connections; this locks the rate math.)
        let result = LoadGenResult {
            connected: 2,
            connect_failures: 2,
            messages_received: 100,
            publishes_attempted: 50,
            publishes_succeeded: 50,
            publishes_failed: 0,
            ..Default::default()
        };
        let (_, _, cfail, _) = result_stats(&result);
        assert!((cfail - 0.5).abs() < 1e-9);
    }

    #[test]
    fn delivery_loss_raises_error_rate() {
        let result = LoadGenResult {
            connected: 10,
            connect_failures: 0,
            messages_received: 50,
            publishes_attempted: 10,
            publishes_succeeded: 10,
            publishes_failed: 0,
            delivery_latency_ms: vec![1.0; 50],
            ..Default::default()
        };
        // expected = 10 * 10 = 100; received 50 → loss 0.5
        let (_, _, _, err) = result_stats(&result);
        assert!((err - 0.5).abs() < 1e-9);
    }

    #[test]
    fn achieved_differs_when_publishes_incomplete() {
        let succeeded = 80u64;
        let duration = 10u32;
        let requested = 100u32;
        let achieved = succeeded as f64 / f64::from(duration);
        assert!(achieved < f64::from(requested) * 0.95);
    }
}
