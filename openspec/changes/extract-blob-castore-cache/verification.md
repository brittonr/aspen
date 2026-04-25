# Verification Evidence

## Implementation Evidence

- Changed file: `docs/crate-extraction/blob-castore-cache.md`
- Changed file: `docs/crate-extraction/policy.ncl`
- Changed file: `scripts/check-crate-extraction-readiness.rs`
- Changed file: `openspec/changes/extract-blob-castore-cache/tasks.md`
- Changed file: `openspec/changes/extract-blob-castore-cache/verification.md`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/v1-v5-summary.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/v1-v5-implementation-diff.txt`
- Changed file: `openspec/changes/extract-blob-castore-cache/evidence/v1-v5-openspec-preflight.txt`

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

- [x] V1 Save compile and `cargo tree` evidence for reusable feature sets of `aspen-blob`, `aspen-castore`, and `aspen-cache`, proving only documented backend-purpose dependencies are present and forbidden app/runtime shells are absent. [covers=blob-castore-cache-extraction.blob-default-avoids-app-shells.iroh-backend-is-documented-exception,blob-castore-cache-extraction.blob-default-avoids-app-shells.replication-rpc-is-adapter-only,blob-castore-cache-extraction.castore-cache-avoid-app-shells.castore-circuit-breaker-is-reusable-or-gated,blob-castore-cache-extraction.castore-cache-avoid-app-shells.cache-metadata-signing-reusable] ✅ completed: 2026-04-25T19:21:55Z
  - Evidence: `openspec/changes/extract-blob-castore-cache/evidence/v1-cargo-check-aspen-blob-default.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v1-cargo-tree-aspen-blob-default.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v1-aspen-blob-forbidden-grep.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v1-cargo-check-aspen-castore-default.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v1-cargo-tree-aspen-castore-default.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v1-aspen-castore-forbidden-grep.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v1-cargo-check-aspen-cache-default.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v1-cargo-tree-aspen-cache-default.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v1-aspen-cache-forbidden-grep.txt`

- [x] V2 Save downstream fixture compile/metadata evidence for blob and cache/castore fixtures. [covers=blob-castore-cache-extraction.downstream-fixtures.blob-fixture-uses-canonical-api,blob-castore-cache-extraction.downstream-fixtures.cache-castore-fixture-uses-domain-apis] ✅ completed: 2026-04-25T19:21:55Z
  - Evidence: `openspec/changes/extract-blob-castore-cache/evidence/i6-downstream-blob-test.txt`, `openspec/changes/extract-blob-castore-cache/evidence/i6-downstream-blob-metadata.json`, `openspec/changes/extract-blob-castore-cache/evidence/i6-downstream-cache-castore-test.txt`, `openspec/changes/extract-blob-castore-cache/evidence/i6-downstream-cache-castore-metadata.json`

- [x] V3 Save focused tests for blob storage operations, castore adapter behavior, cache metadata/signing positive cases, and negative cases for malformed narinfo/signature data or unavailable adapter features. [covers=blob-castore-cache-extraction.castore-cache-avoid-app-shells.cache-metadata-signing-reusable] ✅ completed: 2026-04-25T19:21:55Z
  - Evidence: `openspec/changes/extract-blob-castore-cache/evidence/v3-cargo-test-aspen-blob-memory-store.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v3-cargo-test-aspen-castore-circuit-breaker.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v3-cargo-test-aspen-cache-reusable.txt`

- [x] V4 Save compatibility compile/test evidence for the full runtime consumer set named in the proposal: `aspen-rpc-core`, blob RPC handlers, `aspen-snix`, `aspen-snix-bridge`, `aspen-nix-cache-gateway`, and CI/cache executors with explicit integration features. [covers=blob-castore-cache-extraction.compatibility-adapters-verified,blob-castore-cache-extraction.compatibility-adapters-verified.runtime-consumers-compile] ✅ completed: 2026-04-25T19:21:55Z
  - Evidence: `openspec/changes/extract-blob-castore-cache/evidence/v4-cargo-check-aspen-rpc-core-blob.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v4-cargo-check-aspen-blob-handler.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v4-cargo-check-aspen-snix.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v4-cargo-check-aspen-snix-bridge.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v4-cargo-check-aspen-nix-cache-gateway.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v4-cargo-check-aspen-ci-executor-nix.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v4-cargo-check-aspen-nix-handler-cache.txt`

- [x] V5 Run `scripts/check-crate-extraction-readiness.rs --candidate-family blob-castore-cache` and save output plus negative mutations for forbidden app-shell dependency, undocumented backend exception, missing owner, invalid readiness state, and missing downstream fixture evidence. [covers=blob-castore-cache-extraction.inventory-and-policy.checker-verifies-backend-exceptions] ✅ completed: 2026-04-25T19:21:55Z
  - Evidence: `openspec/changes/extract-blob-castore-cache/evidence/v5-readiness.md`, `openspec/changes/extract-blob-castore-cache/evidence/v5-readiness.json`, `openspec/changes/extract-blob-castore-cache/evidence/v5-negative-mutations-summary.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v5-negative-forbidden-app-summary.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v5-negative-undocumented-backend-summary.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v5-negative-missing-owner-summary.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v5-negative-invalid-readiness-summary.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v5-negative-missing-downstream.txt`

## Review Scope Snapshot

### `git diff HEAD -- docs/crate-extraction/blob-castore-cache.md docs/crate-extraction/policy.ncl scripts/check-crate-extraction-readiness.rs openspec/changes/extract-blob-castore-cache/tasks.md openspec/changes/extract-blob-castore-cache/verification.md`

- Status: captured after V1-V5 verification
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/v1-v5-implementation-diff.txt`

