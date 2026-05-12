# Verification — complete-transport-rpc-readiness

Evidence directory: `openspec/changes/complete-transport-rpc-readiness/evidence/`

## Task Coverage

- Task 1.1 (`aspen-transport` default graph): done.
  - Evidence: `v1-transport-default-cargo-tree.txt`, `v1-transport-default-cargo-check.txt`, `i7-downstream-transport-tests.txt`, `i7-downstream-transport-metadata.json`, `i7-downstream-transport-forbidden-grep.txt`.
- Task 1.2 (`aspen-rpc-core` default graph): done.
  - Evidence: `v2-rpc-core-default-cargo-tree.txt`, `v2-rpc-core-default-cargo-check.txt`, `i7-downstream-rpc-tests.txt`, `i7-downstream-rpc-metadata.json`, `i7-downstream-rpc-forbidden-grep.txt`.
- Task 1.3 (runtime compatibility consumers): done.
  - Evidence: `v4-compatibility-summary.txt`.
- Task 2.1 (manifest/inventory/policy readiness): done.
  - Evidence: `docs/crate-extraction/transport-rpc.md`, `docs/crate-extraction.md`, `docs/crate-extraction/policy.ncl`, `v5-readiness.md`, `v5-readiness.json`, `v5-readiness.txt`.

## Positive downstream fixtures

- `i7-downstream-transport-tests.txt`: `cargo test --manifest-path .../fixtures/downstream-transport/Cargo.toml` passed; 3 unit tests passed.
- `i7-downstream-transport-metadata.json`: cargo metadata for the transport fixture.
- `i7-downstream-rpc-tests.txt`: `cargo test --manifest-path .../fixtures/downstream-rpc-core/Cargo.toml` passed; 2 unit tests passed.
- `i7-downstream-rpc-metadata.json`: cargo metadata for the RPC core fixture.

## Negative boundary scans

- `i7-downstream-transport-forbidden-grep.txt`: default transport fixture metadata has no forbidden runtime/root Aspen dependencies.
- `i7-downstream-rpc-forbidden-grep.txt`: default RPC fixture metadata has no forbidden runtime/root Aspen dependencies; `tokio` is documented as dev-dependency test runtime only.

## Default graph and compatibility checks

- `v1-transport-default-cargo-tree.txt`: default `aspen-transport` normal dependency tree.
- `v1-transport-default-cargo-check.txt`: `cargo check -p aspen-transport` passed.
- `v2-rpc-core-default-cargo-tree.txt`: default `aspen-rpc-core` normal dependency tree.
- `v2-rpc-core-default-cargo-check.txt`: `cargo check -p aspen-rpc-core` passed.
- `v4-compatibility-summary.txt`: `cargo check -p aspen-raft-network`, `cargo check -p aspen-client`, and `cargo check -p aspen-rpc-handlers` passed against explicit feature bundles.

## Readiness checker

- `v5-readiness.txt`: `./scripts/check-crate-extraction-readiness.rs --policy docs/crate-extraction/policy.ncl --inventory docs/crate-extraction.md --manifest-dir docs/crate-extraction --candidate-family transport-rpc --output-json openspec/changes/complete-transport-rpc-readiness/evidence/v5-readiness.json --output-markdown openspec/changes/complete-transport-rpc-readiness/evidence/v5-readiness.md` passed.
- `v5-negative-invalid-readiness.txt`: checker rejects a selected candidate demoted back to `workspace-internal`.
- `v5-negative-missing-compatibility.txt`: checker rejects missing compatibility evidence.
- `v5-negative-stale-inventory.txt`: checker rejects a ready inventory row that still says to finish downstream evidence before raising readiness.
- `v5-negative-mutations-summary.txt`: compact summary of the negative mutation checks.

## Known non-blocking warnings

- Existing unknown lint warnings from `acronym_style`/`sentinel_fallback` appear in dependent crates.
- Existing unused import warning appears in `aspen-core-essentials-handler` during broad handler compatibility checks.
