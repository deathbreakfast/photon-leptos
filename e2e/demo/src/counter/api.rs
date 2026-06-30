//! Test-only Axum routes and Photon topic publish handlers.

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use photon::topic;
use serde::Deserialize;

use crate::state::AppState;

#[topic(name = "counter.updated")]
pub struct CounterUpdated {
    pub namespace: String,
}

#[topic(name = "counter.auth.updated", keyed_by = "partition")]
pub struct CounterAuthUpdated {
    pub namespace: String,
    pub partition: String,
}

#[derive(Deserialize)]
pub struct NamespaceBody {
    pub namespace: String,
}

#[derive(Deserialize)]
pub struct ScenarioBody {
    pub namespace: String,
    #[serde(default)]
    pub fail_read: Option<bool>,
    #[serde(default)]
    pub fail_publish: Option<bool>,
}

pub fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/api/counter/increment", post(increment))
        .route("/api/counter/increment-auth", post(increment_auth))
        .route("/api/counter/reset", post(reset))
        .route("/api/e2e/scenario", post(set_scenario))
}

async fn increment(
    State(state): State<AppState>,
    Json(body): Json<NamespaceBody>,
) -> Result<StatusCode, StatusCode> {
    let flags = state.store.flags(&body.namespace);
    if flags.fail_publish {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    state.store.increment(&body.namespace);
    CounterUpdated {
        namespace: body.namespace,
    }
    .publish()
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn increment_auth(Json(body): Json<NamespaceBody>) -> Result<StatusCode, StatusCode> {
    CounterAuthUpdated {
        namespace: body.namespace,
        partition: "secret-partition".to_string(),
    }
    .publish()
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn reset(
    State(state): State<AppState>,
    Json(body): Json<NamespaceBody>,
) -> StatusCode {
    state.store.reset(&body.namespace);
    StatusCode::NO_CONTENT
}

async fn set_scenario(
    State(state): State<AppState>,
    Json(body): Json<ScenarioBody>,
) -> StatusCode {
    state.store.set_scenario(&body.namespace, body.fail_read, body.fail_publish);
    StatusCode::NO_CONTENT
}
