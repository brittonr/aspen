## Phase 1: Research and Fixture Boundary

- [x] [serial] Record current `ci-dogfood-full-loop-test` failure class and root fixture dependency seam. ✅ researched from `/nix/store/3ip63zr7b9hbi3901c7nba8qbh1a02nx-vm-test-run-ci-dogfood-full-loop.drv` showing `syntax-check` failed on public `nixpkgs` registry/DNS lookup.
- [ ] [serial] Replace lockless `inputs.nixpkgs.url = "nixpkgs"` in the full-loop sample flake with a deterministic local/store-backed input strategy or equivalent no-external-input fixture.
- [ ] [depends:fixture-offline] Add or preserve a negative assertion/evidence check that job logs do not contain `channels.nixos.org`, `flake-registry.json`, or public registry lock-update attempts.

## Phase 2: CI Full-Loop Proof

- [ ] [depends:fixture-offline] Verify the VM node package feature set for `ci-dogfood-full-loop-test` includes CI Nix job execution support (`ci`, `shell-worker`, `snix`, `snix-build`, and expected fallback support).
- [ ] [depends:fixture-offline] Run focused `nix build .#checks.x86_64-linux.ci-dogfood-full-loop-test --no-link -L` and capture evidence that all three stages complete in order.
- [ ] [depends:full-loop-pass] Confirm the focused test still executes or inspects the CI-built artifact/stage output rather than merely passing status polling.

## Phase 3: Flake Rail Recovery

- [ ] [depends:full-loop-pass] Run `git diff --check`, `scripts/test-harness.sh export`, and `scripts/test-harness.sh check` after implementation.
- [ ] [depends:hygiene] Run a fresh full `nix flake check -L` and capture the log path; do not promote full dogfood/self-hosting acceptance unless this passes.
