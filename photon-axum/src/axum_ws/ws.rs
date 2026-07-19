//! WebSocket endpoint that subscribes to Photon and forwards events to clients.
//!
//! Each message is a JSON-serialized Photon [`photon_backend::Event`] envelope.
//! Clients parse `payload_json` (see photon-leptos client helpers).
//!
//! ## Fanout modes
//!
//! - [`WsFanoutMode::PerSubscribe`] (default): one `photon.subscribe` + serialize
//!   per socket (legacy path).
//! - [`WsFanoutMode::BroadcastHub`]: shared subscribe + serialize per
//!   `(topic, key_filter)` via [`super::hub::WsBroadcastHub`].
//!
//! Mode defaults to [`WsFanoutMode::from_env`] (`PHOTON_AXUM_WS_FANOUT`), which
//! returns [`FanoutConfigError`] for unknown values. Prefer [`SyncedWsConfig::try_new`]
//! at boot; [`SyncedWsConfig::new`] panics on invalid env.
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
//!     let config = SyncedWsConfig::new("notifications.updated", None);
//!     synced_ws_handler(ws, state.photon_arc(), state.ws_hub(), config).await
//! }
//!
//! let app = Router::new().route("/ws/notifications", get(notifications_ws));
//! ```
//!
//! This bridge is **fire-and-forget** (live-tail only). Durable named Photon
//! subscriptions belong in backend workers, not browser WebSocket routes.

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};

use photon_backend::instrumentation::log_ops;
use photon_runtime::Photon;

use super::hub::WsBroadcastHub;

/// How WebSocket handlers obtain and share Photon subscribe pipelines.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum WsFanoutMode {
    /// One `photon.subscribe` + JSON serialize per socket (default).
    #[default]
    PerSubscribe,
    /// Shared subscribe + serialize per `(topic, key_filter)` via [`WsBroadcastHub`].
    BroadcastHub,
}

impl WsFanoutMode {
    /// Resolve mode from `PHOTON_AXUM_WS_FANOUT` (`per_subscribe` | `broadcast_hub`).
    ///
    /// Unset or empty → [`WsFanoutMode::PerSubscribe`].
    /// Unknown values → [`Err`].
    pub fn from_env() -> Result<Self, FanoutConfigError> {
        match std::env::var("PHOTON_AXUM_WS_FANOUT") {
            Err(_) => Ok(Self::PerSubscribe),
            Ok(raw) => {
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    return Ok(Self::PerSubscribe);
                }
                Self::parse(trimmed).ok_or(FanoutConfigError::UnknownEnvValue(raw))
            }
        }
    }

    /// Parse a CLI / config string (`per_subscribe` | `broadcast_hub`).
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "per_subscribe" | "per-subscribe" => Some(Self::PerSubscribe),
            "broadcast_hub" | "hub" => Some(Self::BroadcastHub),
            _ => None,
        }
    }

    /// Stable slug for reports and logs.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PerSubscribe => "per_subscribe",
            Self::BroadcastHub => "broadcast_hub",
        }
    }
}

/// Invalid WebSocket fanout configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FanoutConfigError {
    /// `PHOTON_AXUM_WS_FANOUT` was set to an unrecognized value.
    UnknownEnvValue(String),
    /// [`WsFanoutMode::BroadcastHub`] was requested but no hub is on app state.
    HubRequiredButMissing,
}

impl std::fmt::Display for FanoutConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownEnvValue(v) => write!(
                f,
                "invalid PHOTON_AXUM_WS_FANOUT={v:?}; expected per_subscribe or broadcast_hub"
            ),
            Self::HubRequiredButMissing => write!(
                f,
                "PHOTON_AXUM_WS_FANOUT=broadcast_hub requires HasPhoton::ws_hub() to return Some"
            ),
        }
    }
}

impl std::error::Error for FanoutConfigError {}

/// Configuration for a WebSocket endpoint that forwards Photon events.
///
/// Browser clients use ephemeral `Photon::subscribe(topic, key_filter, None)`.
#[derive(Clone, Debug)]
pub struct SyncedWsConfig {
    /// Photon topic name (e.g. `"user.notifications"`).
    pub topic: String,

    /// Optional key filter (e.g. user_id) for scoping events to a specific key.
    pub key_filter: Option<String>,

    /// Fanout strategy for this connection.
    pub fanout: WsFanoutMode,
}

