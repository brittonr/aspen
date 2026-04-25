# Verification Evidence

## Implementation Evidence

- Changed file: `crates/aspen-client-api/tests/client_rpc_postcard_baseline.rs`
- Changed file: `docs/crate-extraction.md`
- Changed file: `docs/crate-extraction/protocol-wire.md`
- Changed file: `docs/crate-extraction/policy.ncl`
- Changed file: `scripts/check-crate-extraction-readiness.rs`
- Changed file: `openspec/changes/extract-protocol-wire-crates/tasks.md`
- Changed file: `openspec/changes/extract-protocol-wire-crates/verification.md`
- Changed file: `openspec/changes/extract-protocol-wire-crates/fixtures/downstream-protocol-wire/Cargo.toml`
- Changed file: `openspec/changes/extract-protocol-wire-crates/fixtures/downstream-protocol-wire/Cargo.lock`
- Changed file: `openspec/changes/extract-protocol-wire-crates/fixtures/downstream-protocol-wire/src/lib.rs`
- Changed file: `openspec/changes/extract-protocol-wire-crates/evidence/implementation-diff.txt`
- Changed file: `openspec/changes/extract-protocol-wire-crates/evidence/summary.txt`
- Changed file: `openspec/changes/extract-protocol-wire-crates/evidence/run-verification.sh`
- Changed file: `openspec/changes/extract-protocol-wire-crates/evidence/v5-run-negative-mutations.sh`
- Changed file: `openspec/changes/extract-protocol-wire-crates/evidence/v5-readiness.md`
- Changed file: `openspec/changes/extract-protocol-wire-crates/evidence/v5-negative-mutations-summary.txt`
- Changed file: `openspec/changes/extract-protocol-wire-crates/evidence/openspec-preflight.txt`

## Task Coverage

- [x] R1 Capture baseline compile, `cargo tree`, feature, and existing compatibility-test evidence for `aspen-client-api`, `aspen-forge-protocol`, `aspen-jobs-protocol`, and `aspen-coordination-protocol`, including any test-only `std` serializer usage. [covers=protocol-wire-extraction.defaults-avoid-runtime-shells,protocol-wire-extraction.defaults-avoid-runtime-shells.client-api-default-graph-is-reusable,protocol-wire-extraction.defaults-avoid-runtime-shells.domain-protocols-have-no-runtime-deps] ✅ completed: 2026-04-25T19:47:10Z
  - Evidence: `openspec/changes/extract-protocol-wire-crates/evidence/r1-baseline.md`, `openspec/changes/extract-protocol-wire-crates/evidence/r1-baseline-logs/`

- [x] I1 Expand `docs/crate-extraction/protocol-wire.md` into a complete manifest with per-crate metadata, feature contracts, dependencies, compatibility policy, representative consumers, and verification rails. [covers=protocol-wire-extraction.inventory-and-policy,protocol-wire-extraction.inventory-and-policy.policy-checker-verifies-family] ✅ completed: 2026-04-25T19:47:10Z
  - Evidence: `docs/crate-extraction/protocol-wire.md`, `openspec/changes/extract-protocol-wire-crates/evidence/implementation-diff.txt`

- [x] I2 Add protocol/wire candidates to `docs/crate-extraction/policy.ncl` and update `docs/crate-extraction.md` readiness rows and next actions. [covers=protocol-wire-extraction.inventory-and-policy.policy-checker-verifies-family] ✅ completed: 2026-04-25T19:47:10Z
  - Evidence: `docs/crate-extraction/policy.ncl`, `docs/crate-extraction.md`, `scripts/check-crate-extraction-readiness.rs`, `openspec/changes/extract-protocol-wire-crates/evidence/implementation-diff.txt`

- [x] I3 Add or refresh golden/postcard compatibility tests for `ClientRpcRequest`, `ClientRpcResponse`, and schema-critical Forge/jobs/coordination protocol types, including negative tests that catch inserted enum variants or changed encoded baselines. [covers=protocol-wire-extraction.wire-compatibility-reviewable,protocol-wire-extraction.wire-compatibility-reviewable.enum-discriminants-stay-stable,protocol-wire-extraction.wire-compatibility-reviewable.golden-artifacts-are-saved] ✅ completed: 2026-04-25T19:47:10Z
  - Evidence: `crates/aspen-client-api/tests/client_rpc_postcard_baseline.rs`, `openspec/changes/extract-protocol-wire-crates/evidence/i3-client-api-compatibility-tests.txt`, `openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence/client-rpc-postcard-baseline.json`

- [x] I4 Ensure auth and hook/ticket wire surfaces use portable leaf crates (`aspen-auth-core`, `aspen-hooks-ticket`, protocol crates) by default and do not depend on runtime `aspen-auth` or `aspen-hooks` handlers. [covers=protocol-wire-extraction.portable-auth-ticket-types,protocol-wire-extraction.portable-auth-ticket-types.auth-feature-uses-auth-core,protocol-wire-extraction.portable-auth-ticket-types.hook-tickets-use-ticket-crate] ✅ completed: 2026-04-25T19:47:10Z
  - Evidence: `crates/aspen-client-api/Cargo.toml`, `openspec/changes/extract-protocol-wire-crates/evidence/v2-cargo-tree-aspen-client-api.txt`, `openspec/changes/extract-protocol-wire-crates/evidence/v2-client-api-forbidden-grep.txt`

