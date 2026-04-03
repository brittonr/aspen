# snix-eval Flake Evaluation Gap Analysis

## Current State (updated 2026-04-02)

Aspen's CI executor has a three-stage pipeline for building Nix flakes:

1. **Eval** — resolve `flake.nix` + `flake.lock` → `.drv` path
2. **Build** — execute the derivation in a sandbox → output path
3. **Upload** — ingest output into PathInfoService + cache gateway

**All three stages are now native** for trivial flakes (no nixpkgs dependency).

The eval stage uses `EvalMode::Lazy` with embedded flake-compat to resolve
flake inputs and extract `.drvPath` without spawning any `nix` subprocess.
The build stage uses snix-build's `LocalStoreBuildService` with bubblewrap
sandboxing. Zero subprocess dependency for the full pipeline.

```
flake-compat + snix-eval (lazy)  →  OK (zero subprocesses)
call-flake.nix + snix-eval       →  fallback (parse limitations)
nix eval --raw .drvPath          →  last resort (nix-cli-fallback feature)
```

## Root Cause Analysis (historical)

Two issues blocked the zero-subprocess path. Both are now fixed.

### Issue 1: `EvalMode::Strict` deep-forcing (FIXED)

All eval callsites used `EvalMode::Strict`, which triggers `final_deep_force`
on the entire result tree. Even though we only need `.drvPath`, strict mode
walked every thunk including unimplemented builtins in unrelated parts of
the flake outputs.

**Fix**: `EvalMode::Lazy` for `evaluate_flake_via_compat`,
`evaluate_flake_derivation`, and `evaluate_npins_derivation`. The `.drvPath`
selection in the expression forces only what's needed. `evaluate_pure` and
`validate_flake` keep `EvalMode::Strict` for exhaustive validation.

### Issue 2: rnix `or` keyword parse error (FIXED)

flake-compat's `default.nix` used `node.locked.type or null`, which rnix
parsed as `TOKEN_OR` instead of the attribute-or-default syntax.

**Fix**: Patched the bundled flake-compat to replace `x or default` with
`if x ? attr then x.attr else default` patterns. See
`flake_compat_bundled.nix` header comment.

### Historical: call-flake.nix parse error (superseded)

The alternative call-flake.nix eval path hit rnix parser edge cases with
complex string interpolation. This path is retained as a fallback but is
no longer the primary eval strategy.

## Changes Made

### 1. `EvalMode::Lazy` for flake eval paths

`evaluate_flake_via_compat`, `evaluate_flake_derivation`, and
`evaluate_npins_derivation` use `EvalMode::Lazy`. The Nix expression
selects `.drvPath` directly — snix-eval forces only the selected attribute.
`evaluate_pure` and `validate_flake` keep `EvalMode::Strict`.

A shared `evaluate_with_store_mode` method takes the `EvalMode` parameter;
`evaluate_with_store` (strict) and `evaluate_with_store_lazy` (lazy) are
convenience wrappers.

### 2. Error chain reporting

`format_error_chain()` walks `std::error::Error::source()` and joins all
messages with ` → `. `convert_eval_result` uses this for all eval errors.
Single-level errors produce identical output to `format!("{e}")`.

### 3. Patched flake-compat for rnix

Bundled `flake_compat_bundled.nix` replaces `or` keyword usage with
`if ? then else` equivalents. rnix parses `or` as `TOKEN_OR` rather than
the attribute-or-default operator.

### 4. `Value::Thunk` handling in `extract_drv_path_string`

When lazy eval returns an unforced thunk, the function returns an error
with a clear message instead of panicking.

### 5. Dead code removal

`evaluate_with_fallback`, `evaluate_subprocess`, and
`evaluate_flake_attribute` removed — superseded by the specialized
`evaluate_flake_via_compat` and `evaluate_flake_derivation` methods.

## Files Involved

| File | Role |
|------|------|
| `crates/aspen-ci-executor-nix/src/eval.rs` | NixEvaluator, eval callsites, `format_error_chain` |
| `crates/aspen-ci-executor-nix/src/flake_compat.rs` | Expression builder for flake-compat |
| `crates/aspen-ci-executor-nix/src/flake_compat_bundled.nix` | Embedded NixOS/flake-compat (patched) |
| `crates/aspen-ci-executor-nix/src/call_flake.rs` | Alternative call-flake.nix fallback path |
| `crates/aspen-ci-executor-nix/src/executor.rs` | Orchestrates eval → build → upload |

## Remaining Gaps

- **nixpkgs evaluation** — nixpkgs is too large/complex for snix-eval today.
  Flakes that depend on nixpkgs still need the `nix eval` subprocess fallback.
- **IFD (import-from-derivation)** — requires a real build service during eval.
  Falls back to subprocess.
- **Complex fetchGit variants** — shallow clones, submodules, allRefs may
  hit edge cases in snix-glue's fetchGit implementation.

The `nix-cli-fallback` feature is retained as a safety net for these cases.

## Test Evidence

Unit tests: 269 tests pass in `aspen-ci-executor-nix` (cargo nextest).

VM tests validate the zero-subprocess path end-to-end:

- `snix-flake-native-build-test` — asserts "zero subprocesses" in logs
- `snix-pure-build-test` — builds without nix CLI in PATH
- `snix-native-build-test` — native bwrap sandbox build
