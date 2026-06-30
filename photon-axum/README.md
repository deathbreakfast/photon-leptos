# photon-axum

Axum WebSocket integration for Photon browser clients — boot-time route registration and WS handlers.

**App authors** should start with the [repository README](../README.md) and `#[photon_leptos::synced]` — this crate is server wiring.

## Boot checklist

1. **Photon on app state** — implement [`HasPhoton`](src/axum_ws/state.rs) on your Axum `State` type:

   ```rust
   impl HasPhoton for AppState {
       fn photon_arc(&self) -> Arc<Photon> {
           Arc::clone(&self.photon)
       }
   }
   ```

2. **Synced routes in the binary** — ensure a linked crate uses `#[photon_leptos::synced]` so `inventory::submit!` registers [`WsRouteDescriptor`](src/axum_ws/descriptor.rs) entries.

3. **Merge WS routes at boot:**

   ```rust
   use photon_axum::{HeadlessWsAuth, ws_router};

   // Headless / demo — no user key extraction
   app = ws_router::<AppState, HeadlessWsAuth>(app);

   // Your auth newtype implements PhotonUserExtractor + FromRequestParts<S>
   // app = ws_router::<AppState, YourAuth>(app);
   ```

4. **Auth modes** — macro attribute `auth = "none"` vs `auth = "user"` selects [`WsAuthMode`](src/axum_ws/descriptor.rs). User routes call `auth.user_key()` for partition filtering.

## Exports

| Symbol | Role |
|--------|------|
| [`ws_router`](src/lib.rs) | Discover inventory routes and mount GET handlers |
| [`apply_ws_routes`](src/axum_ws/routes.rs) | Lower-level route registration |
| [`synced_ws_handler`](src/axum_ws/ws.rs) | Manual per-topic handler if not using inventory |
| [`PhotonUserExtractor`](src/axum_ws/auth.rs) | Host auth trait for `auth = "user"` |
| [`HeadlessWsAuth`](src/axum_ws/auth.rs) | No-op auth for demos and headless servers |

## Docs

`cargo doc -p photon-axum --features ssr --open`

## Features

| Feature | Purpose |
|---------|---------|
| `ssr` | Enable Axum WS handlers (required) |

Depends on [`photon-runtime`](https://github.com/deathbreakfast/photon) via workspace path or git.
