//! photon-leptos — Leptos integration for Photon events
//!
//! Keeps Leptos resources in sync with Photon events via WebSocket,
//! enabling automatic UI updates when relevant events are published.
//!
//! ## Client APIs (`hydrate`)
//!
//! - `subscribe_ws`: low-level topic subscription helper.
//! - `synced_resource`: reconnect-safe resource synchronization primitive.
//! - `synced_resource_append`: append-style update strategy for list feeds.
//! - `use_topic_subscription`: reusable hook for topic lifecycle wiring.
//!
//! ## Server APIs (`ssr`)
//!
//! - [`server`]: endpoint utilities and integration glue for Photon-backed
//!   websocket subscription handling in SSR applications.
//!
//! ## Typical usage
//!
//! ```rust,ignore
//! let notifications = synced_resource(
//!     || (),
//!     |_| async move { fetch_notifications().await },
//!     SyncedResourceOpts::topic("notifications.updated"),
//! );
//! ```

mod error;
mod opts;

pub use error::PhotonLeptosError;
pub use opts::{SyncStrategy, SyncedResourceOpts};

cfg_if::cfg_if! {
    if #[cfg(feature = "hydrate")] {
        mod client;
        pub use client::{
            subscribe_ws, synced_resource, synced_resource_append,
            use_topic_subscription, PhotonSubscription,
        };
    }
}

#[cfg(feature = "ssr")]
/// SSR-side websocket endpoint and bridge utilities.
pub mod server;

#[cfg(feature = "ssr")]
pub use quark::inventory;
