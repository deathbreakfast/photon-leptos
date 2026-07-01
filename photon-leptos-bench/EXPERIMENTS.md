# photon-leptos benchmark experiment registry

Pre-registered **BM-PLS*** experiments for WebSocket + Leptos integration scaling.

**Methodology:** [`PERFORMANCE_STUDY.md`](PERFORMANCE_STUDY.md). **Substrate pairing:** run `photon-bench bm-p2` + `bm-pl2` on the same instance before interpreting WS-layer results.

**Fixed config:** `sqlite` storage, `embedded` backend, `telemetry=off`, `embedded-composite` topology, 256 B payload.

---

## Experiment log

| ID | Workload | Primary metric | Pass criteria | Status |
|----|----------|----------------|---------------|--------|
| **BM-PLS0** | WS subscriber sweep N @ 100/s and 1k/s | knee N, ws p99 | p99 < 500 ms, err < 0.1% | **baseline measured** |
| **BM-PLS1** | Publish rate × N matrix | max ops/s at N | same thresholds | **baseline measured** |
| **BM-PLS2** | M connections from one client | max M | connect fail = 0 | ready |
| **BM-PLS3** | Refetch HTTP tax (synthetic + Playwright) | refetch p99 | err < 0.1% | ready |
| **BM-PLS4** | Payload 64B–4KB @ N=64 | ws p99 | delivery count > 0 | ready |
| **BM-PLS6** | Keyed vs broadcast WS | ws p99 | broadcast delivery > 0 | ready |
| **BM-PLS7** | Reconnect storm | reconnect success | connected ≥ N/2 | ready |
| **BM-PLS8** | Soak (default 300s lab / 3600s AWS) | err, disconnect | err < 1%, connect fail < 5% | ready |
| **BM-PLS9** | Multi-server URLs (ALB smoke) | aggregate ws p99 | err < 0.1% | fleet |
| **BM-PLS0-hub** _(proposed)_ | Same as PLS0 with broadcast hub WS mode | knee N vs per-subscribe | same thresholds | **not implemented** |

---

## Phase 1 conclusions (2026-07 AWS campaign)

Campaign `1a4c2dbd-a9eb-4092-8e52-3df3bafd1e6f` — full notes in [`reports/campaign-1a4c2dbd-a9eb-4092-8e52-3df3bafd1e6f-notes.md`](reports/campaign-1a4c2dbd-a9eb-4092-8e52-3df3bafd1e6f-notes.md).

**What we learned:**

- **Knee N ≈ 512** on both `aws-t3-small` and `aws-t3-medium` at 100/s and 1k/s publish rates.
- **PLS1 @ N=64 reached 10k ops/s** — publish/sqlite is not the first bottleneck for broadcast WS fanout at moderate N.
- **Instance size (2 vs 4 GiB) did not change knee** — same 2 vCPU; WS path is CPU/subscribe-count bound.
- **t4g profiles skipped** — substrate OOM on 2 GiB + arm64 campaign friction; not a statement about Graviton WS performance.

**Primary bottleneck (current code):** `photon-axum` `synced_ws_handler` — one `photon.subscribe()` + `serde_json::to_string` + WS send **per connection**. See [`photon-axum/src/axum_ws/ws.rs`](../photon-axum/src/axum_ws/ws.rs).

**Not the bottleneck (for BM-PLS0 shape):** Photon publish rate at R=100–1000/s with N≤512; Continuum sqlite headroom at those publish rates (substrate `bm-pl2` OOM on 2 GiB is a memory floor issue, not evidence that sqlite limits WS fanout).

---

## Recommendations (actionable in this repo)

