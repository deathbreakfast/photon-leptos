//! Counter synced reads and test-only HTTP triggers.

#[cfg(feature = "ssr")]
mod api;

mod fns;

#[cfg(feature = "ssr")]
pub use api::api_routes;

pub use fns::{counter_get, counter_get_auth_user};

pub use fns::{subscribe_counter_get, subscribe_counter_get_auth_user};
