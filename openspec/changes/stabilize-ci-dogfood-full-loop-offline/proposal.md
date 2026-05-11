## Why

A fresh full `nix flake check -L` still fails at `ci-dogfood-full-loop-test`. The focused VM run now reaches the CI pipeline, starts the `syntax-check` Nix job, and fails because the checked-out sample flake has `inputs.nixpkgs.url = "nixpkgs"` with no `flake.lock`; inside the VM, `nix build` tries to update the flake registry and fetch `https://channels.nixos.org/flake-registry.json`, which is unavailable in the NixOS test sandbox. This is not proof that Aspen's CI stage orchestration is broken; it is a non-deterministic fixture/input-resolution boundary leaking into the acceptance rail.

The existing `multi-node-dogfood-test` was made deterministic by replacing registry/network-dependent sample inputs with local store-backed fixtures. `ci-dogfood-full-loop-test` needs the same explicit offline contract while preserving its higher-level purpose: proving staged Forge→CI orchestration, job dependencies, final artifact execution, and blob artifact storage.

## What Changes

- Make full-loop CI dogfood fixture inputs deterministic and offline-safe.
- Require the VM test to fail if inner Nix attempts the public flake registry or network-dependent `nixpkgs` resolution.
- Preserve the three-stage pipeline proof: `format-check` and `syntax-check` in `check`, `build-and-test` after `check`, and `unit-tests` after `build`.
- Require failure evidence to distinguish Aspen CI/job orchestration failures from fixture dependency-resolution failures.

## Capabilities

### Modified Capabilities
- `snix-build-default`: CI-capable dogfood VM tests that execute Nix jobs must use feature-complete CI binaries and deterministic flake inputs.

## Impact

- **Files**: `nix/tests/ci-dogfood-full-loop.nix`, possibly `flake.nix` CI VM feature wiring, and test evidence under the active change.
- **APIs**: No public Rust API changes are required.
- **Dependencies**: No new network dependency is allowed; the fixture should use store-resident or copied local inputs.
- **Testing**: focused `nix build .#checks.x86_64-linux.ci-dogfood-full-loop-test --no-link -L`, `git diff --check`, `scripts/test-harness.sh export`, `scripts/test-harness.sh check`, and then a fresh full `nix flake check -L` before dogfood acceptance is promoted.
