Evidence-ID: extend-no-std-foundation-and-wire.r1
Task-ID: R1
Artifact-Type: verification-plan
Covers: core.no-std-core-baseline.compile-slice-verification-is-reviewable, architecture.modularity.acyclic-no-std-core-boundary.leaf-crate-verification-is-reviewable, architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces.protocol-dependency-verification-is-reviewable

# Baseline capture plan

This file owns the pre-implementation baseline artifacts required by design migration step 1.

Baseline commands to capture before seam changes land:

- `cargo check -p aspen-core`
- `cargo check -p aspen-core --no-default-features`
- `cargo check -p aspen-core-no-std-smoke`
- `python3 scripts/check-aspen-core-feature-claims.py`
- `cargo tree -p aspen-core --no-default-features -e normal --depth 1`
- `cargo tree -p aspen-core --no-default-features -e normal`
- `cargo tree -p aspen-core --no-default-features -e features`
- `python3 scripts/check-aspen-core-no-std-surface.py`
- `python3 scripts/check-aspen-core-no-std-boundary.py`
- `cargo check -p aspen-storage-types`
- `cargo tree -p aspen-storage-types -e normal`
- `cargo tree -p aspen-storage-types -e features`
- `cargo check -p aspen-traits`
- `cargo tree -p aspen-traits -e normal`
- `cargo tree -p aspen-traits -e features`
- `cargo tree -p aspen-traits -e features -i aspen-cluster-types`
- `cargo check -p aspen-client-api`
- `cargo tree -p aspen-client-api -e normal`
- `cargo tree -p aspen-client-api -e features`
- `cargo check -p aspen-coordination-protocol`
- `cargo tree -p aspen-coordination-protocol -e normal`
- `cargo tree -p aspen-coordination-protocol -e features`
- `cargo check -p aspen-jobs-protocol`
- `cargo tree -p aspen-jobs-protocol -e normal`
- `cargo tree -p aspen-jobs-protocol -e features`
- `cargo check -p aspen-forge-protocol`
- `cargo tree -p aspen-forge-protocol -e normal`
- `cargo tree -p aspen-forge-protocol -e features`

Baseline artifact destinations:

- `openspec/changes/extend-no-std-foundation-and-wire/evidence/baseline-storage-seam-verification.md`
- `openspec/changes/extend-no-std-foundation-and-wire/evidence/baseline-core-foundation-verification.md`
- `openspec/changes/extend-no-std-foundation-and-wire/evidence/baseline-wire-dependency-verification.md`

Also record the `wasm32-unknown-unknown` target installation or equivalent environment-provided setup for later target-build rails.
