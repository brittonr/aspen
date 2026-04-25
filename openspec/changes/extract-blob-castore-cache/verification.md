# Verification Evidence

## Implementation Evidence

- Changed file: `openspec/changes/extract-blob-castore-cache/fixtures/downstream-blob/Cargo.toml`
- Changed file: `openspec/changes/extract-blob-castore-cache/fixtures/downstream-blob/src/lib.rs`
- Changed file: `openspec/changes/extract-blob-castore-cache/fixtures/downstream-cache-castore/Cargo.toml`
- Changed file: `openspec/changes/extract-blob-castore-cache/fixtures/downstream-cache-castore/src/lib.rs`
- Changed file: `openspec/changes/extract-blob-castore-cache/tasks.md`
- Changed file: `openspec/changes/extract-blob-castore-cache/verification.md`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i6-downstream-blob-test.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i6-downstream-blob-metadata.json`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i6-downstream-blob-packages.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i6-downstream-blob-forbidden-grep.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i6-downstream-cache-castore-test.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i6-downstream-cache-castore-metadata.json`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i6-downstream-cache-castore-packages.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i6-downstream-cache-castore-forbidden-grep.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i6-rustfmt-check.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i6-summary.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i6-implementation-diff.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/i6-openspec-preflight.txt`

## Task Coverage

- [x] R1 Capture baseline compile and dependency graphs for `aspen-blob`, `aspen-castore`, `aspen-cache`, and representative consumers, classifying each dependency as backend-purpose, reusable domain, adapter/runtime, test-only, or forbidden. [covers=blob-castore-cache-extraction.castore-cache-avoid-app-shells,blob-castore-cache-extraction.blob-default-avoids-app-shells,blob-castore-cache-extraction.blob-default-avoids-app-shells.iroh-backend-is-documented-exception,blob-castore-cache-extraction.castore-cache-avoid-app-shells.castore-circuit-breaker-is-reusable-or-gated,blob-castore-cache-extraction.castore-cache-avoid-app-shells.cache-metadata-signing-reusable] ✅ completed: 2026-04-25T18:49:10Z
  - Evidence: `openspec/changes/extract-blob-castore-cache/evidence/r1-baseline.md`, `openspec/changes/extract-blob-castore-cache/evidence/r1-baseline-logs/`

- [x] I1 Create `docs/crate-extraction/blob-castore-cache.md` with per-crate metadata, backend-purpose dependency exceptions, feature contracts, compatibility plan, representative consumers, and verification rails. [covers=blob-castore-cache-extraction.inventory-and-policy,blob-castore-cache-extraction.inventory-and-policy.checker-verifies-backend-exceptions] ✅ completed: 2026-04-25T19:09:10Z
  - Evidence: `docs/crate-extraction/blob-castore-cache.md`, `openspec/changes/extract-blob-castore-cache/evidence/i1-i2-policy-summary.txt`, `openspec/changes/extract-blob-castore-cache/evidence/i1-i2-implementation-diff.txt`

- [x] I2 Add blob/castore/cache candidates to `docs/crate-extraction/policy.ncl` and update `docs/crate-extraction.md` with readiness state and next action. [covers=blob-castore-cache-extraction.inventory-and-policy.checker-verifies-backend-exceptions] ✅ completed: 2026-04-25T19:09:10Z
  - Evidence: `docs/crate-extraction/policy.ncl`, `docs/crate-extraction.md`, `scripts/check-crate-extraction-readiness.rs`, `openspec/changes/extract-blob-castore-cache/evidence/i1-i2-policy-summary.txt`, `openspec/changes/extract-blob-castore-cache/evidence/i1-i2-implementation-diff.txt`

- [x] I3 Move or gate `aspen-blob` replication/client-RPC dependencies so default reusable blob APIs do not depend on `aspen-client-api`, handlers, root Aspen, or node bootstrap crates. [covers=blob-castore-cache-extraction.blob-default-avoids-app-shells.replication-rpc-is-adapter-only] ✅ completed: 2026-04-25T18:51:25Z
  - Evidence: `crates/aspen-blob/Cargo.toml`, `crates/aspen-blob/src/lib.rs`, `crates/aspen-rpc-core/Cargo.toml`, `crates/aspen-cluster/Cargo.toml`, `crates/aspen-rpc-handlers/Cargo.toml`, `Cargo.toml`, `openspec/changes/extract-blob-castore-cache/evidence/i3-cargo-tree-aspen-blob-default.txt`, `openspec/changes/extract-blob-castore-cache/evidence/i3-forbidden-tree-grep.txt`

- [x] I4 Replace, localize, or feature-gate `aspen-castore`'s `aspen-core-shell` circuit-breaker dependency so reusable castore APIs avoid core-shell/runtime app crates by default. [covers=blob-castore-cache-extraction.castore-cache-avoid-app-shells.castore-circuit-breaker-is-reusable-or-gated] ✅ completed: 2026-04-25T18:55:00Z
  - Evidence: `crates/aspen-castore/Cargo.toml`, `crates/aspen-castore/src/circuit_breaker.rs`, `crates/aspen-castore/src/client.rs`, `crates/aspen-castore/src/lib.rs`, `openspec/changes/extract-blob-castore-cache/evidence/i4-cargo-check-aspen-castore-default-afterfmt.txt`, `openspec/changes/extract-blob-castore-cache/evidence/i4-forbidden-tree-grep.txt`

- [x] I5 Separate `aspen-cache` reusable Nix cache metadata/signing helpers from cluster/testing/runtime integration, keeping publication paths behind named features or adapter crates. [covers=blob-castore-cache-extraction.castore-cache-avoid-app-shells.cache-metadata-signing-reusable] ✅ completed: 2026-04-25T19:03:50Z
  - Evidence: `crates/aspen-cache/Cargo.toml`, `crates/aspen-cache/src/index.rs`, `crates/aspen-cache/src/lib.rs`, `crates/aspen-cache/src/signing.rs`, `crates/aspen-nix-handler/Cargo.toml`, `crates/aspen-snix/Cargo.toml`, `openspec/changes/extract-blob-castore-cache/evidence/i5-cargo-tree-aspen-cache-default.txt`, `openspec/changes/extract-blob-castore-cache/evidence/i5-forbidden-tree-grep.txt`, `openspec/changes/extract-blob-castore-cache/evidence/i5-cargo-check-aspen-cache-kv-index-afterfmt.txt`

- [x] I6 Add downstream fixtures for canonical blob APIs and cache/castore domain APIs, with cargo metadata proving app-shell and handler crates are absent. [covers=blob-castore-cache-extraction.downstream-fixtures,blob-castore-cache-extraction.downstream-fixtures.blob-fixture-uses-canonical-api,blob-castore-cache-extraction.downstream-fixtures.cache-castore-fixture-uses-domain-apis] ✅ completed: 2026-04-25T19:15:10Z
  - Evidence: `openspec/changes/extract-blob-castore-cache/fixtures/downstream-blob/`, `openspec/changes/extract-blob-castore-cache/fixtures/downstream-cache-castore/`, `openspec/changes/extract-blob-castore-cache/evidence/i6-downstream-blob-test.txt`, `openspec/changes/extract-blob-castore-cache/evidence/i6-downstream-cache-castore-test.txt`, `openspec/changes/extract-blob-castore-cache/evidence/i6-downstream-blob-forbidden-grep.txt`, `openspec/changes/extract-blob-castore-cache/evidence/i6-downstream-cache-castore-forbidden-grep.txt`

## Review Scope Snapshot

### `git diff HEAD -- openspec/changes/extract-blob-castore-cache/fixtures openspec/changes/extract-blob-castore-cache/tasks.md openspec/changes/extract-blob-castore-cache/verification.md`

- Status: captured after I6 downstream fixtures
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i6-implementation-diff.txt`

