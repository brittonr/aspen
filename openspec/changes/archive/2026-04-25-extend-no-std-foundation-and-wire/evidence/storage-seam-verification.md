Evidence-ID: extend-no-std-foundation-and-wire.v1
Task-ID: V1
Artifact-Type: verification
Covers: core.no-std-core-baseline.alloc-only-build-rejects-shell-imports, core.no-std-core-baseline.compile-fail-verification-is-reviewable, core.no-std-core-baseline.std-dependent-helpers-require-explicit-opt-in, core.no-std-core-baseline.std-gated-shell-apis-keep-current-public-paths, core.no-std-core-baseline.module-family-boundary-matches-documented-inventory, core.no-std-core-baseline.alloc-only-storage-surface-excludes-redb-table-definitions, core.no-std-core-baseline.shell-alias-path-proof-is-reviewable, architecture.modularity.acyclic-no-std-core-boundary.runtime-shells-depend-outward-on-core, architecture.modularity.acyclic-no-std-core-boundary.acyclic-boundary-proof-is-reviewable, architecture.modularity.acyclic-no-std-core-boundary.leaf-crates-keep-alloc-safe-production-surfaces

# Storage seam verification

## Commands run

- `cargo check -p aspen-core-shell`
  - Result: success
- `cargo check -p aspen-core-shell --features layer`
  - Result: success
- `cargo check -p aspen-core-shell --features global-discovery`
  - Result: success
- `cargo check -p aspen-core-shell --features sql`
  - Result: success
- `cargo check --manifest-path crates/aspen-core/tests/fixtures/shell-alias-smoke/Cargo.toml`
  - Result: success
- `cargo test -p aspen-core --test ui`
  - Result: success after refreshing stderr snapshots for the moved evidence directory and new `storage-table-no-std` fixture
- `python3 scripts/check-aspen-core-no-std-surface.py --crate-dir crates/aspen-core/src --output-dir openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence`
  - Result: success; generated `surface-inventory.md`, `export-map.md`, and `source-audit.txt`

## Durable artifacts

- UI fixture index: `openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence/ui-fixtures.txt`
- UI stderr snapshots:
  - `openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence/ui-app-registry-no-std.stderr`
  - `openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence/ui-network-transport-no-std.stderr`
  - `openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence/ui-simulation-artifact-no-std.stderr`
  - `openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence/ui-content-discovery-std-without-global-discovery.stderr`
  - `openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence/ui-directory-layer-std-without-layer.stderr`
  - `openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence/ui-storage-table-no-std.stderr`
- Boundary inventory: `openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence/surface-inventory.md`
- Export map: `openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence/export-map.md`
- Source audit: `openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence/source-audit.txt`
- Alias-form fixture: `crates/aspen-core/tests/fixtures/shell-alias-smoke/Cargo.toml`, `crates/aspen-core/tests/fixtures/shell-alias-smoke/src/lib.rs`
- Negative storage fixture: `crates/aspen-core/tests/ui/storage-table-no-std/Cargo.toml`, `crates/aspen-core/tests/ui/storage-table-no-std/src/main.rs`

## Claims proved

- Alloc-only `aspen-core::storage` no longer exposes `SM_KV_TABLE`; the dedicated `storage-table-no-std` fixture fails and its stderr snapshot is saved.
- Shell consumers preserve `aspen_core::storage::SM_KV_TABLE` through the alias-path fixture that depends on `aspen-core = { package = "aspen-core-shell", ... }`.
- Boundary inventory artifacts now enumerate shell-owned `storage` alongside the existing runtime-only families.
