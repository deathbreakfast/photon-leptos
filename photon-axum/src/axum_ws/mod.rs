//! Axum WebSocket registration for realtime resource routes (SSR).
//!
//! Headless hosts register WS routes without pulling a UI framework.

mod auth;
mod descriptor;
mod routes;
pub mod state;
pub mod ws;

pub use auth::{HeadlessWsAuth, PhotonUserExtractor};
pub use descriptor::{WsAuthMode, WsRouteDescriptor};
pub use routes::apply_ws_routes;
pub use state::HasPhoton;