## Verification Commands

### `cargo test --manifest-path openspec/changes/extract-blob-castore-cache/fixtures/downstream-blob/Cargo.toml`

- Status: pass
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i6-downstream-blob-test.txt`

### `cargo metadata --format-version 1 --manifest-path openspec/changes/extract-blob-castore-cache/fixtures/downstream-blob/Cargo.toml`

- Status: pass
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i6-downstream-blob-metadata.json`

### `rg -n '^(aspen|aspen-client-api|aspen-rpc-core|aspen-rpc-handlers|aspen-blob-handler|aspen-cluster|aspen-cli|aspen-tui|aspen-dogfood)$' openspec/changes/extract-blob-castore-cache/evidence/i6-downstream-blob-packages.txt`

- Status: pass, no matches
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i6-downstream-blob-forbidden-grep.txt`

### `cargo test --manifest-path openspec/changes/extract-blob-castore-cache/fixtures/downstream-cache-castore/Cargo.toml`

- Status: pass
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i6-downstream-cache-castore-test.txt`

### `cargo metadata --format-version 1 --manifest-path openspec/changes/extract-blob-castore-cache/fixtures/downstream-cache-castore/Cargo.toml`

- Status: pass
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i6-downstream-cache-castore-metadata.json`

### `rg -n '^(aspen|aspen-client-api|aspen-rpc-core|aspen-rpc-handlers|aspen-blob-handler|aspen-cluster|aspen-cli|aspen-tui|aspen-dogfood)$' openspec/changes/extract-blob-castore-cache/evidence/i6-downstream-cache-castore-packages.txt`

- Status: pass, no matches
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i6-downstream-cache-castore-forbidden-grep.txt`

### `rustfmt --check openspec/changes/extract-blob-castore-cache/fixtures/downstream-blob/src/lib.rs openspec/changes/extract-blob-castore-cache/fixtures/downstream-cache-castore/src/lib.rs`

- Status: pass
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i6-rustfmt-check.txt`

### `scripts/openspec-preflight.sh extract-blob-castore-cache`

- Status: pass after I6
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/i6-openspec-preflight.txt`

## Notes

- Fixture builds used `CARGO_TARGET_DIR=/tmp/aspen-downstream-fixtures` to avoid filling the nearly-full repo root filesystem.
