//! # photon-leptos — Leptos integration for Photon events
//!
//! photon-leptos keeps Leptos `Resource`s in sync with Photon events via WebSocket,
//! so UI updates when relevant events are published — without hand-rolling WS wiring.
//!
//! See the [repository README](https://github.com/deathbreakfast/photon-leptos) for a
//! quick-start hero example.
//!
//! ## Architecture
//!
//! ```text
//! Server (any path) --publish--> Photon topic
//!                                      |
//!                                      v
//!                              Axum WS handler
//!                                      |
//!                                      v
//! Browser: subscribe_<fn> trigger --> Resource refetch --> UI
//! ```
//!
//! ```mermaid
//! sequenceDiagram
//!     participant Job as BackgroundJob
//!     participant Photon as PhotonRuntime
//!     participant WS as ws_endpoint
//!     participant Sub as subscribe_helper
//!     participant Res as Resource
//!     participant UI as PageView
//!
//!     Job->>Photon: topic.publish
//!     Photon->>WS: stream event
//!     WS->>Sub: WebSocket envelope
//!     Sub->>Sub: trigger bump
//!     Sub->>Res: refetch synced read fn
//!     Res->>UI: updated view
//! ```
//!
//! Publish can originate from any server path (background job, webhook, mutation handler).
//! Subscribers only need the topic name and a synced read server function.
//!
//! ## Guarantees
//!
//! - **Automatic refetch** — synced resources react when relevant Photon events arrive.
//! - **WebSocket endpoint** — server forwards topic streams to browser clients.
//! - **Type-safe topics** — use [`photon::topic`] types for publish and subscribe.
//! - **Declarative API** — [`synced`] macro or [`synced_resource`] helpers.
//! - **Leptos server functions** — works with standard `#[server]` read fns.
//! - **Keyed subscriptions** — optional partition filter (e.g. per-user scoping).
//! - **Multiple strategies** — refetch, append, or replace (see [`SyncStrategy`]).
//! - **Reconnection** — client WebSocket reconnects on disconnect.
//! - **Lifecycle** — subscriptions clean up when the reactive owner is disposed.
//!
//! ## Core concepts
//!
//! **Synced resource** — a Leptos `Resource` wired to a Photon topic. When an event
//! arrives on the WebSocket, the resource refetches or applies the configured strategy.
//!
//! **WebSocket endpoint** — an Axum GET handler that subscribes to a Photon topic and
//! forwards serialized events to connected clients. Registered automatically via
//! [`photon_axum::ws_router`] when using [`synced`], or manually via [`server::ws::synced_ws_handler`].
//!
//! **Event strategy** — how incoming events update UI state ([`SyncStrategy`]):
//! refetch re-calls the server function; replace writes the WS payload directly;
//! append adds items to a list via [`synced_resource_append`].
//!
//! ## Quick flow
//!
//! 1. Define a topic with [`photon::topic`] in the **photon** crate.
//! 2. Annotate a read server function with [`synced`] (`#[photon_leptos::synced(...)]`).
//! 3. Publish from any server code via `.publish().await?`.
//! 4. On the client, use the generated `subscribe_<fn>` trigger with a Leptos `Resource`.
//! 5. At host boot, call [`photon_axum::ws_router`] (re-exported from [`server`]).
//!
//! ## Feature flags
//!
//! | Feature | Enables |
//! |---------|---------|
//! | `hydrate` | [`subscribe_ws`], [`synced_resource`], macro client hooks |
//! | `ssr` | [`server`] re-exports, [`inventory`] for route discovery |
//!
//! Enable both on app crates that compile server and client targets.
//!
//! ## Modules
//!
//! - **client** (`hydrate`) — WebSocket subscription primitives and synced resources
//! - [`server`] (`ssr`) — re-exports from `photon_axum` for Axum boot wiring
//! - **opts** — [`SyncStrategy`] and [`SyncedResourceOpts`]
//! - **error** — [`PhotonLeptosError`]
//!
//! Host integrators should also read [`photon_axum`](https://docs.rs/photon_axum) for
//! `ws_router`, `HasPhoton`, and `PhotonUserExtractor`.

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
