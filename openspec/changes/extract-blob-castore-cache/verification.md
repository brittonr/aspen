# Verification Evidence

## Implementation Evidence

- Changed file: `Cargo.lock`
- Changed file: `Cargo.toml`
- Changed file: `crates/aspen-cache/Cargo.toml`
- Changed file: `crates/aspen-cache/src/index.rs`
- Changed file: `crates/aspen-cache/src/lib.rs`
- Changed file: `crates/aspen-cache/src/signing.rs`
- Changed file: `crates/aspen-castore/Cargo.toml`
- Changed file: `crates/aspen-castore/src/circuit_breaker.rs`
- Changed file: `crates/aspen-castore/src/client.rs`
- Changed file: `crates/aspen-castore/src/lib.rs`
- Changed file: `crates/aspen-nix-handler/Cargo.toml`
- Changed file: `crates/aspen-snix/Cargo.toml`
- Changed file: `openspec/changes/extract-blob-castore-cache/tasks.md`
- Changed file: `openspec/changes/extract-blob-castore-cache/verification.md`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i4-cargo-test-aspen-castore-circuit-breaker.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i4-cargo-check-aspen-castore-default.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i4-cargo-check-aspen-castore-default-afterfmt.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i4-cargo-tree-aspen-castore-default.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i4-forbidden-tree-grep.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i4-rustfmt-note.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i5-cargo-test-aspen-cache-reusable.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i5-cargo-check-aspen-cache-default.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i5-cargo-check-aspen-cache-default-afterfmt.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i5-cargo-tree-aspen-cache-default.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i5-forbidden-tree-grep.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i5-cargo-check-aspen-cache-kv-index.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i5-cargo-check-aspen-cache-kv-index-afterfmt.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i5-cargo-check-aspen-nix-handler-cache.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i5-cargo-check-aspen-ci-executor-nix.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i5-cargo-check-aspen-nix-cache-gateway.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i5-cargo-check-aspen-snix.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i4-i5-rustfmt-check.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i4-i5-implementation-diff.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i4-i5-openspec-preflight.txt`

## Task Coverage

- [x] R1 Capture baseline compile and dependency graphs for `aspen-blob`, `aspen-castore`, `aspen-cache`, and representative consumers, classifying each dependency as backend-purpose, reusable domain, adapter/runtime, test-only, or forbidden. [covers=blob-castore-cache-extraction.castore-cache-avoid-app-shells,blob-castore-cache-extraction.blob-default-avoids-app-shells,blob-castore-cache-extraction.blob-default-avoids-app-shells.iroh-backend-is-documented-exception,blob-castore-cache-extraction.castore-cache-avoid-app-shells.castore-circuit-breaker-is-reusable-or-gated,blob-castore-cache-extraction.castore-cache-avoid-app-shells.cache-metadata-signing-reusable] ✅ completed: 2026-04-25T18:49:10Z
  - Evidence: `openspec/changes/extract-blob-castore-cache/evidence/r1-baseline.md`, `openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/`

- [x] I3 Move or gate `aspen-blob` replication/client-RPC dependencies so default reusable blob APIs do not depend on `aspen-client-api`, handlers, root Aspen, or node bootstrap crates. [covers=blob-castore-cache-extraction.blob-default-avoids-app-shells.replication-rpc-is-adapter-only] ✅ completed: 2026-04-25T18:51:25Z
  - Evidence: `crates/aspen-blob/Cargo.toml`, `crates/aspen-blob/src/lib.rs`, `crates/aspen-rpc-core/Cargo.toml`, `crates/aspen-cluster/Cargo.toml`, `crates/aspen-rpc-handlers/Cargo.toml`, `Cargo.toml`, `openspec/changes/extract-blob-castore-cache/evidence/i3-cargo-tree-aspen-blob-default.txt`, `openspec/changes/extract-blob-castore-cache/evidence/i3-forbidden-tree-grep.txt`

- [x] I4 Replace, localize, or feature-gate `aspen-castore`'s `aspen-core-shell` circuit-breaker dependency so reusable castore APIs avoid core-shell/runtime app crates by default. [covers=blob-castore-cache-extraction.castore-cache-avoid-app-shells.castore-circuit-breaker-is-reusable-or-gated] ✅ completed: 2026-04-25T18:55:00Z
  - Evidence: `crates/aspen-castore/Cargo.toml`, `crates/aspen-castore/src/circuit_breaker.rs`, `crates/aspen-castore/src/client.rs`, `crates/aspen-castore/src/lib.rs`, `openspec/changes/extract-blob-castore-cache/evidence/i4-cargo-check-aspen-castore-default-afterfmt.txt`, `openspec/changes/extract-blob-castore-cache/evidence/i4-forbidden-tree-grep.txt`

- [x] I5 Separate `aspen-cache` reusable Nix cache metadata/signing helpers from cluster/testing/runtime integration, keeping publication paths behind named features or adapter crates. [covers=blob-castore-cache-extraction.castore-cache-avoid-app-shells.cache-metadata-signing-reusable] ✅ completed: 2026-04-25T19:03:50Z
  - Evidence: `crates/aspen-cache/Cargo.toml`, `crates/aspen-cache/src/index.rs`, `crates/aspen-cache/src/lib.rs`, `crates/aspen-cache/src/signing.rs`, `crates/aspen-nix-handler/Cargo.toml`, `crates/aspen-snix/Cargo.toml`, `openspec/changes/extract-blob-castore-cache/evidence/i5-cargo-tree-aspen-cache-default.txt`, `openspec/changes/extract-blob-castore-cache/evidence/i5-forbidden-tree-grep.txt`, `openspec/changes/extract-blob-castore-cache/evidence/i5-cargo-check-aspen-cache-kv-index-afterfmt.txt`

## Review Scope Snapshot

### `git diff HEAD -- Cargo.lock Cargo.toml crates/aspen-cache/Cargo.toml crates/aspen-cache/src/index.rs crates/aspen-cache/src/lib.rs crates/aspen-cache/src/signing.rs crates/aspen-castore/Cargo.toml crates/aspen-castore/src/circuit_breaker.rs crates/aspen-castore/src/client.rs crates/aspen-castore/src/lib.rs crates/aspen-nix-handler/Cargo.toml crates/aspen-snix/Cargo.toml openspec/changes/extract-blob-castore-cache/tasks.md openspec/changes/extract-blob-castore-cache/verification.md`

- Status: captured after I4/I5 implementation
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i4-i5-implementation-diff.txt`

