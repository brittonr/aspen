# Verification Evidence

## Implementation Evidence

- Changed file: `.agent/napkin.md`
- Changed file: `AGENTS.md`
- Changed file: `openspec/changes/testing-harness-control-plane/evidence/markdownlint-harness-docs.txt`
- Changed file: `openspec/changes/testing-harness-control-plane/evidence/metadata-inventory.diff`
- Changed file: `openspec/changes/testing-harness-control-plane/evidence/openspec-preflight.txt`
- Changed file: `openspec/changes/testing-harness-control-plane/evidence/shellcheck-test-harness.txt`
- Changed file: `openspec/changes/testing-harness-control-plane/verification.md`

## Task Coverage

- [x] 1.1 Define a Nickel schema for test suite manifests, including suite ID, layer, owner, runtime class, prerequisites, tags, and execution target fields.
  - Evidence: `test-harness/schema.ncl`, `openspec/changes/testing-harness-control-plane/evidence/metadata-inventory.diff`, `openspec/changes/testing-harness-control-plane/evidence/test-harness-check.txt`

- [x] 1.2 Add a manifest directory and author initial Nickel manifests for representative Rust integration, patchbay, and NixOS VM suites.
  - Evidence: `test-harness/suites/rust/job-integration.ncl`, `test-harness/suites/rust/blob-replication.ncl`, `test-harness/suites/patchbay/patchbay-nat.ncl`, `test-harness/suites/patchbay/patchbay-fault.ncl`, `test-harness/suites/vm/multi-node-kv.ncl`, `test-harness/suites/vm/multi-node-blob.ncl`, `openspec/changes/testing-harness-control-plane/evidence/metadata-inventory.diff`

- [x] 1.3 Add a validation/export command that fails on invalid or stale suite metadata.
  - Evidence: `crates/aspen-testing/src/suite_inventory.rs`, `crates/aspen-testing/src/bin/aspen-test-harness.rs`, `scripts/test-harness.sh`, `openspec/changes/testing-harness-control-plane/evidence/test-harness-export.txt`, `openspec/changes/testing-harness-control-plane/evidence/test-harness-check.txt`, `openspec/changes/testing-harness-control-plane/evidence/cargo-test-suite-inventory.txt`, `openspec/changes/testing-harness-control-plane/evidence/nix-eval-harness-metadata-fail.txt`

- [x] 2.1 Generate a normalized machine-consumable inventory from the Nickel manifests for Rust and Nix consumers.
  - Evidence: `crates/aspen-testing/src/suite_inventory.rs`, `test-harness/generated/inventory.json`, `openspec/changes/testing-harness-control-plane/evidence/test-harness-export.txt`, `openspec/changes/testing-harness-control-plane/evidence/metadata-inventory.diff`

- [x] 2.2 Wire `flake.nix` VM or long-running suite registration to generated inventory data instead of a hand-maintained per-suite block.
  - Evidence: `nix/tests/lib/generated-harness-checks.nix`, `nix/tests/lib/test-harness-metadata.nix`, `flake.nix`, `test-harness/generated/inventory.json`, `openspec/changes/testing-harness-control-plane/evidence/nix-eval-generated-vm-checks.txt`, `openspec/changes/testing-harness-control-plane/evidence/nix-eval-flake-stale-inventory-fail.txt`, `openspec/changes/testing-harness-control-plane/evidence/nix-eval-harness-metadata-pass.txt`, `openspec/changes/testing-harness-control-plane/evidence/nix-eval-harness-metadata-fail.txt`, `openspec/changes/testing-harness-control-plane/evidence/metadata-inventory.diff`

- [x] 2.3 Update focused test selection entry points (`nextest` helpers, test scripts, or equivalent commands) to resolve grouping from generated inventory data.
  - Evidence: `scripts/test-harness.sh`, `test-harness/generated/inventory.json`, `openspec/changes/testing-harness-control-plane/evidence/test-harness-check.txt`, `openspec/changes/testing-harness-control-plane/evidence/test-harness-list-patchbay.txt`, `openspec/changes/testing-harness-control-plane/evidence/test-harness-list-vm-commands.txt`, `openspec/changes/testing-harness-control-plane/evidence/test-harness-list-stale-inventory.txt`, `openspec/changes/testing-harness-control-plane/evidence/shellcheck-test-harness.txt`, `openspec/changes/testing-harness-control-plane/evidence/metadata-inventory.diff`

