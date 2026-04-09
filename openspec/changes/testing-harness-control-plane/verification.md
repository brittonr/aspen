# Verification Evidence

## Implementation Evidence

- Changed file: `AGENTS.md`
- Changed file: `.agent/napkin.md`
- Changed file: `crates/aspen-testing/Cargo.toml`
- Changed file: `crates/aspen-testing/src/lib.rs`
- Changed file: `crates/aspen-testing/src/suite_inventory.rs`
- Changed file: `crates/aspen-testing/src/bin/aspen-test-harness.rs`
- Changed file: `scripts/test-harness.sh`
- Changed file: `test-harness/schema.ncl`
- Changed file: `test-harness/suites/rust/job-integration.ncl`
- Changed file: `test-harness/suites/rust/blob-replication.ncl`
- Changed file: `test-harness/suites/patchbay/patchbay-nat.ncl`
- Changed file: `test-harness/suites/patchbay/patchbay-fault.ncl`
- Changed file: `test-harness/suites/vm/multi-node-kv.ncl`
- Changed file: `test-harness/suites/vm/multi-node-blob.ncl`
- Changed file: `test-harness/generated/inventory.json`
- Changed file: `nix/tests/lib/generated-harness-checks.nix`
- Changed file: `flake.nix`
- Changed file: `openspec/changes/testing-harness-control-plane/tasks.md`
- Changed file: `openspec/changes/testing-harness-control-plane/verification.md`

## Task Coverage

- [x] 1.1 Define a Nickel schema for test suite manifests, including suite ID, layer, owner, runtime class, prerequisites, tags, and execution target fields.
  - Evidence: `test-harness/schema.ncl`, `openspec/changes/testing-harness-control-plane/evidence/metadata-inventory.diff`, `openspec/changes/testing-harness-control-plane/evidence/test-harness-check.txt`

- [x] 1.2 Add a manifest directory and author initial Nickel manifests for representative Rust integration, patchbay, and NixOS VM suites.
  - Evidence: `test-harness/suites/rust/job-integration.ncl`, `test-harness/suites/rust/blob-replication.ncl`, `test-harness/suites/patchbay/patchbay-nat.ncl`, `test-harness/suites/patchbay/patchbay-fault.ncl`, `test-harness/suites/vm/multi-node-kv.ncl`, `test-harness/suites/vm/multi-node-blob.ncl`, `openspec/changes/testing-harness-control-plane/evidence/metadata-inventory.diff`

- [x] 1.3 Add a validation/export command that fails on invalid or stale suite metadata.
  - Evidence: `crates/aspen-testing/src/suite_inventory.rs`, `crates/aspen-testing/src/bin/aspen-test-harness.rs`, `scripts/test-harness.sh`, `openspec/changes/testing-harness-control-plane/evidence/test-harness-export.txt`, `openspec/changes/testing-harness-control-plane/evidence/test-harness-check.txt`, `openspec/changes/testing-harness-control-plane/evidence/cargo-test-suite-inventory.txt`

- [x] 2.1 Generate a normalized machine-consumable inventory from the Nickel manifests for Rust and Nix consumers.
  - Evidence: `crates/aspen-testing/src/suite_inventory.rs`, `test-harness/generated/inventory.json`, `openspec/changes/testing-harness-control-plane/evidence/test-harness-export.txt`, `openspec/changes/testing-harness-control-plane/evidence/metadata-inventory.diff`

- [x] 2.2 Wire `flake.nix` VM or long-running suite registration to generated inventory data instead of a hand-maintained per-suite block.
  - Evidence: `nix/tests/lib/generated-harness-checks.nix`, `flake.nix`, `test-harness/generated/inventory.json`, `openspec/changes/testing-harness-control-plane/evidence/nix-eval-generated-vm-checks.txt`, `openspec/changes/testing-harness-control-plane/evidence/metadata-inventory.diff`

- [x] 2.3 Update focused test selection entry points (`nextest` helpers, test scripts, or equivalent commands) to resolve grouping from generated inventory data.
  - Evidence: `scripts/test-harness.sh`, `test-harness/generated/inventory.json`, `openspec/changes/testing-harness-control-plane/evidence/test-harness-list-patchbay.txt`, `openspec/changes/testing-harness-control-plane/evidence/test-harness-list-vm-commands.txt`, `openspec/changes/testing-harness-control-plane/evidence/metadata-inventory.diff`

- [x] 2.4 Register patchbay suites in the shared metadata and generated inventory path.
  - Evidence: `test-harness/suites/patchbay/patchbay-nat.ncl`, `test-harness/suites/patchbay/patchbay-fault.ncl`, `test-harness/generated/inventory.json`, `openspec/changes/testing-harness-control-plane/evidence/test-harness-list-patchbay.txt`, `openspec/changes/testing-harness-control-plane/evidence/metadata-inventory.diff`

- [x] 6.3 Update `AGENTS.md` and `.agent/napkin.md` with any non-obvious harness workflow changes discovered during implementation.
  - Evidence: `AGENTS.md`, `.agent/napkin.md`, `openspec/changes/testing-harness-control-plane/evidence/metadata-inventory.diff`

## Review Scope Snapshot

### `git diff HEAD -- AGENTS.md .agent/napkin.md crates/aspen-testing/Cargo.toml crates/aspen-testing/src/lib.rs crates/aspen-testing/src/suite_inventory.rs crates/aspen-testing/src/bin/aspen-test-harness.rs scripts/test-harness.sh test-harness/schema.ncl test-harness/suites test-harness/generated/inventory.json nix/tests/lib/generated-harness-checks.nix flake.nix openspec/changes/testing-harness-control-plane/tasks.md openspec/changes/testing-harness-control-plane/verification.md`

- Status: captured
- Artifact: `openspec/changes/testing-harness-control-plane/evidence/metadata-inventory.diff`

## Verification Commands

### `cargo test -p aspen-testing suite_inventory --lib`

- Status: pass
- Artifact: `openspec/changes/testing-harness-control-plane/evidence/cargo-test-suite-inventory.txt`

### `scripts/test-harness.sh export`

- Status: pass
- Artifact: `openspec/changes/testing-harness-control-plane/evidence/test-harness-export.txt`

### `scripts/test-harness.sh check`

- Status: pass
- Artifact: `openspec/changes/testing-harness-control-plane/evidence/test-harness-check.txt`

### `scripts/test-harness.sh list --layer patchbay --format ids`

- Status: pass
- Artifact: `openspec/changes/testing-harness-control-plane/evidence/test-harness-list-patchbay.txt`

### `scripts/test-harness.sh list --layer vm --format commands`

- Status: pass
- Artifact: `openspec/changes/testing-harness-control-plane/evidence/test-harness-list-vm-commands.txt`

### `nix eval --raw .#checks.x86_64-linux.multi-node-kv-test.name && nix eval --raw .#checks.x86_64-linux.multi-node-blob-test.name`

- Status: pass
- Artifact: `openspec/changes/testing-harness-control-plane/evidence/nix-eval-generated-vm-checks.txt`

### `scripts/openspec-preflight.sh openspec/changes/testing-harness-control-plane`

- Status: pass
- Artifact: `openspec/changes/testing-harness-control-plane/evidence/openspec-preflight.txt`

## Notes

- This verification covers the metadata/inventory/control-plane slice only. Shared harness facade, wait helpers, and reporting tasks remain open.
