# snix-eval Flake Evaluation Gap Analysis

## Current State

Aspen's CI executor has a three-stage pipeline for building Nix flakes:

1. **Eval** — resolve `flake.nix` + `flake.lock` → `.drv` path
2. **Build** — execute the derivation in a sandbox → output path
3. **Upload** — ingest output into PathInfoService + cache gateway

**Stage 2 (build) is fully native** — uses snix-build's `LocalStoreBuildService`
with bubblewrap sandboxing. No `nix build` subprocess. Works today, validated
by 7 passing VM tests.

**Stage 1 (eval) falls back to `nix eval` subprocess.** The in-process path
(snix-eval) is attempted first via two strategies, both of which fail:

```
flake-compat + snix-eval  →  FAIL (deep_force error)
call-flake.nix + snix-eval  →  FAIL (parse error)
nix eval --raw .drvPath  →  OK (subprocess fallback)
```

Eliminating the `nix eval` subprocess would give us a fully self-contained
build pipeline — no Nix CLI dependency at all.

## Root Cause Analysis

### Failure 1: flake-compat + `EvalMode::Strict`

**Error**: `flake-compat eval failed: while evaluating this as native code (final_deep_force)`

The eval expression is:

```nix
let
  compat = import /tmp/aspen-flake-compat-XXX/default.nix {
    src = { outPath = /path/to/flake; };
    system = "x86_64-linux";
  };
in compat.outputs.packages."x86_64-linux".default.drvPath
```

**What works**: snix-eval successfully:

- Parses flake-compat's 367-line `default.nix`
- Reads and parses `flake.lock` JSON
- Fetches tarball inputs via `builtins.fetchTarball` (HTTP download, unpack, narHash verify)
- Resolves `builtins.path` inputs
- Calls the flake's `outputs` function
- Navigates to `.packages.x86_64-linux.default.drvPath`

**What fails**: `EvalMode::Strict` triggers `final_deep_force` on the entire
result value. Even though we only need the `.drvPath` string, strict mode
walks every thunk in the result tree. When it hits a lazy value that depends
on an unimplemented builtin or triggers an eval error, the whole thing fails.

The `final_deep_force` function in `snix/eval/src/vm/mod.rs` is a generator
that deep-forces the top-level value before returning. It recursively forces
every `Thunk`, every `Attrs` value, every `List` element. For a flake that
produces `packages.x86_64-linux.default` — which is a derivation attrset
containing `drvPath`, `outPath`, `type`, `name`, etc — this means forcing
the entire derivation result, including parts we don't need.

**The fix is straightforward**: use `EvalMode::Lazy` instead of `EvalMode::Strict`.
We're selecting `.drvPath` which evaluates to a string — we don't need the
VM to deep-force anything. The attribute selection in the expression itself
forces only what's needed.

All 5 callsites in `eval.rs` use `EvalMode::Strict`. This was likely a
conservative choice to catch errors early, but it's counterproductive for
flake evaluation where the outputs attrset contains many lazy values we
don't need.

### Failure 2: call-flake.nix parse error

**Error**: `in-process flake eval failed: failed to parse Nix code:`

The call-flake.nix path generates a larger expression that embeds Nix's
`call-flake.nix` (from `nix/src/libflake/call-flake.nix`) with flake.lock
JSON and resolved input overrides. This expression uses Nix language
features that snix-eval's rnix parser doesn't support, specifically:

- `node.locked.type or null` — the `or` keyword in attribute access
  (`a.b or default`) is valid Nix but may hit parser edge cases in rnix
- Complex string interpolation patterns in the embedded expression

This is a secondary path — if flake-compat works (with lazy eval), this
path isn't needed.

## Proposed Fix: Three Changes

### Change 1: `EvalMode::Lazy` for flake-compat eval (Aspen-side)

In `crates/aspen-ci-executor-nix/src/eval.rs`, change `evaluate_flake_via_compat`
and `evaluate_with_store` to use `EvalMode::Lazy` when the expression
already selects a specific attribute (like `.drvPath`):

```rust
// Before (all callsites):
let eval = snix_eval::Evaluation::builder(io)
    .enable_import()
    .mode(snix_eval::EvalMode::Strict);

// After (flake eval):
let eval = snix_eval::Evaluation::builder(io)
    .enable_import()
    .mode(snix_eval::EvalMode::Lazy);
```

This should unblock flake-compat eval immediately — the `.drvPath` selection
in the expression forces only the derivation path string, not the entire
outputs tree.

### Change 2: Better error chain reporting (Aspen-side)

