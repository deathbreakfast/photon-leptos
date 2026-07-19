# photon-leptos benchmark experiment registry

Pre-registered **BM-PLS*** experiments for WebSocket + Leptos integration scaling.

**Methodology:** [`PERFORMANCE_STUDY.md`](PERFORMANCE_STUDY.md). **Substrate pairing:** run `photon-bench bm-p2` + `bm-pl2` on the same instance before interpreting WS-layer results.

**Fixed config:** `mem` storage, `embedded` backend, `telemetry=off`, `embedded-composite` topology, 256 B payload.

### Capacity at a glance (2-vCPU AWS DUT)

| Metric | Measured | Source |
|--------|----------|--------|
| **Sockets / server (per_subscribe)** | **256** observed knee | BM-PLS0 Campaign C (`aws-t3-medium`) |
| **Sockets / server (broadcast_hub)** | **768** observed knee | BM-PLS0-hub Campaign C |
| **Ops/s** | **10,000** measured publish/s @ N=64 | BM-PLS1 (`succeeded/elapsed`) |
| **WS delivery p99** | **~37–48 ms** @ N=256; **~44 ms** @ N=768 hub; Campaign B soak/PLS5 still apply below | PLS0 logs, PLS8, PLS5 |
| **Refetch p99** | **~32 ms** | BM-PLS3 Campaign B |
| **M sockets / client** | **≥256** | BM-PLS2 Campaign B |

Pass bar: WS p99 < 500 ms, delivery-loss / publish-fail < 0.1%, connect_fail = 0. Full narrative: [`PERFORMANCE_STUDY.md`](PERFORMANCE_STUDY.md) § Capacity at a glance / §3.

---

## Experiment log

| ID | Workload | Primary metric | Pass criteria | Status |
|----|----------|----------------|---------------|--------|
| **BM-PLS0** | WS subscriber sweep N @ 100/s and 1k/s | knee N, ws p99 | p99 < 500 ms, err < 0.1%, connect_fail = 0 | **measured** (observed knee **256** @ t3.medium, Campaign C) |
| **BM-PLS0-hub** | Same as PLS0 with `broadcast_hub` | knee N vs per_subscribe | same thresholds | **measured** (observed knee **768** @ t3.medium; hub lift) |
| **BM-PLS1** | Publish rate × N matrix | max **measured** ops/s at N | same thresholds + achieved ≥ 95% requested | **measured** (10k ops/s @ N=64) |
| **BM-PLS2** | M connections from one client | max M | connect fail = 0 | **measured** (M=256 @ t3.medium, Campaign B) |
| **BM-PLS3** | Refetch HTTP tax (synthetic + Playwright) | refetch p99 | err < 0.1% | **measured** (p99 32 ms @ t3.medium) |
| **BM-PLS4** | Payload 64B–4KB @ N=64 | ws p99 | delivery count > 0 | **measured** (pass @ t3.medium) |
| **BM-PLS5** | N clients × G key groups (`i % G`) | last PASS G, ws p99 | same as PLS0 | **measured** (G=256 @ N=256 both modes) |
| **BM-PLS5-hub** | PLS5 with `broadcast_hub` | G vs p99 under hub | same thresholds | **measured** (same G pass; p99 ≈ per_subscribe) |
| **BM-PLS6** | Keyed vs broadcast (both stay live) | both p99 | both phases pass | **measured** (pass @ t3.medium) |
| **BM-PLS7** | Reconnect storm | reconnect success | connected ≥ N/2 | **measured** (pass @ t3.medium) |
| **BM-PLS8** | Soak (default 300s lab / 3600s AWS) | err, disconnect | err < 1%, connect fail < 5% | **measured** (300s @ N=410 Campaign B; remount soak TODO) |
| **BM-PLS9** | Multi-server URLs (ALB smoke) | aggregate ws p99 | err < 0.1% | **measured** (2× c7i, N=256, pass) |

Fanout mode is selected via `BENCH_WS_MODE` / `POST /api/bench/mode` / `--ws-mode`. Experiments ending in `-hub` force `broadcast_hub`.

---

## Phase 1 conclusions (2026-07 AWS campaigns)

**Campaign A** `1a4c2dbd-…` — per_subscribe baseline on t3.small/medium. Notes: [`reports/campaign-1a4c2dbd-a9eb-4092-8e52-3df3bafd1e6f-notes.md`](reports/campaign-1a4c2dbd-a9eb-4092-8e52-3df3bafd1e6f-notes.md).

**Campaign B** `30841be8-…` — hub A/B, PLS5 G-sweep, client/shape/soak, c7i ceiling, PLS9. Notes: [`reports/campaign-30841be8-32e9-40bf-8547-823df3a9b5d0-notes.md`](reports/campaign-30841be8-32e9-40bf-8547-823df3a9b5d0-notes.md).

