//! WebSocket endpoint that subscribes to Photon and forwards events to clients.
//!
//! Each message is a JSON-serialized Photon [`photon_backend::Event`] envelope.
//! Clients parse `payload_json` (see photon-leptos client helpers).
//!
//! ## Manual registration
//!
//! When not using inventory auto-discovery, register a handler directly:
//!
//! ```rust,ignore
//! use axum::{extract::ws::WebSocketUpgrade, routing::get, Router};
//! use photon_axum::{synced_ws_handler, SyncedWsConfig, HasPhoton};
//! use std::sync::Arc;
//!
//! async fn notifications_ws(
//!     ws: WebSocketUpgrade,
//!     State(state): State<AppState>,
//! ) -> axum::response::Response {
//!     let config = SyncedWsConfig {
//!         topic: "notifications.updated".into(),
//!         key_filter: None,
//!         subscription_name: None,
//!     };
//!     synced_ws_handler(ws, state.photon_arc(), config).await
//! }
//!
//! let app = Router::new().route("/ws/notifications", get(notifications_ws));
//! ```

use std::sync::Arc;

use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use futures::{SinkExt, StreamExt};

use photon_backend::instrumentation::log_ops;
use photon_runtime::Photon;

/// Configuration for a WebSocket endpoint that forwards Photon events.
#[derive(Clone, Debug)]
pub struct SyncedWsConfig {
    /// Photon topic name (e.g. `"user.notifications"`).
    pub topic: String,

    /// Optional key filter (e.g. user_id) for scoping events to a specific key.
    pub key_filter: Option<String>,

    /// Optional subscription name for ephemeral subscriptions.
    pub subscription_name: Option<String>,
}

/// Upgrade handler: subscribe to `config.topic` and forward serialized events to the client.
pub async fn synced_ws_handler(
    ws: WebSocketUpgrade,
    photon: Arc<Photon>,
    config: SyncedWsConfig,
) -> axum::response::Response {
    ws.on_upgrade(move |socket| handle_socket(socket, photon, config))
}

async fn handle_socket(mut socket: WebSocket, photon: Arc<Photon>, config: SyncedWsConfig) {
    let key_filter = config.key_filter.clone();
    let topic = config.topic.clone();

    log_ops("axum_ws", "connect", "client connected", &topic, "", "");

    let mut stream = photon.subscribe(&topic, key_filter.as_deref(), None);

    while let Some(ev) = stream.next().await {
        match ev {
            Ok(event) => match serde_json::to_string(&event) {
                Ok(json) => {
                    if socket
                        .send(axum::extract::ws::Message::Text(json.into()))
                        .await
                        .is_err()
                    {
                        log_ops(
                            "axum_ws",
                            "disconnect",
                            "send failed (client gone)",
                            &topic,
                            "",
                            "",
                        );
                        break;
                    }
                }
                Err(e) => {
                    log_ops(
                        "axum_ws",
                        "serialize_error",
                        "failed to serialize event for ws",
                        &topic,
                        "",
                        &e.to_string(),
                    );
                }
            },
            Err(e) => {
                log_ops(
                    "axum_ws",
                    "subscription_error",
                    "photon subscription error",
                    &topic,
                    "",
                    &e.to_string(),
                );
                break;
            }
        }
    }

    log_ops(
        "axum_ws",
        "disconnect",
        "client disconnected",
        &topic,
        "",
        "",
    );

    let _ = SinkExt::close(&mut socket).await;
}
