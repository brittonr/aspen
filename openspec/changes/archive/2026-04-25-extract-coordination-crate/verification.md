# Verification Evidence

## Implementation Evidence

- Changed file: `Cargo.lock`
- Changed file: `crates/aspen-coordination/Cargo.toml`
- Changed file: `crates/aspen-coordination/src/lib.rs`
- Changed file: `crates/aspen-coordination/src/queue/helpers.rs`
- Changed file: `crates/aspen-coordination/src/worker_coordinator/work_stealing.rs`
- Changed file: `crates/aspen-testing-madsim/src/madsim_tester/cluster_ops.rs`
- Changed file: `crates/aspen-testing-madsim/src/madsim_tester/mod.rs`
- Changed file: `docs/crate-extraction.md`
- Changed file: `docs/crate-extraction/coordination.md`
- Changed file: `docs/crate-extraction/policy.ncl`
- Changed file: `scripts/check-crate-extraction-readiness.rs`
- Changed file: `openspec/changes/archive/2026-04-25-extract-coordination-crate/specs/architecture-modularity/spec.md`
- Changed file: `openspec/changes/archive/2026-04-25-extract-coordination-crate/specs/coordination-extraction/spec.md`
- Changed file: `openspec/changes/prepare-crate-extraction/specs/architecture-modularity/spec.md`
- Changed file: `openspec/changes/archive/2026-04-25-extract-coordination-crate/tasks.md`
- Changed file: `openspec/changes/archive/2026-04-25-extract-coordination-crate/verification.md`
- Changed file: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/consumer-compat.md`
- Changed file: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/coordination-checks.md`
- Changed file: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/dependency-boundary.md`
- Changed file: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/downstream-consumer.md`
- Changed file: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/implementation-diff.patch`
- Changed file: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/protocol-check.md`
- Changed file: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/protocol-standalone.md`
- Changed file: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/openspec-preflight.txt`
- Changed file: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/openspec-validate.txt`
- Changed file: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/readiness-checker.json`
- Changed file: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/readiness-checker.md`
- Changed file: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/test-results.md`
- Changed file: `openspec/changes/archive/2026-04-25-extract-coordination-crate/fixtures/downstream-consumer/Cargo.lock`
- Changed file: `openspec/changes/archive/2026-04-25-extract-coordination-crate/fixtures/downstream-consumer/Cargo.toml`
- Changed file: `openspec/changes/archive/2026-04-25-extract-coordination-crate/fixtures/downstream-consumer/src/lib.rs`

## Task Coverage

- [x] I1 Replace `aspen-coordination`'s `aspen-core` dependency with direct `aspen-kv-types` imports: change `use aspen_core::ReadRequest` / `use aspen_core::ScanRequest` to `use aspen_kv_types::ReadRequest` / `use aspen_kv_types::ScanRequest` in `src/worker_coordinator/work_stealing.rs` and `src/queue/helpers.rs`, then remove `aspen-core` from `crates/aspen-coordination/Cargo.toml`. [covers=coordination-extraction.coordination-defaults-avoid-app-bundles.default-features-do-not-depend-on-aspen-core]
  - Evidence: `crates/aspen-coordination/Cargo.toml`, `crates/aspen-coordination/src/queue/helpers.rs`, `crates/aspen-coordination/src/worker_coordinator/work_stealing.rs`, `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/implementation-diff.patch`

- [x] I2 Verify `cargo check -p aspen-coordination` and `cargo check -p aspen-coordination --no-default-features` both pass after the dependency removal. [covers=coordination-extraction.coordination-defaults-avoid-app-bundles.default-features-compile-without-app-bundles]
  - Evidence: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/coordination-checks.md`

- [x] I3 Create `docs/crate-extraction/coordination.md` with candidate name, canonical class (`service library` for coordination, `protocol/wire` for protocol), intended audience, public API owner, readiness state, feature contract table, dependency decisions, compatibility plan, and verification rails following the established manifest format. [covers=coordination-extraction.has-extraction-manifest.manifest-exists-with-required-fields]
  - Evidence: `docs/crate-extraction/coordination.md`, `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/implementation-diff.patch`

- [x] I4 Add `aspen-coordination` and `aspen-coordination-protocol` entries to `docs/crate-extraction/policy.ncl` with appropriate classes, forbidden categories, allowed dependency categories, and tested feature sets. [covers=coordination-extraction.registered-in-policy.policy-includes-coordination-candidates]
  - Evidence: `docs/crate-extraction/policy.ncl`, `scripts/check-crate-extraction-readiness.rs`, `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/readiness-checker.md`, `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/implementation-diff.patch`

- [x] I5 Update the coordination family row in `docs/crate-extraction.md` broader candidate inventory with verified readiness state, manifest link to `docs/crate-extraction/coordination.md`, owner status, and next action. [covers=architecture-modularity.extraction-inventory-tracks-coordination.inventory-row-exists-for-coordination]
  - Evidence: `docs/crate-extraction.md`, `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/implementation-diff.patch`

- [x] V1 Save dependency-boundary evidence proving `aspen-coordination` default and no-default features do not pull `aspen-core`, `aspen-core-shell`, root `aspen`, handlers, binaries, trust/secrets/SQL crates (`aspen-trust`, `aspen-secrets`, `aspen-sql`), UI/web/gateway crates (`aspen-cli`, `aspen-tui`, `aspen-forge-web`, `aspen-nix-cache-gateway`, `aspen-snix-bridge`), concrete transport crates (`iroh`, `iroh-base`, `irpc`), or forbidden app-bundle crates through direct or transitive paths; save `cargo tree` output and forbidden-dependency grep for both `cargo tree -p aspen-coordination --edges normal` and `cargo tree -p aspen-coordination --no-default-features --edges normal` at `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/dependency-boundary.md`. [covers=coordination-extraction.coordination-defaults-avoid-app-bundles.default-features-compile-without-app-bundles,coordination-extraction.coordination-defaults-avoid-app-bundles.default-features-do-not-depend-on-aspen-core]
  - Evidence: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/dependency-boundary.md`

