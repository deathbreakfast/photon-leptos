# photon-axum

Axum WebSocket integration for realtime resource sync.

## Exports

- `axum_ws` — route registration, `HeadlessWsAuth`, `HasPhoton`
- `ws_router` — merge WS routes onto an Axum router

`#[photon::synced]` is a product-layer macro — compile-error in this standalone crate.

## Status

Shipped. Depends on [`photon-runtime`](../photon-runtime/).
