# photon-leptos WebSocket Integration Performance Study

Standalone methodology for **BM-PLS*** benchmarks measuring Axum WebSocket fanout and Leptos refetch overhead above the Photon pub/sub floor.

> **Summary:** Photon-bench quantifies publish/subscribe on Continuum. This study quantifies **WS delivery** (today: one `photon.subscribe()` + JSON frame per connection) and optional **server-fn refetch** tax. Phase 1 AWS results show the WS integration layer — not sqlite publish — is the knee on small instances.

---

## 1. Scope

| In scope (this repo) | Out of scope |
|---------------------|--------------|
| `photon-axum` WS bridge (`synced_ws_handler`) | Photon storage/backend matrix (see photon-bench) |
| `photon-leptos` client sync strategies (Refetch / Replace / Append) | Distributed Photon / BM-PF* |
| `photon-leptos-bench` synthetic + Playwright subset | Product-level fanout tier (ALB, CDN, regional shards) |
| Fixed `sqlite` + `embedded` + `telemetry=off` | Consumer-group worker scaling (different API + semantics) |
| Phase 1 AWS small–medium DUT profiles | Crypto/executor tuning (inherit photon results) |

**Scope boundary:** We can maximize **per-process WS fanout** and **per-tab client efficiency**. We cannot fix global capacity with code in this repo alone — that requires infra (more WS nodes, load balancing, sharding). Recommendations below are ordered by what **this codebase** controls.

---

## 2. System model

### 2.1 Current hot path (measured baseline)

```text
POST /api/bench/publish  →  Photon publish (1×)  →  sqlite append + broadcast tx
                                                              ↓
                    N × synced_ws_handler (one tokio task each)
                         each: photon.subscribe() → serde_json::to_string → WS Text send
                                                              ↓
                                              N synthetic / browser clients
```

Each WS connection in `photon-axum` today allocates **one tokio task** and **one full Photon subscription stream** (`photon.subscribe`, broadcast semantics). At N=512 that is 512 independent subscribe pipelines reading the same topic.

### 2.2 Broadcast vs consumer group (do not conflate)

Photon exposes two subscribe models:

| API | Semantics | Typical use |
|-----|-----------|-------------|
| `photon.subscribe()` | Every subscriber sees every event | WS live push, dashboards |
| `photon.subscribe_consumer_group()` | Each shard processed once across the group | Background workers, handlers |

A **WS broadcast hub** (one subscribe + serialize once + fan out bytes) applies only to the **broadcast lane on one process** for a given `(topic, key_filter)`. It does **not** replace consumer groups or imply one consumer for the whole deployment. Horizontally scaled WS nodes each run their own hub for local clients.

### 2.3 Optional Leptos refetch path (BM-PLS3)

```text
WS event  →  trigger bump  →  server-fn HTTP refetch  →  Resource update  →  UI
```

Refetch strategy adds **one HTTP round-trip per event per resource** on top of WS delivery.

---

## 3. Phase 1 findings (AWS campaign 2026-07)

Campaign `1a4c2dbd-a9eb-4092-8e52-3df3bafd1e6f` — reports under `photon-leptos-bench/reports/`.

| Profile | BM-PLS0 knee @ 100/s | BM-PLS0 knee @ 1k/s | BM-PLS1 max ops/s @ N=64 |
|---------|---------------------|---------------------|--------------------------|
| `aws-t3-small` (2 vCPU, 2 GiB) | **512** | **512** | **10,000** |
| `aws-t3-medium` (2 vCPU, 4 GiB) | **512** | **512** | **10,000** |

**Interpretation:**

