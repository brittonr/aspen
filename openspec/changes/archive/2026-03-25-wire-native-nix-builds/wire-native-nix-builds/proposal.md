## Why

The snix-build `NativeBuildService`, snix-eval `NixEvaluator`, and the `derivation_to_build_request` conversion all exist and compile, but `execute_build()` in executor.rs unconditionally shells out to `nix build` every time. The native path (`init_native_build_service` → `build_derivation`) is plumbed but never called from the main build flow. Wiring these together eliminates the subprocess dependency for builds, cuts process-spawn overhead, and lets Aspen run CI builds on systems without the Nix CLI installed.

## What Changes

- `execute_build()` in executor.rs gains a native-first path: when `snix-build` feature is enabled and `NativeBuildService` is initialized, it evaluates the flake to a `.drv` path, parses the derivation, and calls `execute_native()` from build_service.rs
- Falls back to `spawn_nix_build()` subprocess on any native-path failure (eval error, sandbox unavailable, IFD)
- The eval→drv bridge uses a hybrid approach: `nix eval --raw .#attr.drvPath` subprocess to get the drv path, then `parse_derivation()` on the `.drv` file — avoids the unsolved snix-eval-to-derivation-struct gap while keeping the actual build in-process
- `NixBuildWorker::init_native_build_service()` is called during worker startup (in the node binary) when `snix-build` feature is enabled
- Native build outputs are uploaded to PathInfoService via `upload_native_outputs()`, which already exists

## Capabilities

### New Capabilities

- `native-build-dispatch`: The logic in `execute_build()` that decides between native and subprocess paths, resolves flake refs to derivations, and handles fallback

### Modified Capabilities

## Impact

- `crates/aspen-ci-executor-nix/src/executor.rs`: Main change — native-first dispatch in `execute_build()`
- `crates/aspen-ci-executor-nix/src/build_service.rs`: Minor — `execute_native()` loses `#[allow(dead_code)]`
- Node binary startup code: Calls `init_native_build_service()` on the worker
- Feature gating: All new code behind `#[cfg(feature = "snix-build")]`, zero impact when feature is off
- No API changes, no new dependencies, no breaking changes
