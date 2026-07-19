# photon-leptos WebSocket Integration Performance Study

Standalone methodology for **BM-PLS*** benchmarks measuring Axum WebSocket fanout and Leptos refetch overhead above the Photon pub/sub floor.

> **Summary:** Photon-bench quantifies publish/subscribe on Photon storage adapters. This study quantifies **WS delivery** under `per_subscribe` and optional **broadcast hub**, plus **server-fn refetch** tax. After fixing the PLS0 harness (process-per-N search until FAIL + honest connect/delivery metrics), Campaign C on `aws-t3-medium` measures an **observed** per_subscribe knee at **N=256** and a hub knee at **N=768**. Prior Campaign A/B “~512” figures were capped max PASS, not observed fails — superseded here for t3.medium PLS0.

### Capacity at a glance (2-vCPU AWS DUT)

Measured on `aws-t3-medium` (Campaign C remount, 2026-07-19) unless noted. Pass threshold: WS delivery p99 < 500 ms, delivery-loss / publish-fail < 0.1%, connect_fail = 0.

| Metric | Value | Notes |
|--------|-------|-------|
| **Sockets / server (per_subscribe)** | **256** observed knee | PLS0; FAIL at 512 (connect_fail > 0) |
| **Sockets / server (broadcast_hub)** | **768** observed knee | PLS0-hub; PASS @ 768; N=1024 probe timed out → FAIL |
| **Ops/s** | **10,000** measured publish/s @ N=64 | PLS1 `succeeded/elapsed` (not requested rate) |
| **WS p99 @ N=256** | **~37–48 ms** | per_subscribe @ 100/s–1k/s |
| **WS p99 @ N=768 hub** | **~44 ms** | broadcast_hub @ 100/s |
| **WS p99 @ N=410 soak** | **~40 ms** (p50 ~8–9 ms) | PLS8 Campaign B (pre-remount soak; N≈80% of old capped 512) |
| **WS p99 @ N=256, G=1** | **~5 ms** | Campaign B PLS5 |
| **WS p99 @ N=256, G=N** | **~1 ms** | Campaign B PLS5 |
| **Refetch p99** | **~32 ms** | Campaign B PLS3 |
| **M sockets / client** | **≥256** | Campaign B PLS2 |

**Planning:** At **256** viewers per 2-vCPU host (per_subscribe knee), 100k live viewers needs **~390** such nodes. Hub at **768** cuts that to **~130** nodes when all sockets share one `(topic, key_filter)`.

---

## 1. Scope

| In scope (this repo) | Out of scope |
|---------------------|--------------|
| `photon-axum` WS bridge (`synced_ws_handler`) | Photon storage/backend matrix (see photon-bench) |
| `photon-leptos` client sync strategies (Refetch / Replace / Append) | Distributed Photon / BM-PF* |
| `photon-leptos-bench` synthetic + Playwright subset | Product-level fanout tier (ALB, CDN, regional shards) |
| Fixed `mem` + `embedded` + `telemetry=off` | Consumer-group worker scaling (different API + semantics) |
| Phase 1 AWS small–medium DUT profiles | Crypto/executor tuning (inherit photon results) |

**Scope boundary:** We can maximize **per-process WS fanout** and **per-tab client efficiency**. We cannot fix global capacity with code in this repo alone — that requires infra (more WS nodes, load balancing, sharding). Recommendations below are ordered by what **this codebase** controls.

---

## 2. System model

### 2.1 Current hot path (measured baseline)

```text
POST /api/bench/publish  →  Photon publish (1×)  →  mem append + broadcast tx
                                                              ↓
                    N × synced_ws_handler (one tokio task each)
                         each: photon.subscribe() → serde_json::to_string → WS Text send
                                                              ↓
                                              N synthetic / browser clients
```

Each WS connection in `photon-axum` today allocates **one tokio task** and **one full Photon subscription stream** (`photon.subscribe`, broadcast semantics). At N=512 that is 512 independent subscribe pipelines reading the same topic.

### 2.2 “Shared WS connection pool” (naming → what to build)