## Verification Commands

### Reusable feature compile/tree checks

- Status: pass
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/v1-cargo-check-aspen-blob-default.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v1-cargo-check-aspen-castore-default.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v1-cargo-check-aspen-cache-default.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v1-cargo-tree-aspen-blob-default.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v1-cargo-tree-aspen-castore-default.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v1-cargo-tree-aspen-cache-default.txt`

### Focused test checks

- Status: pass
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/v3-cargo-test-aspen-blob-memory-store.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v3-cargo-test-aspen-castore-circuit-breaker.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v3-cargo-test-aspen-cache-reusable.txt`

### Runtime compatibility checks

- Status: pass
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/v4-cargo-check-aspen-rpc-core-blob.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v4-cargo-check-aspen-blob-handler.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v4-cargo-check-aspen-snix.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v4-cargo-check-aspen-snix-bridge.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v4-cargo-check-aspen-nix-cache-gateway.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v4-cargo-check-aspen-ci-executor-nix.txt`, `openspec/changes/extract-blob-castore-cache/evidence/v4-cargo-check-aspen-nix-handler-cache.txt`

### `scripts/check-crate-extraction-readiness.rs --policy docs/crate-extraction/policy.ncl --inventory docs/crate-extraction.md --manifest-dir docs/crate-extraction --candidate-family blob-castore-cache --output-json openspec/changes/extract-blob-castore-cache/evidence/v5-readiness.json --output-markdown openspec/changes/extract-blob-castore-cache/evidence/v5-readiness.md`

- Status: pass
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/v5-readiness.md`, `openspec/changes/extract-blob-castore-cache/evidence/v5-readiness.json`, `openspec/changes/extract-blob-castore-cache/evidence/v5-readiness.stdout`, `openspec/changes/extract-blob-castore-cache/evidence/v5-readiness.stderr`

### `openspec/changes/extract-blob-castore-cache/evidence/v5-run-negative-mutations.sh`

- Status: pass
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/v5-negative-mutations-summary.txt`
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/v5-negative-forbidden-app-summary.txt`
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/v5-negative-undocumented-backend-summary.txt`
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/v5-negative-missing-owner-summary.txt`
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/v5-negative-invalid-readiness-summary.txt`
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/v5-negative-missing-downstream.txt`

### `scripts/openspec-preflight.sh extract-blob-castore-cache`

- Status: pass after V1-V5
- Artifact: `openspec/changes/extract-blob-castore-cache/evidence/v1-v5-openspec-preflight.txt`

## Notes

- `aspen-blob` still carries a documented default exception for `aspen-core` via `BlobAwareKeyValueStore`; the manifest records this as a later seam.
- The checker now enforces downstream fixture evidence for the blob/castore/cache family.
