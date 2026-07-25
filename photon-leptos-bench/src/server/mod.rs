//! Minimal Axum bench server (WS + publish API).

pub mod bind_guard;
pub mod boot;
pub mod routes;
pub mod state;

pub use bind_guard::{ensure_bench_bind_allowed, BENCH_ALLOW_NONLOCAL_ENV};
pub use boot::build_photon;
pub use routes::build_router;
pub use state::BenchState;
