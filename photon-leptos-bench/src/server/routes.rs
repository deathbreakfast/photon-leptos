//! Bench HTTP + WebSocket routes.

use std::time::Duration;

use axum::extract::{Query, State, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use photon::topic;
use photon_axum::{synced_ws_handler, SyncedWsConfig};
use serde::Deserialize;
use tokio::time::sleep;

use crate::server::BenchState;
use crate::BENCH_TOPIC;

#[topic(name = "bench.event")]
pub struct BenchEvent {
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
    #[serde(default)]
    pub topic_key: Option<String>,
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

pub fn build_router(state: BenchState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/bench/publish", post(publish))
        .route("/api/bench/value", get(bench_value))
        .route("/ws/bench", get(ws_bench))
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok")
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
        subscription_name: None,
    };
    synced_ws_handler(ws, state.photon, config).await
}

async fn publish(
    State(_state): State<BenchState>,
    Json(body): Json<PublishBody>,
) -> Result<StatusCode, StatusCode> {
    let payload = "x".repeat(body.payload_bytes.max(1));
    let interval = if body.rate_per_sec == 0 {
        Duration::from_secs(3600)
    } else {
        Duration::from_secs_f64(1.0 / body.rate_per_sec as f64)
    };
    let total = body.rate_per_sec as u64 * body.duration_secs as u64;
    tokio::spawn(async move {
        for seq in 0..total {
            let published_at_ms = chrono::Utc::now().timestamp_millis() as u64;
            let ev = BenchEvent {
                seq,
                published_at_ms,
                payload: payload.clone(),
            };
            if ev.publish().await.is_err() {
                break;
            }
            sleep(interval).await;
        }
    });
    Ok(StatusCode::ACCEPTED)
}
