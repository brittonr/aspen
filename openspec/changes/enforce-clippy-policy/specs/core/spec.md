## ADDED Requirements

### Requirement: Crate roots enforce Clippy as policy
ID: core.crate-roots-enforce-clippy-as-policy

The system MUST ensure Aspen crate roots in the documented rollout scope enforce the repository Clippy baseline through crate-level attributes instead of warning-only defaults.

The required baseline for each participating crate root is:

```rust
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::undocumented_unsafe_blocks)]
#![allow(clippy::module_name_repetitions)]
```

#### Scenario: Local clone sees hard lint failures
ID: core.crate-roots-enforce-clippy-as-policy.local-clone-sees-hard-lint-failures

- **GIVEN** an engineer clones the Aspen repository and runs `cargo clippy` for a crate in the documented rollout scope
- **WHEN** that crate contains a missing-docs or denied Clippy violation
- **THEN** the compiler MUST report the violation as an error
- **AND** the standards enforcement MUST come from checked-in repository policy rather than external documentation

#### Scenario: Module name repetition stays explicitly exempted
ID: core.crate-roots-enforce-clippy-as-policy.module-name-repetition-stays-explicitly-exempted

- **GIVEN** the enforced crate-root lint policy block
- **WHEN** a participating crate is reviewed
- **THEN** `clippy::module_name_repetitions` MUST remain an explicit allow in that block
- **AND** the rest of the baseline lints MUST stay denied

### Requirement: Repository Clippy thresholds travel with the checkout
ID: core.repository-clippy-thresholds-travel-with-the-checkout

The system MUST version its project-wide Clippy threshold policy in a repository-root `clippy.toml` so local and CI Clippy runs share the same executable standards.

#### Scenario: Clone-local thresholds match repository policy
ID: core.repository-clippy-thresholds-travel-with-the-checkout.clone-local-thresholds-match-repository-policy

- **GIVEN** a fresh Aspen checkout
- **WHEN** `cargo clippy` resolves repository configuration
- **THEN** it MUST load `clippy.toml` from the repository root
- **AND** that file MUST set `avoid-breaking-exported-api = false`, `cognitive-complexity-threshold = 15`, `too-many-arguments-threshold = 5`, and `type-complexity-threshold = 200`
- **AND** engineers MUST NOT need a separate wiki or local-only config to discover those thresholds

### Requirement: Rollout-scope warnings and threshold overruns fail
ID: core.rollout-scope-warnings-and-threshold-overruns-fail

The system MUST treat rollout-scope warnings and configured Clippy threshold overruns as enforcement failures rather than advisory output.

#### Scenario: Canonical lint path fails on warning-level feedback
ID: core.rollout-scope-warnings-and-threshold-overruns-fail.canonical-lint-path-fails-on-warning-level-feedback

- **GIVEN** a crate in the documented rollout scope
- **WHEN** the canonical lint entrypoint encounters a denied lint, rustdoc warning, or configured threshold overrun for that crate
- **THEN** the command MUST fail
- **AND** the result MUST be treated as repository policy enforcement rather than optional guidance

### Requirement: Rustdoc warnings are enforced as policy
ID: core.rustdoc-warnings-are-enforced-as-policy

The system MUST enforce rustdoc warnings as errors for the same documented rollout scope that adopts the crate-root Clippy deny baseline.

#### Scenario: Rollout-scope docs warnings fail the build
ID: core.rustdoc-warnings-are-enforced-as-policy.rollout-scope-docs-warnings-fail-the-build

- **GIVEN** a crate in the documented rollout scope
- **WHEN** its docs build emits a rustdoc warning under the repository's canonical enforcement command
- **THEN** the warning MUST fail the build as an error
- **AND** the enforcement mechanism MUST be documented in checked-in repository policy or checked-in commands rather than external instructions

### Requirement: Local and CI lint semantics stay identical
ID: core.local-and-ci-lint-semantics-stay-identical

The system MUST keep the rollout-scope local lint command semantics and CI lint command semantics identical so contributors are taught the same standards locally that automation enforces remotely.

#### Scenario: Canonical lint command matches CI behavior
ID: core.local-and-ci-lint-semantics-stay-identical.canonical-lint-command-matches-ci-behavior

- **GIVEN** the documented rollout-scope enforcement command or fixed command set
- **WHEN** an engineer runs it locally and CI runs the repository lint gate for the same scope
- **THEN** both paths MUST enforce the same deny-level Clippy and rustdoc behavior
- **AND** CI MUST NOT rely on a stricter hidden lint variant for that scope

### Requirement: Canonical lint entrypoint is checked in and shared
ID: core.canonical-lint-entrypoint-is-checked-in-and-shared

The system MUST provide one canonical checked-in lint entrypoint for the rollout scope so contributors and CI share the same obvious enforcement path.

#### Scenario: Contributor and CI use the same lint entrypoint semantics
ID: core.canonical-lint-entrypoint-is-checked-in-and-shared.contributor-and-ci-use-the-same-lint-entrypoint-semantics

- **GIVEN** a contributor wants to run the repository lint gate for the rollout scope
- **WHEN** they consult checked-in repository guidance or automation wiring
- **THEN** the repository SHALL name one canonical checked-in command, script, or Nix target for that scope
- **AND** CI SHALL invoke that entrypoint or the same deny-level Clippy and rustdoc semantics it defines
- **AND** contributors SHALL NOT need to guess among multiple undocumented equivalents