- [x] I5 Add a downstream serialization fixture that imports canonical protocol crates, encodes/decodes representative client and domain protocol values, and records metadata proving runtime/app crates are absent. [covers=protocol-wire-extraction.downstream-serialization-proof,protocol-wire-extraction.downstream-serialization-proof.fixture-serializes-canonical-types] ✅ completed: 2026-04-25T19:47:10Z
  - Evidence: `openspec/changes/extract-protocol-wire-crates/fixtures/downstream-protocol-wire/`, `openspec/changes/extract-protocol-wire-crates/evidence/i5-downstream-protocol-wire-test.txt`, `openspec/changes/extract-protocol-wire-crates/evidence/i5-downstream-protocol-wire-metadata.json`, `openspec/changes/extract-protocol-wire-crates/evidence/i5-downstream-protocol-wire-forbidden-grep.txt`

- [x] V1 Save compile evidence for `cargo check -p aspen-client-api`, `cargo check -p aspen-client-api --no-default-features`, supported wasm target checks, and equivalent checks for Forge/jobs/coordination protocol crates. [covers=protocol-wire-extraction.defaults-avoid-runtime-shells.client-api-default-graph-is-reusable,protocol-wire-extraction.defaults-avoid-runtime-shells.domain-protocols-have-no-runtime-deps] ✅ completed: 2026-04-25T19:47:10Z
  - Evidence: `openspec/changes/extract-protocol-wire-crates/evidence/v1-cargo-check-aspen-client-api.txt`, `openspec/changes/extract-protocol-wire-crates/evidence/v1-cargo-check-aspen-client-api-no-default.txt`, `openspec/changes/extract-protocol-wire-crates/evidence/v1-cargo-check-client-api-wasm.txt`, `openspec/changes/extract-protocol-wire-crates/evidence/v1-cargo-check-aspen-forge-protocol.txt`, `openspec/changes/extract-protocol-wire-crates/evidence/v1-cargo-check-aspen-forge-protocol-no-default.txt`, `openspec/changes/extract-protocol-wire-crates/evidence/v1-cargo-check-aspen-jobs-protocol.txt`, `openspec/changes/extract-protocol-wire-crates/evidence/v1-cargo-check-aspen-jobs-protocol-no-default.txt`, `openspec/changes/extract-protocol-wire-crates/evidence/v1-cargo-check-aspen-coordination-protocol.txt`, `openspec/changes/extract-protocol-wire-crates/evidence/v1-cargo-check-aspen-coordination-protocol-no-default.txt`

- [x] V2 Save dependency-boundary evidence from `cargo tree` and deterministic checker output proving forbidden runtime/app crates are absent from default reusable protocol graphs. [covers=protocol-wire-extraction.defaults-avoid-runtime-shells.client-api-default-graph-is-reusable,protocol-wire-extraction.defaults-avoid-runtime-shells.domain-protocols-have-no-runtime-deps] ✅ completed: 2026-04-25T19:47:10Z
  - Evidence: `openspec/changes/extract-protocol-wire-crates/evidence/v2-cargo-tree-aspen-client-api.txt`, `openspec/changes/extract-protocol-wire-crates/evidence/v2-cargo-tree-aspen-client-api-no-default.txt`, `openspec/changes/extract-protocol-wire-crates/evidence/v2-client-api-forbidden-grep.txt`, `openspec/changes/extract-protocol-wire-crates/evidence/v2-cargo-tree-aspen-forge-protocol.txt`, `openspec/changes/extract-protocol-wire-crates/evidence/v2-aspen-forge-protocol-forbidden-grep.txt`, `openspec/changes/extract-protocol-wire-crates/evidence/v2-cargo-tree-aspen-jobs-protocol.txt`, `openspec/changes/extract-protocol-wire-crates/evidence/v2-aspen-jobs-protocol-forbidden-grep.txt`, `openspec/changes/extract-protocol-wire-crates/evidence/v2-cargo-tree-aspen-coordination-protocol.txt`, `openspec/changes/extract-protocol-wire-crates/evidence/v2-aspen-coordination-protocol-forbidden-grep.txt`

- [x] V3 Run and save protocol compatibility tests with positive encode/decode cases and negative baseline-change cases. [covers=protocol-wire-extraction.wire-compatibility-reviewable.enum-discriminants-stay-stable,protocol-wire-extraction.wire-compatibility-reviewable.golden-artifacts-are-saved] ✅ completed: 2026-04-25T19:47:10Z
  - Evidence: `openspec/changes/extract-protocol-wire-crates/evidence/v3-cargo-test-aspen-client-api.txt`, `openspec/changes/extract-protocol-wire-crates/evidence/v3-cargo-test-aspen-forge-protocol.txt`, `openspec/changes/extract-protocol-wire-crates/evidence/v3-cargo-test-aspen-jobs-protocol.txt`, `openspec/changes/extract-protocol-wire-crates/evidence/v3-cargo-test-aspen-coordination-protocol.txt`

