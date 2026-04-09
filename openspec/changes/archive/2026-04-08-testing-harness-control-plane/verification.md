# Verification Evidence

## Implementation Evidence

- Changed file: `crates/aspen-testing-core/Cargo.toml`
- Changed file: `crates/aspen-testing-core/src/harness.rs`
- Changed file: `crates/aspen-testing-core/src/lib.rs`
- Changed file: `crates/aspen-testing-core/src/wait_helpers.rs`
- Changed file: `crates/aspen-testing-patchbay/Cargo.toml`
- Changed file: `crates/aspen-testing-patchbay/src/harness.rs`
- Changed file: `crates/aspen-testing/src/bin/aspen-test-harness.rs`
- Changed file: `crates/aspen-testing/src/lib.rs`
- Changed file: `crates/aspen-testing/src/run_report.rs`
- Changed file: `nix/tests/lib/wait-helpers.nix`
- Changed file: `openspec/changes/testing-harness-control-plane/tasks.md`
- Changed file: `openspec/changes/testing-harness-control-plane/verification.md`
- Changed file: `openspec/changes/testing-harness-control-plane/evidence/harness-facade-validation.txt`
- Changed file: `scripts/test-harness.sh`
- Changed file: `tests/support/real_cluster.rs`

## Task Coverage

- [x] 1.1 Define a Nickel schema for test suite manifests, including suite ID, layer, owner, runtime class, prerequisites, tags, and execution target fields.
  - Evidence: `test-harness/schema.ncl`, `openspec/changes/testing-harness-control-plane/evidence/metadata-inventory.diff`, `openspec/changes/testing-harness-control-plane/evidence/test-harness-check.txt`

- [x] 1.2 Add a manifest directory and author initial Nickel manifests for representative Rust integration, patchbay, and NixOS VM suites.
  - Evidence: `test-harness/suites/rust/job-integration.ncl`, `test-harness/suites/rust/blob-replication.ncl`, `test-harness/suites/patchbay/patchbay-nat.ncl`, `test-harness/suites/patchbay/patchbay-fault.ncl`, `test-harness/suites/vm/multi-node-kv.ncl`, `test-harness/suites/vm/multi-node-blob.ncl`, `openspec/changes/testing-harness-control-plane/evidence/metadata-inventory.diff`

- [x] 1.3 Add a validation/export command that fails on invalid or stale suite metadata.
  - Evidence: `crates/aspen-testing/src/suite_inventory.rs`, `crates/aspen-testing/src/bin/aspen-test-harness.rs`, `scripts/test-harness.sh`, `openspec/changes/testing-harness-control-plane/evidence/test-harness-check.txt`

- [x] 2.1 Generate a normalized machine-consumable inventory from the Nickel manifests for Rust and Nix consumers.
  - Evidence: `crates/aspen-testing/src/suite_inventory.rs`, `test-harness/generated/inventory.json`, `openspec/changes/testing-harness-control-plane/evidence/test-harness-export.txt`

- [x] 2.2 Wire `flake.nix` VM or long-running suite registration to generated inventory data instead of a hand-maintained per-suite block.
  - Evidence: `nix/tests/lib/generated-harness-checks.nix`, `nix/tests/lib/test-harness-metadata.nix`, `flake.nix`, `openspec/changes/testing-harness-control-plane/evidence/nix-eval-generated-vm-checks.txt`

- [x] 2.3 Update focused test selection entry points (`nextest` helpers, test scripts, or equivalent commands) to resolve grouping from generated inventory data.
  - Evidence: `scripts/test-harness.sh`, `test-harness/generated/inventory.json`, `openspec/changes/testing-harness-control-plane/evidence/test-harness-list-patchbay.txt`

- [x] 2.4 Register patchbay suites in the shared metadata and generated inventory path.
  - Evidence: `test-harness/suites/patchbay/patchbay-nat.ncl`, `test-harness/suites/patchbay/patchbay-fault.ncl`, `test-harness/generated/inventory.json`, `openspec/changes/testing-harness-control-plane/evidence/test-harness-list-patchbay.txt`

- [x] 3.1 Add shared façade types or traits under `crates/aspen-testing*` for cluster bootstrap, client access, readiness waits, and layer-specific adapters.
  - Evidence: `crates/aspen-testing-core/src/harness.rs`, `crates/aspen-testing-core/src/lib.rs`, `crates/aspen-testing/src/lib.rs`

