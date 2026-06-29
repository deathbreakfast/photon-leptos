# photon-leptos

Leptos integration for real-time resource syncing with Photon events.

photon-leptos provides a helper crate that keeps Leptos resources in sync with Photon events, enabling automatic UI updates when events are published.

## Mental Model

* Real-time resource syncing
* WebSocket event forwarding
* Automatic refetch on events
* Declarative resource management

## Narrative

*photon-leptos keeps resources in sync when signals arrive.*

## Key Features

* **Synced Resources** — Leptos resources that automatically refetch on Photon events
* **WebSocket Integration** — Server-side WebSocket endpoint that forwards Photon events
* **Type-Safe** — Typed event handling with Photon topic types
* **Declarative** — Macro or helper function for easy integration

## Current State in Repo

photon-leptos is implemented and integrates with:
* **Photon** — For subscribing to events (see [photon](../photon/README.md))
* **Leptos** — For resource management and reactivity
* **Orbital** — For SSR utilities and server function patterns (see [orbital](../orbital/README.md))

### Implemented

| Component | Status |
|-----------|--------|
| `synced_resource()` helper | Done — Refetch, Replace (uses Refetch), Append (via `synced_resource_append`) |
| `synced_resource_append()` | Done — for appendable lists |
| `#[photon::synced]` macro | Done — generates `use_<fn_name>()` hook |
| Server WebSocket handler | Done — `synced_ws_handler` + `SyncedWsConfig` |
| Client WebSocket connection | Done — connect, listen, cleanup on drop |
| WebSocket reconnection | Done — automatic reconnect with backoff on disconnect |
| Key filter | Done — caller extracts user_id from auth session and passes as `key_filter` |

## Quick Start

Add `photon-leptos` with `hydrate` (client) and `ssr` (server) features, plus `photon` for the macro:

```toml
[dependencies]
photon = { path = "../photon" }
photon-leptos = { path = "../photon-leptos", features = ["hydrate", "ssr"] }
```

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
    // Query notifications with v
    Ok(vec![])
}

// The macro generates a client hook: use_list_notifications()
```

**Currently** the WebSocket handler and route must be registered manually
(see [DESIGN.md](DESIGN.md) for the current boilerplate).
A [Phase 3 enhancement](DESIGN.md#zero-boilerplate-real-time-resources-photonsynced-phase-3)
will automate handler generation and route registration so the macro
annotation above is the **only** step required.

## Documentation

**Walkthrough guide**: [Real-Time Values](../docs/src/02-counter-app-walkthrough/08-realtime-values.md)
walks through implementing the notification badge count as a case study,
covering every layer from Photon topic to SSR-safe reactive UI.

See [DESIGN.md](DESIGN.md) for the complete engineering design document, including:

* Synced resource pattern
* WebSocket endpoint integration
* Macro vs helper function API
* Integration with Orbital SSR
* Testing strategy
* Phased delivery plan

## Related Crates

* `photon` — Event pipeline (see [photon](../photon/README.md))
* `photon-leptos` — Leptos integration (this crate)
* `orbital` — SSR utilities and server functions (see [orbital](../orbital/README.md))

## Integration Points

* **Photon** — Subscribes to events for real-time delivery
* **Leptos** — Resource management and reactivity
* **Orbital** — Server functions and SSR context (see [orbital](../orbital/README.md))
* **Axum** — WebSocket endpoint under `/ws/*`
