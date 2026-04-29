# Verification Evidence

Use this file to back every checked task in `tasks.md` with durable repo evidence.
Do not rely on chat-only summaries, `/tmp` logs, or memory.

## Implementation Evidence

- Changed file: `docs/crate-extraction/blob-castore-cache.md`
- Changed file: `docs/crate-extraction/policy.ncl`
- Changed file: `openspec/changes/remove-blob-replication-core-dependency/tasks.md`
- Changed file: `openspec/changes/remove-blob-replication-core-dependency/verification.md`
- Changed file: `openspec/changes/remove-blob-replication-core-dependency/evidence/i3-doc-policy-audit.txt`
- Changed file: `openspec/changes/remove-blob-replication-core-dependency/evidence/i3-doc-policy-diff.txt`
- Changed file: `openspec/changes/remove-blob-replication-core-dependency/evidence/i3-policy-export.json`
- Changed file: `openspec/changes/remove-blob-replication-core-dependency/evidence/i3-policy-export.stderr`

## Task Coverage

- [x] R1 Capture baseline evidence under `openspec/changes/remove-blob-replication-core-dependency/evidence/` for `aspen-blob --features replication`, including current `aspen-core` dependency path, current `KvReplicaMetadataStore` imports, and current blob/castore/cache policy exception. ✅ 2m (started: 2026-04-29T03:46:30Z → completed: 2026-04-29T03:48:55Z) [covers=blob-castore-cache-extraction.blob-replication-kv-uses-leaf-contracts.replication-adapter-compiles-without-root-core,architecture.modularity.blob-replication-prefers-leaf-kv-contracts.kv-metadata-uses-leaf-kv-contracts]
  - Evidence: `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-source-audit.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-cargo-tree.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-aspen-core-path.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-cargo-check-replication.txt`
- [x] I1 Remove optional `aspen-core` dependency and `replication` feature edge from `crates/aspen-blob/Cargo.toml`, keeping `aspen-client-api` behind `replication`. ✅ 1m (started: 2026-04-29T03:49:35Z → completed: 2026-04-29T03:50:10Z) [covers=blob-castore-cache-extraction.blob-replication-kv-uses-leaf-contracts.replication-adapter-compiles-without-root-core,architecture.modularity.blob-replication-prefers-leaf-kv-contracts.runtime-wire-schemas-remain-separately-gated]
  - Evidence: `crates/aspen-blob/Cargo.toml`, `Cargo.lock`, `openspec/changes/remove-blob-replication-core-dependency/evidence/i1-manifest-audit.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/i1-manifest-diff.txt`
- [x] I2 Migrate `KvReplicaMetadataStore` and its tests from `aspen_core::traits`, `aspen_core::kv`, and `aspen_core::error` to `aspen-traits` and `aspen-kv-types`, preserving get/save/delete/scan behavior and error context. ✅ 3m (started: 2026-04-29T03:50:30Z → completed: 2026-04-29T03:53:39Z) [covers=blob-castore-cache-extraction.blob-replication-kv-uses-leaf-contracts.replica-metadata-behavior-preserved,architecture.modularity.blob-replication-prefers-leaf-kv-contracts.kv-metadata-uses-leaf-kv-contracts]
  - Evidence: `crates/aspen-blob/src/replication/adapters.rs`, `openspec/changes/remove-blob-replication-core-dependency/evidence/i2-adapter-import-audit.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/i2-adapter-diff.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/i2-cargo-check-replication.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/i2-kv-metadata-tests.txt`
- [x] I3 Update `docs/crate-extraction/blob-castore-cache.md` and `docs/crate-extraction/policy.ncl` so `aspen-blob -> aspen-core` is no longer documented or allowed for replication metadata storage. ✅ 2m (started: 2026-04-29T03:54:40Z → completed: 2026-04-29T03:56:50Z) [covers=blob-castore-cache-extraction.blob-replication-kv-uses-leaf-contracts.policy-rejects-stale-core-exception,architecture.modularity.blob-replication-prefers-leaf-kv-contracts.boundary-policy-catches-stale-blob-core-dependency]
  - Evidence: `docs/crate-extraction/blob-castore-cache.md`, `docs/crate-extraction/policy.ncl`, `openspec/changes/remove-blob-replication-core-dependency/evidence/i3-doc-policy-audit.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/i3-doc-policy-diff.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/i3-policy-export.json`, `openspec/changes/remove-blob-replication-core-dependency/evidence/i3-policy-export.stderr`

## Review Scope Snapshot

### `git diff HEAD -- docs/crate-extraction/blob-castore-cache.md docs/crate-extraction/policy.ncl openspec/changes/remove-blob-replication-core-dependency/tasks.md`

- Status: captured
- Artifact: `openspec/changes/remove-blob-replication-core-dependency/evidence/i3-doc-policy-diff.txt`

## Verification Commands

### `cargo check -p aspen-blob --features replication`

- Status: pass before implementation
- Artifact: `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-cargo-check-replication.txt`

### `cargo tree -p aspen-blob --features replication -e normal`

- Status: pass before implementation
- Artifact: `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-cargo-tree.txt`

### `cargo tree -p aspen-blob --features replication -e normal -i aspen-core`

- Status: pass before implementation
- Artifact: `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-aspen-core-path.txt`

### `rg` source and policy baseline audit

- Status: pass
- Artifact: `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-source-audit.txt`

### `rg` manifest audit

- Status: pass
- Artifact: `openspec/changes/remove-blob-replication-core-dependency/evidence/i1-manifest-audit.txt`

### `cargo check -p aspen-blob --features replication` after adapter migration

- Status: pass
- Artifact: `openspec/changes/remove-blob-replication-core-dependency/evidence/i2-cargo-check-replication.txt`

### `cargo test -p aspen-blob --features replication kv_metadata_store -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/remove-blob-replication-core-dependency/evidence/i2-kv-metadata-tests.txt`

### `rg` adapter import audit

- Status: pass
- Artifact: `openspec/changes/remove-blob-replication-core-dependency/evidence/i2-adapter-import-audit.txt`

### `rg` docs/policy audit

- Status: pass
- Artifact: `openspec/changes/remove-blob-replication-core-dependency/evidence/i3-doc-policy-audit.txt`

### `nix run nixpkgs#nickel -- export --format json docs/crate-extraction/policy.ncl`

- Status: pass
- Artifact: `openspec/changes/remove-blob-replication-core-dependency/evidence/i3-policy-export.stderr`

## Notes

I3 removes the stale `aspen-blob -> aspen-core` exception and adds root `aspen-core` to the blob candidate forbidden list so future default-graph regressions fail policy checks.