- [x] V4 Save downstream fixture `cargo metadata`, `cargo check`, and serialization test evidence. [covers=protocol-wire-extraction.downstream-serialization-proof.fixture-serializes-canonical-types] ✅ completed: 2026-04-25T19:47:10Z
  - Evidence: `openspec/changes/extract-protocol-wire-crates/evidence/i5-downstream-protocol-wire-test.txt`, `openspec/changes/extract-protocol-wire-crates/evidence/i5-downstream-protocol-wire-metadata.json`, `openspec/changes/extract-protocol-wire-crates/evidence/i5-downstream-protocol-wire-packages.txt`, `openspec/changes/extract-protocol-wire-crates/evidence/i5-downstream-protocol-wire-forbidden-grep.txt`

- [x] V5 Run `scripts/check-crate-extraction-readiness.rs --candidate-family protocol-wire` and save output plus negative mutations for forbidden dependency, missing owner, missing compatibility rail, and invalid readiness state. [covers=protocol-wire-extraction.inventory-and-policy.policy-checker-verifies-family] ✅ completed: 2026-04-25T19:47:10Z
  - Evidence: `openspec/changes/extract-protocol-wire-crates/evidence/v5-readiness.md`, `openspec/changes/extract-protocol-wire-crates/evidence/v5-readiness.json`, `openspec/changes/extract-protocol-wire-crates/evidence/v5-negative-mutations-summary.txt`, `openspec/changes/extract-protocol-wire-crates/evidence/v5-negative-forbidden-runtime-summary.txt`, `openspec/changes/extract-protocol-wire-crates/evidence/v5-negative-missing-owner-summary.txt`, `openspec/changes/extract-protocol-wire-crates/evidence/v5-negative-invalid-readiness-summary.txt`, `openspec/changes/extract-protocol-wire-crates/evidence/v5-negative-missing-compatibility-summary.txt`, `openspec/changes/extract-protocol-wire-crates/evidence/v5-negative-missing-downstream-summary.txt`

## Verification Commands

### `openspec/changes/extract-protocol-wire-crates/evidence/run-verification.sh`

- Status: pass after baseline path fix
- Artifact: `openspec/changes/extract-protocol-wire-crates/evidence/v1-cargo-check-aspen-client-api.txt`
- Artifact: `openspec/changes/extract-protocol-wire-crates/evidence/v2-client-api-forbidden-grep.txt`
- Artifact: `openspec/changes/extract-protocol-wire-crates/evidence/v3-cargo-test-aspen-client-api.txt`

### `CARGO_TARGET_DIR=/tmp/aspen-protocol-wire-fixture cargo test --manifest-path openspec/changes/extract-protocol-wire-crates/fixtures/downstream-protocol-wire/Cargo.toml`

- Status: pass
- Artifact: `openspec/changes/extract-protocol-wire-crates/evidence/i5-downstream-protocol-wire-test.txt`

### `scripts/check-crate-extraction-readiness.rs --policy docs/crate-extraction/policy.ncl --inventory docs/crate-extraction.md --manifest-dir docs/crate-extraction --candidate-family protocol-wire --output-json openspec/changes/extract-protocol-wire-crates/evidence/v5-readiness.json --output-markdown openspec/changes/extract-protocol-wire-crates/evidence/v5-readiness.md`

- Status: pass
- Artifact: `openspec/changes/extract-protocol-wire-crates/evidence/v5-readiness.md`
- Artifact: `openspec/changes/extract-protocol-wire-crates/evidence/v5-readiness.json`

### `openspec/changes/extract-protocol-wire-crates/evidence/v5-run-negative-mutations.sh`

- Status: pass
- Artifact: `openspec/changes/extract-protocol-wire-crates/evidence/v5-negative-mutations-summary.txt`
- Artifact: `openspec/changes/extract-protocol-wire-crates/evidence/v5-negative-forbidden-runtime-summary.txt`
- Artifact: `openspec/changes/extract-protocol-wire-crates/evidence/v5-negative-missing-owner-summary.txt`
- Artifact: `openspec/changes/extract-protocol-wire-crates/evidence/v5-negative-invalid-readiness-summary.txt`
- Artifact: `openspec/changes/extract-protocol-wire-crates/evidence/v5-negative-missing-compatibility-summary.txt`
- Artifact: `openspec/changes/extract-protocol-wire-crates/evidence/v5-negative-missing-downstream-summary.txt`

### `scripts/openspec-preflight.sh extract-protocol-wire-crates`

- Status: pass
- Artifact: `openspec/changes/extract-protocol-wire-crates/evidence/openspec-preflight.txt`
