//! Server-side WebSocket integration for Photon-backed Leptos apps.
//!
//! Re-exports [`photon_axum`] types so app crates depend on a single `photon-leptos`
//! surface for both client hooks and server registration. Primary teaching for boot
//! and user-auth lives on the crate root ([Mount WS routes](crate#boot-ws-routes),
//! [User-auth WebSocket](crate#user-auth-ws)); this module holds the re-export surface
//! and a short boot checklist.
//!
//! Mounting inventory routes happens once at host boot: build Photon, store it on
//! app state via [`photon_axum::HasPhoton`], then call [`photon_axum::ws_router`] so
//! browsers can open the paths submitted by `#[synced]`.
//!
//! ## Boot sequence
//!
//! 1. Build and [`photon::configure`] a [`photon::Photon`] instance (see photon crate docs).
//! 2. Store `Arc<Photon>` on Axum app state via [`photon_axum::HasPhoton`].
//! 3. Ensure synced server functions are linked (macro submits `WsRouteDescriptor` via inventory).
//! 4. Merge routes: [`photon_axum::ws_router`] or [`photon_axum::apply_ws_routes`].
//!
//! ```rust,ignore
//! use photon_axum::{HeadlessWsAuth, HasPhoton, ws_router};
//!
//! // At host boot, after Router assembly:
//! let app = ws_router::<AppState, HeadlessWsAuth>(app);
//! ```
//!
//! For `auth = "user"` synced routes, pass a host auth type that implements
//! [`photon_axum::PhotonUserExtractor`] and `axum::extract::FromRequestParts<AppState>` into
//! `ws_router::<S, Auth>`. When the request upgrades to WebSocket, the extractor
//! supplies the user id for key policy. Implement the trait on your session or auth
//! newtype — no product-specific auth crate required.
//!
//! ## Submodules
//!
//! - `server::ws` — low-level [`photon_axum::synced_ws_handler`] when registering routes manually

#[cfg(feature = "ssr")]
pub use photon_axum::{
    apply_ws_routes, origin_from_headers, reject_origin, resolve_subscribe_key, synced_ws_handler,
    ws_router, HasPhoton, HeadlessWsAuth, KeyResolveError, PhotonUserExtractor, SyncedWsConfig,
    WsAuthMode, WsBroadcastHub, WsFanoutMode, WsRouteDescriptor,
};

#[cfg(feature = "ssr")]
pub use photon_axum::axum_ws::ws_query::client_key_from_uri;

#[cfg(feature = "ssr")]
pub mod ws {
    //! Manual WebSocket handler utilities (bypassing inventory auto-discovery).
    pub use photon_axum::{synced_ws_handler, SyncedWsConfig};
}
