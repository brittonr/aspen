## Phase 1: Research and Fixture Boundary

- [x] [serial] Record current `ci-dogfood-full-loop-test` failure class and root fixture dependency seam. ✅ researched from `/nix/store/3ip63zr7b9hbi3901c7nba8qbh1a02nx-vm-test-run-ci-dogfood-full-loop.drv` showing `syntax-check` failed on public `nixpkgs` registry/DNS lookup.
- [x] [serial] Replace lockless `inputs.nixpkgs.url = "nixpkgs"` in the full-loop sample flake with a deterministic local/store-backed input strategy or equivalent no-external-input fixture. ✅ replaced with an input-free `builtins.derivation` fixture using store-resident BusyBox builders.
- [x] [depends:fixture-offline] Add or preserve a negative assertion/evidence check that job logs do not contain `channels.nixos.org`, `flake-registry.json`, or public registry lock-update attempts. ✅ VM test checks job logs plus `aspen-node` journal for registry/lock-update markers.

## Phase 2: CI Full-Loop Proof

- [x] [depends:fixture-offline] Verify the VM node package feature set for `ci-dogfood-full-loop-test` includes CI Nix job execution support (`ci`, `shell-worker`, `snix`, `snix-build`, and expected fallback support). ✅ `flake.nix` uses `ciVmTestBin` with `ci`, `shell-worker`, `snix`, `snix-build`, and `nix-cli-fallback` for this check.
- [x] [depends:fixture-offline] Run focused `nix build .#checks.x86_64-linux.ci-dogfood-full-loop-test --no-link -L` and capture evidence that all three stages complete in order. ✅ passed with log `target/flake-check/ci-dogfood-full-loop-20260511T133134Z.log`; marker: `FULL-LOOP PASSED: 3-stage pipeline → format + check + build + unit-tests → binary execution`.
- [x] [depends:full-loop-pass] Confirm the focused test still executes or inspects the CI-built artifact/stage output rather than merely passing status polling. ✅ focused VM ran `/nix/store/hahnzaqk070r06vzf9ccaq07k1dfnij8-aspen-constants-0.1.0/bin/aspen-constants-check`, verified output markers, and confirmed blob upload metadata.

## Phase 3: Flake Rail Recovery

- [x] [depends:full-loop-pass] Run `git diff --check`, `scripts/test-harness.sh export`, and `scripts/test-harness.sh check` after implementation. ✅ all passed after implementation.
- [x] [depends:hygiene] Run a fresh full `nix flake check -L` and capture the log path; do not promote full dogfood/self-hosting acceptance unless this passes. ✅ passed with serialized local rail `nix flake check -L --max-jobs 1`; log `target/flake-check/full-serial-20260511T155741Z.log`; marker: `all checks passed!`. A parallel `--max-jobs auto` attempt exposed VM-test host contention in `multi-node-kv`, but the focused `multi-node-kv-test` passed separately and the serialized full rail passed.
