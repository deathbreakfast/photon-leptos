# photon-leptos (Zone C)

Leptos + Axum WebSocket integration for **browser clients** consuming Photon topics.

This repo is **not** the core pub/sub library — that lives in [deathbreakfast/photon](https://github.com/deathbreakfast/photon) (Zone A).

## Crates

| Crate | Role |
|-------|------|
| `photon-leptos` | Client hooks (`subscribe_ws`, `synced_resource`) and server re-exports |
| `photon-axum` | Axum WebSocket routes, quark auto-discovery, `synced_ws_handler` |

## Local development

Clone sibling repos:

```bash
git clone https://github.com/deathbreakfast/photon.git ../photon
git clone https://github.com/deathbreakfast/photon-leptos.git
```

Workspace `[workspace.dependencies]` uses `path = "../photon/..."` for local builds. For downstream apps, depend via git and optional `[patch]`.

## Features

- `photon-leptos/hydrate` — client WebSocket subscription helpers
- `photon-leptos/ssr` — server WS route registration (via `photon-axum`)

## Related

- Unified Field template consumes this repo as Zone C (see `WEB_APP_TEMPLATE_MIGRATION.md` in photon repo).
- `#[photon::synced]` macro: planned as `photon-leptos-macros` or host patch until extracted from template vendor.
