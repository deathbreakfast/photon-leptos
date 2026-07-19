//! Minimal Axum bench server (WS + publish API).

pub mod boot;
pub mod routes;
pub mod state;

pub use boot::build_photon;
pub use routes::build_router;
pub use state::BenchState;