Older roadmap shorthand **“shared WS connection pool”** meant: **do not duplicate subscribe/serialize work** when many clients (or many UI resources) want the same `(topic, key_filter)`. It is a **performance** track, not auth/key correctness. Two layers are easy to conflate:

| Layer | Idea | Status | How we measure |
|-------|------|--------|----------------|
| **Client** (`photon-leptos`) | One browser WebSocket per tab, shared by many Resources via [`use_topic_subscription`](../photon-leptos/src/client/mod.rs) | **Already available** | **BM-PLS2** (max M sockets from one client); app pattern in §6.3 |
| **Server** (`photon-axum`) | One Photon `subscribe` + serialize once per `(topic, key_filter)` in-process, then fan out frames to many sockets (**broadcast hub**) | **Implemented** — `WsBroadcastHub` + `WsFanoutMode`; default remains **per_subscribe** | **BM-PLS0** vs **BM-PLS0-hub**; key cardinality via **BM-PLS5** / **BM-PLS5-hub** (§4 RQ-HUB, RQ-KEY-G) |

**What it is not:** a durable Photon `subscription_name` / checkpoint pool, a cross-process connection multiplexor, or “one socket for the whole fleet.” Horizontally scaled WS nodes each keep their own hub for local clients (see §2.3). Distinct `key_filter` values (auth-scoped or `?key=`) remain separate hub groups — pooling only merges **identical** subscribe scopes.

When docs or issues say “shared WS pool,” prefer the concrete terms **client topic subscription** and **server broadcast hub** above.

### 2.3 Broadcast vs consumer group (do not conflate)

Photon exposes two subscribe models:

| API | Semantics | Typical use |
|-----|-----------|-------------|
| `photon.subscribe()` | Every subscriber sees every event | WS live push, dashboards |
| `photon.subscribe_consumer_group()` | Each shard processed once across the group | Background workers, handlers |

A **WS broadcast hub** (one subscribe + serialize once + fan out bytes) applies only to the **broadcast lane on one process** for a given `(topic, key_filter)`. It does **not** replace consumer groups or imply one consumer for the whole deployment. Horizontally scaled WS nodes each run their own hub for local clients. This hub **is** the server half of “shared WS connection pool” (§2.2).

### 2.4 Optional Leptos refetch path (BM-PLS3)

```text
WS event  →  trigger bump  →  server-fn HTTP refetch  →  Resource update  →  UI
```

Refetch strategy adds **one HTTP round-trip per event per resource** on top of WS delivery.

---

## 3. Phase 1–2 findings (AWS campaigns 2026-07)

**Campaign A** `1a4c2dbd-…` (per_subscribe baseline). **Campaign B** `30841be8-…` (hub + PLS5 + c7i + PLS9). **Campaign C** `2b0693b7-…` (harness remount — observed knees). Notes under `photon-leptos-bench/reports/`.

### Campaign C (authoritative PLS0 / PLS1 on t3.medium)

| Profile | Mode | PLS0 knee | knee_kind | PLS1 ops/s @ N=64 |
|---------|------|-----------|-----------|-------------------|
| `aws-t3-medium` | per_subscribe | **256** | observed | **10,000** measured |
| `aws-t3-medium` | broadcast_hub | **768** | observed | — |

Knee = last PASS before an observed FAIL (or timed-out probe treated as FAIL). Process-per-N continues past the old in-process cap of 512.

### Campaign A/B (superseded for PLS0 capacity; still useful for PLS2–9)

| Profile | Mode | PLS0 “knee”† | PLS1 ops/s @ N=64 | PLS5 last G @ N=256 |
|---------|------|--------------|-------------------|---------------------|
| `aws-t3-small` | per_subscribe | 512† | 10,000‡ | — |
| `aws-t3-medium` | per_subscribe | 512† | 10,000‡ | **256** (p99≈2 ms) |
| `aws-t3-medium` | broadcast_hub | 512† | — | **256** (p99≈1 ms) |
| `aws-c7i-large` | per_subscribe | 512† | 10,000‡ | **256** (G1 p99=5 / G256 p99=1) |
| `aws-c7i-large` | broadcast_hub | 512† | 10,000‡ | **256** (G1 p99=5 / G256 p99=1) |