impl SyncedWsConfig {
    /// Build config with fanout from [`WsFanoutMode::from_env`].
    ///
    /// # Errors
    ///
    /// Returns [`FanoutConfigError`] when the environment value is invalid.
    pub fn try_new(
        topic: impl Into<String>,
        key_filter: Option<String>,
    ) -> Result<Self, FanoutConfigError> {
        Ok(Self {
            topic: topic.into(),
            key_filter,
            fanout: WsFanoutMode::from_env()?,
        })
    }

    /// Build config with fanout from [`WsFanoutMode::from_env`].
    ///
    /// # Panics
    ///
    /// Panics if `PHOTON_AXUM_WS_FANOUT` is set to an invalid value. Prefer
    /// [`Self::try_new`] in production boot paths.
    #[must_use]
    pub fn new(topic: impl Into<String>, key_filter: Option<String>) -> Self {
        Self::try_new(topic, key_filter).unwrap_or_else(|e| {
            panic!("SyncedWsConfig::new: {e}");
        })
    }

    /// Builder: set fanout mode.
    #[must_use]
    pub fn with_fanout(mut self, fanout: WsFanoutMode) -> Self {
        self.fanout = fanout;
        self
    }
}

/// Upgrade handler: subscribe to `config.topic` and forward serialized events.
///
/// When `config.fanout` is [`WsFanoutMode::BroadcastHub`], `hub` must be `Some`
/// or the connection is closed without falling back to per-subscribe.
pub async fn synced_ws_handler(
    ws: WebSocketUpgrade,
    photon: Arc<Photon>,
    hub: Option<Arc<WsBroadcastHub>>,
    config: SyncedWsConfig,
) -> axum::response::Response {
    if matches!(config.fanout, WsFanoutMode::BroadcastHub) && hub.is_none() {
        let msg = FanoutConfigError::HubRequiredButMissing.to_string();
        log_ops("axum_ws", "hub_missing", &msg, &config.topic, "", "");
        return (axum::http::StatusCode::SERVICE_UNAVAILABLE, msg).into_response();
    }
    ws.on_upgrade(move |socket| handle_socket(socket, photon, hub, config))
}

async fn handle_socket(
    socket: WebSocket,
    photon: Arc<Photon>,
    hub: Option<Arc<WsBroadcastHub>>,
    config: SyncedWsConfig,
) {
    let key_filter = config.key_filter.clone();
    let topic = config.topic.clone();

    log_ops("axum_ws", "connect", "client connected", &topic, "", "");

    if matches!(config.fanout, WsFanoutMode::BroadcastHub) {
        let Some(hub) = hub else {
            // Checked in synced_ws_handler; defensive.
            log_ops(
                "axum_ws",
                "hub_missing",
                "broadcast_hub requested but no hub; closing",
                &topic,
                "",
                "",
            );
            return;
        };
        handle_socket_hub(socket, photon, hub, topic, key_filter).await;
        return;
    }

    let (mut sink, mut client_rx) = socket.split();
    let mut stream = photon.subscribe(&topic, key_filter.as_deref(), None);

    loop {
        tokio::select! {
            ev = stream.next() => {
                match ev {
                    Some(Ok(event)) => match serde_json::to_string(&event) {
                        Ok(json) => {
                            if sink.send(Message::Text(json.into())).await.is_err() {
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
                    Some(Err(e)) => {
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
                    None => break,
                }
            }
            msg = client_rx.next() => {
                match msg {
                    None | Some(Ok(Message::Close(_)) | Err(_)) => {
                        log_ops(
                            "axum_ws",
                            "disconnect",
                            "client read closed",
                            &topic,
                            "",
                            "",
                        );
                        break;
                    }
                    Some(Ok(_)) => {}
                }
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

    let _ = SinkExt::close(&mut sink).await;
}

async fn handle_socket_hub(
    socket: WebSocket,
    photon: Arc<Photon>,
    hub: Arc<WsBroadcastHub>,
    topic: String,
    key_filter: Option<String>,
) {
    let (mut sink, mut client_rx) = socket.split();
    let mut sub = hub.join(photon, topic.clone(), key_filter);

    loop {
        tokio::select! {
            json = sub.rx.recv() => {
                match json {
                    Some(json) => {
                        if sink
                            .send(Message::Text(json.as_ref().to_owned().into()))
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
                    None => break,
                }
            }
            msg = client_rx.next() => {
                match msg {
                    None | Some(Ok(Message::Close(_)) | Err(_)) => {
                        log_ops(
                            "axum_ws",
                            "disconnect",
                            "client read closed",
                            &topic,
                            "",
                            "",
                        );
                        break;
                    }
                    Some(Ok(_)) => {}
                }
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

    let _ = SinkExt::close(&mut sink).await;
}