Prioritized changes to improve BM-PLS0 knee and real-app WS capacity. Details in [`PERFORMANCE_STUDY.md` §6](PERFORMANCE_STUDY.md#6-recommendations-what-we-control-in-this-app).

### Implement next (photon-axum)

1. **Broadcast hub mode** — one subscribe per `(topic, key_filter)` per process; serialize once; fan out to socket send queues. Fallback to per-connection subscribe when clients need distinct cursors, key filters, or durable subscription names.
2. **Per-socket send tasks** with bounded backpressure so slow clients do not block the hub reader.
3. **Bench toggle** — `BM-PLS0-hub` or `--ws-mode hub|per_subscribe` to regression-test both paths.

### Application patterns (photon-leptos)

4. Use **`SyncStrategy::Replace`** when the WS payload is sufficient — measure tax with **BM-PLS3**.
5. Use **`use_topic_subscription`** so one tab opens one WS for multiple Resources (already supported).
6. Avoid refetch-on-every-event for high-frequency topics; debounce at the app layer.

### Wire format (optional, after hub)

7. Slim WS envelope (payload-only) and/or binary codec — requires client helper updates.

### Benchmark hygiene

8. Re-run PLS0/PLS1 on **`aws-c7i-large`** after hub lands (fixed CPU, clearer ceiling than burstable t3).
9. Complete **BM-PLS8** soak at ~80% knee before claiming production readiness.

### Explicitly out of scope here

- ALB / multi-region WS routing (**BM-PLS9** smoke only)
- Photon consumer-group worker scaling
- sqlite / Continuum tuning for write-heavy workloads

---

## Campaign slices

| Slice | Experiments |
|-------|-------------|
| `pls-substrate` | Pair with photon `bm-p2`, `bm-pl2` |
| `pls-connection` | BM-PLS0, BM-PLS1 |
| `pls-client` | BM-PLS2, BM-PLS3 |
| `pls-shape` | BM-PLS4, BM-PLS6, BM-PLS7 |
| `pls-soak` | BM-PLS8 |
| `pls-fleet` | BM-PLS9 |
| `pls-hardware` | PLS0 + PLS1 × Phase 1 profiles |

```bash
cargo run -p photon-leptos-bench -- experiments
cargo run -p photon-leptos-bench -- matrix --slice pls-connection --server-url http://127.0.0.1:8080
cargo run -p photon-leptos-bench -- hardware --profile aws-t3-medium
```

---

## Phase 1 cloud results (small–medium matrix)

Reports: `photon-leptos-bench/reports/`. Campaign completed for t3 profiles only.

| Profile | BM-PLS0 knee @ 100/s | BM-PLS0 knee @ 1k/s | BM-PLS1 max ops/s @ N=64 | Date | Notes |
|---------|---------------------|---------------------|--------------------------|------|-------|
| `aws-t3-small` | 512 | 512 | 10000 | 2026-07-01 | substrate bm-pl2 OOM (2 GiB); per-subscribe WS baseline |
| `aws-t3-medium` | 512 | 512 | 10000 | 2026-07-01 | same knee as t3.small — RAM not limiting |
| `aws-t4g-small` | — | — | — | 2026-07-01 | **skipped** — substrate OOM / arm64 build issues |
| `aws-t4g-medium` | — | — | — | 2026-07-01 | **skipped** — campaign stopped after t3 results |
| `aws-t4g-large` | | | | | not run |
| `aws-c7i-large` | | | | | **recommended** post-hub ceiling DUT |
| `aws-c7i-xlarge` | | | | | not run |

**Capacity planning (current architecture):** ~512 concurrent broadcast WS clients per 2-vCPU t3 DUT before p99 > 500 ms. Scaling to larger audiences requires **code changes (hub)** and/or **more WS nodes** — not larger t3 RAM alone.

---

## Phase 2 profiles (stubbed)

Registered in [`infra/aws/mcp/profiles.json`](../infra/aws/mcp/profiles.json) with `"wired": false`: `aws-c7i-2xlarge`, `aws-c7i-4xlarge`, `aws-c7i-8xlarge`, `aws-i4i-xlarge`.

---

## Run examples

```bash
# Server
BENCH_ADDR=0.0.0.0:8080 cargo run --release -p photon-leptos-bench --bin photon-leptos-bench-server

# Connection sweep
cargo run -p photon-leptos-bench -- run --experiment bm-pls0 \
  --hardware aws-t3-medium --server-url http://127.0.0.1:8080 \
  --report photon-leptos-bench/reports/bm-pls0-sqlite-embedded-aws-t3-medium.json

# ALB / multi-server
cargo run -p photon-leptos-bench -- run --experiment bm-pls9 \
  --server-urls http://10.0.1.10:8080,http://10.0.1.11:8080 \
  --connections 256 --hardware aws-t3-medium
```

Pair with substrate:

```bash
# On same EC2 before PLS*
cargo run -p photon-bench -- run --experiment bm-p2 --storage sqlite --backend embedded --hardware aws-t3-medium
cargo run -p photon-leptos-bench -- run --experiment bm-pls0 --substrate-report photon-bench/reports/bm-p2-....json
```

After hub mode exists:

```bash
# Proposed — compare modes on same hardware
cargo run -p photon-leptos-bench -- run --experiment bm-pls0 \
  --ws-mode broadcast_hub --hardware aws-c7i-large --server-url http://127.0.0.1:8080
```