† Last PASS in harness sweep **capped at N=512** — not an observed fail. ‡ Requested rate copied into `achieved_ops_per_sec` (pre-remount metric bug).

**Interpretation (post Campaign C):**

1. **Knee is connection-count bound** — still fails on connect/delivery before publish rate at moderate N; PLS1 still reaches 10k measured ops/s @ N=64.
2. **RQ-HUB:** Hub **raises** knee on t3.medium (**768 vs 256**) when sockets share one broadcast scope.
3. **RQ-KEY-G:** Campaign B at N=256 still stands — G=1…256 PASS; p99 set by delivery fanout.
4. **Prior “~512 sockets/server” and “~200 hosts for 100k”** are withdrawn; use Capacity at a glance above.
5. **Client / shape / soak (Campaign B):** PLS2–8 results unchanged pending remount; soak N=410 was ~80% of the old capped 512 — prefer ~80% of **256** (≈205) or hub **768** (≈614) next soak.
6. **RQ-HORIZONTAL / RQ-HW:** Campaign B smoke only; remount on c7i still TODO.

---

## 4. Research questions

1. **RQ-WS-N:** How many WS subscribers can one server fan out to before degradation? → **BM-PLS0**
2. **RQ-WS-RATE:** Max publish rate at N connections? → **BM-PLS1**
3. **RQ-CLIENT-M:** How many connections can one client maintain? → **BM-PLS2**
4. **RQ-REFETCH:** Refetch vs replace latency tax? → **BM-PLS3**
5. **RQ-HW:** How does hardware change the knee? → **`pls-hardware` slice** / prefer `aws-c7i-large`
6. **RQ-Δ:** WS layer ms above photon BM-P2? → Pair PLS0 @ N∈{16,64,256} with substrate
7. **RQ-HUB:** Does server **broadcast hub** raise knee N vs per-subscribe? → **BM-PLS0-hub** (`--ws-mode` / `BENCH_WS_MODE`)
8. **RQ-KEY-G:** How does the working set of distinct `(topic, key_filter)` groups affect capacity? Hub gains collapse as `G → N`. → **BM-PLS5** / **BM-PLS5-hub** (fixed N, sweep G)
9. **RQ-HORIZONTAL:** Multi-instance scaling? → **BM-PLS9** (infra + bench; smoke, not full fleet design)

### 4.1 Tightened campaign order

| Priority | Question | Experiments |
|----------|----------|-------------|
| P0 | Knee on fixed-CPU (c7i)? Still connection-bound? | PLS0, PLS0-hub, PLS1 |
| P1 | Key working-set; M sockets; refetch; soak | PLS5/5-hub, PLS2, PLS3, PLS8 |
| P2 | Payload / keyed / reconnect; 2-host projection | PLS4, PLS6, PLS7, PLS9 |
| P3 | Post-optimization re-validate | Re-run P0 after further wire-format work |

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
| 1 | **Broadcast hub** (server “shared WS pool”) for identical `(topic, key_filter)` WS clients: one `photon.subscribe()` → serialize once → fan out to per-socket bounded queues **(done)** | Removes O(N) subscribe streams and O(N) JSON serialization for the common BM-PLS0 shape | Distinct key filters stay separate hub groups — measure with **BM-PLS5**; enable via `PHOTON_AXUM_WS_FANOUT=broadcast_hub` + `HasPhoton::ws_hub` |
| 2 | **Decouple read/write per socket**: hub reader + bounded mpsc; slow clients disconnected on full queue **(done)** | Prevents one slow consumer from stalling the shared reader | Disconnect (not drop-oldest); tune `HUB_QUEUE_CAPACITY` if needed |
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
| 8 | **`use_topic_subscription`** (client “shared WS pool”) — one WS per tab, many Resources on one trigger | Already available; reduces client-side connection count | Server still has one socket per tab until hub (item 1) |
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
- **Host-app / sqlite tuning** — relevant for write-heavy apps, not the measured BM-PLS0 knee
- **Larger/fixed-CPU instances** (c7i+) — raises ceiling after code fixes; burstable t3 masks sustained load

