## Context

Aspen's `NixEvaluator` wraps snix-eval with snix-glue's store-aware builtins (derivationStrict, fetchTarball, fetchGit, import). The flake-compat eval path works end-to-end — input fetching, lock parsing, outputs function call — but fails at the final step because `EvalMode::Strict` deep-forces the entire result value.

The eval expression selects `.drvPath` (a string), but strict mode forces every thunk in the result tree before returning, including unimplemented or erroring parts of the flake outputs we never need.

Five callsites in `eval.rs` all use `EvalMode::Strict`. The `convert_eval_result` function formats errors with `format!("{e}")`, which only shows the outermost error kind and drops the inner cause chain.

## Goals / Non-Goals

**Goals:**

- Eliminate `nix eval` subprocess fallback for trivial flakes (no nixpkgs, tarball/path inputs)
- Surface full error chains from snix-eval so failures are diagnosable
- Validate via the existing `snix-flake-native-build-test` VM test (assert "zero subprocesses")
- Keep `nix-cli-fallback` feature as a safety net — don't remove it

**Non-Goals:**

- Full nixpkgs evaluation (nixpkgs is too large/complex for snix-eval today)
- Forking snix (try Aspen-side fixes first; fork only if builtins are missing)
- Changing the build stage (already native, working)
- Supporting IFD (import-from-derivation) in-process

## Decisions

### D1: Selective `EvalMode::Lazy` for flake eval paths

Use `EvalMode::Lazy` for `evaluate_flake_via_compat` and `evaluate_call_flake_derivation` — the two flake eval methods where the expression itself selects `.drvPath`. The Nix expression forces only what it needs.

Keep `EvalMode::Strict` for `evaluate_pure` (preflight validation) and `validate_flake` where we want to catch all errors eagerly.

For `evaluate_npins_derivation`, switch to `Lazy` too — same pattern, the expression selects `.drvPath`.

**Alternative considered**: Add a new `EvalMode::Select(path)` to snix-eval that forces only a specific attribute path. Rejected — requires snix fork for marginal benefit over lazy + expression-level selection.

### D2: Error chain formatting in `convert_eval_result`

Walk `std::error::Error::source()` chain and join all messages with ` → `. This surfaces the inner error from `NativeError { err }` which currently gets swallowed.

### D3: Separate `evaluate_with_store` into strict and lazy variants

Rather than changing the shared `evaluate_with_store` (which other callers may depend on for strict semantics), add `evaluate_with_store_lazy` that uses `EvalMode::Lazy`. The flake eval methods call the lazy variant; validation methods keep calling the strict one.

**Alternative**: Add an `EvalMode` parameter to `evaluate_with_store`. Rejected — leaks eval mode choice to callers who shouldn't need to think about it.

### D4: VM test assertion upgrade

The `snix-flake-native-build-test` currently accepts both "BEST" (zero subprocesses) and "GOOD" (native build with subprocess eval) as passing. After this change, assert "zero subprocesses" as the required outcome. If lazy eval doesn't fix it, the test failure will tell us exactly what's still broken.

## Risks / Trade-offs

**[Risk] Lazy mode returns unevaluated thunks** → The `.drvPath` selection in the expression forces the derivation path string. If snix-eval's lazy mode doesn't force the selected attribute sufficiently, `extract_drv_path_string` will get a `Value::Thunk` instead of `Value::String`. Mitigation: add explicit force of the result value if it's a thunk, before extracting the string.

**[Risk] flake-compat uses builtins not implemented in snix-eval** → The `EvalMode::Strict` error may be masking a builtin error that also occurs during lazy attribute selection. Mitigation: the improved error chain reporting (D2) will surface the exact missing builtin. If found, evaluate whether a snix fork or a workaround is appropriate.

**[Risk] Regression in npins eval path** → Switching `evaluate_npins_derivation` to lazy mode could miss errors that strict mode caught. Mitigation: npins eval tests still run; if they regress, revert just that callsite.
