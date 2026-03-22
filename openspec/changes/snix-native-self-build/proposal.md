## Why

The CI self-build test (`ci-dogfood-self-build`) proves Aspen can build its own Rust code, but it shells out to `nix build` for the actual compilation. The native snix-build pipeline (LocalStoreBuildService + bubblewrap sandbox) exists but has three gaps that prevent it from handling real stdenv derivations: missing input closures in the sandbox, no output registration in the local nix store, and snix-eval always falling back to the nix CLI. Closing these gaps makes the self-build genuinely native — Aspen builds its own code without delegating to the nix CLI.

## What Changes

- Fix output registration so native build results appear at the expected `/nix/store` path (currently `nix store add` content-addresses to a different hash, and the read-only store overlay blocks `cp -a`)
- Fix snix-eval for real flakes so the eval phase doesn't fall back to `nix eval` subprocess (currently fails with missing `flake.lock` and parse errors)
- Switch `ci-dogfood-self-build-test` to use `full-aspen-node-plugins-snix-build` once both fixes land
- Add focused unit/integration tests for each fix to enable fast iteration (the current VM test takes ~7 minutes per cycle)

## Capabilities

### New Capabilities

- `native-output-registration`: Register bwrap sandbox build outputs in the local `/nix/store` at the correct derivation output path, working through the nix daemon on read-only store overlays
- `snix-eval-flake-support`: Make snix-eval handle real flakes with nixpkgs inputs — generate `flake.lock` when missing, handle flake-compat eval for stdenv derivations

### Modified Capabilities

## Impact

- `crates/aspen-ci-executor-nix/src/build_service.rs` — output registration after bwrap build
- `crates/aspen-ci-executor-nix/src/eval.rs` — snix-eval flake handling
- `crates/aspen-ci-executor-nix/src/executor.rs` — eval fallback logic
- `flake.nix` — self-build test package switch
- `nix/tests/ci-dogfood-self-build.nix` — test assertions for native path
- New test files for focused unit/integration tests of each component
