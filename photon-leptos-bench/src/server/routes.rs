//! Bench HTTP + WebSocket routes.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use axum::extract::{Query, State, WebSocketUpgrade};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use photon::topic;
use photon_axum::{synced_ws_handler, SyncedWsConfig, WsFanoutMode};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

use crate::server::BenchState;
use crate::BENCH_TOPIC;

/// Hard caps for control-plane publish requests (SEC-005).
const MAX_RATE_PER_SEC: u32 = 50_000;
const MAX_DURATION_SECS: u32 = 300;
const MAX_PAYLOAD_BYTES: usize = 65_536;
const MAX_KEY_GROUPS: u32 = 10_000;

static PUBLISH_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

#[topic(name = "bench.event", keyed_by = "partition")]
pub struct BenchEvent {
    pub partition: String,
    pub seq: u64,
    pub published_at_ms: u64,
    pub payload: String,
}

#[derive(Deserialize)]
pub struct PublishBody {
    #[serde(default = "default_rate")]
    pub rate_per_sec: u32,
    #[serde(default = "default_duration")]
    pub duration_secs: u32,
    #[serde(default = "default_payload_bytes")]
    pub payload_bytes: usize,
    /// Single partition key for every event in this publish burst.
    #[serde(default)]
    pub topic_key: Option<String>,
    /// When set, round-robin `key-0` .. `key-(G-1)` across the burst.
    #[serde(default)]
    pub key_groups: Option<u32>,
}

fn default_rate() -> u32 {
    100
}

fn default_duration() -> u32 {
    60
}

fn default_payload_bytes() -> usize {
    crate::DEFAULT_PAYLOAD_BYTES
}

#[derive(Deserialize)]
pub struct WsQuery {
    pub key: Option<String>,
}

#[derive(Deserialize)]
pub struct ModeBody {
    pub mode: String,
}

#[derive(Serialize)]
pub struct ModeResponse {
    pub mode: String,
}

fn control_authorized(headers: &HeaderMap) -> bool {
    let expected = match std::env::var("BENCH_CONTROL_TOKEN") {
        Ok(t) if !t.is_empty() => t,
        _ => {
            // No token configured: allow only when explicitly opted into open mode.
            return std::env::var("BENCH_CONTROL_OPEN")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false);
        }
    };
    headers
        .get("x-bench-token")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|got| got == expected)
}

fn require_control(headers: &HeaderMap) -> Result<(), StatusCode> {
    if control_authorized(headers) {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

pub fn build_router(state: BenchState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/bench/publish", post(publish))
        .route("/api/bench/value", get(bench_value))
        .route("/api/bench/mode", get(get_mode).post(set_mode))
        .route("/ws/bench", get(ws_bench))
        .with_state(state)
}

async fn health(State(state): State<BenchState>) -> impl IntoResponse {
    (
        StatusCode::OK,
        format!("ok mode={}", state.fanout_mode().as_str()),
    )
}

async fn get_mode(State(state): State<BenchState>) -> Json<ModeResponse> {
    Json(ModeResponse {
        mode: state.fanout_mode().as_str().to_string(),
    })
}

async fn set_mode(
    State(state): State<BenchState>,
    headers: HeaderMap,
    Json(body): Json<ModeBody>,
) -> Result<Json<ModeResponse>, StatusCode> {
    require_control(&headers)?;
    let mode = WsFanoutMode::parse(&body.mode).ok_or(StatusCode::BAD_REQUEST)?;
    state.set_fanout_mode(mode);
    Ok(Json(ModeResponse {
        mode: mode.as_str().to_string(),
    }))
}

async fn bench_value() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "ok": true, "ts": chrono::Utc::now().timestamp_millis() }))
}

async fn ws_bench(
    ws: WebSocketUpgrade,
    State(state): State<BenchState>,
    Query(query): Query<WsQuery>,
) -> impl IntoResponse {
    let config = SyncedWsConfig {
        topic: BENCH_TOPIC.to_string(),
        key_filter: query.key,
        fanout: state.fanout_mode(),
    };
    synced_ws_handler(ws, state.photon.clone(), Some(state.hub.clone()), config).await
}

async fn publish(
    State(_state): State<BenchState>,
    headers: HeaderMap,
    Json(body): Json<PublishBody>,
) -> Result<Json<PublishResult>, StatusCode> {
    require_control(&headers)?;

    if body.rate_per_sec > MAX_RATE_PER_SEC
        || body.duration_secs > MAX_DURATION_SECS
        || body.payload_bytes > MAX_PAYLOAD_BYTES
        || body.key_groups.is_some_and(|g| g > MAX_KEY_GROUPS)
    {
        return Err(StatusCode::BAD_REQUEST);
    }

    if PUBLISH_IN_FLIGHT
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err(StatusCode::CONFLICT);
    }

    let result = run_publish(body).await;
    PUBLISH_IN_FLIGHT.store(false, Ordering::SeqCst);
    result
}

async fn run_publish(body: PublishBody) -> Result<Json<PublishResult>, StatusCode> {
    let payload = "x".repeat(body.payload_bytes.max(1));
    let interval = if body.rate_per_sec == 0 {
        Duration::from_secs(3600)
    } else {
        Duration::from_secs_f64(1.0 / body.rate_per_sec as f64)
    };
    let total = body.rate_per_sec as u64 * body.duration_secs as u64;
    let topic_key = body.topic_key.clone();
    let key_groups = body.key_groups.filter(|&g| g > 0);

    let mut succeeded = 0u64;
    let mut failed = 0u64;
    for seq in 0..total {
        let published_at_ms = chrono::Utc::now().timestamp_millis() as u64;
        let partition = if let Some(g) = key_groups {
            format!("key-{}", seq % u64::from(g))
        } else if let Some(ref k) = topic_key {
            k.clone()
        } else {
            "_".into()
        };
        let ev = BenchEvent {
            partition,
            seq,
            published_at_ms,
            payload: payload.clone(),
        };
        match ev.publish().await {
            Ok(_) => succeeded += 1,
            Err(_) => {
                failed += 1;
                break;
            }
        }
        if seq + 1 < total {
            sleep(interval).await;
        }
    }

    Ok(Json(PublishResult {
        attempted: total,
        succeeded,
        failed,
    }))
}

#[derive(Serialize)]
pub struct PublishResult {
    pub attempted: u64,
    pub succeeded: u64,
    pub failed: u64,
}
