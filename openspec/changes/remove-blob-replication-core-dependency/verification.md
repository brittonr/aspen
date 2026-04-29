# Verification Evidence

Use this file to back every checked task in `tasks.md` with durable repo evidence.
Do not rely on chat-only summaries, `/tmp` logs, or memory.

## Implementation Evidence

- Changed file: `openspec/changes/remove-blob-replication-core-dependency/tasks.md`
- Changed file: `openspec/changes/remove-blob-replication-core-dependency/verification.md`
- Changed file: `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-source-audit.txt`
- Changed file: `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-cargo-tree.txt`
- Changed file: `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-aspen-core-path.txt`
- Changed file: `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-cargo-check-replication.txt`

## Task Coverage

- [x] R1 Capture baseline evidence under `openspec/changes/remove-blob-replication-core-dependency/evidence/` for `aspen-blob --features replication`, including current `aspen-core` dependency path, current `KvReplicaMetadataStore` imports, and current blob/castore/cache policy exception. ✅ 2m (started: 2026-04-29T03:46:30Z → completed: 2026-04-29T03:48:55Z) [covers=blob-castore-cache-extraction.blob-replication-kv-uses-leaf-contracts.replication-adapter-compiles-without-root-core,architecture.modularity.blob-replication-prefers-leaf-kv-contracts.kv-metadata-uses-leaf-kv-contracts]
  - Evidence: `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-source-audit.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-cargo-tree.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-aspen-core-path.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-cargo-check-replication.txt`

## Review Scope Snapshot

No implementation diff yet; R1 records baseline-only state before code changes.

## Verification Commands

### `cargo check -p aspen-blob --features replication`

- Status: pass
- Artifact: `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-cargo-check-replication.txt`

### `cargo tree -p aspen-blob --features replication -e normal`

- Status: pass
- Artifact: `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-cargo-tree.txt`

### `cargo tree -p aspen-blob --features replication -e normal -i aspen-core`

- Status: pass
- Artifact: `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-aspen-core-path.txt`

### `rg` source and policy baseline audit

- Status: pass
- Artifact: `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-source-audit.txt`

## Notes

R1 shows the pre-change `replication` feature includes `dep:aspen-core`, `KvReplicaMetadataStore` imports `aspen_core::*`, and the blob/castore/cache policy permits `aspen-blob -> aspen-core`.
