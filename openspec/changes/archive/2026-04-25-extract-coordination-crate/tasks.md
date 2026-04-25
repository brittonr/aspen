## 1. Dependency cleanup

- [x] I1 Replace `aspen-coordination`'s `aspen-core` dependency with direct `aspen-kv-types` imports: change `use aspen_core::ReadRequest` / `use aspen_core::ScanRequest` to `use aspen_kv_types::ReadRequest` / `use aspen_kv_types::ScanRequest` in `src/worker_coordinator/work_stealing.rs` and `src/queue/helpers.rs`, then remove `aspen-core` from `crates/aspen-coordination/Cargo.toml`. [covers=coordination-extraction.coordination-defaults-avoid-app-bundles.default-features-do-not-depend-on-aspen-core]

- [x] I2 Verify `cargo check -p aspen-coordination` and `cargo check -p aspen-coordination --no-default-features` both pass after the dependency removal. [covers=coordination-extraction.coordination-defaults-avoid-app-bundles.default-features-compile-without-app-bundles]

## 2. Extraction manifest and policy

- [x] I3 Create `docs/crate-extraction/coordination.md` with candidate name, canonical class (`service library` for coordination, `protocol/wire` for protocol), intended audience, public API owner, readiness state, feature contract table, dependency decisions, compatibility plan, and verification rails following the established manifest format. [covers=coordination-extraction.has-extraction-manifest.manifest-exists-with-required-fields]

- [x] I4 Add `aspen-coordination` and `aspen-coordination-protocol` entries to `docs/crate-extraction/policy.ncl` with appropriate classes, forbidden categories, allowed dependency categories, and tested feature sets. [covers=coordination-extraction.registered-in-policy.policy-includes-coordination-candidates]

- [x] I5 Update the coordination family row in `docs/crate-extraction.md` broader candidate inventory with verified readiness state, manifest link to `docs/crate-extraction/coordination.md`, owner status, and next action. [covers=architecture-modularity.extraction-inventory-tracks-coordination.inventory-row-exists-for-coordination]

## 3. Verification

- [x] V1 Save dependency-boundary evidence proving `aspen-coordination` default and no-default features do not pull `aspen-core`, `aspen-core-shell`, root `aspen`, handlers, binaries, trust/secrets/SQL crates (`aspen-trust`, `aspen-secrets`, `aspen-sql`), UI/web/gateway crates (`aspen-cli`, `aspen-tui`, `aspen-forge-web`, `aspen-nix-cache-gateway`, `aspen-snix-bridge`), concrete transport crates (`iroh`, `iroh-base`, `irpc`), or forbidden app-bundle crates through direct or transitive paths; save `cargo tree` output and forbidden-dependency grep for both `cargo tree -p aspen-coordination --edges normal` and `cargo tree -p aspen-coordination --no-default-features --edges normal` at `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/dependency-boundary.md`. [covers=coordination-extraction.coordination-defaults-avoid-app-bundles.default-features-compile-without-app-bundles,coordination-extraction.coordination-defaults-avoid-app-bundles.default-features-do-not-depend-on-aspen-core]

- [x] V2 Save protocol standalone evidence proving `aspen-coordination-protocol` has no Aspen dependencies and has only serialization-library normal dependencies; save `cargo tree` output and direct-dependency check at `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/protocol-standalone.md`. [covers=coordination-extraction.protocol-is-standalone.protocol-crate-has-no-aspen-dependencies]

- [x] V7 Run `cargo check -p aspen-coordination-protocol` and save compile evidence at `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/protocol-check.md`; compilation SHALL succeed. [covers=coordination-extraction.protocol-is-standalone.protocol-crate-compiles-standalone]

- [x] V3 Run the extraction-readiness checker with `--candidate-family coordination` and save the output at `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/readiness-checker.md`; the checker SHALL exit 0. [covers=coordination-extraction.checker-verifies-coordination.checker-passes-for-coordination-family]

- [x] V4 Create a downstream-style consumer fixture that depends only on `aspen-coordination`, `aspen-kv-types`, and `aspen-traits`, uses at least one coordination primitive type, and compile it; save `cargo metadata` evidence at `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/downstream-consumer.md` proving no dependency on root `aspen` package or any Aspen binary/handler crate. [covers=coordination-extraction.downstream-consumer-proof.consumer-compiles-without-root-aspen-dependency]

- [x] V5 Save consumer compatibility evidence for all 9 workspace consumers: `cargo check` for `aspen-cluster`, `aspen-core-essentials-handler`, `aspen-job-handler`, `aspen-jobs`, `aspen-rpc-core`, `aspen-rpc-handlers`, `aspen-testing`, `aspen-client-api`, plus `cargo check -p aspen-testing --features jobs` and `cargo check -p aspen-raft --features coordination`; save results at `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/consumer-compat.md`. [covers=coordination-extraction.consumers-still-compile.direct-consumers-compile,coordination-extraction.consumers-still-compile.optional-consumers-compile]

- [x] V6 Run `cargo nextest run -p aspen-coordination` and save test results at `openspec/changes/archive/2026-04-25-extract-coordination-crate/evidence/test-results.md`; all existing tests SHALL pass. [covers=coordination-extraction.coordination-defaults-avoid-app-bundles.default-features-compile-without-app-bundles]