1. **Knee is connection-count bound, not publish-rate bound** at these workloads. Raising publish rate from 100/s to 1k/s did not lower knee N. PLS1 still passed 10k ops/s at N=64. Publish path + sqlite is not the limiter for BM-PLS0 shape (publish load is R events/s, not N×R).
2. **t3.small ≈ t3.medium for WS knee** — same 2 vCPU; extra RAM did not move N. CPU per connection dominates, not memory headroom (within tested range).
3. **~512 concurrent WS subscribers** on a 2-vCPU burstable instance is the current architecture ceiling before p99 > 500 ms (degradation threshold). This is an **integration-layer** number, not Photon's raw pub/sub floor.
4. **Substrate pairing caveat:** `bm-pl2` OOM'd on 2 GiB DUTs during campaign; use `bm-p2` / local Continuum numbers when comparing sqlite headroom. Do not assume sqlite is the WS knee cause at 100–1000 publish/s.

---

## 4. Research questions

1. **RQ-WS-N:** How many WS subscribers can one server fan out to before degradation? → **BM-PLS0**
2. **RQ-WS-RATE:** Max publish rate at N connections? → **BM-PLS1**
3. **RQ-CLIENT-M:** How many connections can one client maintain? → **BM-PLS2**
4. **RQ-REFETCH:** Refetch vs replace latency tax? → **BM-PLS3**
5. **RQ-HW:** How does hardware change the knee? → **`pls-hardware` slice**
6. **RQ-Δ:** WS layer ms above photon BM-P2? → Pair PLS0 @ N∈{16,64,256} with substrate
7. **RQ-HUB:** Does broadcast hub mode raise knee N vs per-subscribe? → **future BM-PLS0 variant** (not yet implemented)
8. **RQ-HORIZONTAL:** Multi-instance scaling? → **BM-PLS9** (infra + bench; not a code-only fix)

---

## 5. Degradation thresholds

| Signal | FAIL when |
|--------|-----------|
| WS delivery p99 | > 500 ms |
| Error rate | > 0.1% |
| Connect fail rate | > 0% (PLS0/1) |

Knee = last PASS step in N or rate sweep.

---

## 6. Recommendations (what we control in this app)

Ordered by expected impact on BM-PLS0-style broadcast fanout. Items marked **(not done)** need implementation before re-benchmark.

### 6.1 High impact — `photon-axum`

| # | Change | Why | Caveat |
|---|--------|-----|--------|
| 1 | **Broadcast hub** for identical `(topic, key_filter)` WS clients: one `photon.subscribe()` → serialize once → fan out `Bytes` to per-socket bounded queues **(not done)** | Removes O(N) subscribe streams and O(N) JSON serialization for the common BM-PLS0 shape | Wrong for per-client `after_seq` replay, distinct key filters, or durable named subscriptions — keep per-connection `subscribe` as fallback |
| 2 | **Decouple read/write per socket**: dedicated send task + bounded mpsc so slow clients do not block the hub **(not done)** | Prevents one slow consumer from stalling the shared reader | Needs backpressure policy (drop vs disconnect slow clients) |
| 3 | **Release builds + `telemetry=off`** in production bench config | Already fixed in BM-PLS* matrix; verify in deployment | Obvious but easy to regress |

### 6.2 Medium impact — wire format & serialization

| # | Change | Why | Caveat |
|---|--------|-----|--------|
| 4 | **WS payload slimming**: send `payload_json` (+ minimal metadata) instead of full `Event` envelope when clients only need payload **(not done)** | Less bytes on wire and less serde work | Breaking change for clients expecting full Event; gate behind config |
| 5 | **Binary frames** (MessagePack / CBOR) instead of JSON text **(not done)** | Lower CPU and bandwidth vs `serde_json::to_string` | Browser client must decode; Leptos helpers today parse JSON |
| 6 | Reuse **pre-serialized `Arc<str>` / `Bytes`** from hub for all recipients | Serialize once per event, not once per socket | Requires hub (item 1) |

### 6.3 Medium impact — `photon-leptos` client

| # | Change | Why | Caveat |
|---|--------|-----|--------|
| 7 | Prefer **`SyncStrategy::Replace`** when WS payload is the full new value | Avoids HTTP refetch storm (BM-PLS3); WS-only update path | Not valid for auth-scoped / joined server queries |
| 8 | **`use_topic_subscription`** — one WS per tab, many Resources on one trigger | Already available; reduces client-side connection count | Server still has one socket per tab |
| 9 | Debounce / coalesce refetch triggers on bursty topics **(app pattern)** | Cuts server-fn load when events arrive faster than UI needs refresh | Adds staleness; product decision |

