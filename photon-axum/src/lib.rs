//! Axum WebSocket registration for realtime topic streams.
//!
//! [`ws_router`] applies inventory-discovered routes (from `#[photon::synced]` in product hosts).
//! Typed topic streams can also be bridged manually via [`photon::Photon::subscribe`].

#![cfg(feature = "ssr")]

pub mod axum_ws;

pub use axum_ws::{
    apply_ws_routes, HeadlessWsAuth, HasPhoton, PhotonUserExtractor, WsAuthMode, WsRouteDescriptor,
};
pub use axum_ws::ws::{synced_ws_handler, SyncedWsConfig};

use axum::Router;

/// Apply Photon WebSocket routes to `app`.
pub fn ws_router<S, Auth>(app: Router<S>) -> Router<S>
where
    S: axum_ws::HasPhoton + Clone + Send + Sync + 'static,
    Auth: axum_ws::PhotonUserExtractor + axum::extract::FromRequestParts<S> + Send + 'static,
    <Auth as axum::extract::FromRequestParts<S>>::Rejection: axum::response::IntoResponse + Send,
{
    axum_ws::apply_ws_routes::<S, Auth>(app)
}
