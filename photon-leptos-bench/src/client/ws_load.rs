//! tokio-tungstenite load generator for BM-PLS*.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use futures::StreamExt;
use serde::Deserialize;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;

use crate::stats::MetricStats;
use crate::{DEFAULT_PAYLOAD_BYTES, WS_PATH};

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
}

#[derive(Debug, Clone, Default)]
pub struct LoadGenResult {
    pub connected: u32,
    pub connect_failures: u32,
    pub connect_latency_ms: Vec<f64>,
    pub delivery_latency_ms: Vec<f64>,
    pub messages_received: u64,
    pub publish_errors: u64,
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
    handles: Vec<JoinHandle<Option<f64>>>,
    delivery: Arc<Mutex<Vec<f64>>>,
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
    let delivery = Arc::new(Mutex::new(Vec::<f64>::new()));
    let received = Arc::new(AtomicU64::new(0));
    let mut handles = Vec::new();
    let mut connected = 0u32;
    let mut connect_failures = 0u32;
    let mut connect_latency_ms = Vec::new();

    for _ in 0..opts.count {
        let target = target.clone();
        let key = opts.key_filter.clone();
        let delivery = Arc::clone(&delivery);
        let received = Arc::clone(&received);
        let handle = tokio::spawn(async move {
            let started = Instant::now();
            let mut url = match target.ws_url() {
                Ok(u) => u,
                Err(_) => return None,
            };
            if let Some(key) = key {
                url.query_pairs_mut().append_pair("key", &key);
            }
            let Ok((mut ws, _)) = connect_async(url.as_str()).await else {
                return None;
            };
            let connect_ms = started.elapsed().as_secs_f64() * 1000.0;
            while let Some(msg) = ws.next().await {
                let Ok(msg) = msg else { break };
                if let Message::Text(text) = msg {
                    if let Ok(ev) = serde_json::from_str::<PhotonWsEvent>(&text) {
                        if let Ok(payload) =
                            serde_json::from_value::<BenchPayload>(ev.payload_json)
                        {
                            let now_ms = chrono::Utc::now().timestamp_millis() as u64;
                            if payload.published_at_ms > 0 && now_ms >= payload.published_at_ms {
                                delivery
                                    .lock()
                                    .await
                                    .push((now_ms - payload.published_at_ms) as f64);
                                received.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                }
            }
            let _ = ws.close(None).await;
            Some(connect_ms)
        });
        handles.push(handle);
    }

    tokio::time::sleep(Duration::from_millis(500)).await;
    let mut live_handles = Vec::new();
    for handle in handles {
        if handle.is_finished() {
            match handle.await {
                Ok(Some(ms)) => {
                    connected += 1;
                    connect_latency_ms.push(ms);
                }
                _ => connect_failures += 1,
            }
        } else {
            connected += 1;
            live_handles.push(handle);
        }
    }

    Ok(WsSession {
        connected,
        connect_failures,
        connect_latency_ms,
        handles: live_handles,
        delivery,
        received,
    })
}

pub async fn finish_session(mut session: WsSession) -> LoadGenResult {
    for handle in session.handles.drain(..) {
        handle.abort();
    }
    LoadGenResult {
        connected: session.connected,
        connect_failures: session.connect_failures,
        connect_latency_ms: session.connect_latency_ms,
        delivery_latency_ms: session.delivery.lock().await.clone(),
        messages_received: session.received.load(Ordering::Relaxed),
        publish_errors: 0,
    }
}

pub async fn connect_many(
    target: &ServerTarget,
    opts: &ConnectOptions,
    steady_secs: u64,
) -> Result<LoadGenResult> {
    let session = spawn_connections(target, opts).await?;
    tokio::time::sleep(Duration::from_secs(steady_secs)).await;
    let mut result = finish_session(session).await;
    result.publish_errors = 0;
    Ok(result)
}

pub async fn run_paced_publish(target: &ServerTarget, opts: &PublishOptions) -> Result<()> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "rate_per_sec": opts.rate_per_sec,
        "duration_secs": opts.duration_secs,
        "payload_bytes": opts.payload_bytes,
    });
    let resp = client
        .post(target.publish_url())
        .json(&body)
        .send()
        .await
        .context("publish request")?;
    if !resp.status().is_success() {
        anyhow::bail!("publish failed: {}", resp.status());
    }
    Ok(())
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

    let mut publish_errors = 0u64;
    for (i, target) in targets.iter().enumerate() {
        let share = publish.rate_per_sec / targets.len() as u32;
        let rate = if i == 0 {
            publish.rate_per_sec - share * (targets.len() as u32 - 1)
        } else {
            share
        };
        if run_paced_publish(
            target,
            &PublishOptions {
                rate_per_sec: rate,
                duration_secs: publish.duration_secs,
                payload_bytes: publish.payload_bytes,
            },
        )
        .await
        .is_err()
        {
            publish_errors += 1;
        }
    }

    tokio::time::sleep(Duration::from_secs(publish.duration_secs as u64)).await;

    let mut combined = LoadGenResult {
        publish_errors,
        ..Default::default()
    };
    for session in sessions {
        let partial = finish_session(session).await;
        combined.connected += partial.connected;
        combined.connect_failures += partial.connect_failures;
        combined.connect_latency_ms.extend(partial.connect_latency_ms);
        combined.delivery_latency_ms.extend(partial.delivery_latency_ms);
        combined.messages_received += partial.messages_received;
    }
    Ok(combined)
}

pub fn result_stats(result: &LoadGenResult) -> (MetricStats, MetricStats, f64, f64) {
    let delivery = MetricStats::summarize(result.delivery_latency_ms.clone());
    let connect = MetricStats::summarize(result.connect_latency_ms.clone());
    let total = result.connected + result.connect_failures;
    let connect_fail_rate = if total == 0 {
        1.0
    } else {
        result.connect_failures as f64 / total as f64
    };
    let error_rate = if result.messages_received == 0 {
        1.0
    } else {
        result.publish_errors as f64 / result.messages_received as f64
    };
    (delivery, connect, connect_fail_rate, error_rate)
}

pub fn default_target(base: &str) -> ServerTarget {
    ServerTarget {
        base_http: base.to_string(),
        ws_path: WS_PATH.to_string(),
    }
}

pub fn payload_default() -> usize {
    DEFAULT_PAYLOAD_BYTES
}
