## Context

The snix native build pipeline in `aspen-ci-executor-nix` has three tiers of nix CLI dependency:

1. **Tier 1 — Pure functions**: `derivation_to_build_request`, `parse_closure_output`, `replace_output_placeholders`, `output_sandbox_path_to_store_path` — no subprocess calls, fully testable today.
2. **Tier 2 — Eval with snix-eval**: `NixEvaluator::evaluate_pure`, `evaluate_with_store`, `evaluate_npins_derivation`, `evaluate_flake_derivation`, `evaluate_flake_via_compat` — in-process via snix-eval/snix-glue, but `ensure_flake_lock` and `evaluate_subprocess` shell out as fallbacks.
3. **Tier 3 — Build execution**: `LocalStoreBuildService::do_build` uses `cp` and `nix-store -qR` (closure computation). `NixBuildWorker::try_native_build` shells out to `nix-store --realise` for input materialisation. `register_output_in_store` uses `nix-store --dump/--restore`.

Current tests don't distinguish these tiers. A test can pass by silently falling back to subprocess. The existing VM tests (`snix-native-build`, `snix-flake-native-build`) prove the pipeline works but don't prove it works *without* `nix`.

Six specific `Command::new("nix*")` call sites exist in the crate:

- `build_service.rs:649` — `nix-store -qR` (closure computation)
- `build_service.rs:775` — `nix-store --dump` (output registration)
- `build_service.rs:805` — `nix-store --restore` (output registration)
- `eval.rs:774` — `nix flake lock` (lock generation)
- `executor.rs:338` — `nix-store --realise` (input materialisation)
- `executor.rs:399` — `nix-store --realise` (extra paths fetch)

Plus `executor.rs` `spawn_nix_build` which is the full subprocess fallback path.

## Goals / Non-Goals

**Goals:**

- Make it structurally impossible for a test marked "snix-only" to succeed if any `nix`/`nix-store` subprocess is spawned
- Unit test coverage for every pure function in `build_service.rs` and `eval.rs`
- Integration tests that drive eval→build→upload through in-memory services with a fake `BuildService`
- Property tests for derivation conversion and closure parsing
- One NixOS VM test that removes `nix` from PATH and proves the build still works (or fails at a documented, expected point)
- Document every remaining subprocess escape hatch with a tracking comment

**Non-Goals:**

- Eliminating the subprocess calls themselves (that's future work — this change builds the test scaffolding that makes that work safe)
- Testing the `nix build` subprocess fallback path (already covered by existing tests)
- Modifying `aspen-snix` trait implementations (those have their own test suite)
- Testing snix-bridge daemon protocol (already covered in `daemon_test.rs`)
- Achieving 100% code coverage — focus on the snix-only paths

## Decisions

### 1. Subprocess firewall via PATH manipulation, not monkey-patching

**Decision**: Tests that need to guarantee no subprocess create a temporary directory, symlink only safe binaries (cp, sh, echo), and set `PATH` to that directory. For unit tests, use a wrapper module `test_support::no_nix_env()` that returns a `CommandEnvGuard` RAII type overriding `PATH` for the duration.

**Alternatives considered**:

- *LD_PRELOAD interception*: Too fragile, platform-specific, doesn't work in static binaries
- *Compile-time feature gate removing all Command::new calls*: Too invasive, changes production code structure for test ergonomics
- *Custom Command wrapper trait*: Too much refactoring of existing code

**Rationale**: PATH manipulation is the simplest mechanism that actually prevents subprocess execution. It works in both unit tests and VM tests. In VM tests, we literally don't install `nix` in the VM's PATH.

### 2. Fake BuildService for integration tests

**Decision**: Create `FakeBuildService` implementing `snix_build::buildservice::BuildService` that returns canned `BuildResult` values. Configurable to return success with predetermined output nodes, or failure with specific error. Lives in a `test_support` module under `#[cfg(test)]`.

**Rationale**: The real `LocalStoreBuildService` needs bwrap, /nix/store, and real filesystem operations. `DummyBuildService` (from snix-build) always fails. We need something that succeeds predictably for pipeline testing.

### 3. Test file organisation

**Decision**:

- `crates/aspen-ci-executor-nix/src/test_support.rs` — `#[cfg(test)]` module with `FakeBuildService`, `no_nix_env()`, test service constructors
- `crates/aspen-ci-executor-nix/tests/snix_only_unit_test.rs` — unit tests for pure functions
- `crates/aspen-ci-executor-nix/tests/snix_only_integration_test.rs` — eval→build→upload pipeline tests
- `crates/aspen-ci-executor-nix/tests/snix_only_proptest.rs` — property-based tests
- `nix/tests/snix-pure-build-test.nix` — NixOS VM test

**Rationale**: Separate files per test category. Integration tests in `tests/` directory so they compile as separate crates and can have different feature requirements. `test_support` is a shared internal module.

### 4. In-memory service stack for integration tests

**Decision**: Reuse the same pattern from `eval.rs` tests — `MemoryBlobService`, `RedbDirectoryService::new_temporary`, `LruPathInfoService::with_capacity`. Wrap in a `TestSnixStack` struct that provides all three services plus the `FakeBuildService`, pre-wired into a `NativeBuildService`.

**Rationale**: These in-memory implementations already exist and are proven in `aspen-snix/tests/`. No new dependencies needed.

### 5. VM test: remove nix, keep bwrap

**Decision**: The NixOS VM test installs `aspen-node` (with snix-build), `bwrap`, and `busybox` but does NOT add `nix` to the system PATH. The test creates a pre-evaluated `.drv` file and pre-populated input closure in `/nix/store` (via the VM's nix store from the build), then submits a job. The native pipeline must succeed using only `bwrap` + `cp` + in-memory snix services.

**Key subtlety**: `compute_input_closure` calls `nix-store -qR`. When `nix-store` is absent, it falls back to using direct inputs only. The test validates this fallback works for simple derivations where direct inputs are sufficient.

**Rationale**: This proves the pipeline works without `nix` for the common case (simple derivations). Complex derivations with deep transitive closures may still need `nix-store -qR` — those are documented as known gaps.

### 6. Property test generators

**Decision**: Use `proptest` (already a dev-dependency) with custom `Arbitrary`-like strategies for:

- `Derivation` — constrained: 1-5 outputs, 0-10 input sources, valid store paths, valid system strings
- `NixBuildPayload` — fuzz flake_url, attribute, timeout_secs, extra_args
- `StorePath` — valid 20-byte digest + name component via `prop::string::string_regex`

**Rationale**: proptest is already in dev-dependencies. These generators exercise edge cases in `derivation_to_build_request` (empty inputs, many outputs, placeholder replacement) and `parse_closure_output` (malformed lines, duplicates, empty input).

## Risks / Trade-offs

- **[Risk] PATH manipulation is process-global** → Unit tests that modify PATH must run sequentially or use per-thread env (not possible with std). Mitigate by running snix-only tests in a separate nextest group with `threads-required = 1`, or by not modifying process-global PATH in unit tests and instead passing env overrides to Command builders.
- **[Risk] FakeBuildService may drift from real BuildService contract** → Mitigate by keeping FakeBuildService minimal (just returns canned data) and relying on the VM test for real end-to-end validation.
- **[Risk] VM test is slow (~2-5 min)** → Acceptable. It runs in CI as a nix flake check, not in the fast nextest loop. The unit/integration tests cover the fast feedback path.
- **[Trade-off] Not eliminating subprocess calls yet** → This change is test scaffolding. Eliminating the calls is a separate change that can be made safely once these tests exist to catch regressions.
