## Why

The CI nix build pipeline has three build paths tried in priority order: (1) npins native — zero subprocesses, (2) flake native — in-process eval + bwrap sandbox, (3) `nix build` subprocess fallback. Path 1 works end-to-end. Path 2 has all the pieces (snix-eval for flake eval, LocalStoreBuildService for bwrap builds) but requires ~2,270 lines of custom Rust (flake.lock parsing, HTTP tarball fetching, call-flake.nix expression generation, narHash verification, store path computation) that reimplements what Nix already does. Meanwhile, NixOS/flake-compat is a ~250-line pure Nix file that does exactly this — parses flake.lock, fetches inputs via `fetchTarball`, and calls the flake's outputs function. snix-eval already implements `fetchTarball`, `builtins.path`, and `import` — the three builtins flake-compat needs. We should embed flake-compat and let snix-eval handle input resolution natively instead of reimplementing it in Rust.

## What Changes

- Embed NixOS/flake-compat's `default.nix` (MIT licensed) as a bundled resource in aspen-ci-executor-nix
- Replace `evaluate_flake_derivation`'s call-flake.nix + manual input resolution with a single expression: `(import <flake-compat> { src = /path/to/flake; }).outputs.<attr>.drvPath`
- snix-eval's builtin `fetchTarball` handles all HTTP downloading, tarball unpacking, narHash verification, and store path registration — no custom Rust fetch code needed for the eval phase
- Delete or deprecate `call_flake.rs`, the flake.lock parsing in `flake_lock.rs`, and the HTTP fetch code in `fetch.rs` (keep for backward compat behind feature flag if needed)
- Add a VM integration test proving the flake-compat path works for a flake with a tarball input (zero subprocesses)

## Capabilities

### New Capabilities

- `flake-native-build`: End-to-end native build pipeline for flake projects using embedded flake-compat — snix-eval evaluates the flake expression with native `fetchTarball` for input resolution, extracts Derivation from KnownPaths, builds via LocalStoreBuildService (bwrap). Zero subprocess dependency on `nix` binary for github/gitlab/tarball/path/sourcehut inputs.

### Modified Capabilities

## Impact

- `crates/aspen-ci-executor-nix/src/eval.rs` — new `evaluate_flake_via_compat` method
- `crates/aspen-ci-executor-nix/src/executor.rs` — wire flake-compat eval into `try_flake_eval_native`
- `crates/aspen-ci-executor-nix/src/` — new `flake_compat.nix` embedded resource
- `crates/aspen-ci-executor-nix/src/call_flake.rs` — deprecated (kept as fallback)
- `crates/aspen-ci-executor-nix/src/flake_lock.rs` — manual flake.lock parsing no longer primary path
- `crates/aspen-ci-executor-nix/src/fetch.rs` — HTTP fetch code no longer needed for eval phase
- `nix/tests/` — new VM integration test
- `docs/nix-integration.md` — updated build path documentation
