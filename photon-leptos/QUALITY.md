# photon-leptos Quality Gates

This crate uses Sentrux MCP (`user-sentrux`) as a structure-health signal and
`cargo llvm-cov` as the source of truth for executable line coverage.

## Current Health Report

Last updated: 2026-03-17

Baseline metric source:
- `user-sentrux.scan(path="/home/seanorourke/web-app-template/photon-leptos")`
- `cargo test -p photon-leptos`
- `cargo llvm-cov -p photon-leptos --summary-only`

- Sentrux MCP scan:
  - Overall grade: `unknown`
  - Structure grade: `D`
  - Architecture grade: `B`
  - Graph summary: `15` files, `1065` lines, `11` import edges



- Tests: `not captured`
- LLVM line coverage: `not captured`

## Targets

- Preserve or improve structure and architecture grades.
- Keep circular dependencies at zero and prevent unexpected coupling regressions.
- Raise LLVM line coverage over time with targeted module tests.
- Tighten public surface area and remove dead or unused paths where practical.

## Local Commands

### Quality CLI (recommended)

Generate this crate baseline end-to-end with the shared tool:

```bash
cargo run -p quality -- check --target photon-leptos
```

### Sentrux MCP (preferred)

Run in this order:
1. `scan(path="/home/seanorourke/web-app-template/photon-leptos")`
2. `health()`
3. `cycles()`
4. `coupling()`
5. `architecture()`
6. `test_gaps(limit=20)`
7. `hottest(limit=10)`

### Proper test-coverage measurement

Use LLVM source-based coverage:

```bash
cargo llvm-cov -p photon-leptos --text --show-missing-lines
```

Optional summary export:

```bash
cargo llvm-cov -p photon-leptos --json --summary-only --output-path photon-leptos/coverage-summary.json
```

## CI Gate Policy

- CI should enforce `cargo test -p photon-leptos`.
- CI should capture LLVM coverage summary when `cargo-llvm-cov` is available.
- Sentrux checks should run as best-effort in CI (`scan`, `cycles`, `coupling`, `health`) when Sentrux CLI is available.
- Trend structure grade, architecture grade, and LLVM line coverage for this crate.
