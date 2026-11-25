# Testing harness deltas

## ADDED Requirements

### Requirement: CI and hermetic Nix checks execute every workspace member test suite
r[molten.testing.ci_workspace_member_coverage] The `ci` and `deterministic` nextest profiles and `nix flake check` MUST execute tests for every workspace member crate (`molten`, `molten-core`, `molten-node-host`, and `molten-release-policy`). A workspace member MUST NOT be silently excluded by a `package(molten)` profile filter.

#### Scenario: Pure core regression fails CI
- GIVEN a failing unit test in `crates/molten-core` or `crates/molten-release-policy`
- WHEN `cargo nextest run --profile ci` or `nix flake check` runs
- THEN the check MUST fail and report the failing member test.

#### Scenario: All workspace members execute in hermetic Nix check
- GIVEN a clean repository candidate
- WHEN `nix flake check` completes
- THEN test execution MUST include `molten`, `molten-core`, `molten-node-host`, and `molten-release-policy`.

### Requirement: Optional Cargo features and ignored tests have explicit CI accounting
r[molten.testing.feature_and_ignore_ci_accounting] Self-contained optional Cargo features MUST have explicit CI compilation and test checks rather than remaining unbuilt in default CI. Every `#[ignore]` test MUST declare an explicit external prerequisite category, and same-host hermetic tests MUST NOT remain unexecuted by default CI without an explicit reviewed exclusion.

#### Scenario: Broken optional feature fails feature check
- GIVEN a compilation error behind an optional feature such as `chaoscontrol-conformance` or `job-dag-construction`
- WHEN the CI feature-matrix check runs
- THEN the check MUST fail rather than silently skipping the feature.

#### Scenario: Unjustified ignored test is rejected
- GIVEN a `#[ignore]` test lacking a documented external prerequisite or runnable locally without external hardware
- WHEN the ignored-test audit runs
- THEN the audit MUST flag the test for promotion or explicit justification.

### Requirement: Pure decision cores carry bounded Hegel property coverage
r[molten.testing.molten_core_hegel_properties] `crates/molten-core` MUST maintain bounded `#[hegel::test]` property suites for core pure state-machine, merge, reservation/lease, and identity invariants, executed under the deterministic CI rail.

#### Scenario: Core property shrinks a pure invariant violation
- GIVEN a pure core invariant violation in `crates/molten-core`
- WHEN the deterministic Hegel suite runs
- THEN the property test MUST fail deterministically and emit a reproducible counterexample.

#### Scenario: Zero-property core check denies
- GIVEN a CI check verifying `molten-core` property presence
- WHEN `crates/molten-core` compiles with zero `#[hegel::test]` cases
- THEN the coverage check MUST fail closed.
