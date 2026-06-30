# photon-leptos ROADMAP

## Shipped

- `synced_resource` / `synced_resource_append` / replace strategy
- `#[photon_leptos::synced]` macro + `subscribe_<fn>` helpers
- `photon-axum` inventory auto-discovery + `ws_router`
- `PhotonUserExtractor` generic auth
- `use_topic_subscription` shared subscription primitive
- Effect-based replace strategy (direct payload write, refetch fallback on deser error)
- Unit tests: `SyncStrategy::from_str`, WS path derivation, macro expansion
- Browser E2E demo + Playwright (see [`e2e/README.md`](e2e/README.md))

## Open

- Integration test: publish → WS → Resource refetch
- WS handler integration tests
- Shared WS connection pool for duplicate paths (optimization)
- LLVM coverage baseline (optional)

## Maintainer

- Sentrux `scan` + `check_rules` before large changes (see [`.sentrux/rules.toml`](.sentrux/rules.toml))
- CI commands (mirror root README Verify):

```bash
cargo clippy --workspace --all-targets --features ssr -- -D warnings
cargo test -p photon-axum -p photon-leptos -p photon-leptos-macros --features ssr
cargo doc -p photon-leptos -p photon-axum -p photon-leptos-macros --features ssr,hydrate --no-deps
```

- Target ≤400 LOC per `.rs` file; hard stop at 450 (Sentrux policy)
