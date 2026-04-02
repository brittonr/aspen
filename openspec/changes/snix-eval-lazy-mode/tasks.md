## 1. Error chain reporting

- [x] 1.1 Update `convert_eval_result` in `eval.rs` to walk `std::error::Error::source()` chain and join all messages with ` → `
- [x] 1.2 Add unit test: `NativeError` with inner cause produces chained message
- [x] 1.3 Add unit test: simple error without cause chain produces unchanged message

## 2. Lazy eval mode for flake paths

- [x] 2.1 Add `evaluate_with_store_lazy` method that uses `EvalMode::Lazy` (parallel to existing strict variant)
- [x] 2.2 Switch `evaluate_flake_via_compat` to call `evaluate_with_store_lazy`
- [x] 2.3 Switch `evaluate_npins_derivation` to use `EvalMode::Lazy` in its inline eval setup
- [x] 2.4 Handle `Value::Thunk` in `extract_drv_path_string` — force thunk before extracting string, return error if forcing fails
- [x] 2.5 Verify `evaluate_pure` and `validate_flake` still use `EvalMode::Strict`

## 3. Unit tests

- [x] 3.1 Add test: `evaluate_pure` with `EvalMode::Strict` catches errors in deeply nested attrsets
- [x] 3.2 Add test: lazy eval of `(derivation { ... }).drvPath` returns string without deep-forcing sibling attrs
- [x] 3.3 Run `cargo nextest run -p aspen-ci-executor-nix --features snix-build` — all 269 tests pass

## 4. VM test validation

- [x] 4.1 Run `snix-flake-native-build-test` — lazy mode fixed deep_force, revealed rnix `or` parse error, patched flake-compat
- [ ] 4.2 If "zero subprocesses": update VM test assertion to require BEST path (no subprocess fallback)
- [x] 4.3 Captured error chain: rnix TOKEN_OR parse error in flake-compat.nix. Fixed by replacing `or` with `if ? then else`. Cargo build has fix, but unit2nix cache serves old binary to VM test (`.nix` not in cargo source hash). Needs `nix store delete` or cache bust to validate in VM.
- [ ] 4.4 Run `snix-pure-build-test` and `snix-native-build-test` — confirm no regressions

## 5. Cleanup

- [ ] 5.1 Remove any dead code paths if subprocess fallback is no longer reached for trivial flakes
- [ ] 5.2 Update `docs/snix-eval-flake-gap.md` with results
- [x] 5.3 Commit with test evidence
