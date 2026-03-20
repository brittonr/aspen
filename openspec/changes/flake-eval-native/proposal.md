## Why

The CI executor's native build path for flake projects still shells out to `nix eval --raw .drvPath` to resolve a flake reference to a derivation store path. The npins path is already fully in-process (zero subprocesses) via snix-eval. Eliminating the last `nix eval` subprocess from the flake path removes the runtime dependency on the `nix` binary for build execution, reduces latency, and unifies the evaluation model across project types.

## What Changes

- Parse `flake.lock` in Rust, resolve locked inputs to store paths using nix-compat's `build_ca_path`
- Implement a `fetchTreeFinal` equivalent that fetches missing inputs (github tarballs via snix's `fetchTarball`, path inputs via copy)
- Embed Nix's `call-flake.nix` expression (90 lines, pure Nix) and evaluate it via snix-eval with pre-resolved overrides
- Navigate the evaluated flake outputs to `.drvPath`, extract the `Derivation` from snix-glue's `KnownPaths`
- Wire the new `evaluate_flake_derivation` method into `NixBuildWorker::try_native_build` as the primary path, falling back to `nix eval` subprocess only when in-process eval fails
- **BREAKING**: The `nix-cli-fallback` feature flag changes meaning — it now also covers `nix eval` fallback, not just `nix build`

## Capabilities

### New Capabilities

- `flake-lock-resolution`: Parse flake.lock JSON, resolve input graph (follows, relative paths), compute store paths from narHash via nix-compat, fetch missing inputs
- `flake-eval-native`: Evaluate flake.nix in-process via snix-eval using call-flake.nix protocol with pre-resolved overrides, extract Derivation from KnownPaths

### Modified Capabilities

## Impact

- `crates/aspen-ci-executor-nix/src/eval.rs` — new `evaluate_flake_derivation` method on `NixEvaluator`
- `crates/aspen-ci-executor-nix/src/executor.rs` — `try_native_build` calls in-process eval before falling back to `resolve_drv_path` subprocess
- New module `crates/aspen-ci-executor-nix/src/flake_lock.rs` — lock file parsing and input resolution
- NixOS VM test `nix/tests/snix-native-build.nix` updated to verify zero-subprocess path for flakes
- Dependencies: nix-compat (already present), serde_json (already present), snix-eval/snix-glue (already present behind `snix-eval` feature)
