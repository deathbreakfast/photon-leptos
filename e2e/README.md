# photon-leptos E2E

**Status:** implemented — browser tests under `e2e/tests/` exercise a minimal Leptos + Axum demo in `e2e/demo/`.

## Goal

Prove **publish → WebSocket → Leptos refetch** with a self-contained demo. No external app repos required.

## Layout

```
e2e/
  README.md       # this file
  demo/           # Leptos + Axum workspace member (mem Continuum transport)
  tests/          # Playwright specs + worker-scoped namespace fixture
```

All demo code — synced read fns, topics, WS routes, and test-only HTTP triggers — lives under `e2e/demo/`. Library crates do not gain test-only routes.

## Per-worker isolation (not global)

`cargo leptos end-to-end` runs **one** server; Playwright may run chromium/firefox/webkit × N workers in parallel. Counters are **not** process-global.

| Layer | Mechanism |
|-------|-----------|
| Server | `CounterStore { counters: DashMap<String, u64>, flags: DashMap<String, ScenarioFlags> }` keyed by `namespace` |
| Playwright | Worker fixture: `namespace = \`${project.name}-w${workerIndex}\`` |
| Page URL | `/?ns={namespace}&mode={scenario}` — namespace mirrored to an `e2e_ns` cookie for client server-fn refetches |
| Hygiene | `beforeEach`: `POST /api/counter/reset { namespace }` |

## Demo server (`e2e/demo/`)

| Piece | Description |
|-------|-------------|
| Photon boot | In-memory Continuum transport (`mem`) — same pattern as [photon Getting started](https://github.com/deathbreakfast/photon#getting-started) |
| App state | `HasPhoton` + `Arc<Photon>` + namespace-keyed `CounterStore` |
| Router | `photon_axum::ws_router::<AppState, HeadlessWsAuth>(app)` |
| Topic | `#[photon::topic(name = "counter.updated")]` on `CounterUpdated { namespace }` |
| Read fn | `counter_get` with `#[photon_leptos::synced(topic = "counter.updated", ws = "/ws/counter", auth = "none")]` |
| Client | `subscribe_counter_get` + `Resource::new` refetching `counter_get` |
| Publish trigger | `POST /api/counter/increment` — mutates store and publishes; Playwright calls via `request.post` |

### Test-only API routes

| Route | Purpose |
|-------|---------|
| `POST /api/counter/increment` | Bump counter; publish `CounterUpdated`; honor `fail_publish` |
| `POST /api/counter/increment-auth` | Bump counter; publish keyed `CounterAuthUpdated` |
| `POST /api/counter/reset` | Reset counter to 0; clear scenario flags |
| `POST /api/e2e/scenario` | Set per-namespace `fail_read` / `fail_publish` flags |

### Client `mode` query params

| `mode` | Behavior |
|--------|----------|
| *(default / happy)* | `subscribe_counter_get` + WS at `/ws/counter` |
| `no-ws` | SSR-only `Resource`; no subscription |
| `server-error` | Used with `fail_read` scenario flag (see sad-path tests) |

`/auth-mismatch` uses `counter_get_auth_user` (`auth = "user"`, WS `/ws/counter-auth`) with keyed publish on increment-auth.

## Running locally

```bash
cd e2e/tests && npm ci && npx playwright install --with-deps
cargo leptos end-to-end --project photon-leptos-e2e   # from workspace root
```

Chromium-only (faster iteration):

```bash
cargo leptos build --project photon-leptos-e2e
# in another terminal, from workspace root:
cargo leptos serve --project photon-leptos-e2e
cd e2e/tests && npx playwright test --project=chromium
```

Requires sibling `../photon` checkout (workspace path deps).

## Playwright specs

| File | Coverage |
|------|----------|
| `counter.happy.spec.ts` | publish → WS → refetch; second tab sync |
| `counter.sad.spec.ts` | server error, publish failure, no-ws, WS disconnect/reconnect, auth mismatch |

## CI

The `e2e` job in [`.github/workflows/ci.yml`](../.github/workflows/ci.yml) runs on every push and PR alongside `clippy` and `test`. It checks out sibling `photon`, installs Playwright (chromium, firefox, and webkit in CI), and runs `cargo leptos end-to-end`.
