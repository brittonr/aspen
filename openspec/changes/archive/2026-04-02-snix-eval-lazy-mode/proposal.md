## Why

Aspen's CI executor achieves native builds (bwrap sandbox, 48ms) but still shells out to `nix eval` for flake evaluation. The in-process snix-eval path fails because all eval callsites use `EvalMode::Strict`, which deep-forces the entire result tree — hitting unimplemented thunks in parts of the flake outputs we don't need. Switching to lazy evaluation and improving error diagnostics would eliminate the last `nix` subprocess dependency in the build pipeline.

## What Changes

- Switch `EvalMode::Strict` → `EvalMode::Lazy` for flake-compat and call-flake eval paths where we only need `.drvPath`
- Add error chain reporting to `convert_eval_result` so inner causes aren't swallowed
- Update the `snix-flake-native-build-test` VM test to assert "zero subprocesses" when lazy eval succeeds
- If lazy mode alone doesn't resolve all cases, vendor snix-eval with targeted fixes (builtins.path filter, fetchGit completeness)

## Capabilities

### New Capabilities

- `lazy-flake-eval`: Switch snix-eval to lazy mode for flake evaluation, eliminating unnecessary deep-forcing of the entire outputs tree when only `.drvPath` is needed
- `eval-error-chain`: Surface the full error chain from snix-eval's `NativeError` wrapper so root causes are visible in logs and job results

### Modified Capabilities

## Impact

- `crates/aspen-ci-executor-nix/src/eval.rs` — 5 `EvalMode::Strict` callsites, `convert_eval_result` error formatting
- `crates/aspen-ci-executor-nix/src/executor.rs` — eval fallback logic may need adjustment if lazy mode changes return semantics
- `nix/tests/snix-flake-native-build.nix` — strengthen assertion from "GOOD" (subprocess fallback) to "BEST" (zero subprocesses)
- snix-eval dependency (`Cargo.toml`) — may need to vendor/fork if builtins are missing
