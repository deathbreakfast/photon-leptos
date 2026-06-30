//! # photon-leptos — Leptos integration for Photon events
//!
//! **Audience:** app authors building realtime Leptos UIs on Photon topics.
//!
//! photon-leptos keeps Leptos `Resource`s in sync with Photon events via WebSocket,
//! so UI updates when relevant events are published — without hand-rolling WS wiring.
//!
//! ## Quick flow
//!
//! 1. Define a topic with [`photon::topic`] in the **photon** crate.
//! 2. Annotate a read server function with [`synced`] (`#[photon_leptos::synced(...)]`).
//! 3. Publish from mutation server functions via `.publish().await?`.
//! 4. On the client, use the generated `subscribe_<fn>` trigger with a Leptos `Resource`.
//! 5. At host boot, call [`photon_axum::ws_router`] (re-exported from [`server`]).
//!
//! See the [repository README](https://github.com/deathbreakfast/photon-leptos) for a full hero example.
//!
//! ## Feature flags
//!
//! | Feature | Audience | Enables |
//! |---------|----------|---------|
//! | `hydrate` | App author | [`subscribe_ws`], [`synced_resource`], macro client hooks |
//! | `ssr` | Integrator | [`server`] re-exports, [`inventory`] for route discovery |
//!
//! Enable both on app crates that compile server and client targets.
//!
//! ## Module guide
//!
//! | Module | Audience | Contents |
//! |--------|----------|----------|
//! | client (`hydrate`) | App author | WS subscription primitives and synced resources |
//! | [`server`] (`ssr`) | Integrator | Re-exports from `photon_axum` |
//! | opts | App author | [`SyncStrategy`], [`SyncedResourceOpts`] |
//! | error | App author | [`PhotonLeptosError`] |
//!
//! **Integrators** wiring Axum should also read [`photon_axum`](https://docs.rs/photon_axum) docs.
//! **Maintainers** see `DESIGN.md` in this crate directory.

#![warn(missing_docs)]

mod error;
mod opts;

pub use error::PhotonLeptosError;
pub use opts::{SyncStrategy, SyncedResourceOpts};
pub use photon_leptos_macros::synced;

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
/// SSR-side WebSocket route registration (re-exports from [`photon_axum`]).
pub mod server;

#[cfg(feature = "ssr")]
pub use quark::inventory;
