# Verification Evidence

Use this file to back every checked task in `tasks.md` with durable repo evidence.
Do not rely on chat-only summaries, `/tmp` logs, or memory.

## Implementation Evidence

- Changed file: `openspec/changes/remove-blob-replication-core-dependency/tasks.md`
- Changed file: `openspec/changes/remove-blob-replication-core-dependency/verification.md`
- Changed file: `openspec/changes/remove-blob-replication-core-dependency/evidence/v4-source-doc-diff.txt`
- Changed file: `openspec/changes/remove-blob-replication-core-dependency/evidence/v4-openspec-validate.txt`
- Changed file: `openspec/changes/remove-blob-replication-core-dependency/evidence/v4-openspec-preflight.txt`

## Task Coverage

- [x] R1 Capture baseline evidence under `openspec/changes/remove-blob-replication-core-dependency/evidence/` for `aspen-blob --features replication`, including current `aspen-core` dependency path, current `KvReplicaMetadataStore` imports, and current blob/castore/cache policy exception. ✅ 2m (started: 2026-04-29T03:46:30Z → completed: 2026-04-29T03:48:55Z) [covers=blob-castore-cache-extraction.blob-replication-kv-uses-leaf-contracts.replication-adapter-compiles-without-root-core,architecture.modularity.blob-replication-prefers-leaf-kv-contracts.kv-metadata-uses-leaf-kv-contracts]
  - Evidence: `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-source-audit.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-cargo-tree.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-aspen-core-path.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/baseline-cargo-check-replication.txt`
- [x] I1 Remove optional `aspen-core` dependency and `replication` feature edge from `crates/aspen-blob/Cargo.toml`, keeping `aspen-client-api` behind `replication`. ✅ 1m (started: 2026-04-29T03:49:35Z → completed: 2026-04-29T03:50:10Z) [covers=blob-castore-cache-extraction.blob-replication-kv-uses-leaf-contracts.replication-adapter-compiles-without-root-core,architecture.modularity.blob-replication-prefers-leaf-kv-contracts.runtime-wire-schemas-remain-separately-gated]
  - Evidence: `crates/aspen-blob/Cargo.toml`, `Cargo.lock`, `openspec/changes/remove-blob-replication-core-dependency/evidence/i1-manifest-audit.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/i1-manifest-diff.txt`
- [x] I2 Migrate `KvReplicaMetadataStore` and its tests from `aspen_core::traits`, `aspen_core::kv`, and `aspen_core::error` to `aspen-traits` and `aspen-kv-types`, preserving get/save/delete/scan behavior and error context. ✅ 3m (started: 2026-04-29T03:50:30Z → completed: 2026-04-29T03:53:39Z) [covers=blob-castore-cache-extraction.blob-replication-kv-uses-leaf-contracts.replica-metadata-behavior-preserved,architecture.modularity.blob-replication-prefers-leaf-kv-contracts.kv-metadata-uses-leaf-kv-contracts]
  - Evidence: `crates/aspen-blob/src/replication/adapters.rs`, `openspec/changes/remove-blob-replication-core-dependency/evidence/i2-adapter-import-audit.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/i2-adapter-diff.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/i2-cargo-check-replication.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/i2-kv-metadata-tests.txt`
- [x] I3 Update `docs/crate-extraction/blob-castore-cache.md` and `docs/crate-extraction/policy.ncl` so `aspen-blob -> aspen-core` is no longer documented or allowed for replication metadata storage. ✅ 2m (started: 2026-04-29T03:54:40Z → completed: 2026-04-29T03:56:50Z) [covers=blob-castore-cache-extraction.blob-replication-kv-uses-leaf-contracts.policy-rejects-stale-core-exception,architecture.modularity.blob-replication-prefers-leaf-kv-contracts.boundary-policy-catches-stale-blob-core-dependency]
  - Evidence: `docs/crate-extraction/blob-castore-cache.md`, `docs/crate-extraction/policy.ncl`, `openspec/changes/remove-blob-replication-core-dependency/evidence/i3-doc-policy-audit.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/i3-doc-policy-diff.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/i3-policy-export.json`, `openspec/changes/remove-blob-replication-core-dependency/evidence/i3-policy-export.stderr`
- [x] V1 Run `cargo check -p aspen-blob --features replication` plus focused replica metadata adapter tests, including positive get/save/delete/scan behavior and negative missing or malformed metadata behavior; save transcripts under `openspec/changes/remove-blob-replication-core-dependency/evidence/`. ✅ 1m (started: 2026-04-29T03:57:45Z → completed: 2026-04-29T03:58:33Z) [covers=blob-castore-cache-extraction.blob-replication-kv-uses-leaf-contracts.replica-metadata-behavior-preserved]
  - Evidence: `openspec/changes/remove-blob-replication-core-dependency/evidence/v1-cargo-check-replication.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/v1-kv-metadata-tests.txt`