**Planning math:** At **256** WS per 2-vCPU host (per_subscribe observed knee) or **768** with broadcast hub on a shared scope, 100k live viewers needs ~390 or ~130 such nodes respectively.

### 6.6 Fleet sizing cheat sheet (devs)

Size for **peak concurrent WebSocket clients**, not registered users. Each Axum process only serves sockets connected to it (LB **sticky sessions** required). Photon does not route an event to “the right” host: every node subscribed to that `(topic, key_filter)` receives it and fans out **locally**.

| App shape | Which knee | Concurrent sockets / 2-vCPU host | Example: 1k concurrent clients |
|-----------|------------|----------------------------------|--------------------------------|
| **Shared live feed** — many sockets, **same** topic + **same** key | Hub **~768** | Use hub | **2 hosts** enough (~500 sticky each under 768) |
| **Per-user push** — same topic, **unique** key per user (notifications) | Per-subscribe **~256** (hub ≈ no help; `G → N`) | Unique keys = separate hub groups | **~4 hosts** (1000 ÷ 256); **2 hosts ≈ 512** only |
| Mix | Weighted by share of shared vs unique scopes | — | Shared cohort can sit denser on hub-enabled nodes |

Horizontal capacity ≈ **(# Axum WS nodes) × (knee for that traffic shape)**. Hub is **process-local**; it never pools sockets across hosts.

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

## 9. Validation results (hub Campaign B)

Campaign `30841be8-32e9-40bf-8547-823df3a9b5d0` completed P0–P2 under the **old** capped harness. Campaign `2b0693b7-d913-4804-aef4-e64d52714e95` remounted PLS0/PLS1:

| Check | Result |
|-------|--------|
| PLS0 per_subscribe on t3.medium | **observed knee 256** |
| PLS0-hub on t3.medium | **observed knee 768** (hub lift vs per_subscribe) |
| PLS1 @ N=64 | **10k measured** ops/s |
| PLS5 / client / soak / PLS9 | Still Campaign B (not remounted) |

### Hub ergonomics (post-campaign)

| Surface | Behavior / takeaway |
|---------|---------------------|
| Default | Keep **`PerSubscribe`** for correctness; capacity knee is lower |
| Enable hub when | Many sockets, **identical** `(topic, key_filter)` — **~3×** knee on t3.medium (768 vs 256) |
| Do not expect gains when | Unique keys (`G ≈ N`) — Campaign B p99 tracks delivery fanout |
| Env / API | `PHOTON_AXUM_WS_FANOUT`, `HasPhoton::ws_hub()`, bench `--ws-mode` / `BENCH_WS_MODE` |

### Next measurements

1. Remount PLS0 on `aws-c7i-large`; add wall-clock timeout on process-per-N probes (N=1024 hub hung).
2. Soak at ~80% of **observed** knee (≈205 per_subscribe / ≈614 hub).
3. PLS9 near 2× measured knee with ALB sticky.

---

## 10. AWS MCP orchestration

Full lifecycle documented in [`infra/aws/mcp/RUNBOOK.md`](../infra/aws/mcp/RUNBOOK.md): preflight → provision → bootstrap → matrix → report collect → teardown.

---

## References

- [photon-bench PERFORMANCE_STUDY](https://github.com/unified-field-dev/photon/blob/main/photon-bench/PERFORMANCE_STUDY.md)
- [photon-bench EXPERIMENTS](https://github.com/unified-field-dev/photon/blob/main/photon-bench/EXPERIMENTS.md)
- Campaign A notes: [`reports/campaign-1a4c2dbd-a9eb-4092-8e52-3df3bafd1e6f-notes.md`](reports/campaign-1a4c2dbd-a9eb-4092-8e52-3df3bafd1e6f-notes.md)
- Campaign B notes: [`reports/campaign-30841be8-32e9-40bf-8547-823df3a9b5d0-notes.md`](reports/campaign-30841be8-32e9-40bf-8547-823df3a9b5d0-notes.md)
- Campaign C notes: [`reports/campaign-2b0693b7-d913-4804-aef4-e64d52714e95-notes.md`](reports/campaign-2b0693b7-d913-4804-aef4-e64d52714e95-notes.md)
