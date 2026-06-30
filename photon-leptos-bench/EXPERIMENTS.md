# photon-leptos benchmark experiment registry

Pre-registered **BM-PLS*** experiments for WebSocket + Leptos integration scaling.

**Methodology:** [`PERFORMANCE_STUDY.md`](PERFORMANCE_STUDY.md). **Substrate pairing:** run `photon-bench bm-p2` + `bm-pl2` on the same instance before interpreting WS-layer results.

**Fixed config:** `sqlite` storage, `embedded` backend, `telemetry=off`, `embedded-composite` topology, 256 B payload.

---

## Experiment log

| ID | Workload | Primary metric | Pass criteria | Status |
|----|----------|----------------|---------------|--------|
| **BM-PLS0** | WS subscriber sweep N @ 100/s and 1k/s | knee N, ws p99 | p99 < 500 ms, err < 0.1% | ready |
| **BM-PLS1** | Publish rate × N matrix | max ops/s at N | same thresholds | ready |
| **BM-PLS2** | M connections from one client | max M | connect fail = 0 | ready |
| **BM-PLS3** | Refetch HTTP tax (synthetic + Playwright) | refetch p99 | err < 0.1% | ready |
| **BM-PLS4** | Payload 64B–4KB @ N=64 | ws p99 | delivery count > 0 | ready |
| **BM-PLS6** | Keyed vs broadcast WS | ws p99 | broadcast delivery > 0 | ready |
| **BM-PLS7** | Reconnect storm | reconnect success | connected ≥ N/2 | ready |
| **BM-PLS8** | Soak (default 300s lab / 3600s AWS) | err, disconnect | err < 1%, connect fail < 5% | ready |
| **BM-PLS9** | Multi-server URLs (ALB smoke) | aggregate ws p99 | err < 0.1% | fleet |

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
| `pls-hardware` | PLS0 + PLS1 × all 7 Phase 1 profiles |

```bash
cargo run -p photon-leptos-bench -- experiments
cargo run -p photon-leptos-bench -- matrix --slice pls-connection --server-url http://127.0.0.1:8080
cargo run -p photon-leptos-bench -- hardware --profile aws-t3-medium
```

---

## Phase 1 cloud results (small–medium matrix)

Reports: `photon-leptos-bench/reports/`. Fill after AWS MCP campaign.

| Profile | BM-PLS0 knee @ 100/s | BM-PLS0 knee @ 1k/s | BM-PLS1 max ops/s @ N=64 | Date | Notes |
|---------|---------------------|---------------------|--------------------------|------|-------|
| `aws-t3-small` | | | | | |
| `aws-t3-medium` | | | | | baseline |
| `aws-t4g-small` | | | | | |
| `aws-t4g-medium` | | | | | |
| `aws-t4g-large` | | | | | |
| `aws-c7i-large` | | | | | |
| `aws-c7i-xlarge` | | | | | |

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