- [x] V2 Run `cargo tree -p aspen-blob --features replication -e normal` with a deterministic audit proving root `aspen-core` is absent while `aspen-client-api` remains feature-gated for replication RPC; save the tree and audit under `openspec/changes/remove-blob-replication-core-dependency/evidence/`. ✅ 1m (started: 2026-04-29T03:59:10Z → completed: 2026-04-29T03:59:36Z) [covers=blob-castore-cache-extraction.blob-replication-kv-uses-leaf-contracts.replication-adapter-compiles-without-root-core,architecture.modularity.blob-replication-prefers-leaf-kv-contracts.runtime-wire-schemas-remain-separately-gated]
  - Evidence: `openspec/changes/remove-blob-replication-core-dependency/evidence/v2-cargo-tree-replication.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/v2-cargo-tree-audit.txt`
- [x] V3 Run `scripts/check-crate-extraction-readiness.rs --candidate-family blob-castore-cache` plus a negative mutation proving stale `aspen-blob -> aspen-core` dependency or exception is rejected; save outputs under `openspec/changes/remove-blob-replication-core-dependency/evidence/`. ✅ 1m (started: 2026-04-29T04:00:40Z → completed: 2026-04-29T04:01:58Z) [covers=blob-castore-cache-extraction.blob-replication-kv-uses-leaf-contracts.policy-rejects-stale-core-exception,architecture.modularity.blob-replication-prefers-leaf-kv-contracts.boundary-policy-catches-stale-blob-core-dependency]
  - Evidence: `openspec/changes/remove-blob-replication-core-dependency/evidence/v3-readiness.md`, `openspec/changes/remove-blob-replication-core-dependency/evidence/v3-readiness.json`, `openspec/changes/remove-blob-replication-core-dependency/evidence/v3-negative-aspen-core-dep-summary.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/v3-negative-aspen-core-dep.md`, `openspec/changes/remove-blob-replication-core-dependency/evidence/i6-downstream-blob-forbidden-grep.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/i6-downstream-cache-castore-forbidden-grep.txt`
- [x] V4 Save implementation diff evidence, `openspec validate remove-blob-replication-core-dependency --type change --strict --no-interactive`, and `scripts/openspec-preflight.sh remove-blob-replication-core-dependency` transcripts under `openspec/changes/remove-blob-replication-core-dependency/evidence/`; update `openspec/changes/remove-blob-replication-core-dependency/verification.md` to map every checked task to evidence before checking implementation tasks complete. ✅ 1m (started: 2026-04-29T04:02:55Z → completed: 2026-04-29T04:03:26Z) [covers=blob-castore-cache-extraction.blob-replication-kv-uses-leaf-contracts,architecture.modularity.blob-replication-prefers-leaf-kv-contracts]
  - Evidence: `openspec/changes/remove-blob-replication-core-dependency/evidence/v4-source-doc-diff.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/v4-openspec-validate.txt`, `openspec/changes/remove-blob-replication-core-dependency/evidence/v4-openspec-preflight.txt`

## Review Scope Snapshot

### `git diff ee672db26..HEAD -- Cargo.lock crates/aspen-blob/Cargo.toml crates/aspen-blob/src/replication/adapters.rs docs/crate-extraction/blob-castore-cache.md docs/crate-extraction/policy.ncl`

- Status: captured
- Artifact: `openspec/changes/remove-blob-replication-core-dependency/evidence/v4-source-doc-diff.txt`

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

### `cargo check -p aspen-blob --features replication` final V1 run

- Status: pass
- Artifact: `openspec/changes/remove-blob-replication-core-dependency/evidence/v1-cargo-check-replication.txt`

### `cargo test -p aspen-blob --features replication kv_metadata_store -- --nocapture` final V1 run

- Status: pass
- Artifact: `openspec/changes/remove-blob-replication-core-dependency/evidence/v1-kv-metadata-tests.txt`

### `cargo tree -p aspen-blob --features replication -e normal`

- Status: pass
- Artifact: `openspec/changes/remove-blob-replication-core-dependency/evidence/v2-cargo-tree-replication.txt`

### deterministic V2 cargo tree audit

- Status: pass
- Artifact: `openspec/changes/remove-blob-replication-core-dependency/evidence/v2-cargo-tree-audit.txt`

### `scripts/check-crate-extraction-readiness.rs --candidate-family blob-castore-cache`

- Status: pass
- Artifact: `openspec/changes/remove-blob-replication-core-dependency/evidence/v3-readiness.md`

### negative mutation: `aspen-blob -> aspen-core` direct dependency

- Status: pass; checker rejected the mutation
- Artifact: `openspec/changes/remove-blob-replication-core-dependency/evidence/v3-negative-aspen-core-dep-summary.txt`

### `openspec validate remove-blob-replication-core-dependency --type change --strict --no-interactive`

- Status: pass
- Artifact: `openspec/changes/remove-blob-replication-core-dependency/evidence/v4-openspec-validate.txt`

### `scripts/openspec-preflight.sh remove-blob-replication-core-dependency`

- Status: pass
- Artifact: `openspec/changes/remove-blob-replication-core-dependency/evidence/v4-openspec-preflight.txt`

## Notes

Final preflight was staged before execution so the preflight artifact was tracked, then replaced with the captured transcript.
