//! Axum WebSocket integration modules for Photon realtime routes.
//!
//! | Submodule | Purpose |
//! |-----------|---------|
//! | auth (private) | [`PhotonUserExtractor`], [`HeadlessWsAuth`] |
//! | descriptor (private) | [`WsRouteDescriptor`], [`WsAuthMode`] for inventory |
//! | [`key_resolve`] | Auth + client-key policy for Photon `key_filter` |
//! | routes (private) | [`apply_ws_routes`] auto-discovery |
//! | [`state`] | [`HasPhoton`] app-state trait |
//! | [`hub`] | Process-local broadcast fanout (`WsBroadcastHub`) |
//! | [`ws`] | `SyncedWsConfig`, `synced_ws_handler`, `WsFanoutMode` |
//! | [`ws_query`] | Parse `?key=` from the upgrade URI |

mod auth;
mod descriptor;
pub mod hub;
pub mod key_resolve;
mod routes;
pub mod state;
pub mod ws;
pub mod ws_query;

pub use auth::{HeadlessWsAuth, PhotonUserExtractor};
pub use descriptor::{WsAuthMode, WsRouteDescriptor};
pub use hub::WsBroadcastHub;
pub use key_resolve::{resolve_subscribe_key, KeyResolveError};
pub use routes::apply_ws_routes;
pub use state::HasPhoton;
pub use ws::{synced_ws_handler, FanoutConfigError, SyncedWsConfig, WsFanoutMode};
