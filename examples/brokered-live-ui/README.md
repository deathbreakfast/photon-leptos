# brokered-live-ui

**Teaches:** Same Leptos refetch UI as the quickstart, but Photon storage is **NATS JetStream** instead of in-process mem — the distributed storage path under a browser host.

**Topology:** Brokered storage (one Leptos process + shared NATS). Skips cleanly when `PHOTON_NATS_URL` is unset.

## Prerequisites

```bash
export PHOTON_TRANSPORT_KEY=cGhvdG9uLWRldi10cmFuc3BvcnQta2V5LTMyYnl0ZXM=
docker run -d --name photon-nats -p 4222:4222 nats:2.10 -js
export PHOTON_NATS_URL=nats://127.0.0.1:4222
export PHOTON_NATS_STREAM=photon
export PHOTON_ALLOW_INSECURE_BROKER=1   # local plaintext only
```

## Run

```bash
cargo leptos watch --split --project brokered-live-ui
```

Open <http://127.0.0.1:3023/> — **Increment** publishes through NATS-backed Photon; the WS client still refetches via `#[synced]`.

Without `PHOTON_NATS_URL`, the binary prints the runbook and exits `Ok` (no panic).

**Open first:** [`src/photon_boot.rs`](src/photon_boot.rs)

**Success:** tracing shows NATS storage boot; counter updates after Increment.

**Next step:** Photon repo `nats_worker` / `nats_publisher` for split publisher–worker binaries; [`../secure-refetch-host`](../secure-refetch-host/) for Origin + session on the UI host.