The `convert_eval_result` function uses `format!("{e}")` which only shows
the top-level error kind (`"while evaluating this as native code
(final_deep_force)"`). The inner error — which tells you *what* failed
during deep forcing — is lost. Fix:

```rust
// Before:
let msg = format!("{e}");

// After:
let msg = {
    let mut parts = vec![format!("{e}")];
    let mut source = std::error::Error::source(e);
    while let Some(cause) = source {
        parts.push(format!("  caused by: {cause}"));
        source = cause.source();
    }
    parts.join("\n")
};
```

### Change 3: snix-eval fork improvements (if needed)

If `EvalMode::Lazy` alone doesn't fix it (i.e., the error occurs during
attribute selection, not deep forcing), then snix-eval needs fixes:

**a) `builtins.path` with `filter` argument**: flake-compat uses
`builtins.path { path = ...; filter = ...; }` for path inputs. If
snix-eval's `builtins.path` doesn't support the `filter` argument, path
inputs will fail.

**b) `removeAttrs` on locked node**: call-flake.nix uses
`removeAttrs node.locked [ "dir" ]` — check if snix-eval implements
`removeAttrs`.

**c) `builtins.fetchGit`**: Git inputs in flake.lock require `fetchGit`.
snix-glue has this but it may be incomplete for all git input variants
(shallow, submodules, allRefs).

**d) String context tracking**: Nix derivation paths carry string contexts
(store path references). snix-eval tracks these via `NixString` context
sets. If flake-compat's path manipulation loses context, `derivationStrict`
may fail.

## Verification Plan

1. Change `EvalMode::Strict` → `EvalMode::Lazy` in `evaluate_flake_via_compat`
2. Add error chain reporting to `convert_eval_result`
3. Re-run `snix-flake-native-build-test` VM test
4. Check logs for "zero subprocesses" (success) or detailed error (next blocker)
5. If still failing, build snix-eval from source with debug logging to trace
   the exact builtin/thunk that errors

## snix Fork Scope (if needed)

The snix project (https://git.snix.dev/snix/snix) is Apache-2.0 licensed.
A fork would focus on:

1. **Flake-compat eval support** — ensure all builtins used by flake-compat
   work correctly in non-strict mode
2. **fetchGit completeness** — full support for git flake inputs including
   shallow clones and rev pinning
3. **Error reporting** — expose inner error chains through `NativeError`
4. **EvalMode::Select(path)** — a new eval mode that forces only the
   selected attribute path, not the entire result (optimization)

The fork would track upstream snix closely — these are targeted fixes,
not architectural changes.

## Files Involved

| File | Role |
|------|------|
| `crates/aspen-ci-executor-nix/src/eval.rs` | NixEvaluator, all eval callsites |
| `crates/aspen-ci-executor-nix/src/flake_compat.rs` | Expression builder for flake-compat |
| `crates/aspen-ci-executor-nix/src/flake_compat_bundled.nix` | Embedded NixOS/flake-compat |
| `crates/aspen-ci-executor-nix/src/call_flake.rs` | Alternative call-flake.nix path |
| `crates/aspen-ci-executor-nix/src/executor.rs` | Orchestrates eval → build → upload |
| `~/.cargo/git/checkouts/snix-*/snix/eval/src/vm/mod.rs` | `final_deep_force`, `EvalMode` |
| `~/.cargo/git/checkouts/snix-*/snix/eval/src/value/mod.rs` | `deep_force_` implementation |
| `~/.cargo/git/checkouts/snix-*/snix/eval/src/errors.rs` | `NativeError` error chain |

## Current Test Evidence

From `snix-flake-native-build-test` VM logs:

```
evaluating flake via embedded flake-compat + snix-eval
  flake_dir=/root/test-flake attribute=packages.x86_64-linux.default

fetching tarball via reqwest url=http://127.0.0.1:8888/tarball-input.tar.gz  ← WORKS
stripped single top-level directory from tarball prefix=source              ← WORKS
narHash verified hash=sha256-8cyXw/Dn5Q8UQZK+Zy6zHRHZvaRic/UBuXAMQZFQnvs= ← WORKS

flake-compat eval failed: while evaluating this as native code (final_deep_force)  ← FAILS HERE

falling back to nix eval subprocess
resolved flake to derivation path drv_path=/nix/store/...-flake-compat-native-test.drv

parsed derivation, starting native build
starting local-store bwrap build                                           ← NATIVE
native build completed output_count=1 build_ms=48                          ← 48ms!
native build succeeded
uploaded native build output to Aspen store
```

The eval infrastructure (fetchers, input resolution, flake.lock parsing) all
works. The failure is specifically in the strict-mode deep forcing of the
result. `EvalMode::Lazy` is the first thing to try.
