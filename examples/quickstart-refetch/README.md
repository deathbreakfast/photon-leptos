# quickstart-refetch

**Teaches:** In-process mem Photon + `#[photon_leptos::synced]` refetch over WebSocket — broadcast happy path, auth/key isolation, and subscription status / close.

**Topology:** Embedded (one Axum + Leptos process, mem Photon).

## Prerequisites

```bash
export PHOTON_TRANSPORT_KEY=cGhvdG9uLWRldi10cmFuc3BvcnQta2V5LTMyYnl0ZXM=
# cargo install cargo-leptos   # once
```

## Run

```bash
cargo leptos watch --split --project quickstart-refetch
```

Open <http://127.0.0.1:3020/> — click **Increment**; value updates via WS refetch (no page reload).

| Path | Proves |
|------|--------|
| `/` | Broadcast refetch + WS status / Close |
| `/auth?user=alice` | `auth = "user"` isolation |
| `/key?key=room-1` | client `?key=` partition |

**Open first:** [`src/synced.rs`](src/synced.rs) → [`src/main.rs`](src/main.rs)

**Success:** counter bumps after Increment; Status shows `Open`; Close → `Closed`.

**Next step:** [`../secure-refetch-host`](../secure-refetch-host/) for Origin allowlist + session extractor.
