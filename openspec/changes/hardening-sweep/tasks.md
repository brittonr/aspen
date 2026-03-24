## 1. Dogfood Pipeline Validation

- [x] 1.1 Run `nix run .#dogfood-local -- start` and verify single-node cluster boots and reports healthy
- [x] 1.2 Run `nix run .#dogfood-local -- push` and verify source pushes to Forge without errors
- [x] 1.3 Run `nix run .#dogfood-local -- build` and verify CI pipeline triggers, builds, and completes
- [x] 1.4 Run `nix run .#dogfood-local -- deploy` and verify deploy to running cluster succeeds
- [x] 1.5 Run `nix run .#dogfood-local -- verify` and confirm CI-built binary matches local build
- [x] 1.6 Fix any failures found in 1.1–1.5 and re-run affected phases
- [x] 1.7 Run `nix run .#dogfood-local -- full-loop` end-to-end to confirm the complete pipeline passes
- [x] 1.8 Run `nix run .#dogfood-local -- stop` to clean up

## 2. NixOS VM Test Audit

- [x] 2.1 Build a list of all 63 VM test check attrs from `nix eval .#checks.x86_64-linux`
- [x] 2.2 Run all VM tests in parallel via pueue (`nix build .#checks.x86_64-linux.<name>` per test)
- [x] 2.3 Collect results and categorize each test as pass / fail (regression) / fail (pre-existing) / skip
- [x] 2.4 Fix any regressions caused by hardening sprint changes (cluster discovery, write forwarding, tiger style, health gates)
- [x] 2.5 Re-run fixed tests to confirm they pass
- [x] 2.6 Write an audit report: pass count, fail count, list of pre-existing failures with root causes

## 3. snix System Detection

- [x] 3.1 Add `detect_host_system() -> &'static str` function to `aspen-ci-executor-nix` that maps `std::env::consts::{ARCH, OS}` to nix system strings (`x86_64-linux`, `aarch64-linux`, `x86_64-darwin`, `aarch64-darwin`)
- [x] 3.2 Update `evaluate_flake_derivation` to read `payload.system` and fall back to `detect_host_system()`
- [x] 3.3 Remove the hardcoded `let system = "x86_64-linux"` line in `eval.rs`
- [x] 3.4 Thread the resolved system string through `build_flake_compat_expr` and any downstream callers
- [x] 3.5 Add unit tests for `detect_host_system()` (at least: returns valid nix system string, matches current platform)
- [x] 3.6 Add unit test for `evaluate_flake_derivation` with explicit `payload.system` override

## 4. Flake Lock Submodule Parsing

- [x] 4.1 Update the flake lock git input parser in `flake_lock.rs` to read `"submodules"` bool from the locked node JSON
- [x] 4.2 Default to `false` when the `submodules` field is absent (backwards compatible)
- [x] 4.3 Pass the parsed `submodules` value through to `fetch_and_clone_git`
- [x] 4.4 Add unit test with synthetic flake.lock containing `"submodules": true` in a git input
- [x] 4.5 Add unit test with synthetic flake.lock where `submodules` is absent, confirming default `false`

## 5. Verification

- [x] 5.1 Run `cargo nextest run -P quick` — all tests pass
- [x] 5.2 Run `cargo clippy --all-targets -- --deny warnings` — clean
- [x] 5.3 Run `nix run .#rustfmt` — formatted
