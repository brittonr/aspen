Evidence-ID: extend-no-std-foundation-and-wire.i5-docs
Task-ID: I5
Artifact-Type: docs-review
Covers: core.no-std-core-baseline.std-gated-shell-apis-keep-current-public-paths, core.no-std-core-baseline.shell-alias-path-proof-is-reviewable, architecture.modularity.feature-bundles-are-explicit-and-bounded.std-compatibility-is-an-explicit-opt-in

# `docs/no-std-core.md` review

## Reviewed paths

- `docs/no-std-core.md`
- `crates/aspen-core/src/storage.rs`
- `crates/aspen-core-shell/src/storage.rs`
- `crates/aspen-core/tests/fixtures/shell-alias-smoke/Cargo.toml`
- `crates/aspen-core/tests/fixtures/shell-alias-smoke/src/lib.rs`

## Findings

- The package matrix now names `aspen-core-shell` as the explicit shell opt-in for Redb storage helpers.
- The shell-only path list now documents `storage::SM_KV_TABLE` as shell-owned while alloc-only `aspen-core::storage` keeps `KvEntry` only.
- The alias-pattern example remains the supported way to preserve `aspen_core::*` imports in std consumers.
- The docs now explicitly state that the alias path preserves `aspen_core::storage::SM_KV_TABLE` for shell users.

## Verdict

Documentation matches the implemented storage seam and shell opt-in path for this cut.
