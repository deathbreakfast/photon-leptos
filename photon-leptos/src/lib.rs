//! # photon-leptos — Leptos integration for Photon events
//!
//! photon-leptos keeps Leptos `Resource`s in sync with Photon events via WebSocket,
//! so UI updates when relevant events are published — without hand-rolling WS wiring.
//!
//! Start with [Synced refetch](#synced-refetch) for the common path, then [Mount WS
//! routes](#boot-ws-routes) at host boot. Runnable hosts live under `examples/`
//! (`quickstart-refetch`, `secure-refetch-host`, `replace-and-append-demo`).
//!
//! ## Features
//!
//! - **Synced refetch** — Wires a Leptos `Resource` to a Photon topic so the UI
//!   re-calls the server function when an event arrives. [Get started](#synced-refetch)
//! - **Mount WS routes** — Registers inventory WebSocket handlers on the Axum router
//!   once at host boot so browsers can subscribe. [Get started](#boot-ws-routes)
//! - **Replace and Append** — Updates UI from the WebSocket payload (replace) or
//!   appends to a live list (append) when refetch is the wrong fit.
//!   [Get started](#replace-and-append)
//! - **User-auth WebSocket** — Scopes subscribe routes to the authenticated user when
//!   the browser opens the socket. [Get started](#user-auth-ws)
//! - **Shared subscription** — Shares one WebSocket path across multiple resources or
//!   effects with status and error signals. [Get started](#shared-subscription)
//!
//! ## Synced refetch
//!
//! Synced refetch offers a declarative path from a Photon topic to a Leptos
//! `Resource`: when an event arrives on the WebSocket, the resource re-calls the
//! annotated server function so the UI stays authoritative. Use it for list and
//! detail reads that already live behind `#[server]`. Integrators call this from
//! hydrate components after the host mounts inventory WS routes.
//!
//! ### Prerequisites
//!
//! - App crate enables `hydrate` (client hooks) and `ssr` (inventory + server re-exports).
//! - A Photon topic type exists (photon crate [`photon::topic`]) whose name matches
//!   the `topic = "..."` attribute.
//! - Host boot mounts WS routes ([Mount WS routes](#boot-ws-routes)).
//!
//! ### Call sequence
//!
//! ```rust,ignore
//! use leptos::prelude::*;
//! use photon_leptos::synced;
//!
//! #[server]
//! #[synced(
//!     topic = "notifications.updated",
//!     ws = "/ws/notifications",
//!     strategy = "refetch",
//!     auth = "none",
//! )]
//! pub async fn list_notifications() -> Result<Vec<Notification>, ServerFnError> {
//!     Ok(load_notifications().await?)
//! }
//!
//! #[component]
//! pub fn NotificationsPage() -> impl IntoView {
//!     let trigger = subscribe_list_notifications(|| {});
//!     let items = Resource::new(
//!         move || trigger.get(),
//!         move |_| list_notifications(),
//!     );
//!
//!     view! {
//!         <Suspense fallback=move || view! { <p>"Loading…"</p> }>
//!             {move || match items.get() {
//!                 Some(Ok(list)) => view! {
//!                     <ul>
//!                         {list.into_iter().map(|n| view! { <li>{n.title}</li> }).collect_view()}
//!                     </ul>
//!                 }.into_any(),
//!                 Some(Err(err)) => view! { <p>{err.to_string()}</p> }.into_any(),
//!                 None => view! { <p>"Loading…"</p> }.into_any(),
//!             }}
//!         </Suspense>
//!     }
//! }
//! ```
//!
//! Observable outcome: `items.get()` yields `Some(Ok(list))` after the initial fetch
//! and again after each topic event bumps `trigger`. Next: publish with the matching
//! topic type, then [Mount WS routes](#boot-ws-routes). For payload-driven updates see
//! [Replace and Append](#replace-and-append).
//!
//! ## Boot WS routes
//!
//! Mount WS routes installs inventory-discovered Photon WebSocket GET handlers on the
//! Axum router so browser clients can subscribe. Call this once at host boot, after
//! Photon is built and app state implements [`HasPhoton`](photon_axum::HasPhoton),
//! because the default Origin policy rejects every WebSocket origin until you allowlist.
//!
//! ### Prerequisites
//!
//! - `ssr` feature on the app crate; binary links crates that submit `#[synced]` inventory.
//! - `PHOTON_TRANSPORT_KEY` set when Photon fail-closed crypto is enabled.
//! - Production hosts override [`HasPhoton::allow_ws_origin`](photon_axum::HasPhoton::allow_ws_origin).
//!
//! ### Call sequence
//!
//! ```rust,ignore
//! use std::sync::Arc;
//!
//! use axum::Router;
//! use photon::Photon;
//! use photon_axum::{HasPhoton, HeadlessWsAuth, ws_router};
//!
//! #[derive(Clone)]
//! struct AppState {
//!     photon: Arc<Photon>,
//! }
//!
//! impl HasPhoton for AppState {
//!     fn photon_arc(&self) -> Arc<Photon> {
//!         Arc::clone(&self.photon)
//!     }
//!
//!     fn allow_ws_origin(&self, origin: Option<&str>) -> bool {
//!         matches!(origin, Some("https://app.example.com"))
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let photon = /* PhotonBuilder::new()...build()? */;
//!     let state = AppState { photon: Arc::new(photon) };
//!
//!     let app = Router::new();
//!     // …leptos_routes / API routes…
//!     let app = ws_router::<AppState, HeadlessWsAuth>(app).with_state(state);
//!
//!     // axum::serve(listener, app).await.unwrap();
//! }
//! ```
//!
//! Observable outcome: inventory paths such as `/ws/notifications` are mounted and
//! Origin checks use your `allow_ws_origin` allowlist. Variant without inventory:
//! [`server::ws::synced_ws_handler`]. For session-scoped routes see
//! [User-auth WebSocket](#user-auth-ws). API reference: [`server`].
//!
//! ## Replace and Append
//!
//! Replace and Append offer payload-driven sync strategies when refetching the server
//! function is unnecessary or too heavy. Replace writes the WebSocket payload into the
//! resource; Append tails list items as events arrive. Choose these from hydrate UI
//! code when the event payload already is the new value or the next list row.
//!
//! ### Prerequisites
//!
//! - `hydrate` feature for [`synced_resource_replace_result`] and [`synced_resource_append`].
//! - Replace on `Result<T, E>` uses the `Ok` payload type; Append expects `Result<Vec<_>, _>`.
//! - Matching `#[synced(..., strategy = "replace" | "append")]` or the helpers below.
//!
//! ### Call sequence
//!
//! ```rust,ignore
//! use leptos::prelude::*;
//! use photon_leptos::{
//!     synced_resource_append, synced_resource_replace_result, SyncedResourceOpts, SyncStrategy,
//! };
//!
//! // Replace: WS payload is `T` (here the Ok type of Result<T, E>).
//! pub fn use_counter() -> Resource<Result<u64, ServerFnError>> {
//!     synced_resource_replace_result(
//!         counter_get,
//!         SyncedResourceOpts {
//!             topic: "counter.updated".into(),
//!             ws_path: "/ws/counter".into(),
//!             strategy: SyncStrategy::Replace,
//!             key_filter: None,
//!         },
//!     )
//! }
//!
//! // Append: live tail into Vec items (pair with Refetch for authoritative reload).
//! pub fn use_feed() -> Resource<Result<Vec<Item>, ServerFnError>> {
//!     synced_resource_append(
//!         feed_list,
//!         SyncedResourceOpts {
//!             topic: "feed.appended".into(),
//!             ws_path: "/ws/feed".into(),
//!             strategy: SyncStrategy::Append,
//!             key_filter: None,
//!         },
//!     )
//! }
//!
//! // After an event, Resource::get() shows Ok(updated) without re-calling the server fn
//! // (replace) or Ok(vec_with_new_row) (append).
//! ```
//!
//! Observable outcome: `Resource` holds `Ok(...)` updated from the payload path.
//! Detailed host: `examples/replace-and-append-demo`. Prefer [Synced refetch](#synced-refetch)
//! when the server must recompute joins or auth-scoped queries.
//!
//! ## User-auth WS
//!
//! User-auth WebSocket scopes inventory subscribe routes to the signed-in user so
//! browsers only receive events for that principal. Set `auth = "user"` on `#[synced]`
//! and pass a host extractor into `ws_router` at boot; when the request opens the
//! WebSocket, PhotonUserExtractor resolves the user id for key policy.
//!
//! ### Prerequisites
//!
//! - Host auth type implements [`PhotonUserExtractor`](photon_axum::PhotonUserExtractor)
//!   and `FromRequestParts` for app state.
//! - Synced routes declare `auth = "user"`; boot uses `ws_router::<S, YourAuth>`.
//!
//! ### Call sequence
//!
//! ```rust,ignore
//! use photon_leptos::synced;
//! use photon_axum::{PhotonUserExtractor, ws_router};
//!
//! #[server]
//! #[synced(
//!     topic = "inbox.updated",
//!     ws = "/ws/inbox",
//!     strategy = "refetch",
//!     auth = "user",
//! )]
//! pub async fn list_inbox() -> Result<Vec<Msg>, ServerFnError> {
//!     Ok(load_inbox().await?)
//! }
//!
//! // At host boot — YourSession: PhotonUserExtractor + FromRequestParts<AppState>
//! let app = ws_router::<AppState, YourSession>(app);
//! ```
//!
//! Observable outcome: user-auth routes reject unauthenticated upgrades and bind
//! subscribe keys via `PhotonUserExtractor`. See `examples/secure-refetch-host`.
//! Next: [Mount WS routes](#boot-ws-routes) Origin allowlist still applies.
//!
//! ## Shared subscription
//!
//! Shared subscription exposes one WebSocket subscription for several resources or
//! effects that share a topic path, including connection status and last error. Use it
//! from hydrate components when Tier-2 `use_<fn>()` would open duplicate sockets or you
//! need explicit status UI.
//!
//! ### Prerequisites
//!
//! - `hydrate` feature; topic name and WS path match the server registration.
//!
//! ### Call sequence
//!
//! ```rust,ignore
//! use leptos::prelude::*;
//! use photon_leptos::{use_topic_subscription, PhotonSubscription};
//!
//! #[component]
//! pub fn SharedPanel() -> impl IntoView {
//!     let sub: PhotonSubscription = use_topic_subscription("/ws/notifications", None);
//!     let trigger = sub.trigger;
//!     let status = sub.status;
//!
//!     let items = Resource::new(
//!         move || trigger.get(),
//!         move |_| list_notifications(),
//!     );
//!
//!     view! {
//!         <p>{move || format!("ws: {:?}", status.get())}</p>
//!         // …render items from `items` Resource…
//!     }
//! }
//! ```
//!
//! Observable outcome: `trigger` bumps on each event and `status` reflects
//! [`WsConnectionStatus`]. Lower-level variant: [`subscribe_ws`] → [`PhotonWsHandle`].
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
//! - **0.1 experimental** — browser WebSocket is an ephemeral invalidation / live-update channel.
//! - **Refetch** — supported; server function remains authoritative (preferred after reconnect).
//! - **Replace** — experimental; payload is `T` or the `Ok` type of `Result<T, E>`.
//! - **Append** — best-effort live tail (buffers during initial load); pair with Refetch for authoritative lists.
//! - **WebSocket endpoint** — server forwards topic streams to browser clients.
//! - **Type-safe topics** — use [`photon::topic`] types for publish and subscribe.
//! - **Declarative API** — [`synced`] macro or `synced_resource` helpers (hydrate).
//! - **Keyed subscriptions** — optional partition filter (e.g. per-user scoping).
//! - **Reconnection** — client WebSocket reconnects on disconnect.
//! - **Lifecycle** — subscriptions clean up when the reactive owner is disposed.
//! - **Observability** — `subscribe_ws` returns `PhotonWsHandle` with status / last error / close (hydrate).
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
//! refetch re-calls the server function; replace writes the WS payload directly
//! (`T`, or `Ok` of `Result<T, E>` via `synced_resource_replace_result`);
//! append is a best-effort live tail via `synced_resource_append`.
//!
//! **Subscription handle** — `subscribe_ws` returns `PhotonWsHandle` with
//! reactive `WsConnectionStatus`, `last_error`, and `close()`.
//! `use_topic_subscription` exposes the same signals on `PhotonSubscription`.
//! Enable the `hydrate` feature for these client APIs.
//!
//! ## Quick flow
//!
//! ### 1. Define a topic
//!
//! Use [`photon::topic`] in shared/server code (the **photon** crate API — not this crate):
//!
//! ```rust,ignore
//! use photon::topic;
//! use serde::{Deserialize, Serialize};
//!
//! #[topic(name = "notifications.updated")]
//! #[derive(Clone, Debug, Serialize, Deserialize)]
//! pub struct NotificationUpdated {
//!     pub user_id: String,
//! }
//! ```
//!
//! ### 2. Annotate a synced read server function
//!
//! Pair `#[server]` with [`synced`]. The macro generates `subscribe_list_notifications` and
//! registers a WS route for inventory discovery:
//!
//! ```rust,ignore
//! use leptos::prelude::*;
//! use photon_leptos::synced;
//!
//! #[server]
//! #[synced(
//!     topic = "notifications.updated",
//!     ws = "/ws/notifications",
//!     strategy = "refetch",
//!     auth = "none",
//! )]
//! pub async fn list_notifications() -> Result<Vec<Notification>, ServerFnError> {
//!     // Load current notifications from your store / DB.
//!     Ok(load_notifications().await?)
//! }
//! ```
//!
//! ### 3. Publish after a mutation
//!
//! Any server path can publish — background job, webhook, or another user's write:
//!
//! ```rust,ignore
//! async fn on_import_job_finished(user_id: String) -> Result<(), Box<dyn std::error::Error>> {
//!     // Persist the new notification first, then notify subscribers.
//!     NotificationUpdated { user_id }.publish().await?;
//!     Ok(())
//! }
//! ```
//!
//! ### 4. Subscribe in the Leptos UI
//!
//! Wire the generated trigger into a `Resource` so the UI refetches on each event:
//!
//! ```rust,ignore
//! use leptos::prelude::*;
//!
//! #[component]
//! pub fn NotificationsPage() -> impl IntoView {
//!     let trigger = subscribe_list_notifications(|| {});
//!     let items = Resource::new(
//!         move || trigger.get(),
//!         move |_| list_notifications(),
//!     );
//!
//!     view! {
//!         <Suspense fallback=move || view! { <p>"Loading…"</p> }>
//!             {move || match items.get() {
//!                 Some(Ok(list)) => view! {
//!                     <ul>
//!                         {list.into_iter().map(|n| view! { <li>{n.title}</li> }).collect_view()}
//!                     </ul>
//!                 }.into_any(),
//!                 Some(Err(err)) => view! { <p>{err.to_string()}</p> }.into_any(),
//!                 None => view! { <p>"Loading…"</p> }.into_any(),
//!             }}
//!         </Suspense>
//!     }
//! }
//! ```
//!
//! ### 5. Mount WS routes at host boot
//!
//! App state must implement [`photon_axum::HasPhoton`]. Call [`photon_axum::ws_router`]
//! (re-exported from [`server`]) so inventory routes like `/ws/notifications` are mounted:
//! production hosts must also implement an Origin allowlist because the default
//! rejects all WebSocket origins.
//!
//! Set `PHOTON_TRANSPORT_KEY` (base64-encoded 32-byte transport key) before building Photon
//! when fail-closed crypto is enabled (Photon 0.1.1+). See the repository README and
//! `e2e/README.md` for dev values.
//!
//! ```rust,ignore
//! use std::sync::Arc;
//!
//! use axum::Router;
//! use photon::Photon;
//! use photon_axum::{HasPhoton, HeadlessWsAuth, ws_router};
//!
//! // std::env::set_var("PHOTON_TRANSPORT_KEY", "…");
//!
//! #[derive(Clone)]
//! struct AppState {
//!     photon: Arc<Photon>,
//! }
//!
//! impl HasPhoton for AppState {
//!     fn photon_arc(&self) -> Arc<Photon> {
//!         Arc::clone(&self.photon)
//!     }
//!
//!     fn allow_ws_origin(&self, origin: Option<&str>) -> bool {
//!         matches!(origin, Some("https://app.example.com"))
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let photon = /* PhotonBuilder::new()...build()? */;
//!     let state = AppState { photon: Arc::new(photon) };
//!
//!     let app = Router::new();
//!     // …leptos_routes / API routes…
//!     let app = ws_router::<AppState, HeadlessWsAuth>(app).with_state(state);
//!
//!     // axum::serve(listener, app).await.unwrap();
//! }
//! ```
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
//!   (see [Synced refetch](#synced-refetch), [Shared subscription](#shared-subscription),
//!   [Replace and Append](#replace-and-append))
//! - [`server`] (`ssr`) — re-exports from `photon_axum` for Axum boot wiring
//!   (see [Mount WS routes](#boot-ws-routes), [User-auth WebSocket](#user-auth-ws))
//! - **opts** — [`SyncStrategy`] and [`SyncedResourceOpts`]
//! - **error** — [`PhotonLeptosError`]
//!
//! Host integrators should also read [`photon_axum`](https://docs.rs/photon_axum) for
//! `ws_router`, `HasPhoton`, and `PhotonUserExtractor`.

#![warn(missing_docs)]

mod error;
mod opts;
mod ws_url;

pub use error::PhotonLeptosError;
pub use opts::{SyncStrategy, SyncedResourceOpts};
pub use photon_leptos_macros::synced;
pub use ws_url::{ws_url_log_fields, ws_url_with_key};

cfg_if::cfg_if! {
    if #[cfg(feature = "hydrate")] {
        mod client;
        pub use client::{
            subscribe_ws, synced_resource, synced_resource_append, synced_resource_replace_result,
            use_topic_subscription, PhotonSubscription, PhotonWsHandle, WsConnectionStatus,
        };
    }
}

#[cfg(feature = "ssr")]
/// SSR-side WebSocket route registration (re-exports from [`photon_axum`]).
pub mod server;

#[cfg(feature = "ssr")]
pub use quark::inventory;