- [x] V2 Save protocol standalone evidence proving `aspen-coordination-protocol` has no Aspen dependencies and has only serialization-library normal dependencies; save `cargo tree` output and direct-dependency check at `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/protocol-standalone.md`. [covers=coordination-extraction.protocol-is-standalone.protocol-crate-has-no-aspen-dependencies]
  - Evidence: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/protocol-standalone.md`

- [x] V7 Run `cargo check -p aspen-coordination-protocol` and save compile evidence at `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/protocol-check.md`; compilation SHALL succeed. [covers=coordination-extraction.protocol-is-standalone.protocol-crate-compiles-standalone]
  - Evidence: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/protocol-check.md`

- [x] V3 Run the extraction-readiness checker with `--candidate-family coordination` and save the output at `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/readiness-checker.md`; the checker SHALL exit 0. [covers=coordination-extraction.checker-verifies-coordination.checker-passes-for-coordination-family]
  - Evidence: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/readiness-checker.md`, `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/readiness-checker.json`

- [x] V4 Create a downstream-style consumer fixture that depends only on `aspen-coordination`, `aspen-kv-types`, and `aspen-traits`, uses at least one coordination primitive type, and compile it; save `cargo metadata` evidence at `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/downstream-consumer.md` proving no dependency on root `aspen` package or any Aspen binary/handler crate. [covers=coordination-extraction.downstream-consumer-proof.consumer-compiles-without-root-aspen-dependency]
  - Evidence: `openspec/changes/archive/2026-04-25-extract-coordination-crate/fixtures/downstream-consumer/Cargo.toml`, `openspec/changes/archive/2026-04-25-extract-coordination-crate/fixtures/downstream-consumer/src/lib.rs`, `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/downstream-consumer.md`

- [x] V5 Save consumer compatibility evidence for all 9 workspace consumers: `cargo check` for `aspen-cluster`, `aspen-core-essentials-handler`, `aspen-job-handler`, `aspen-jobs`, `aspen-rpc-core`, `aspen-rpc-handlers`, `aspen-testing`, `aspen-client-api`, plus `cargo check -p aspen-testing --features jobs` and `cargo check -p aspen-raft --features coordination`; save results at `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/consumer-compat.md`. [covers=coordination-extraction.consumers-still-compile.direct-consumers-compile,coordination-extraction.consumers-still-compile.optional-consumers-compile]
  - Evidence: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/consumer-compat.md`

- [x] V6 Run `cargo nextest run -p aspen-coordination` and save test results at `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/test-results.md`; all existing tests SHALL pass. [covers=coordination-extraction.coordination-defaults-avoid-app-bundles.default-features-compile-without-app-bundles]
  - Evidence: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/test-results.md`

## Review Scope Snapshot

### `git diff --cached HEAD -- Cargo.lock crates/aspen-coordination crates/aspen-testing-madsim docs/crate-extraction.md docs/crate-extraction scripts/check-crate-extraction-readiness.rs openspec/changes/archive/2026-04-25-extract-coordination-crate/specs openspec/changes/archive/2026-04-25-extract-coordination-crate/tasks.md openspec/changes/archive/2026-04-25-extract-coordination-crate/fixtures openspec/changes/prepare-crate-extraction/specs/architecture-modularity/spec.md`

- Status: captured
- Artifact: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/implementation-diff.patch`

## Verification Commands

### `cargo check -p aspen-coordination`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/coordination-checks.md`

### `cargo check -p aspen-coordination --no-default-features`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/coordination-checks.md`

### `cargo tree -p aspen-coordination --edges normal`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/dependency-boundary.md`

### `cargo tree -p aspen-coordination-protocol --edges normal`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/protocol-standalone.md`

### `cargo check -p aspen-coordination-protocol`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/protocol-check.md`

### `scripts/check-crate-extraction-readiness.rs --policy docs/crate-extraction/policy.ncl --inventory docs/crate-extraction.md --manifest-dir docs/crate-extraction --candidate-family coordination --output-json openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/readiness-checker.json --output-markdown openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/readiness-checker.md`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/readiness-checker.md`

### `cargo check --manifest-path openspec/changes/archive/2026-04-25-extract-coordination-crate/fixtures/downstream-consumer/Cargo.toml`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/downstream-consumer.md`

### Consumer compatibility `cargo check` matrix

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/consumer-compat.md`

### `cargo nextest run -p aspen-coordination`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/test-results.md`

### `openspec validate extract-coordination-crate --strict` (pre-archive)

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/openspec-validate.txt`

### `openspec validate architecture-modularity --strict`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/openspec-validate.txt`

### `openspec validate coordination-extraction --strict`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/openspec-validate.txt`

### `scripts/openspec-preflight.sh openspec/changes/archive/2026-04-25-extract-coordination-crate`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/openspec-preflight.txt`