## Verification Commands

### `cargo test -p aspen-castore circuit_breaker`

- Status: pass
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i4-cargo-test-aspen-castore-circuit-breaker.txt`

### `cargo check -p aspen-castore --no-default-features`

- Status: pass after formatting fallback
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i4-cargo-check-aspen-castore-default-afterfmt.txt`

### `cargo tree -p aspen-castore --no-default-features -e normal`

- Status: pass
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i4-cargo-tree-aspen-castore-default.txt`

### `rg -n 'aspen-core-shell|aspen-rpc-core|aspen-rpc-handlers|aspen-blob-handler' /tmp/i4-castore-tree.txt`

- Status: pass, no matches
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i4-forbidden-tree-grep.txt`

### `cargo test -p aspen-cache --no-default-features`

- Status: pass
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i5-cargo-test-aspen-cache-reusable.txt`

### `cargo check -p aspen-cache --no-default-features`

- Status: pass after formatting fallback
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i5-cargo-check-aspen-cache-default-afterfmt.txt`

### `cargo tree -p aspen-cache --no-default-features -e normal`

- Status: pass
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i5-cargo-tree-aspen-cache-default.txt`

### `rg -n 'aspen-core|aspen-core-shell|aspen-kv-types|aspen-blob|aspen-rpc-core|aspen-rpc-handlers|aspen-testing' /tmp/i5-cache-tree-default.txt`

- Status: pass, no matches
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i5-forbidden-tree-grep.txt`

### `cargo check -p aspen-cache --features kv-index`

- Status: pass after formatting fallback
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i5-cargo-check-aspen-cache-kv-index-afterfmt.txt`

### `cargo check -p aspen-nix-handler --features cache`

- Status: pass
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i5-cargo-check-aspen-nix-handler-cache.txt`

### `cargo check -p aspen-ci-executor-nix`

- Status: pass
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i5-cargo-check-aspen-ci-executor-nix.txt`

### `cargo check -p aspen-nix-cache-gateway`

- Status: pass
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i5-cargo-check-aspen-nix-cache-gateway.txt`

### `cargo check -p aspen-snix`

- Status: pass
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i5-cargo-check-aspen-snix.txt`

### `rustfmt --check crates/aspen-castore/src/circuit_breaker.rs crates/aspen-castore/src/client.rs crates/aspen-castore/src/lib.rs crates/aspen-cache/src/index.rs crates/aspen-cache/src/signing.rs crates/aspen-cache/src/lib.rs`

- Status: pass
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i4-i5-rustfmt-check.txt`

### `scripts/openspec-preflight.sh extract-blob-castore-cache`

- Status: pass after I4/I5
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i4-i5-openspec-preflight.txt`

## Notes

- `nix run .#rustfmt` was attempted twice but blocked on Nix auto-GC / big garbage collector lock. The changed Rust files were formatted with the repo toolchain `rustfmt` binary and verified with `rustfmt --check`; see `openspec/changes/extract-blob-castore-cache/evidence/i4-rustfmt-note.txt` and `openspec/changes/extract-blob-castore-cache/evidence/i4-i5-rustfmt-check.txt`.
- The castore tree can still include foundational `aspen-core` through `aspen-blob`; I4 specifically removes the castore circuit-breaker dependency on core-shell/runtime app code.
