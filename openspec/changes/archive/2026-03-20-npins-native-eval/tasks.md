## 1. NixEvaluator: add drvPath resolution for npins projects

- [x] 1.1 Add `evaluate_npins_drv_path()` method to `NixEvaluator` in `eval.rs` — takes project dir + attribute, constructs Nix expression, evaluates with `evaluate_with_store()`, extracts drvPath string from result value
- [x] 1.2 Run eval on `spawn_blocking` since `snix_eval::Evaluation::evaluate()` is sync/CPU-bound
- [x] 1.3 Add unit test: evaluate a synthetic npins expression (no real fetching) to verify the eval→string extraction works

## 2. Executor: npins detection and native eval path

- [x] 2.1 Add `detect_project_type()` in `executor.rs` — checks for `npins/sources.json` in project dir, returns enum `{Flake, Npins, Unknown}`
- [x] 2.2 Add `try_npins_native_build()` in `executor.rs` — calls `evaluate_npins_drv_path()`, then follows the same path as `try_native_build()` (parse .drv → `LocalStoreBuildService` → ingest)
- [x] 2.3 Wire into `execute_build()`: if project is npins and snix-eval feature is enabled, try `try_npins_native_build()` first, fall back to subprocess on any error (including `NotImplemented` builtins)
- [x] 2.4 Add `NixEvaluator` to `NixBuildWorkerConfig` / `NixBuildWorker` when snix-eval feature is enabled

## 3. NixOS VM integration test

- [x] 3.1 Create `nix/tests/npins-native-eval.nix` — boots aspen-node, creates a minimal npins project (no nixpkgs, just a `derivation { ... }` literal), submits a CI job, verifies native build completes without subprocess
- [x] 3.2 Wire test into `flake.nix` checks
- [x] 3.3 Verify the cache gateway serves narinfo for the built output

## 4. Cleanup

- [x] 4.1 Update napkin with results
