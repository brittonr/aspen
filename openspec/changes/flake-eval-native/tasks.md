## 1. Flake Lock Parser

- [x] 1.1 Create `flake_lock.rs` module with `FlakeLock`, `FlakeNode`, `LockedInput` structs and `parse()` from JSON
- [x] 1.2 Implement follows resolution (`resolve_input` traverses input spec paths from root)
- [x] 1.3 Implement `compute_store_path` from narHash using nix-compat `build_ca_path`
- [x] 1.4 Implement `check_local_availability` — probe `/nix/store/<path>` existence for each resolved node
- [x] 1.5 Unit tests: parse real flake.lock (Aspen's own), follows resolution, narHash → store path computation, resource bounds (>500 nodes, bad version, malformed narHash)

## 2. Input Fetching

- [x] 2.1 Implement `fetch_github_input` — download tarball from GitHub API, unpack to store path
- [x] 2.2 Implement `fetch_tarball_input` — download URL, unpack to store path
- [x] 2.3 Implement `resolve_path_input` — resolve relative/absolute path inputs
- [x] 2.4 Wire fetching into `resolve_all_inputs` — iterate nodes, compute paths, fetch missing, return overrides map

## 3. Overrides and Eval Expression

- [x] 3.1 Embed `call-flake.nix` as a `const &str` (from Nix 2.28, with source commit reference)
- [x] 3.2 Implement `build_overrides_expr` — generate Nix attrset string from resolved inputs (outPath, narHash, rev, shortRev, lastModified, dir)
- [x] 3.3 Implement `build_flake_eval_expr` — compose final expression: call-flake.nix invocation + attribute navigation + `.drvPath` access
- [x] 3.4 Unit tests: overrides expr generation, full eval expr for simple flakes, attribute path quoting

## 4. In-Process Flake Evaluation

- [x] 4.1 Add `evaluate_flake_derivation` method to `NixEvaluator` — takes flake dir, attribute, returns `(StorePath, Derivation)`
- [x] 4.2 Wire into `NixBuildWorker::try_native_build` — try in-process eval first, fall back to `resolve_drv_path` subprocess on failure
- [x] 4.3 Add "zero subprocesses" INFO log on successful in-process flake eval
- [x] 4.4 Integration test: evaluate a simple flake (inputs = {}) fully in-process, extract Derivation, verify output path matches `nix eval`

## 5. VM Test and Validation

- [x] 5.1 Update `snix-native-build.nix` VM test to verify "zero subprocesses" in journal for the trivial flake test case
- [ ] 5.2 Add a VM test case with a flake that has one real input (e.g., nixpkgs pinned to a specific rev) to validate input resolution end-to-end
- [x] 5.3 Property test: for N random flake.lock files, `compute_store_path` matches `nix store path-from-hash-part`
