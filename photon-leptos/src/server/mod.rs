//! Server-side WebSocket integration for Photon-backed Leptos apps.
//!
//! **Audience:** host integrators wiring Axum at application boot.
//!
//! This module re-exports [`photon_axum`] types so app crates depend on a single
//! `photon-leptos` facade for both client hooks and server registration.
//!
//! ## Boot sequence
//!
//! 1. Build and [`photon::configure`] a [`photon::Photon`] instance (see photon crate docs).
//! 2. Store `Arc<Photon>` on Axum app state via `photon_axum::HasPhoton`.
//! 3. Ensure synced server functions are linked (macro submits `WsRouteDescriptor` via inventory).
//! 4. Merge routes: `photon_axum::ws_router` or `photon_axum::apply_ws_routes`.
//!
//! ```rust,ignore
//! use photon_axum::{HeadlessWsAuth, HasPhoton, ws_router};
//!
//! app = ws_router::<AppState, HeadlessWsAuth>(app);
//! ```
//!
//! For `auth = "user"` synced routes, pass a host auth type that implements
//! `photon_axum::PhotonUserExtractor` and `axum::extract::FromRequestParts<AppState>`.
//!
//! ## Submodules
//!
//! - `server::ws` — low-level `synced_ws_handler` when registering routes manually

#[cfg(feature = "ssr")]
pub use photon_axum::{
    apply_ws_routes, HeadlessWsAuth, HasPhoton, PhotonUserExtractor, WsAuthMode, WsRouteDescriptor,
    ws_router,
};

#[cfg(feature = "ssr")]
pub mod ws {
    //! Manual WebSocket handler utilities (bypassing inventory auto-discovery).
    pub use photon_axum::{synced_ws_handler, SyncedWsConfig};
}
