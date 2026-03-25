## Why

The snix build path (`NativeBuildService`, `NixEvaluator`, `LocalStoreBuildService`) still shells out to `nix-store`, `nix eval`, and `nix flake lock` in several places — input closure computation, `.drv` path resolution, flake lock generation, and output registration. The existing test suite doesn't enforce that the native path actually avoids subprocesses; tests silently succeed by falling through to subprocess fallbacks. There is no compile-time or runtime mechanism that prevents a "pure snix" build from touching the `nix` CLI. To make snix builds a first-class, fully self-contained pipeline we need a test suite at every layer — unit, integration, NixOS VM — that makes subprocess calls structurally impossible, catches regressions immediately, and documents the remaining subprocess escape hatches so they can be eliminated one by one.

## What Changes

- **Subprocess firewall module**: A `#[cfg(test)]` helper (and optionally a runtime guard) that intercepts or blocks `Command::new("nix")`, `Command::new("nix-store")`, and `Command::new("nix-instantiate")` calls during test execution. Tests opt-in to the guard so any accidental subprocess spawn is a hard failure.
- **Unit tests for every pure function in `build_service.rs` and `eval.rs`**: `derivation_to_build_request`, `replace_output_placeholders`, `parse_closure_output`, `output_sandbox_path_to_store_path`, `compute_input_closure` (with a mock/fake that doesn't call `nix-store -qR`), `parse_derivation`, `extract_drv_path_string`, and `convert_eval_result`. All exercised without any subprocess.
- **Integration tests for `NativeBuildService` and `NixEvaluator`**: Using in-memory snix services (`MemoryBlobService`, `RedbDirectoryService`, `LruPathInfoService`) and a `DummyBuildService` to drive the full eval→resolve→build→upload pipeline without touching disk or nix CLI. Includes a fake `BuildService` that returns canned `BuildResult` values.
- **Property-based tests**: proptest generators for `Derivation`, `NixBuildPayload`, and `StorePath` inputs to `derivation_to_build_request` and `parse_closure_output`, asserting invariants (output count bounded, refscan needles match, no panics on arbitrary inputs).
- **NixOS VM integration test**: A new `snix-pure-build-test` NixOS VM test that boots a node with snix-build enabled, removes `nix` from `$PATH`, and runs a build job end-to-end — proving the native pipeline works without any nix CLI binary available.
- **Audit and document remaining subprocess calls**: Every `Command::new("nix*")` in `aspen-ci-executor-nix` gets a tracking comment with a TODO for the pure-snix replacement, and a test that proves the subprocess is not reached when the native path is available.

## Capabilities

### New Capabilities

- `snix-subprocess-firewall`: Test harness that blocks nix CLI subprocess spawning during test execution. Provides `#[no_nix_subprocess]` annotation or runtime guard for any test function.
- `snix-build-unit-tests`: Comprehensive unit tests for all pure functions in the snix build pipeline (`build_service.rs`, `eval.rs`, `executor.rs`) with zero subprocess dependencies.
- `snix-build-integration-tests`: End-to-end integration tests using in-memory snix services and fake/dummy `BuildService` implementations to validate the full eval→build→upload pipeline.
- `snix-build-property-tests`: Property-based tests for derivation conversion, payload validation, and closure parsing — asserting invariants hold for arbitrary inputs.
- `snix-build-vm-test`: NixOS VM integration test that proves the native snix build pipeline works with `nix` CLI completely absent from `$PATH`.

### Modified Capabilities
<!-- No existing spec-level behavior is changing — this is purely additive test coverage. -->

## Impact

- **Crates**: `aspen-ci-executor-nix` (primary — bulk of new tests), `aspen-snix` (existing tests extended), `aspen-snix-bridge` (daemon test hardened)
- **Features**: Tests require `snix-build` + `snix-eval` features enabled. New `testing` feature gate for subprocess firewall module.
- **NixOS tests**: One new VM test in `nix/tests/snix-pure-build-test.nix`, added to flake checks.
- **CI**: New nextest profile `snix` extended to include the new tests. `scripts/test-snix.sh` updated.
- **No runtime changes**: All modifications are `#[cfg(test)]` or in test files. Zero impact on production binaries.