**Campaign C** `2b0693b7-…` — harness remount (process-per-N + honest metrics). Notes: [`reports/campaign-2b0693b7-d913-4804-aef4-e64d52714e95-notes.md`](reports/campaign-2b0693b7-d913-4804-aef4-e64d52714e95-notes.md).

**What we learned (Campaign C authoritative for PLS0/PLS1):**

- **Observed knee N = 256** (`per_subscribe`) and **768** (`broadcast_hub`) on t3.medium. Hub **raises** capacity for shared scopes.
- Campaign A/B “knee = 512” meant capped max PASS — **superseded**.
- **Still connection-bound:** PLS1 @ N=64 reached **10k measured** ops/s.
- **RQ-KEY-G / client / soak / PLS9:** Campaign B results still apply pending remount.
- **t4g** still skipped.

**Hub ergonomics takeaway:** Keep default `per_subscribe`. Enable hub when many sockets share one `(topic, key_filter)` — measured ~3× knee on t3.medium.

---

## Campaign slices

| Slice | Experiments |
|-------|-------------|
| `pls-substrate` | Pair with photon `bm-p2`, `bm-pl2` |
| `pls-connection` | BM-PLS0, BM-PLS1 |
| `pls-hub` | BM-PLS0, BM-PLS5, BM-PLS0-hub, BM-PLS5-hub |
| `pls-client` | BM-PLS2, BM-PLS3 |
| `pls-shape` | BM-PLS4, BM-PLS5, BM-PLS6, BM-PLS7 |
| `pls-soak` | BM-PLS8 |
| `pls-fleet` | BM-PLS9 |
| `pls-hardware` | PLS0 + PLS1 × Phase 1 profiles |

```bash
# Server (optional BENCH_WS_MODE=broadcast_hub)
BENCH_ADDR=0.0.0.0:8080 cargo run --release -p photon-leptos-bench --bin photon-leptos-bench-server

cargo run -p photon-leptos-bench -- experiments
cargo run -p photon-leptos-bench -- matrix --slice pls-hub \
  --server-url http://127.0.0.1:8080 --ws-mode per_subscribe
cargo run -p photon-leptos-bench -- run --experiment bm-pls5 \
  --ws-mode broadcast_hub --key-groups 16 --connections 256
```

---

## Cloud results matrix

Reports: `photon-leptos-bench/reports/` (+ per-profile subdirs). Campaign C knees are **observed** (FAIL after last PASS).

| Profile | Mode | PLS0 knee | PLS1 ops/s @64 | PLS5 last G @N=256 | Date | Notes |
|---------|------|-----------|----------------|--------------------|------|-------|
| `aws-t3-medium` | per_subscribe | **256** observed | **10000** measured | 256 (p99≈2ms) B | 2026-07-19 | Campaign C remount |
| `aws-t3-medium` | broadcast_hub | **768** observed | — | 256 (p99≈1ms) B | 2026-07-19 | Hub lift vs per_subscribe |
| `aws-t3-small` | per_subscribe | 512† | 10000‡ | — | 2026-07-01 | Campaign A; superseded for capacity |
| `aws-c7i-large` | both | 512† | 10000‡ | 256; G1 p99=5 / G256 p99=1 | 2026-07-19 | Campaign B; remount TODO |
| `aws-t4g-*` | — | — | — | — | 2026-07-01 | skipped |
| `aws-c7i-xlarge` | — | — | — | — | — | not run |

† Capped max PASS (pre-remount). ‡ Pre-remount achieved=requested bug.

**PLS9:** 2× `aws-c7i-large`, N=256, p99=3 ms, pass (2026-07-19 Campaign B).

**Capacity planning:** **256** sockets / 2-vCPU host (per_subscribe) → ~390 hosts for 100k viewers; **768** with hub on shared scopes → ~130 hosts.

---

## Phase 2 profiles (stubbed)

Registered in [`infra/aws/mcp/profiles.json`](../infra/aws/mcp/profiles.json) with `"wired": false`: `aws-c7i-2xlarge`, `aws-c7i-4xlarge`, `aws-c7i-8xlarge`, `aws-i4i-xlarge`.

---

## Follow-ups (post Campaign C)

| Priority | Question | Next step |
|----------|----------|-----------|
| P0 | Remount PLS0 on c7i; harden process-per-N timeout | Wall-clock kill on hung child probes |
| P1 | Soak at ~80% observed knee | PLS8 @ N≈205 / N≈614 |
| P1 | True 2× knee horizontal | PLS9 near 2×256 or 2×768 with ALB sticky |
| P2 | Wire-format slim / binary frames | Re-run PLS0 after payload work |
