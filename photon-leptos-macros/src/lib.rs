//! Proc macros for Photon Leptos integration (Zone C).
//!
//! **Audience:** app authors annotating Leptos server functions for realtime UI.
//!
//! ## [`synced`] attribute reference
//!
//! | Attribute | Required | Default | Description |
//! |-----------|----------|---------|-------------|
//! | `topic` | yes | — | Photon topic name (must match `#[photon::topic]`) |
//! | `ws` | no | `/ws/{fn-with-hyphens}` | WebSocket GET path |
//! | `strategy` | no | `"refetch"` | `"refetch"`, `"replace"`, or `"append"` |
//! | `key` | no | none | Static key filter string for client opts |
//! | `auth` | no | `"none"` | `"none"` or `"user"` (host auth at `ws_router`) |
//!
//! ## Generated symbols (for `counter_get`)
//!
//! | Symbol | Build | Purpose |
//! |--------|-------|---------|
//! | `subscribe_counter_get(on_event)` | all | Returns trigger `RwSignal`; bumps on WS event |
//! | `use_counter_get()` | `hydrate` | Full synced [`Resource`](https://docs.rs/leptos) |
//! | `__photon_ws_counter_get::PATH` | `ssr` | WebSocket path constant |
//! | inventory entry | `ssr` | Auto-discovered by `photon_axum::ws_router` |
//!
//! Requires `photon-leptos` features `hydrate` and/or `ssr` on the app crate.

#![warn(missing_docs)]

mod synced;

use proc_macro::TokenStream;

/// Marks an async server function as a synced Leptos resource backed by Photon events.
///
/// # Example
///
/// ```ignore
/// use photon_leptos::synced;
///
/// #[synced(
///     topic = "counter.updated",
///     ws = "/ws/counter",
///     strategy = "refetch",
///     auth = "none",
/// )]
/// pub async fn counter_get() -> Result<CounterResponse, ServerFnError> {
///     Ok(CounterResponse { value: 0 })
/// }
/// ```
///
/// See crate-level docs for the full attribute reference and generated symbols.
#[proc_macro_attribute]
pub fn synced(attr: TokenStream, item: TokenStream) -> TokenStream {
    synced::synced_impl(attr, item)
}
