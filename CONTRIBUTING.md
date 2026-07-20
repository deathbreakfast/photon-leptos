# Contributing to photon-leptos

## Documentation

When you change public API behavior, macro attributes, or host wiring steps:

1. Update rustdoc on the affected symbols (`photon-leptos` warns on `missing_docs`).
2. Update the root [`README.md`](README.md) and the affected crate README when user-facing flows change.
3. Keep the e2e demo under [`e2e/`](e2e/README.md) aligned when auth, WS routes, or subscription helpers change.

## Verification

Match CI ([`.github/workflows/ci.yml`](.github/workflows/ci.yml)):

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --features ssr -- -D warnings
cargo test -p photon-axum -p photon-leptos -p photon-leptos-macros -p photon-leptos-bench --features ssr
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
cargo package -p photon-leptos-macros --list
cargo package -p photon-axum --list
cargo package -p photon-leptos --list
cargo check -p photon-leptos --target wasm32-unknown-unknown --features hydrate
```

Browser E2E (from the workspace root):

```bash
cd e2e/tests && npm ci && npx playwright install --with-deps
cargo leptos end-to-end --project photon-leptos-e2e
```

CI checks out [`unified-field-dev/photon`](https://github.com/unified-field-dev/photon) alongside this repo.

## Pull requests

- Prefer small, focused PRs.
- Note any intentional API or behavior changes in [`CHANGELOG.md`](CHANGELOG.md).

## Code of conduct

Participation is governed by [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md). Security reports: [`SECURITY.md`](SECURITY.md).