- [x] 2.4 Register patchbay suites in the shared metadata and generated inventory path.
  - Evidence: `test-harness/suites/patchbay/patchbay-nat.ncl`, `test-harness/suites/patchbay/patchbay-fault.ncl`, `test-harness/generated/inventory.json`, `openspec/changes/testing-harness-control-plane/evidence/test-harness-list-patchbay.txt`, `openspec/changes/testing-harness-control-plane/evidence/metadata-inventory.diff`

- [x] 6.3 Update `AGENTS.md` and `.agent/napkin.md` with any non-obvious harness workflow changes discovered during implementation.
  - Evidence: `AGENTS.md`, `.agent/napkin.md`, `openspec/changes/testing-harness-control-plane/evidence/metadata-inventory.diff`

## Review Scope Snapshot

### `git diff HEAD -- .agent/napkin.md AGENTS.md openspec/changes/testing-harness-control-plane/evidence/markdownlint-harness-docs.txt openspec/changes/testing-harness-control-plane/evidence/openspec-preflight.txt openspec/changes/testing-harness-control-plane/evidence/shellcheck-test-harness.txt openspec/changes/testing-harness-control-plane/verification.md`

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

### `python - <<'PY' ...; nix eval --raw .#checks.x86_64-linux.multi-node-kv-test.name`

- Status: fail as expected after temporarily corrupting `test-harness/generated/inventory.json`, then restoring it
- Artifact: `openspec/changes/testing-harness-control-plane/evidence/nix-eval-flake-stale-inventory-fail.txt`

### `nix eval --impure --raw --expr 'let lib = (import <nixpkgs> {}).lib; helper = import ./nix/tests/lib/test-harness-metadata.nix { inherit lib; }; inventory = builtins.fromJSON (builtins.readFile ./test-harness/generated/inventory.json); in (helper.ensureInventoryFresh ./. inventory).metadata.inputs_sha256'`

- Status: pass
- Artifact: `openspec/changes/testing-harness-control-plane/evidence/nix-eval-harness-metadata-pass.txt`

### `nix eval --impure --expr 'let lib = (import <nixpkgs> {}).lib; helper = import ./nix/tests/lib/test-harness-metadata.nix { inherit lib; }; inventory = builtins.fromJSON (builtins.readFile ./test-harness/generated/inventory.json); broken = inventory // { metadata = inventory.metadata // { inputs_sha256 = "broken"; }; }; in helper.ensureInventoryFresh ./. broken'`

- Status: fail as expected
- Artifact: `openspec/changes/testing-harness-control-plane/evidence/nix-eval-harness-metadata-fail.txt`

### `python - <<'PY' ...; scripts/test-harness.sh list --layer vm --format ids`

- Status: fail as expected after temporarily corrupting `test-harness/generated/inventory.json`, then restoring it
- Artifact: `openspec/changes/testing-harness-control-plane/evidence/test-harness-list-stale-inventory.txt`

### `shellcheck scripts/test-harness.sh`

- Status: pass
- Artifact: `openspec/changes/testing-harness-control-plane/evidence/shellcheck-test-harness.txt`

### `markdownlint AGENTS.md openspec/changes/testing-harness-control-plane/*.md openspec/changes/testing-harness-control-plane/specs/**/*.md`

- Status: pass
- Artifact: `openspec/changes/testing-harness-control-plane/evidence/markdownlint-harness-docs.txt`

### `scripts/openspec-preflight.sh openspec/changes/testing-harness-control-plane`

- Status: pass
- Artifact: `openspec/changes/testing-harness-control-plane/evidence/openspec-preflight.txt`

## Notes

- This verification covers the metadata/inventory/control-plane slice only. Shared harness facade, wait helpers, and reporting tasks remain open.
- This follow-up narrows the changed-files snapshot to the review-evidence fixes plus the `AGENTS.md` and `.agent/napkin.md` harness notes; the cited code artifacts above remain unchanged from the earlier metadata implementation commits.
