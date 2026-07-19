# photon-leptos

Leptos client hooks and server re-exports for Photon realtime UI.

**Start here:** [repository README](../README.md) (hero example, Status, getting started).

**API reference:** `cargo doc -p photon-leptos --features ssr,hydrate --open`

**Integrator wiring:** [`photon-axum`](../photon-axum/README.md) for `ws_router`, auth, Origin, and fanout.

## Status (0.1 experimental)

| Strategy | Contract |
|----------|----------|
| **Refetch** | Supported — server function is authoritative |
| **Replace** | Experimental — payload is `T` or `Ok` of `Result<T, E>` (`synced_resource_replace_result`) |
| **Append** | Best-effort live tail — buffers during initial load; no cursor / replay |

The browser WebSocket is ephemeral. Prefer Refetch when exact state matters across reconnect.

## Client API map

| API | Role |
|-----|------|
| `#[synced]` / `use_<fn>()` | Macro-generated resource hook |
| `synced_resource` | Refetch or plain-`T` Replace |
| `synced_resource_replace_result` | Replace for `Result<T, E>` (Ok payload) |
| `synced_resource_append` | Best-effort list append |
| `subscribe_ws` → `PhotonWsHandle` | Raw subscription + status / last_error / close |
| `use_topic_subscription` → `PhotonSubscription` | Shared trigger + same observability signals |

## Features

| Feature | Purpose |
|---------|---------|
| `hydrate` | Browser WebSocket helpers (`leptos-use`) |
| `ssr` | Server re-exports of `photon-axum` + inventory routes |
