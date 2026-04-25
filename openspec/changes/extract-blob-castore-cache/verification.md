# Verification Evidence

## Implementation Evidence

- Changed file: `Cargo.toml`
- Changed file: `crates/aspen-blob/Cargo.toml`
- Changed file: `crates/aspen-blob/src/lib.rs`
- Changed file: `crates/aspen-cluster/Cargo.toml`
- Changed file: `crates/aspen-rpc-core/Cargo.toml`
- Changed file: `crates/aspen-rpc-handlers/Cargo.toml`
- Changed file: `openspec/changes/extract-blob-castore-cache/tasks.md`
- Changed file: `openspec/changes/extract-blob-castore-cache/verification.md`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i3-cargo-check-aspen-blob-default.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i3-cargo-tree-aspen-blob-default.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i3-forbidden-tree-grep.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i3-cargo-check-aspen-blob-replication.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i3-cargo-check-aspen-rpc-core-blob.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i3-implementation-diff.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i3-openspec-preflight.txt`

## Task Coverage

- [x] R1 Capture baseline compile and dependency graphs for `aspen-blob`, `aspen-castore`, `aspen-cache`, and representative consumers, classifying each dependency as backend-purpose, reusable domain, adapter/runtime, test-only, or forbidden. [covers=blob-castore-cache-extraction.castore-cache-avoid-app-shells,blob-castore-cache-extraction.blob-default-avoids-app-shells,blob-castore-cache-extraction.blob-default-avoids-app-shells.iroh-backend-is-documented-exception,blob-castore-cache-extraction.castore-cache-avoid-app-shells.castore-circuit-breaker-is-reusable-or-gated,blob-castore-cache-extraction.castore-cache-avoid-app-shells.cache-metadata-signing-reusable] ✅ completed: 2026-04-25T18:49:10Z
  - Evidence: `openspec/changes/extract-blob-castore-cache/evidence/r1-baseline.md`, `openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/`

- [x] I3 Move or gate `aspen-blob` replication/client-RPC dependencies so default reusable blob APIs do not depend on `aspen-client-api`, handlers, root Aspen, or node bootstrap crates. [covers=blob-castore-cache-extraction.blob-default-avoids-app-shells.replication-rpc-is-adapter-only] ✅ completed: 2026-04-25T18:51:25Z
  - Evidence: `crates/aspen-blob/Cargo.toml`, `crates/aspen-blob/src/lib.rs`, `crates/aspen-rpc-core/Cargo.toml`, `crates/aspen-cluster/Cargo.toml`, `crates/aspen-rpc-handlers/Cargo.toml`, `Cargo.toml`, `openspec/changes/extract-blob-castore-cache/evidence/i3-cargo-tree-aspen-blob-default.txt`, `openspec/changes/extract-blob-castore-cache/evidence/i3-forbidden-tree-grep.txt`

## Review Scope Snapshot

### `git diff HEAD -- Cargo.toml crates/aspen-blob/Cargo.toml crates/aspen-blob/src/lib.rs crates/aspen-cluster/Cargo.toml crates/aspen-rpc-core/Cargo.toml crates/aspen-rpc-handlers/Cargo.toml openspec/changes/extract-blob-castore-cache/tasks.md openspec/changes/extract-blob-castore-cache/verification.md`

- Status: captured after I3 implementation
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i3-implementation-diff.txt`

## Verification Commands

### `bash -n openspec/changes/extract-blob-castore-cache/evidence/r1-capture-baseline.sh`

- Status: pass
- Artifact: command completed without output before running the capture script.

### `./openspec/changes/extract-blob-castore-cache/evidence/r1-capture-baseline.sh`

- Status: pass
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/r1-baseline.md`

### `cargo check -p aspen-blob --no-default-features --locked`

- Status: pass
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i3-cargo-check-aspen-blob-default.txt`

### `cargo tree -p aspen-blob --no-default-features -e normal --locked`

- Status: pass
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i3-cargo-tree-aspen-blob-default.txt`

### `rg -n 'aspen-client-api|aspen-rpc-core|aspen-rpc-handlers|aspen-blob-handler' /tmp/i3-blob-tree.txt`

- Status: pass, no matches
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i3-forbidden-tree-grep.txt`

### `cargo check -p aspen-blob --features replication --locked`

- Status: pass
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i3-cargo-check-aspen-blob-replication.txt`

### `cargo check -p aspen-rpc-core --features blob --locked`

- Status: pass
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i3-cargo-check-aspen-rpc-core-blob.txt`

### `scripts/openspec-preflight.sh extract-blob-castore-cache`

- Status: pass after I3
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i3-openspec-preflight.txt`

## Notes

- `aspen-castore` baseline compile fails before I4 because the existing client passes `Instant` into the `aspen-core` circuit breaker API that now expects `u64` milliseconds. See `openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/cargo-check-aspen-castore.txt`.
