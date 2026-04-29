# Verification Evidence

Use this file to back every checked task in `tasks.md` with durable repo evidence.
Do not rely on chat-only summaries, `/tmp` logs, or memory.

## Implementation Evidence

- Changed file: `crates/aspen-blob/Cargo.toml`
- Changed file: `openspec/changes/remove-blob-replication-core-dependency/tasks.md`
- Changed file: `openspec/changes/remove-blob-replication-core-dependency/verification.md`
- Changed file: `openspec/changes/remove-blob-replication-core-dependency/evidence/i1-manifest-audit.txt`
- Changed file: `openspec/changes/remove-blob-replication-core-dependency/evidence/i1-manifest-diff.txt`

## Task Coverage

- [x] R1 Capture baseline evidence under `openspec/changes/remove-blob-replication-core-dependency/evidence/` for `aspen-blob --features replication`, including current `aspen-core` dependency path, current `KvReplicaMetadataStore` imports, and current blob/castore/cache policy exception. ✅ 2m (started: 2026-04-29T03:46:30Z → completed: 2026-04-29T03:48:55Z) [covers=blob-castore-cache-extraction.blob-replication-kv-uses-leaf-contracts.replication-adapter-compiles-without-root-core,architecture.modularity.blob-replication-prefers-leaf-kv-contracts.kv-metadata-uses-leaf-kv-contracts]
  - Evidence: `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-source-audit.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-cargo-tree.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-aspen-core-path.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-cargo-check-replication.txt`
- [x] I1 Remove optional `aspen-core` dependency and `replication` feature edge from `crates/aspen-blob/Cargo.toml`, keeping `aspen-client-api` behind `replication`. ✅ 1m (started: 2026-04-29T03:49:35Z → completed: 2026-04-29T03:50:10Z) [covers=blob-castore-cache-extraction.blob-replication-kv-uses-leaf-contracts.replication-adapter-compiles-without-root-core,architecture.modularity.blob-replication-prefers-leaf-kv-contracts.runtime-wire-schemas-remain-separately-gated]
  - Evidence: `crates/aspen-blob/Cargo.toml`, `openspec/changes/remove-blob-replication-core-dependency/evidence/i1-manifest-audit.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/i1-manifest-diff.txt`

## Review Scope Snapshot

### `git diff HEAD -- crates/aspen-blob/Cargo.toml openspec/changes/remove-blob-replication-core-dependency/tasks.md`

- Status: captured
- Artifact: `openspec/changes/remove-blob-replication-core-dependency/evidence/i1-manifest-diff.txt`

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

## Notes

I1 intentionally changes only manifest topology. The adapter code still migrates in I2 before post-change cargo verification.
