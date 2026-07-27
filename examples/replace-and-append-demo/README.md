# replace-and-append-demo

**Teaches:** `strategy = "replace"` (payload becomes the resource value) vs `strategy = "append"` (live-tail list). Event `payload_json` must deserialize as the success type (`T` for `Result<T, E>`, or item `U` for `Result<Vec<U>, E>`).

**Topology:** Embedded (mem Photon).

## Prerequisites

```bash
export PHOTON_TRANSPORT_KEY=cGhvdG9uLWRldi10cmFuc3BvcnQta2V5LTMyYnl0ZXM=
```

## Run

```bash
cargo leptos watch --split --project replace-and-append-demo
```

Open <http://127.0.0.1:3022/>

| Section | Action | Success |
|---------|--------|---------|
| Replace | **Bump replace** | Snapshot updates from WS payload (no server-fn refetch) |
| Append | **Append line** | New line appears at the end of the list |

**Open first:** [`src/synced.rs`](src/synced.rs)

**Next step:** [`../brokered-live-ui`](../brokered-live-ui/) for NATS-backed Photon under a Leptos host.
