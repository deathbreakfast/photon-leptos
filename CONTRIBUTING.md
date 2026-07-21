# Contributing to photon-leptos

## Documentation

When you change public API behavior, macro attributes, or host wiring steps:

1. Update rustdoc on the affected symbols (`photon-leptos` warns on `missing_docs`).
2. Update the root [`README.md`](README.md) and the affected crate README when user-facing flows change.
3. Keep the e2e demo under [`e2e/`](e2e/README.md) aligned when auth, WS routes, or subscription helpers change.

## Coding standards

- **Format / Clippy:** `rustfmt` and workspace Clippy (`clippy::all` plus selected pedantic and restriction lints). CI runs Clippy with `-D warnings`.
- **Errors:** Library crates use typed errors (`thiserror`). Binary / demo crates may use `anyhow`. Prefer `?` over `.unwrap()` / `.expect()` in production code (tests may allow unwrap/expect).
- **Logging:** Library code uses the `tracing` facade. Host apps initialize a subscriber (`tracing_subscriber` on the server, `tracing-wasm` in the e2e hydrate build). Do not use `println!` / `log` in library crates.
- **Leptos lints:** Prefer `spawn_local_scoped` over unscoped `spawn_local` unless you intentionally allow `leptos_unscoped_spawn`.

## Verification

Match CI ([`.github/workflows/ci.yml`](.github/workflows/ci.yml)):

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --features ssr -- -D warnings
# Requires: cargo install cargo-dylint dylint-link
# leptos-lints pins an older nightly; lint hydrate UI crates with --no-deps
cargo dylint --all -p photon-leptos --no-deps -- --features hydrate
# Needs: rustup target add wasm32-unknown-unknown --toolchain nightly-2025-05-14
cargo dylint --all -p photon-leptos-e2e-demo --no-deps -- --features hydrate --target wasm32-unknown-unknown
cargo audit
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
