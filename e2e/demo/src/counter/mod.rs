//! Counter synced reads and test-only HTTP triggers.

#[cfg(feature = "ssr")]
mod api;

mod context;
mod fns;

#[cfg(feature = "ssr")]
pub use api::api_routes;

pub use context::E2ePartition;
pub use fns::{
    counter_get, counter_get_auth_key, counter_get_auth_user, counter_get_keyed,
    subscribe_counter_get, subscribe_counter_get_auth_key, subscribe_counter_get_auth_user,
    subscribe_counter_get_keyed,
};
