# photon-leptos Quality Gates

This crate uses Sentrux MCP (`user-sentrux`) as a structure-health signal and
`cargo llvm-cov` as the source of truth for executable line coverage.

## Current Health Report

Last updated: 2026-06-30

Baseline metric source:

- `user-sentrux.scan(path="/home/seanorourke/photon-leptos")`
- `cargo test -p photon-leptos --features ssr`
- `cargo llvm-cov -p photon-leptos --summary-only`

## Targets

- Preserve or improve structure and architecture grades.
- Keep circular dependencies at zero and prevent unexpected coupling regressions.
- Raise LLVM line coverage over time with targeted module tests.
- Tighten public surface area and remove dead or unused paths where practical.

## Local Commands

### Sentrux MCP (preferred)

Run in this order:

1. `scan(path="/home/seanorourke/photon-leptos")`
2. `check_rules()`
3. `health()`
4. `cycles()`
5. `coupling()`

### Test coverage

```bash
cargo llvm-cov -p photon-leptos --features ssr --text --show-missing-lines
```

Optional summary export:

```bash
cargo llvm-cov -p photon-leptos --features ssr --json --summary-only --output-path photon-leptos/coverage-summary.json
```

## CI Gate Policy

- CI enforces `cargo test -p photon-leptos --features ssr` (see workspace [`.github/workflows/ci.yml`](../.github/workflows/ci.yml)).
- Sentrux `scan` + `check_rules` on the repo root before large doc or API changes.
- Target ≤400 LOC per `.rs` file; hard stop at 450 (see [`.sentrux/rules.toml`](../.sentrux/rules.toml)).
