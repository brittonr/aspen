## 1. Test Support Infrastructure

- [x] 1.1 Create `crates/aspen-ci-executor-nix/src/test_support.rs` with `#[cfg(test)]` gate. Add `mod test_support;` to `lib.rs`. Include: `no_nix_env()` returning a `HashMap<String, String>` with sanitised PATH (only cp/sh/echo/env/cat/mkdir/rm/chmod/ls from `/usr/bin`), and a `TestSnixStack` struct holding `Arc<dyn BlobService>`, `Arc<dyn DirectoryService>`, `Arc<dyn PathInfoService>` constructed from in-memory implementations (`MemoryBlobService`, `RedbDirectoryService::new_temporary`, `LruPathInfoService::with_capacity`).
- [x] 1.2 Implement `FakeBuildService` in `test_support.rs` — implements `snix_build::buildservice::BuildService` via `#[tonic::async_trait]`. Constructor takes `Result<BuildResult, io::Error>`. `do_build()` returns the configured result. Add a `FakeBuildService::success(outputs: Vec<BuildOutput>)` and `FakeBuildService::failure(msg: &str)` convenience constructor.
- [x] 1.3 Add `TestSnixStack::with_fake_build_service(fake: FakeBuildService)` method that wires `FakeBuildService` into a `NativeBuildService::with_build_service()` and returns the full stack ready for pipeline tests. Verify it compiles with `cargo check -p aspen-ci-executor-nix --features snix-build`.

## 2. Annotate Subprocess Escape Hatches

- [x] 2.1 Add `// SUBPROCESS-ESCAPE:` comments above every `Command::new("nix*")` call in `build_service.rs` (3 sites: `nix-store -qR` at ~L649, `nix-store --dump` at ~L775, `nix-store --restore` at ~L805). Each comment documents what the call does and what the pure-snix replacement would be.
- [x] 2.2 Add `// SUBPROCESS-ESCAPE:` comments above every `Command::new("nix*")` call in `executor.rs` (2 sites: `nix-store --realise` at ~L338 and ~L399) and `eval.rs` (1 site: `nix flake lock` at ~L774). Document replacement path.
- [x] 2.3 Add `// SUBPROCESS-ESCAPE:` comment above `spawn_nix_build` in `executor.rs` documenting it as the full `nix build` subprocess fallback path.

## 3. Unit Tests — Pure Functions

- [x] 3.1 Create `crates/aspen-ci-executor-nix/tests/snix_only_unit_test.rs`. Gate with `#![cfg(feature = "snix-build")]`. Add tests for `derivation_to_build_request`: empty inputs, inputs with pre-resolved Nodes, environment variable merging (NIX defaults + derivation env), builder path injection into inputs map.
- [x] 3.2 Add unit tests for `derivation_to_build_request` FOD handling: fixed-output derivation gets `NetworkAccess` constraint, non-FOD does not. Test placeholder replacement in arguments and environment bytes.
- [x] 3.3 Add unit tests for `parse_closure_output`: standard multi-line input, empty string, duplicate entries (deduplication), lines without `/nix/store/` prefix (skipped), whitespace handling.
- [x] 3.4 Add unit tests for `output_sandbox_path_to_store_path`: typical sandbox path extraction, path with no `nix/store/` marker returns `None`, empty basename after marker returns `None`, nested sub-paths (only first component extracted).
- [x] 3.5 Add unit tests for `parse_derivation`: valid minimal ATerm derivation bytes, invalid bytes return `Err`, output paths extracted correctly.
- [x] 3.6 Add unit tests for `extract_drv_path_string`: valid drv path in `NixEvalOutput`, non-drv path rejected, non-string value rejected, `None` value rejected.
- [x] 3.7 Add unit tests for `NixEvaluator::evaluate_pure` (using in-memory services from `test_support`): arithmetic, attrset, syntax error, undefined variable, type error, source too large.

## 4. Integration Tests — Pipeline

