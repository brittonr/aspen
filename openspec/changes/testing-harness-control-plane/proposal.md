## Why

Aspen has broad test coverage, but the harness is managed through duplicated filters, hand-maintained flake wiring, and multiple parallel helper layers. That makes the suite harder to evolve, slows focused validation, and hides flake and coverage gaps behind comments, regexes, and tribal knowledge.

## What Changes

- Add a Nickel-based metadata manifest for Rust, simulation, patchbay, and NixOS VM suites so test ownership, requirements, runtime class, and coverage tags live in one source of truth.
- Generate test selection inputs from that metadata instead of hand-maintaining large parts of `flake.nix`, ad hoc shell scripts, and nextest profile routing by regex alone.
- Consolidate shared harness APIs so real-cluster integration tests use the same wait, cluster bootstrap, and fault-control primitives exposed through `crates/aspen-testing*` instead of carrying a second bootstrap path in `tests/support/`.
- Introduce wait-driven helper APIs for common readiness and convergence conditions, then migrate critical test flows away from fixed sleeps where the code is really waiting on state changes.
- Add machine-readable reporting for slow tests, retry-only passes, skipped suites, and layer coverage so harness maintenance can target real pain points.

## Capabilities

### New Capabilities
- `test-suite-metadata`: Nickel manifests define suite metadata and drive generated selection, grouping, and registration outputs.
- `test-harness-runtime`: Shared harness APIs cover cluster lifecycle, wait conditions, and layer-specific helpers for real-cluster, madsim, patchbay, and VM-backed tests.
- `test-harness-reporting`: Test runs emit machine-readable reporting for retries, runtime hotspots, skipped suites, and coverage-by-layer summaries.

### Modified Capabilities
- `patchbay-harness`: Patchbay tests participate in the shared metadata and harness control plane instead of being configured as a standalone special case.

## Impact

- **Code**: `.config/nextest.toml`, `flake.nix`, `scripts/test-*.sh`, `crates/aspen-testing*`, `tests/support/real_cluster.rs`, and shared helpers under `nix/tests/`.
- **Config**: new Nickel metadata files for suite definitions and generated outputs consumed by Rust and Nix test entry points.
- **Developer workflow**: focused test selection, consistent wait helpers, and a clearer promotion path from deterministic tests to real-network and VM coverage.
- **Observability**: new harness reports for retries, slow suites, and coverage gaps across test layers.