### 6.4 Lower impact / situational

| # | Change | Why | Caveat |
|---|--------|-----|--------|
| 10 | Smaller event payloads (BM-PLS4) | Linear reduction in serialize + send cost | Domain modeling, not framework default |
| 11 | Key-scoped WS endpoints (`key_filter` per connection) | Reduces events delivered to each socket | More endpoints / hub groups, not fewer total subscribers |
| 12 | Tune tokio worker threads / `SO_SNDBUF` | May help tail latency slightly | Will not fix O(N) subscribe architecture |

### 6.5 Out of repo scope

These matter for total product capacity but are **not** levers inside photon-leptos:

- **Horizontal WS scaling** — N servers × knee-per-server (BM-PLS9 measures smoke, not full fleet design)
- **ALB sticky sessions / shard routing** — which clients land on which hub
- **Photon consumer groups** — for worker throughput, not browser fanout
- **Continuum/sqlite tuning** — relevant for write-heavy apps, not the measured BM-PLS0 knee
- **Larger/fixed-CPU instances** (c7i+) — raises ceiling after code fixes; burstable t3 masks sustained load

**Planning math (illustrative):** At ~512 WS per 2-vCPU t3 with **current** code, 100k live viewers needs ~200 such nodes before hub improvements. A 10× knee improvement from hub + slim frames shifts that to ~20 nodes — still needs infra, but fewer machines.

---

## 7. Hardware profiles (Phase 1)

See [`infra/aws/mcp/profiles.json`](../infra/aws/mcp/profiles.json). CLI validation:

```bash
cargo run -p photon-leptos-bench -- hardware --profile aws-t3-medium
```

`matrix --slice pls-hardware` expands to registered Phase 1 profiles × (PLS0 + PLS1).

**Recommendation:** Use **t3.medium** as budget baseline DUT; use **c7i.large+** for ceiling testing after hub work lands (fixed CPU, less burst credit noise).

---

## 8. Substrate pairing

On each AWS instance, before BM-PLS*:

```bash
cargo run -p photon-bench -- run --experiment bm-p2 --storage sqlite --backend embedded
cargo run -p photon-bench -- run --experiment bm-pl2 --storage sqlite --backend embedded --ops 60
```

Interpret WS overhead as PLS0 delivery p99 − BM-P2 drain p95 at comparable load (order-of-magnitude; different harness paths). If `bm-pl2` OOMs on 2 GiB, note it and do not infer sqlite as WS bottleneck without direct evidence.

---

## 9. Validation plan after optimizations

When hub mode (or other axum changes) land:

1. Re-run **BM-PLS0** @ 100/s and 1k/s on `aws-t3-medium` and `aws-c7i-large` — compare knee N to Phase 1 baseline (512).
2. Add matrix flag or experiment variant: `per_subscribe` vs `broadcast_hub` (same thresholds).
3. Run **BM-PLS3** with Refetch vs Replace at knee N to quantify client-strategy tax separately from fanout.
4. Run **BM-PLS8** soak at ~80% of new knee to catch queue/memory leaks.

---

## 10. AWS MCP orchestration

Full lifecycle documented in [`infra/aws/mcp/RUNBOOK.md`](../infra/aws/mcp/RUNBOOK.md): preflight → provision → bootstrap → matrix → report collect → teardown.

---

## References

- [photon-bench PERFORMANCE_STUDY](https://github.com/deathbreakfast/photon/blob/main/photon-bench/PERFORMANCE_STUDY.md)
- [photon-bench EXPERIMENTS](https://github.com/deathbreakfast/photon/blob/main/photon-bench/EXPERIMENTS.md)
- Campaign notes: [`reports/campaign-1a4c2dbd-a9eb-4092-8e52-3df3bafd1e6f-notes.md`](reports/campaign-1a4c2dbd-a9eb-4092-8e52-3df3bafd1e6f-notes.md)
