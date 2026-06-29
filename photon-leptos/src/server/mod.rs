//! Server-side WebSocket handler for Photon event forwarding.

#[cfg(feature = "ssr")]
pub use photon_axum::{
    apply_ws_routes, HeadlessWsAuth, HasPhoton, PhotonUserExtractor, WsAuthMode, WsRouteDescriptor,
    ws_router,
};

#[cfg(feature = "ssr")]
pub mod ws {
    pub use photon_axum::{synced_ws_handler, SyncedWsConfig};
}
