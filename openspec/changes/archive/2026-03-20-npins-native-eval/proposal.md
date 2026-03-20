## Why

The native snix-build pipeline still shells out to `nix eval --raw .drvPath` because snix-eval lacks flake resolution (lock files, input fetching, recursive outputs function). For projects that use npins instead of flakes, evaluation is plain Nix (`import ./npins` → `builtins.fetchTarball`) that snix-eval already supports. Wiring this in eliminates the last subprocess call, making CI builds fully in-process.

## What Changes

- Add an npins-based eval path in `aspen-ci-executor-nix` that uses `NixEvaluator::evaluate_with_store()` to resolve `.drvPath` without spawning `nix eval`
- Detect project type (flake vs npins) and select eval strategy accordingly
- Create a test project using npins to prove the fully-native pipeline: snix-eval → parse .drv → bwrap sandbox → ingest output → upload to PathInfoService
- Add NixOS VM integration test exercising the zero-subprocess build path

## Capabilities

### New Capabilities

- `npins-eval`: In-process Nix evaluation for npins-based projects via snix-eval, replacing the `nix eval` subprocess

### Modified Capabilities

## Impact

- `crates/aspen-ci-executor-nix/src/executor.rs` — new `try_native_eval()` path alongside existing `resolve_drv_path()`
- `crates/aspen-ci-executor-nix/src/eval.rs` — `NixEvaluator` gains a method to evaluate an npins project to a drv path
- `nix/tests/` — new VM test for the npins-native pipeline
- No changes to the flake-based eval path (it remains as fallback)
- snix-eval's `fetchGit` is not implemented; only npins projects using `fetchTarball` (GitHub/GitLab/Channel pins) work today