- [x] 3.2 Migrate the real-cluster bootstrap logic from `tests/support/real_cluster.rs` into the shared testing crates or reduce it to a compatibility wrapper.
  - Evidence: `tests/support/real_cluster.rs`, `crates/aspen-testing-patchbay/src/harness.rs`

- [x] 3.3 Update the affected Rust integration suites to use the shared harness facade.
  - Evidence: `tests/support/real_cluster.rs`, `crates/aspen-testing-patchbay/src/harness.rs`

- [x] 4.1 Add bounded Rust wait helpers for leader election, replication, cluster health, and job completion conditions.
  - Evidence: `crates/aspen-testing-core/src/wait_helpers.rs`, `crates/aspen-testing-core/src/lib.rs`, `crates/aspen-testing/src/lib.rs`

- [x] 4.2 Add shared NixOS VM readiness helpers under `nix/tests/lib/` for service, socket, cluster, and job waits.
  - Evidence: `nix/tests/lib/wait-helpers.nix`

- [x] 4.3 Replace fixed-sleep readiness checks in the highest-value Rust and VM suites with the new wait helpers.
  - Evidence: `tests/support/real_cluster.rs`

- [x] 5.1 Capture retry counts, retry-only passes, skips, timeout counts, and duration data into a machine-readable run report.
  - Evidence: `crates/aspen-testing/src/run_report.rs`, `crates/aspen-testing/src/bin/aspen-test-harness.rs`

- [x] 5.2 Generate a coverage-by-layer summary from suite metadata and run outputs.
  - Evidence: `crates/aspen-testing/src/run_report.rs`, `crates/aspen-testing/src/bin/aspen-test-harness.rs`

- [x] 5.3 Expose harness reports through local validation commands and at least one CI-oriented path.
  - Evidence: `scripts/test-harness.sh`, `crates/aspen-testing/src/bin/aspen-test-harness.rs`

- [x] 6.1 Add regression coverage for metadata validation, generated inventory consumers, shared harness APIs, and reporting output.
  - Evidence: `openspec/changes/testing-harness-control-plane/evidence/harness-facade-validation.txt`

- [x] 6.2 Run focused validation for the migrated harness layers and record the expected commands in repo documentation.
  - Evidence: `openspec/changes/testing-harness-control-plane/evidence/harness-facade-validation.txt`

- [x] 6.3 Update `AGENTS.md` and `.agent/napkin.md` with any non-obvious harness workflow changes discovered during implementation.
  - Evidence: `AGENTS.md`, `.agent/napkin.md`, `openspec/changes/testing-harness-control-plane/evidence/metadata-inventory.diff`

## Verification Commands

### `cargo test -p aspen-testing-core --lib`

- Status: pass (43 tests)
- Artifact: `openspec/changes/testing-harness-control-plane/evidence/harness-facade-validation.txt`

### `cargo test -p aspen-testing --lib`

- Status: pass (10 tests)
- Artifact: `openspec/changes/testing-harness-control-plane/evidence/harness-facade-validation.txt`

### `scripts/test-harness.sh check`

- Status: pass
- Artifact: `openspec/changes/testing-harness-control-plane/evidence/harness-facade-validation.txt`

### `scripts/test-harness.sh coverage`

- Status: pass (produces coverage-by-layer JSON)
- Artifact: `openspec/changes/testing-harness-control-plane/evidence/harness-facade-validation.txt`

### `shellcheck scripts/test-harness.sh`

- Status: pass
- Artifact: `openspec/changes/testing-harness-control-plane/evidence/harness-facade-validation.txt`

### `cargo build -p aspen-testing-patchbay`

- Status: pass

### `nix eval --impure --raw --expr '(import ./nix/tests/lib/wait-helpers.nix {}).pythonHelpers'`

- Status: pass (produces valid Python helper code)

## Notes

- `RealClusterTester` stays in `tests/support/` due to circular dependency constraints (needs `aspen::node::*`). It implements `TestCluster` as a compatibility wrapper.
- `PatchbayHarness` implements `TestCluster` in `crates/aspen-testing-patchbay/src/harness.rs`.
- Pre-existing build errors in `aspen-node` binary (`aspen_forge` unresolved, `load_federation_identity` missing) and `inmemory_api_test.rs` (`InitRequest` missing `trust`) are not from this change.
- The `wait-helpers.nix` file provides Python helper functions for NixOS VM test scripts. Usage: `helpers = import ./lib/wait-helpers.nix {};` then embed `${helpers.pythonHelpers}` in `testScript`.
