# photon-leptos WebSocket Integration Performance Study

Standalone methodology for **BM-PLS*** benchmarks measuring Axum WebSocket fanout and Leptos refetch overhead above the Photon pub/sub floor.

> **Summary:** Photon-bench quantifies publish/subscribe on Continuum. This study quantifies **WS delivery** (one `photon.subscribe()` + JSON frame per connection) and optional **server-fn refetch** tax.

---

## 1. Scope

| In scope | Out of scope |
|----------|--------------|
| `photon-leptos-bench` synthetic + Playwright subset | Photon storage/backend matrix |
| Fixed `sqlite` + `embedded` + `telemetry=off` | Distributed photon / BM-PF* |
| Phase 1 AWS small–medium (7 profiles) | Phase 2 large instances until enabled |
| BM-PLS0–PLS9 | Crypto/executor tuning (inherit photon results) |

---

## 2. System model

```text
POST /api/bench/publish  →  Photon publish  →  tailer fanout
                                                    ↓
                              N × synced_ws_handler  →  N WebSocket clients
                                                    ↓
                              (PLS3) HTTP refetch / Leptos Resource
```

Each WS connection in `photon-axum` allocates one tokio task and one Photon subscription stream.

---

## 3. Research questions

1. **RQ-WS-N:** How many WS subscribers can one server fan out to before degradation? → **BM-PLS0**
2. **RQ-WS-RATE:** Max publish rate at N connections? → **BM-PLS1**
3. **RQ-CLIENT-M:** How many connections can one client maintain? → **BM-PLS2**
4. **RQ-REFETCH:** Refetch vs replace latency tax? → **BM-PLS3**
5. **RQ-HW:** How does Phase 1 hardware change the knee? → **`pls-hardware` slice**
6. **RQ-Δ:** WS layer ms above photon BM-P2? → Pair PLS0 @ N∈{16,64,256} with substrate
7. **RQ-HORIZONTAL:** Multi-instance scaling? → **BM-PLS9**

---

## 4. Degradation thresholds

| Signal | FAIL when |
|--------|-----------|
| WS delivery p99 | > 500 ms |
| Error rate | > 0.1% |
| Connect fail rate | > 0% (PLS0/1) |

Knee = last PASS step in N or rate sweep.

---

## 5. Hardware profiles (Phase 1)

See [`infra/aws/mcp/profiles.json`](../infra/aws/mcp/profiles.json). CLI validation:

```bash
cargo run -p photon-leptos-bench -- hardware --profile aws-t3-medium
```

`matrix --slice pls-hardware` expands to 7 profiles × (PLS0 + PLS1).

---

## 6. Substrate pairing

On each AWS instance, before BM-PLS*:

```bash
cargo run -p photon-bench -- run --experiment bm-p2 --storage sqlite --backend embedded
cargo run -p photon-bench -- run --experiment bm-pl2 --storage sqlite --backend embedded --ops 60
```

Interpret WS overhead as PLS0 delivery p99 − BM-P2 drain p95 at the same N (order-of-magnitude; different harness paths).

---

## 7. AWS MCP orchestration

Full lifecycle documented in [`infra/aws/mcp/RUNBOOK.md`](../infra/aws/mcp/RUNBOOK.md): preflight → provision → SSM bootstrap → matrix → CloudWatch collect → S3 reports → teardown.

---

## References

- [photon-bench PERFORMANCE_STUDY](https://github.com/deathbreakfast/photon/blob/main/photon-bench/PERFORMANCE_STUDY.md)
- [photon-bench EXPERIMENTS](https://github.com/deathbreakfast/photon/blob/main/photon-bench/EXPERIMENTS.md)
