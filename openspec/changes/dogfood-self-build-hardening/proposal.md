## Why

The cowsay dogfood loop works end-to-end (Forge → CI → nix build → blob store → run binary). The `ci-dogfood-self-build` VM test builds `aspen-constants` (a single zero-dependency crate). The `ci-dogfood-full-workspace` VM test pushes all 80 crates and builds `aspen-node` inside a VM — but it relies on pre-vendored cargo deps injected via `builtins.storePath` and `--impure`, side-stepping the real CI build pipeline that uses flake checks from the repo's own `flake.nix`.

The `dogfood-local.sh` script orchestrates the real-world loop: start cluster → push to Forge → CI auto-triggers → `nix build .#checks.x86_64-linux.build-node` → deploy → verify. This is the path that must work reliably for Aspen to build itself in production. Current gaps: the script has never been run to completion against the real 80-crate workspace in an automated, reproducible way. Failures are discovered manually and require debugging session-by-session.

## What Changes

- Run `dogfood-local.sh full-loop` end-to-end and fix every failure that surfaces — CI config issues, missing features, build timeouts, flake eval failures, deploy race conditions, binary validation.
- Add a NixOS VM integration test (`ci-dogfood-full-loop`) that exercises the complete `start → push → build → deploy → verify` cycle using the real `.aspen/ci.ncl` pipeline config (check → build → test stages), proving the self-build works in a reproducible, isolated environment.
- Harden the `dogfood-local.sh` script itself: fix error handling, improve log streaming reliability, handle edge cases in deploy (stale tickets, slow restarts, missing store paths).

## Capabilities

### New Capabilities
- `dogfood-full-loop-vm-test`: NixOS VM integration test that runs the complete self-build loop (Forge push → 3-stage CI pipeline → deploy → verify) in an isolated QEMU VM, using the real `.aspen/ci.ncl` config and the repo's own flake checks.

### Modified Capabilities

## Impact

- `scripts/dogfood-local.sh` — bug fixes and hardening throughout
- `.aspen/ci.ncl` — potential adjustments if CI config assumptions don't hold against real flake checks
- `nix/tests/` — new `ci-dogfood-full-loop.nix` VM test
- `flake.nix` — new check entry for the VM test
- `crates/aspen-ci-executor-nix/` — potential fixes if native eval or build execution fails on the real workspace
- `crates/aspen-ci/` — potential fixes if pipeline orchestration breaks under multi-stage real builds
