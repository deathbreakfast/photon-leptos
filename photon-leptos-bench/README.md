# photon-leptos-bench

WebSocket + Leptos integration benchmarks (**BM-PLS***).

| Document | Role |
|----------|------|
| [`PERFORMANCE_STUDY.md`](PERFORMANCE_STUDY.md) | Methodology |
| [`EXPERIMENTS.md`](EXPERIMENTS.md) | Registry and cloud results |
| [`../infra/aws/mcp/RUNBOOK.md`](../infra/aws/mcp/RUNBOOK.md) | AWS MCP campaign |

## Quick start

```bash
# Terminal 1 — bench server (sqlite + embedded photon)
BENCH_ADDR=127.0.0.1:8080 cargo run -p photon-leptos-bench --bin photon-leptos-bench-server

# Terminal 2 — run experiment
cargo run -p photon-leptos-bench -- run --experiment bm-pls0 \
  --server-url http://127.0.0.1:8080 --hardware dev-wsl

cargo run -p photon-leptos-bench -- matrix --slice pls-connection \
  --server-url http://127.0.0.1:8080
```
