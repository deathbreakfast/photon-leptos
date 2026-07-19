# photon-leptos-bench

WebSocket + Leptos integration benchmarks (**BM-PLS***).

| Document | Role |
|----------|------|
| [`PERFORMANCE_STUDY.md`](PERFORMANCE_STUDY.md) | Methodology |
| [`EXPERIMENTS.md`](EXPERIMENTS.md) | Registry and cloud results |
| [`../infra/aws/mcp/RUNBOOK.md`](../infra/aws/mcp/RUNBOOK.md) | AWS MCP campaign |

## Quick start

```bash
# Terminal 1 — bench server (mem + embedded photon)
# Loopback opens the control plane by default. For non-loopback bind, set
# BENCH_CONTROL_TOKEN and send header x-bench-token from the client.
# Optional: BENCH_WS_MODE=broadcast_hub
BENCH_ADDR=127.0.0.1:8080 cargo run -p photon-leptos-bench --bin photon-leptos-bench-server

# Terminal 2 — run experiment (mode can be switched via API)
cargo run -p photon-leptos-bench -- run --experiment bm-pls0 \
  --server-url http://127.0.0.1:8080 --hardware dev-wsl
```

## Control plane (SEC)

Loopback binds open control routes by default (`BENCH_CONTROL_OPEN=1`). For
non-loopback listen addresses, set `BENCH_CONTROL_TOKEN` and send header
`x-bench-token` on `POST /api/bench/publish` and `POST /api/bench/mode`.

Publish requests are capped (rate / duration / payload / key groups) and only
one publish run may be in flight (`409 Conflict` if busy). Data-plane
`GET /ws/bench` stays unauthenticated relative to the control token.
