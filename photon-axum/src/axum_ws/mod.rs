//! Axum WebSocket integration modules for Photon realtime routes.
//!
//! **Audience:** host integrators.
//!
//! | Submodule | Purpose |
//! |-----------|---------|
//! | auth (private) | [`PhotonUserExtractor`], [`HeadlessWsAuth`] |
//! | descriptor (private) | [`WsRouteDescriptor`], [`WsAuthMode`] for inventory |
//! | routes (private) | [`apply_ws_routes`] auto-discovery |
//! | [`state`] | [`HasPhoton`] app-state trait |
//! | [`ws`] | `SyncedWsConfig`, `synced_ws_handler` |

mod auth;
mod descriptor;
mod routes;
pub mod state;
pub mod ws;

pub use auth::{HeadlessWsAuth, PhotonUserExtractor};
pub use descriptor::{WsAuthMode, WsRouteDescriptor};
pub use routes::apply_ws_routes;
pub use state::HasPhoton;
