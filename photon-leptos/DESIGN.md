# photon-leptos — design (forward)

## Overview

photon-leptos is a helper crate that keeps Leptos resources in sync with Photon events, enabling automatic UI updates when events are published. It provides a declarative API for real-time resource management without manual WebSocket wiring.

**Package Name**: `photon-leptos`  
**Purpose**: Provides Leptos integration for Photon events, enabling real-time UI updates through automatic resource refetching when relevant events are published.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    photon-leptos Architecture                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐      │
│  │  Leptos         │     │  photon-leptos  │     │  Photon         │      │
│  │  Resource       │────►│  (Syncer)       │────►│  (Events)       │      │
│  └─────────────────┘     └─────────────────┘     └─────────────────┘      │
│           │                       │                       │                  │
│           │                       │                       │                  │
│           ▼                       ▼                       ▼                  │
│  ┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐      │
│  │  WebSocket      │     │  Event          │     │  Topic          │      │
│  │  Endpoint       │     │  Forwarder      │     │  Subscription   │      │
│  └─────────────────┘     └─────────────────┘     └─────────────────┘      │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Table of Contents

1. [Requirements](#1-requirements)
2. [Core Concepts](#2-core-concepts)
3. [Synced Resource Pattern](#3-synced-resource-pattern)
4. [WebSocket Integration](#4-websocket-integration)
5. [API Design](#5-api-design)
6. [Integration with Orbital](#6-integration-with-orbital)
7. [Testing Strategy](#7-testing-strategy)
8. [Phased Delivery Plan](#8-phased-delivery-plan)

---

## 1. Requirements

### Must-Haves

| Requirement | Description |
|-------------|-------------|
| Automatic refetch | Resource refetches when relevant Photon event arrives |
| WebSocket endpoint | Server-side endpoint that forwards Photon events to clients |
| Type-safe | Typed event handling with Photon topic types |
| Declarative API | Macro or helper function for easy integration |
| Orbital integration | Works with Orbital SSR and server function patterns |

### Additional Constraints

| Constraint | Description |
|------------|-------------|
| Keyed subscriptions | Support filtering by topic key (e.g., user_id) |
| Multiple strategies | Support refetch, append, replace strategies |
| Reconnection | Automatic WebSocket reconnection on disconnect |
| Resource lifecycle | Clean up subscriptions when resource is dropped |

### Shipped API

Macros (`#[photon::synced]`), `synced_resource` / append helpers, WebSocket handlers, and quark route registration
are documented in **`cargo doc -p photon-leptos --open`**. This file keeps architecture, requirements, and roadmap.

**Integration**: [Photon](../photon/DESIGN.md) for event subscriptions, [Orbital](../orbital/README.md) for SSR patterns.

---

## 2. Core Concepts

### Synced Resource

A Leptos `Resource` that automatically refetches when a relevant Photon event is published. The resource is created from a server function and kept in sync via WebSocket events.

### WebSocket Endpoint

A server-side WebSocket endpoint that:
1. Subscribes to Photon events (optionally filtered by key)
2. Forwards events to connected clients
3. Maintains connection per client session

### Event Strategy

How the resource responds to events:
- **refetch**: Call server function again to get fresh data
- **append**: Add new item to resource data (for lists)
- **replace**: Replace resource data with event payload

---

## 3. Synced Resource Pattern

### Usage with Macro

```rust
use orbital::server;

// Server function that owns Valence access (SSR only)
#[photon::synced(
    topic = "user.notifications",
    ws = "/ws/notifications",
    strategy = "refetch"
)]
#[server]
pub async fn list_notifications() -> Result<Vec<Notification>, ServerFnError> {
    let v = orbital::ssr::valence().await?;
    // Query notifications for current user with v
    Ok(vec![])
}

// The macro generates a client hook: use_list_notifications()
```

### Usage with Helper Function

```rust
use photon_leptos::synced_resource;

pub fn use_notifications() -> Resource<Result<Vec<Notification>, ServerFnError>> {
    synced_resource(
        list_notifications,
        SyncedResourceOpts {
            topic: "user.notifications".to_string(),
            ws_path: "/ws/notifications".to_string(),
            strategy: SyncStrategy::Refetch,
            // Optional: filter by a client-available key (e.g., user id from auth)
            key_filter: Some("user-123".to_string()),
        },
    )
}
```

### Component Usage

```rust
#[component]
pub fn NotificationsList() -> impl IntoView {
    let notifications = use_list_notifications();
    
    view! {
        <div>
            {move || {
                notifications.get().and_then(|result| {
                    result.ok().map(|notifications| {
                        notifications.iter().map(|n| {
                            view! { <NotificationCard notification=n.clone() /> }
                        }).collect::<Vec<_>>()
                    })
                })
            }}
        </div>
    }
}
```

---

## 4. WebSocket Integration

### Server-Side Endpoint

photon-leptos provides `synced_ws_handler` for WebSocket endpoints:

```rust
use axum::extract::{State, ws::WebSocketUpgrade};
use axum_login::AuthSession;
use photon_leptos::server::ws::{synced_ws_handler, SyncedWsConfig};
use std::sync::Arc;

// Handler that extracts user_id from auth session for per-user scoping
async fn notifications_ws_handler(
    ws: WebSocketUpgrade,
    auth_session: AuthSession<Backend>,
    State(photon): State<Arc<photon::Photon>>,
) -> axum::response::Response {
    let key_filter = auth_session.user.as_ref().map(|user| {
        use axum_login::AuthUser;
        AuthUser::id(user).key.clone()
    });
    
    let config = SyncedWsConfig {
        topic: "user.notifications".to_string(),
        key_filter,
        subscription_name: None,
    };
    
    synced_ws_handler(ws, photon, config).await
}

// In your router setup
Router::new()
    .route("/ws/notifications", get(notifications_ws_handler))
```

### Client-Side Connection

The synced resource helper manages WebSocket connection lifecycle:

1. Create WebSocket connection on resource creation — **implemented**
2. Subscribe to events from server — **implemented**
3. On event received, trigger resource refetch (or apply strategy) — **implemented** (Refetch, Append)
4. Reconnect on disconnect — **implemented** (exponential backoff)
5. Clean up on resource drop — **implemented**

---

## 5. API Design

### Macro API

```rust
#[photon::synced(
    topic = "user.notifications",      // Photon topic name
    ws = "/ws/notifications",          // WebSocket endpoint path
    strategy = "refetch",               // refetch | append | replace
    key = "user_id"                     // Optional: key field for filtering
)]
#[server]
pub async fn list_notifications() -> Result<Vec<Notification>, ServerFnError> {
    let v = orbital::ssr::valence().await?;
    // Query notifications with v
    Ok(vec![])
}

// The macro generates a client hook: use_list_notifications()
```

### Helper Function API

```rust
pub fn synced_resource<F, Fut, T>(fetcher: F, opts: SyncedResourceOpts) -> Resource<T>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = T> + Send + 'static,
    T: Serialize + DeserializeOwned + Send + Sync + 'static;

pub struct SyncedResourceOpts {
    pub topic: String,
    pub ws_path: String,
    pub strategy: SyncStrategy,
    pub key_filter: Option<String>,
}

pub enum SyncStrategy {
    Refetch,    // Call server function again
    Append,     // Add to list (for Vec<T>)
    Replace,    // Replace with event payload
}
```

---

## 6. Integration with Orbital

### Server Functions

Follows Orbital SSR patterns (see [Orbital API Usage](../docs/src/03-platform-guides/orbital.md)):

```rust
use orbital::server;

#[server]
pub async fn list_notifications() -> Result<Vec<Notification>, ServerFnError> {
    let v = orbital::ssr::valence().await?;
    // Queries Valence with actor-aware privacy
    Ok(vec![])
}
```

### WebSocket Endpoint Registration

WebSocket endpoints are registered in the Axum router alongside server functions:

```rust
// In server/src/main.rs
use axum::{
    Router,
    routing::{get, post},
    extract::ws::WebSocketUpgrade,
};

pub fn build_app() -> Router {
    Router::new()
        .route("/api/{*fn_name}", post(server_fns_handler))
        .route("/ws/notifications", get(ws_notifications_handler))
        // ... other routes
}
```

---

## 7. Testing Strategy

### Unit Tests

| Component | Status | Test Focus |
|-----------|--------|------------|
| `SyncStrategy::from_str` | done | Parsing refetch, append, replace |
| `SyncedResourceOpts` | done | Struct construction |
| WebSocket Handler | — | Event forwarding, connection management |
| Reconnection Logic | done | Automatic reconnection with backoff |

### Integration Tests (Planned)

```rust
#[tokio::test]
async fn test_synced_resource_refetch() {
    // Setup Photon and WebSocket
    let photon = test_photon().await;
    let ws_client = connect_ws("/ws/notifications").await;
    
    // Create synced resource
    let resource = synced_resource(
        list_notifications,
        SyncedResourceOpts {
            topic: "user.notifications",
            ws_path: "/ws/notifications",
            strategy: SyncStrategy::Refetch,
            key_filter: Some("user-123".to_string()),
        },
    );
    
    // Initial fetch
    let initial = resource.get().await;
    assert_eq!(initial.len(), 0);
    
    // Publish event
    NotificationPushed::publish(NotificationPushed {
        user_id: "user-123".to_string(),
        notification_id: uuid::Uuid::new_v4(),
    }).await?;
    
    // Wait for refetch
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Verify resource updated
    let updated = resource.get().await;
    assert_eq!(updated.len(), 1);
}
```

---

## 8. Phased Delivery Plan

### Phase 1: Helper Function — Done

| Component | Status | Description |
|-----------|--------|-------------|
| Helper function | done | `synced_resource()` with refetch strategy |
| WebSocket endpoint | done | Basic event forwarding via `synced_ws_handler` |
| Reconnection | done | Automatic reconnection with exponential backoff |

### Phase 2: Macro & Advanced Features — Done

| Component | Status | Description |
|-----------|--------|-------------|
| Macro | done | `#[photon::synced]` proc-macro generating `use_<fn_name>()` |
| Multiple strategies | done | Append via `synced_resource_append`; Replace uses Refetch |
| Key filtering | done | Caller extracts user_id from auth session and passes as `key_filter` |

**Key guarantee**: Helper function API remains stable; macro is syntactic sugar.

### Phase 3: Zero-Boilerplate Automation — Done

| Component | Status | Description |
|-----------|--------|-------------|
| Server handler codegen | done | `#[photon::synced]` generates `__photon_ws_<fn>` module with `PATH` + `handler<S>` |
| Auth key extraction trait | done | `PhotonUserExtractor` trait in `photon-leptos`, `PhotonAuth` newtype in `orbital-ssr` |
| Route auto-collection | done | `ws_routes![]` declarative macro collects `__photon_ws_*` modules |
| Effect-based replace strategy | done | Direct `RwSignal` write from WS payload, fallback to refetch on deser error |
| Shared subscription primitive | done | `use_topic_subscription()` returns `PhotonSubscription` with `trigger` + `latest_event` |
| Migrate notification bell | done | `get_unread_count` annotated, WS handler auto-generated, manual handler deleted |
| Counter real-time | done | `counter_get` annotated, `CounterUpdated` topic published, counter page subscribes |

### Phase 4: Typed Subscriptions + Quark Auto-Discovery — Done

| Component | Status | Description |
|-----------|--------|-------------|
| Typed subscribe helper | done | `subscribe_<fn_name>(on_event)` generated in all builds, returns `RwSignal<u64>` trigger |
| WsRouteDescriptor | done | Descriptor with path, topic, auth mode; submitted via `inventory::submit!` |
| Quark auto-discovery | done | `apply_ws_routes::<S, Auth>()` replaces manual `ws_routes![]` list |
| PhotonAuth newtype | done | `orbital_ssr::PhotonAuth` wraps `AuthSession<Backend>`, impls `PhotonUserExtractor` + `FromRequestParts` |
| Counter migration | done | Counter page uses `subscribe_counter_get(\|\| {})` — no manual trigger or cfg gate |
| Bell migration | done | Notification bell uses `subscribe_get_unread_count(move \|\| { ... })` with pagination reset |

See [Zero-boilerplate real-time resources](#zero-boilerplate-real-time-resources-photonsynced-phase-3)
for full design.

---

## Next Steps

### ~~Zero-boilerplate real-time resources (`#[photon::synced]` Phase 3)~~ — Done
<!-- priority: P1 | effort: 3 -->

**Implemented.** A developer adds `#[photon::synced]` to a server function
and everything works — client hook, server WS handler, route registration,
auth scoping, and value synchronization are all generated or automatic.

See [docs/src/02-counter-app-walkthrough/08-realtime-values.md](../docs/src/02-counter-app-walkthrough/08-realtime-values.md) for the
updated developer guide.

**Original goal (preserved for reference)**:

#### The gap today

The `#[photon::synced]` macro currently generates only the client-side
`use_<fn_name>()` hook. Everything else is still manual boilerplate:

| Layer | What | Today | Goal |
|-------|------|-------|------|
| Server WS handler | `notifications_ws_handler()` function | Manual per topic (~15 lines) | Generated by macro |
| Route registration | `.route("/ws/notifications", get(...))` | Manual in `server/src/main.rs` | Auto-collected, single call |
| Auth key extraction | Extract user_id from `AuthSession` | Manual per handler | Trait-based, declared once |
| Client subscription | `subscribe_ws(...)` or `use_<fn_name>()` | `use_` hook exists but not used everywhere | Hook is the only API needed |
| Value sync (replace) | Keep initial fetch and WS payload in sync | Refetch round-trip | Effect-based direct update |

#### Ideal developer experience

**Step 1 — Annotate the server function (only step the developer does):**

```rust
#[photon::synced(
    topic = "user.notifications",
    strategy = "refetch",
    auth = "user",           // NEW: scoping mode
)]
#[server]
pub async fn get_unread_count() -> Result<i64, ServerFnError> {
    let v = orbital::ssr::valence().await?;
    // ...
    Ok(count)
}
// Generates: use_get_unread_count() -> Resource<Result<i64, ServerFnError>>
```

**Step 2 — Use the hook in a component (standard Leptos):**

```rust
#[component]
pub fn UnreadBadge() -> impl IntoView {
    let count = use_get_unread_count();
    view! { <span>{move || count.get().map(|r| r.unwrap_or(0))}</span> }
}
```

**That's it.** No handler function. No route wiring. No `subscribe_ws`.

#### Design: four sub-features

##### A. Server WS handler codegen from `#[photon::synced]`

The macro already knows `topic` and `ws` (path). Add an `auth` attribute
that selects a key-extraction strategy:

| `auth` value | Key extraction | Use case |
|--------------|---------------|----------|
| `"none"` (default) | No key filter — all clients get all events | Global values (counter) |
| `"user"` | Extract `AuthUser::id().key` from `AuthSession` | Per-user data (notifications) |

The macro generates a server-side handler function alongside the client hook:

```rust
// Generated (SSR only):
#[cfg(feature = "ssr")]
pub async fn __photon_ws_get_unread_count(
    ws: axum::extract::ws::WebSocketUpgrade,
    auth: axum_login::AuthSession<orbital::auth::ssr::Backend>,
    axum::extract::State(app_state): axum::extract::State<crate::AppState>,
) -> impl axum::response::IntoResponse {
    use photon_leptos::server::ws::{synced_ws_handler, SyncedWsConfig};

    let key_filter = {
        // auth = "user" → extract user key
        use axum_login::AuthUser;
        auth.user.as_ref().map(|u| AuthUser::id(u).key.clone())
    };

    let config = SyncedWsConfig {
        topic: "user.notifications".to_string(),
        key_filter,
        subscription_name: None,
    };

    synced_ws_handler(ws, app_state.photon.clone(), config).await
}
```

The handler name is deterministic (`__photon_ws_<fn_name>`) and the WS
path is derived from the function name: `/ws/<fn_name_with_underscores_to_hyphens>`.
This removes the `ws = "..."` attribute — the path is convention-based
(`/ws/get-unread-count`), though an explicit `ws = "/ws/notifications"`
override remains available for backward compatibility.

**Open question**: The generated handler needs access to `AppState` which
has a `.photon` field. This couples the macro to the app's state type.
Options:

1. **Trait bound** — require `AppState: HasPhoton` (trait already exists
   in `server/src/main.rs`). The macro emits a bound, not a concrete type.
2. **Context extraction** — use `Extension<Arc<Photon>>` instead of
   `State`, avoiding the AppState coupling. This is simpler but slightly
   less idiomatic for Axum.
3. **Macro parameter** — `state = "AppState"` for explicit typing.

Recommended: option 1 (trait bound). `HasPhoton` is already defined.

##### B. Automatic route collection via `inventory`

Each `#[photon::synced]` invocation registers a `WsRouteDescriptor` via
`inventory::submit!`:

```rust
// Generated:
photon_leptos::inventory::submit! {
    photon_leptos::WsRouteDescriptor {
        path: "/ws/get-unread-count",
        handler: __photon_ws_get_unread_count,
    }
}
```

In `server/src/main.rs`, a single call collects all registered routes:

```rust
// Before (manual per topic):
.route("/ws/notifications", get(notifications_ws_handler))

// After (automatic):
let app = photon_leptos::server::apply_ws_routes(app);
```

`apply_ws_routes` iterates `inventory::iter::<WsRouteDescriptor>` and
calls `.route(desc.path, get(desc.handler))` for each.

**Challenge**: `inventory` works with static data, but Axum handler
functions are generic over the state extractor. The descriptor will need
to store a `fn` pointer or use a trait-object wrapper. This may require
`inventory` to store a `Box<dyn WsRouteRegistrar>` that receives the
router and adds itself. Alternatively, a build-time codegen approach
(like `orbital-codegen` for route discovery) could collect the routes.

Fallback if `inventory` proves too complex: a declarative macro that
lists all synced function modules, similar to how `orbital_app!` works:

```rust
// server/src/main.rs
let app = photon_leptos::ws_routes![
    orbital_notifications_core::server::get_unread_count,
    orbital_notifications_core::server::get_unread_notifications_preview,
    counter_app::counter::server::get_counter_value,
];
```

This is still significantly better than manual handlers — no handler
function, no `SyncedWsConfig`, no route path strings. Just a list of the
server functions that have `#[photon::synced]`.

##### C. Effect-based value synchronization (replace strategy)

Currently `SyncStrategy::Replace` falls back to `Refetch` (round-trip to
server). For simple scalar values (counts, statuses, single records), the
WS event payload already contains the new value. An Effect can apply it
directly:

```rust
// Inside synced_resource when strategy = Replace:
let data = RwSignal::new(None);

// Initial fetch via server function (SSR + hydration safe)
let resource = Resource::new(
    move || trigger.get(),
    move |_| fetcher(),
);

// Effect: when WS delivers a payload, deserialize and write directly
subscribe_ws(&opts.ws_path, move |payload_json| {
    if let Ok(value) = serde_json::from_value::<T>(payload_json) {
        // Write the WS-delivered value directly into the resource's
        // underlying signal, skipping a server round-trip.
        // The resource stays consistent because the Effect runs
        // inside the same reactive owner.
        data.set(Some(value));
    }
});
```

The key insight is that the published Photon event already carries the
new value as `payload_json`. For `refetch` the payload is ignored and
serves only as a "something changed" signal. For `replace`, the payload
**is** the new value and the Effect writes it directly into a signal,
avoiding the server round-trip entirely.

**Hydration safety**: The initial value still comes from the server
function (SSR serialized into HTML). The Effect only runs on the client
after hydration, so it cannot cause a mismatch. The resource's initial
render uses the SSR value; subsequent updates come from the WS Effect.

**When to use replace vs. refetch:**

| Scenario | Strategy | Why |
|----------|----------|-----|
| Unread count (single i64) | replace | Payload is the count; no need to re-query |
| Notification list (user-scoped query) | refetch | Payload is one new notification; full list needs server query |
| Counter value (single i64) | replace | Payload is the new value |
| Leaderboard (sorted list) | refetch | Ordering logic lives on the server |

##### D. Shared WS connection for same-topic resources

The notification bell has two resources (`count_res` and `preview_res`)
that both listen to the same topic on the same WS path. Today each would
open a separate WebSocket connection.

`subscribe_ws` already uses `leptos_use::use_websocket` which is scoped
to the reactive owner. Two calls with the same path within the same
component will create two connections. To share:

1. **Connection pool** — Maintain a `HashMap<String, Signal<Option<String>>>`
   keyed by WS path. The first `subscribe_ws` call for a path creates
   the connection; subsequent calls for the same path reuse the signal.
   Implemented as a context-level store (`provide_context` / `use_context`).

2. **Multiple callbacks** — The pool entry holds a `Vec<Callback>` that
   all fire when a message arrives on that path.

This is an optimization, not a blocker. The initial implementation can
open multiple connections (browsers handle this fine). The pool can be
added later without changing the public API.

#### Implementation plan (completed)

| Step | Status | Description |
|------|--------|-------------|
| 1. Auth key extraction trait | done | `PhotonUserExtractor` trait in `photon-leptos/src/server/auth.rs`, `PhotonAuth` newtype in `orbital-ssr/src/lib.rs` |
| 2. Server handler codegen | done | `photon-macros/src/synced.rs` emits `#[cfg(feature = "ssr")]` module with `PATH` + generic `handler<S>`. Manual `notifications_ws_handler` deleted. |
| 3. Route collection | done | `apply_ws_routes::<S, Auth>()` in `photon-leptos/src/server/routes.rs` via quark auto-discovery. Replaces `ws_routes![]` macro. |
| 4. Effect-based replace | done | `synced_resource_replace` in `client.rs` writes WS payload directly to `RwSignal`, falls back to refetch. |
| 5. Shared subscription primitive | done | `use_topic_subscription()` → `PhotonSubscription { trigger, latest_event }` with `refetch()` method. |
| 6. Typed subscribe helper | done | `subscribe_<fn_name>(on_event)` generated by macro, returns `RwSignal<u64>` trigger. No cfg gates for caller. |
| 7. Migrate notification bell | done | Bell uses `subscribe_get_unread_count(move \|\| { ... })` with pagination reset in callback. |
| 8. Migrate counter | done | Counter page uses `subscribe_counter_get(\|\| {})`. |
| 9. Update docs | done | Guide 07 rewritten, design doc updated. |

#### Affected crates

- `photon-macros` — `subscribe_<fn_name>` codegen + `inventory::submit!(WsRouteDescriptor)`
- `photon-leptos` — `PhotonUserExtractor` trait, `WsRouteDescriptor`, `apply_ws_routes`, quark dep
- `orbital-ssr` — `PhotonAuth` newtype with trait impls
- `orbital-notifications-core` — uses `subscribe_get_unread_count`
- `counter-app` — uses `subscribe_counter_get`
- `server` — single `apply_ws_routes` call
- `docs` — updated `07-realtime-values.md`

#### What this supersedes

This item replaces the previous "Typed WebSocket endpoint enum for
subscribe_ws" item. That item proposed a typed enum for WS paths; this
design goes further by eliminating the need for developers to reference
WS paths at all — the macro handles everything end-to-end.

---

## References

- [Real-Time Values guide](../docs/src/02-counter-app-walkthrough/08-realtime-values.md) — Step-by-step walkthrough using the notification badge count
- [Photon Design](../photon/DESIGN.md) — Event pipeline
- [Orbital Overview](../orbital/README.md) — UI framework
- [Orbital API Usage](../docs/src/03-platform-guides/orbital.md) — Server functions and SSR
- [Leptos Resources](https://leptos.dev/book/06_resources.html) — Resource management