- [x] 4.1 Create `crates/aspen-ci-executor-nix/tests/snix_only_integration_test.rs`. Gate with `#![cfg(feature = "snix-build")]`. Import `test_support::TestSnixStack`.
- [x] 4.2 Write `test_native_build_pipeline_in_memory`: construct a `Derivation` (single output, `/bin/sh` builder, no input derivations, one input source), pre-populate `PathInfoService` with a `PathInfo` for the input source, configure `FakeBuildService` to return success with a synthetic `BuildOutput` node. Call `build_derivation` and assert: `NativeBuildResult` has one output, output store path matches derivation output, `resolve_ms` and `build_ms` are populated.
- [x] 4.3 Write `test_upload_native_outputs_stores_pathinfo`: after `build_derivation` succeeds, call `upload_native_outputs` with the results and the in-memory `PathInfoService` + `SimpleRenderer`. Assert each output's `PathInfo` is retrievable via `pathinfo_service.get(digest)`.
- [x] 4.4 Write `test_build_failure_propagates`: configure `FakeBuildService::failure("sandbox crashed")`. Call `build_derivation`. Assert it returns `Err` with the error message.
- [x] 4.5 Write `test_evaluate_flake_derivation_in_memory`: create temp dir with minimal `flake.nix` (inputs = {}, raw derivation) and `flake.lock`. Call `NixEvaluator::evaluate_flake_derivation`. Assert returns `Ok` with valid `StorePath` and `Derivation`.
- [x] 4.6 Write `test_evaluate_npins_derivation_in_memory`: create temp dir with minimal `default.nix` and `npins/sources.json`. Call `evaluate_npins_derivation`. Assert returns `Ok` with valid `StorePath` and `Derivation`.
- [x] 4.7 Write `test_native_build_service_with_build_service_wiring`: construct `NativeBuildService::with_build_service(FakeBuildService::success(...), ...)`. Call `build_derivation` with a trivial derivation. Assert the fake was called (success returned).

## 5. Property-Based Tests

- [x] 5.1 Create `crates/aspen-ci-executor-nix/tests/snix_only_proptest.rs`. Gate with `#![cfg(feature = "snix-build")]`.
- [x] 5.2 Write proptest strategy `arb_derivation()`: generates `Derivation` with 1-5 outputs (valid store path per output), 0-10 `input_sources` (valid store paths), random `system` from `["x86_64-linux", "aarch64-linux"]`, builder `/bin/sh`, 0-5 arguments, 0-10 environment variables with string keys/values.
- [x] 5.3 Write proptest strategy `arb_store_path_with_node()`: generates `(StorePath<String>, Node)` pairs with valid 20-byte digests and either `Directory` or `File` nodes.
- [x] 5.4 Write property test `prop_derivation_to_build_request_never_panics`: generate `(Derivation, BTreeMap<StorePath, Node>)` using above strategies, call `derivation_to_build_request`, assert it either returns `Ok` or `Err` (never panics). When `Ok`, assert `outputs.len() == derivation.outputs.len()` and `refscan_needles.len() >= outputs.len() + inputs.len()`.
- [x] 5.5 Write property test `prop_parse_closure_output_never_panics`: generate arbitrary `String`, call `parse_closure_output`, assert no panic and result has no duplicates.
- [x] 5.6 Write property test `prop_payload_validation`: generate `NixBuildPayload` with varied `flake_url` (empty, normal, very long), `timeout_secs` (0, valid, >86400), `attribute` (empty, normal). Assert: empty URL → Err, excessive timeout → Err, valid combos → Ok. Never panics.

## 6. NixOS VM Integration Test

- [x] 6.1 Create `nix/tests/snix-pure-build-test.nix`. Structure: single-node VM with `aspen-node` (snix-build), `bwrap`, `busybox-sandbox-shell`, but `nix` explicitly excluded from system PATH via `environment.systemPackages` that does NOT include `nix`.
- [x] 6.2 Write test flake as a `pkgs.writeText` derivation (trivial `/bin/sh` builder, no nixpkgs, no inputs — same pattern as `snix-native-build.nix`). Pre-instantiate the `.drv` at VM build time so it's in the store. Write flake.nix + flake.lock to `/root/test-project/`.
- [x] 6.3 Write test script: boot node, `wait_for_unit("aspen-node.service")`, verify `which nix` fails (exit 1), submit `ci_nix_build` job via `aspen-cli`, `wait_until_succeeds` for job completion, assert output store path exists, grep journal for "native build completed" and absence of "nix-store" subprocess errors.
- [x] 6.4 Register the test in the flake: add `snix-pure-build-test` to `nix/tests/default.nix` (or equivalent flake checks wiring) so `nix build .#checks.x86_64-linux.snix-pure-build-test --impure` works.

## 7. CI Integration

- [x] 7.1 Add the new test files to the `snix` nextest profile filter in `.config/nextest.toml` so `cargo nextest run -P snix` picks them up.
- [x] 7.2 Update `scripts/test-snix.sh` to include the new test files.
- [x] 7.3 Run full test suite: `cargo nextest run -p aspen-ci-executor-nix --features snix-build` — all new tests pass, all existing tests still pass. Run `cargo clippy -p aspen-ci-executor-nix --features snix-build --all-targets -- --deny warnings`.
