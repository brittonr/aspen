## Why

Aspen has spent the last month on operational hardening — tiger style cleanup, write forwarding, multi-node dogfood, cluster discovery. All 683 unit tests pass. But no one has run the full dogfood pipeline end-to-end after these changes landed, the 63 NixOS VM tests haven't been audited as a group, and the snix native build path still has hardcoded assumptions that block multi-arch and submodule support.

This sweep validates what we built and closes the gaps before moving to new features.

## What Changes

- Run the full dogfood pipeline (`full-loop`) and fix any breakage from recent cluster discovery, write forwarding, and health gate changes
- Audit all 63 NixOS VM tests, categorize pass/fail, fix regressions
- Remove hardcoded `x86_64-linux` from snix eval and wire up submodule parsing in flake lock resolution
- Add tests for the snix fixes

## Capabilities

### New Capabilities

- `dogfood-validation`: End-to-end dogfood pipeline verification after the hardening sprint
- `vm-test-audit`: Systematic audit of NixOS VM test health across all 63 tests
- `snix-multi-arch`: System detection and submodule support in the snix native build path

### Modified Capabilities

- `snix-native-builds`: Eval system detection and flake lock submodule parsing

## Impact

- `scripts/dogfood-local.sh` — potential fixes if pipeline breaks on recent changes
- `nix/tests/*.nix` — fixes to any regressed VM tests
- `crates/aspen-ci-executor-nix/src/eval.rs` — replace hardcoded system string with detection
- `crates/aspen-ci-executor-nix/src/flake_lock.rs` — parse submodules from locked input
- `crates/aspen-ci-executor-nix/src/worker.rs`, `payload.rs` — thread system through build pipeline
