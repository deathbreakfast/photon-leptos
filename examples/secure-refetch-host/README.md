# secure-refetch-host

**Teaches:** Production-shaped WebSocket Origin allowlist + session cookie → `PhotonUserExtractor` for `auth = "user"` refetch.

**Topology:** Embedded (mem Photon). Same refetch path as quickstart, with secure host policy.

## Prerequisites

```bash
export PHOTON_TRANSPORT_KEY=cGhvdG9uLWRldi10cmFuc3BvcnQta2V5LTMyYnl0ZXM=
# Optional override (comma-separated). Defaults include http://127.0.0.1:3021 and http://localhost:3021
# export PHOTON_LEPTOS_ALLOWED_ORIGINS=http://127.0.0.1:3021
```

## Run

```bash
cargo leptos watch --split --project secure-refetch-host
```

1. Open <http://127.0.0.1:3021/>
2. Enter a user id → **Sign in** (sets demo `demo_session` cookie via `document.cookie`; production should use `Set-Cookie` with `HttpOnly` + `Secure` + `SameSite`)
3. Click **Increment** — only that user's partition updates
4. Unknown Origins are rejected (default `HasPhoton::allow_ws_origin` deny flipped to an allowlist)

**Open first:** [`src/state.rs`](src/state.rs) (`allow_ws_origin`) → [`src/auth.rs`](src/auth.rs)

**Success:** after sign-in, counter syncs over WS; server logs show Origin checks; signing in as another user isolates partitions.

**Next step:** [`../replace-and-append-demo`](../replace-and-append-demo/) for Replace / Append strategies.
