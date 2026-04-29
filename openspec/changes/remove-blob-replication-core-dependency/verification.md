# Verification Evidence

Use this file to back every checked task in `tasks.md` with durable repo evidence.
Do not rely on chat-only summaries, `/tmp` logs, or memory.

## Implementation Evidence

- Changed file: `crates/aspen-blob/src/replication/adapters.rs`
- Changed file: `openspec/changes/remove-blob-replication-core-dependency/tasks.md`
- Changed file: `openspec/changes/remove-blob-replication-core-dependency/verification.md`
- Changed file: `openspec/changes/remove-blob-replication-core-dependency/evidence/i2-adapter-import-audit.txt`
- Changed file: `openspec/changes/remove-blob-replication-core-dependency/evidence/i2-adapter-diff.txt`
- Changed file: `openspec/changes/remove-blob-replication-core-dependency/evidence/i2-cargo-check-replication.txt`
- Changed file: `openspec/changes/remove-blob-replication-core-dependency/evidence/i2-kv-metadata-tests.txt`

## Task Coverage

- [x] R1 Capture baseline evidence under `openspec/changes/remove-blob-replication-core-dependency/evidence/` for `aspen-blob --features replication`, including current `aspen-core` dependency path, current `KvReplicaMetadataStore` imports, and current blob/castore/cache policy exception. ✅ 2m (started: 2026-04-29T03:46:30Z → completed: 2026-04-29T03:48:55Z) [covers=blob-castore-cache-extraction.blob-replication-kv-uses-leaf-contracts.replication-adapter-compiles-without-root-core,architecture.modularity.blob-replication-prefers-leaf-kv-contracts.kv-metadata-uses-leaf-kv-contracts]
  - Evidence: `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-source-audit.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-cargo-tree.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-aspen-core-path.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-cargo-check-replication.txt`
- [x] I1 Remove optional `aspen-core` dependency and `replication` feature edge from `crates/aspen-blob/Cargo.toml`, keeping `aspen-client-api` behind `replication`. ✅ 1m (started: 2026-04-29T03:49:35Z → completed: 2026-04-29T03:50:10Z) [covers=blob-castore-cache-extraction.blob-replication-kv-uses-leaf-contracts.replication-adapter-compiles-without-root-core,architecture.modularity.blob-replication-prefers-leaf-kv-contracts.runtime-wire-schemas-remain-separately-gated]
  - Evidence: `crates/aspen-blob/Cargo.toml`, `openspec/changes/remove-blob-replication-core-dependency/evidence/i1-manifest-audit.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/i1-manifest-diff.txt`
- [x] I2 Migrate `KvReplicaMetadataStore` and its tests from `aspen_core::traits`, `aspen_core::kv`, and `aspen_core::error` to `aspen-traits` and `aspen-kv-types`, preserving get/save/delete/scan behavior and error context. ✅ 3m (started: 2026-04-29T03:50:30Z → completed: 2026-04-29T03:53:39Z) [covers=blob-castore-cache-extraction.blob-replication-kv-uses-leaf-contracts.replica-metadata-behavior-preserved,architecture.modularity.blob-replication-prefers-leaf-kv-contracts.kv-metadata-uses-leaf-kv-contracts]
  - Evidence: `crates/aspen-blob/src/replication/adapters.rs`, `openspec/changes/remove-blob-replication-core-dependency/evidence/i2-adapter-import-audit.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/i2-adapter-diff.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/i2-cargo-check-replication.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/i2-kv-metadata-tests.txt`

## Review Scope Snapshot

### `git diff HEAD -- crates/aspen-blob/src/replication/adapters.rs openspec/changes/remove-blob-replication-core-dependency/tasks.md`

- Status: captured
- Artifact: `openspec/changes/remove-blob-replication-core-dependency/evidence/i2-adapter-diff.txt`

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

## Notes

I2 adds explicit missing and malformed metadata tests while preserving roundtrip and status-scan behavior.