### Requirement: Canonical lint entrypoint has no ambient setup assumptions
ID: core.canonical-lint-entrypoint-has-no-ambient-setup-assumptions

The system MUST keep the canonical rollout-scope lint entrypoint free of hidden per-user setup assumptions.

#### Scenario: New contributor can run the canonical path from the checkout
ID: core.canonical-lint-entrypoint-has-no-ambient-setup-assumptions.new-contributor-can-run-the-canonical-path-from-the-checkout

- **GIVEN** a fresh Aspen checkout and the documented development environment for the repository
- **WHEN** a contributor runs the canonical lint entrypoint
- **THEN** the entrypoint MUST NOT depend on untracked shell aliases, editor-only settings, or undocumented local wrapper scripts
- **AND** checked-in repository files SHALL define the enforcement path they are expected to use

### Requirement: Additional lint exceptions stay narrow and justified
ID: core.additional-lint-exceptions-stay-narrow-and-justified

The system MUST treat new lint suppressions in the rollout scope as explicit exceptions rather than broad policy escapes.

#### Scenario: Non-standard lint allow is justified inline
ID: core.additional-lint-exceptions-stay-narrow-and-justified.non-standard-lint-allow-is-justified-inline

- **GIVEN** a participating crate needs an `#[allow(...)]` beyond the standard `clippy::module_name_repetitions` baseline exemption
- **WHEN** that suppression is added in the rollout scope
- **THEN** it MUST be scoped to the smallest practical item or module boundary
- **AND** it MUST include inline justification
- **AND** broad crate-wide suppressions SHALL NOT be the default escape hatch

### Requirement: Item-scoped lint allows are the default
ID: core.item-scoped-lint-allows-are-the-default

The system MUST prefer item-scoped lint suppressions over broader scopes in the rollout policy.

#### Scenario: Broader suppression requires extra justification
ID: core.item-scoped-lint-allows-are-the-default.broader-suppression-requires-extra-justification

- **GIVEN** a rollout-scope lint suppression cannot be attached to a single item
- **WHEN** a contributor uses module scope or a broader boundary
- **THEN** the change SHALL justify why item scope is impractical
- **AND** the broader scope SHALL remain as narrow as practical

### Requirement: Rollout-scope crate policy text stays uniform
ID: core.rollout-scope-crate-policy-text-stays-uniform

The system MUST keep the rollout-scope crate-root deny policy text uniform unless a documented exception explicitly permits deviation.

#### Scenario: Participating crates share the same deny block
ID: core.rollout-scope-crate-policy-text-stays-uniform.participating-crates-share-the-same-deny-block

- **GIVEN** multiple crates in the documented rollout scope
- **WHEN** their crate roots are reviewed for policy enforcement
- **THEN** they SHALL use the same deny/allow baseline block text
- **AND** any deviation SHALL be documented as an explicit exception in the change evidence

### Requirement: Rollout-scope unsafe sites are inventoried and documented
ID: core.rollout-scope-unsafe-sites-are-inventoried-and-documented

The system MUST save reviewable evidence for `unsafe` usage in rollout-scope crates.

#### Scenario: Unsafe sites remain explicit under enforced policy
ID: core.rollout-scope-unsafe-sites-are-inventoried-and-documented.unsafe-sites-remain-explicit-under-enforced-policy

- **GIVEN** a rollout-scope crate contains one or more `unsafe` blocks
- **WHEN** the lint policy change is verified
- **THEN** the evidence SHALL inventory those sites
- **AND** it SHALL show the corresponding safety comments or justification remain present and reviewable

### Requirement: Negative proof shows enforcement bites
ID: core.negative-proof-shows-enforcement-bites

The system MUST verify not only that the cleaned rollout scope passes, but also that the canonical enforcement path fails on a representative violation.

#### Scenario: Representative violation fails under canonical entrypoint
ID: core.negative-proof-shows-enforcement-bites.representative-violation-fails-under-canonical-entrypoint

- **GIVEN** the canonical rollout-scope lint entrypoint
- **WHEN** it is exercised against a representative known violation or fixture for the enforced policy
- **THEN** the command SHALL fail
- **AND** saved evidence SHALL show that the enforcement path would reject the violation in practice

### Requirement: Enforced Clippy policy verification is reviewable
ID: core.enforced-clippy-policy-verification-is-reviewable

The repository MUST keep durable evidence for the rollout scope, baseline lint state, and final enforced `cargo clippy` result for this policy change.

#### Scenario: Reviewers can inspect rollout proof
ID: core.enforced-clippy-policy-verification-is-reviewable.reviewers-can-inspect-rollout-proof

- **GIVEN** the enforced Clippy rollout change
- **WHEN** a reviewer inspects the OpenSpec artifacts
- **THEN** the documented rollout inventory, deferred-crate inventory, baseline `cargo clippy` transcript, final `cargo clippy` transcript for the same scope, rustdoc enforcement transcript, local/CI parity evidence, canonical-entrypoint evidence, negative-proof evidence, unsafe inventory evidence where applicable, and implementation diff SHALL be saved under `openspec/changes/enforce-clippy-policy/evidence/`
- **AND** `openspec/changes/enforce-clippy-policy/verification.md` SHALL map each checked task to those artifact paths
