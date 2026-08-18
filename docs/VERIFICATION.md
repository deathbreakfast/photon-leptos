# photon-leptos verification

Re-run after code or doc changes. Leptos + Axum WebSocket integration for Photon browser
clients — covered by unit, integration, and doc gates below.

## Environment

```bash
export CARGO_BUILD_JOBS=1
```

## Unit + integration + doc (CI)

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
```

Narrower runs:

```bash
cargo test -p photon-leptos --all-features
cargo test -p photon-axum --features runtime
RUSTDOCFLAGS="-D warnings" cargo doc -p photon-leptos --features ssr,hydrate --no-deps
RUSTDOCFLAGS="-D warnings" cargo doc -p photon-axum --features runtime --no-deps
```

## Notes

- Workspace `missing_docs` is enforced; doc builds use `RUSTDOCFLAGS="-D warnings"`.
- E2E (`cargo leptos end-to-end`) requires `PHOTON_TRANSPORT_KEY`; see `e2e/README.md` and
  `.github/workflows/ci.yml` — not part of the default doc gate above.
