# Core Specification

## Purpose

Aspen core policy captures foundational crate contracts, lint enforcement, no-std boundaries, and reviewable verification evidence for reusable core surfaces.
## Requirements
### Requirement: Crate roots enforce Clippy as policy

The system MUST ensure Aspen crate roots in the documented rollout scope enforce the repository Clippy baseline through crate-level attributes instead of warning-only defaults.

The required baseline for each participating crate root is:

```rust
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::undocumented_unsafe_blocks)]
#![allow(clippy::module_name_repetitions)]
```

ID: core.crate-roots-enforce-clippy-as-policy
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

The system MUST version its project-wide Clippy threshold policy in a repository-root `clippy.toml` so local and CI Clippy runs share the same executable standards.

ID: core.repository-clippy-thresholds-travel-with-the-checkout
#### Scenario: Clone-local thresholds match repository policy
ID: core.repository-clippy-thresholds-travel-with-the-checkout.clone-local-thresholds-match-repository-policy

- **GIVEN** a fresh Aspen checkout
- **WHEN** `cargo clippy` resolves repository configuration
- **THEN** it MUST load `clippy.toml` from the repository root
- **AND** that file MUST set `avoid-breaking-exported-api = false`, `cognitive-complexity-threshold = 15`, `too-many-arguments-threshold = 5`, and `type-complexity-threshold = 200`
- **AND** engineers MUST NOT need a separate wiki or local-only config to discover those thresholds

### Requirement: Rollout-scope warnings and threshold overruns fail

The system MUST treat rollout-scope warnings and configured Clippy threshold overruns as enforcement failures rather than advisory output.

ID: core.rollout-scope-warnings-and-threshold-overruns-fail
#### Scenario: Canonical lint path fails on warning-level feedback
ID: core.rollout-scope-warnings-and-threshold-overruns-fail.canonical-lint-path-fails-on-warning-level-feedback

- **GIVEN** a crate in the documented rollout scope
- **WHEN** the canonical lint entrypoint encounters a denied lint, rustdoc warning, or configured threshold overrun for that crate
- **THEN** the command MUST fail
- **AND** the result MUST be treated as repository policy enforcement rather than optional guidance

### Requirement: Rustdoc warnings are enforced as policy

The system MUST enforce rustdoc warnings as errors for the same documented rollout scope that adopts the crate-root Clippy deny baseline.

ID: core.rustdoc-warnings-are-enforced-as-policy
#### Scenario: Rollout-scope docs warnings fail the build
ID: core.rustdoc-warnings-are-enforced-as-policy.rollout-scope-docs-warnings-fail-the-build

- **GIVEN** a crate in the documented rollout scope
- **WHEN** its docs build emits a rustdoc warning under the repository's canonical enforcement command
- **THEN** the warning MUST fail the build as an error
- **AND** the enforcement mechanism MUST be documented in checked-in repository policy or checked-in commands rather than external instructions

### Requirement: Local and CI lint semantics stay identical

The system MUST keep the rollout-scope local lint command semantics and CI lint command semantics identical so contributors are taught the same standards locally that automation enforces remotely.

ID: core.local-and-ci-lint-semantics-stay-identical
#### Scenario: Canonical lint command matches CI behavior
ID: core.local-and-ci-lint-semantics-stay-identical.canonical-lint-command-matches-ci-behavior

- **GIVEN** the documented rollout-scope enforcement command or fixed command set
- **WHEN** an engineer runs it locally and CI runs the repository lint gate for the same scope
- **THEN** both paths MUST enforce the same deny-level Clippy and rustdoc behavior
- **AND** CI MUST NOT rely on a stricter hidden lint variant for that scope

### Requirement: Canonical lint entrypoint is checked in and shared

The system MUST provide one canonical checked-in lint entrypoint for the rollout scope so contributors and CI share the same obvious enforcement path.

ID: core.canonical-lint-entrypoint-is-checked-in-and-shared
#### Scenario: Contributor and CI use the same lint entrypoint semantics
ID: core.canonical-lint-entrypoint-is-checked-in-and-shared.contributor-and-ci-use-the-same-lint-entrypoint-semantics

- **GIVEN** a contributor wants to run the repository lint gate for the rollout scope
- **WHEN** they consult checked-in repository guidance or automation wiring
- **THEN** the repository SHALL name one canonical checked-in command, script, or Nix target for that scope
- **AND** CI SHALL invoke that entrypoint or the same deny-level Clippy and rustdoc semantics it defines
- **AND** contributors SHALL NOT need to guess among multiple undocumented equivalents

### Requirement: Canonical lint entrypoint has no ambient setup assumptions

The system MUST keep the canonical rollout-scope lint entrypoint free of hidden per-user setup assumptions.

ID: core.canonical-lint-entrypoint-has-no-ambient-setup-assumptions
#### Scenario: New contributor can run the canonical path from the checkout
ID: core.canonical-lint-entrypoint-has-no-ambient-setup-assumptions.new-contributor-can-run-the-canonical-path-from-the-checkout

- **GIVEN** a fresh Aspen checkout and the documented development environment for the repository
- **WHEN** a contributor runs the canonical lint entrypoint
- **THEN** the entrypoint MUST NOT depend on untracked shell aliases, editor-only settings, or undocumented local wrapper scripts
- **AND** checked-in repository files SHALL define the enforcement path they are expected to use

### Requirement: Additional lint exceptions stay narrow and justified

The system MUST treat new lint suppressions in the rollout scope as explicit exceptions rather than broad policy escapes.

ID: core.additional-lint-exceptions-stay-narrow-and-justified
#### Scenario: Non-standard lint allow is justified inline
ID: core.additional-lint-exceptions-stay-narrow-and-justified.non-standard-lint-allow-is-justified-inline

- **GIVEN** a participating crate needs an `#[allow(...)]` beyond the standard `clippy::module_name_repetitions` baseline exemption
- **WHEN** that suppression is added in the rollout scope
- **THEN** it MUST be scoped to the smallest practical item or module boundary
- **AND** it MUST include inline justification
- **AND** broad crate-wide suppressions SHALL NOT be the default escape hatch

### Requirement: Item-scoped lint allows are the default

The system MUST prefer item-scoped lint suppressions over broader scopes in the rollout policy.

ID: core.item-scoped-lint-allows-are-the-default
#### Scenario: Broader suppression requires extra justification
ID: core.item-scoped-lint-allows-are-the-default.broader-suppression-requires-extra-justification

- **GIVEN** a rollout-scope lint suppression cannot be attached to a single item
- **WHEN** a contributor uses module scope or a broader boundary
- **THEN** the change SHALL justify why item scope is impractical
- **AND** the broader scope SHALL remain as narrow as practical

### Requirement: Rollout-scope crate policy text stays uniform

The system MUST keep the rollout-scope crate-root deny policy text uniform unless a documented exception explicitly permits deviation.

ID: core.rollout-scope-crate-policy-text-stays-uniform
#### Scenario: Participating crates share the same deny block
ID: core.rollout-scope-crate-policy-text-stays-uniform.participating-crates-share-the-same-deny-block

- **GIVEN** multiple crates in the documented rollout scope
- **WHEN** their crate roots are reviewed for policy enforcement
- **THEN** they SHALL use the same deny/allow baseline block text
- **AND** any deviation SHALL be documented as an explicit exception in the change evidence

### Requirement: Rollout-scope unsafe sites are inventoried and documented

The system MUST save reviewable evidence for `unsafe` usage in rollout-scope crates.

ID: core.rollout-scope-unsafe-sites-are-inventoried-and-documented
#### Scenario: Unsafe sites remain explicit under enforced policy
ID: core.rollout-scope-unsafe-sites-are-inventoried-and-documented.unsafe-sites-remain-explicit-under-enforced-policy

- **GIVEN** a rollout-scope crate contains one or more `unsafe` blocks
- **WHEN** the lint policy change is verified
- **THEN** the evidence SHALL inventory those sites
- **AND** it SHALL show the corresponding safety comments or justification remain present and reviewable

### Requirement: Negative proof shows enforcement bites

The system MUST verify not only that the cleaned rollout scope passes, but also that the canonical enforcement path fails on a representative violation.

ID: core.negative-proof-shows-enforcement-bites
#### Scenario: Representative violation fails under canonical entrypoint
ID: core.negative-proof-shows-enforcement-bites.representative-violation-fails-under-canonical-entrypoint

- **GIVEN** the canonical rollout-scope lint entrypoint
- **WHEN** it is exercised against a representative known violation or fixture for the enforced policy
- **THEN** the command SHALL fail
- **AND** saved evidence SHALL show that the enforcement path would reject the violation in practice

### Requirement: Enforced Clippy policy verification is reviewable

The repository MUST keep durable evidence for the rollout scope, baseline lint state, and final enforced `cargo clippy` result for this policy change.

ID: core.enforced-clippy-policy-verification-is-reviewable
#### Scenario: Reviewers can inspect rollout proof
ID: core.enforced-clippy-policy-verification-is-reviewable.reviewers-can-inspect-rollout-proof

- **GIVEN** the enforced Clippy rollout change
- **WHEN** a reviewer inspects the OpenSpec artifacts
- **THEN** the documented rollout inventory, deferred-crate inventory, baseline `cargo clippy` transcript, final `cargo clippy` transcript for the same scope, rustdoc enforcement transcript, local/CI parity evidence, canonical-entrypoint evidence, negative-proof evidence, unsafe inventory evidence where applicable, and implementation diff SHALL be saved under `openspec/changes/archive/2026-04-23-enforce-clippy-policy/evidence/`
- **AND** `openspec/changes/archive/2026-04-23-enforce-clippy-policy/verification.md` SHALL map each checked task to those artifact paths

### Requirement: No-std core baseline

The `aspen-core` crate MUST provide an alloc-backed `no_std` build that exposes Aspen's foundational types, traits, constants, and deterministic helper logic without requiring filesystem, process, networking, thread, async-runtime, or storage-engine dependencies.

ID: core.no-std-core-baseline
#### Scenario: Bare dependency uses alloc-only default
ID: core.no-std-core-baseline.bare-dependency-uses-alloc-only-default

- **GIVEN** a consumer that depends on `aspen-core` without overriding features
- **WHEN** Cargo resolves the dependency
- **THEN** the default feature set MUST be empty
- **AND** the consumer MUST receive the alloc-only surface unless it explicitly opts into the documented shell compatibility path through `aspen-core-shell`

#### Scenario: Alloc-only build succeeds
ID: core.no-std-core-baseline.alloc-only-build-succeeds

- **GIVEN** a consumer that depends on `aspen-core` only for Aspen contracts and deterministic helpers
- **WHEN** the crate is built with its documented alloc-only configuration and `std` support disabled
- **THEN** the build MUST succeed
- **AND** Cargo resolution MUST NOT pull `tokio`, `tokio-util`, `iroh`, `iroh-base`, `iroh-blobs`, `anyhow`, `chrono`, `serde_json`, `redb`, `libc`, `aspen-disk`, `aspen-time`, filesystem persistence helpers, or other `std`-only runtime dependencies through that alloc-only surface

#### Scenario: Bare-default downstream consumer remains supported
ID: core.no-std-core-baseline.bare-default-downstream-consumer-remains-supported

- **GIVEN** the workspace smoke consumer `crates/aspen-core-no-std-smoke`
- **WHEN** it depends on `aspen-core` without overriding features
- **THEN** it MUST compile without enabling shell features
- **AND** it MUST be able to import foundational core types or traits from the alloc-only surface, including alloc-safe storage record types such as `aspen_core::storage::KvEntry`

#### Scenario: Alloc-only build rejects shell imports
ID: core.no-std-core-baseline.alloc-only-build-rejects-shell-imports

- **GIVEN** a compile-fail fixture built against alloc-only `aspen-core`
- **WHEN** the fixture imports a `std`-only API such as transport, simulation, or Redb table-definition helpers without enabling the documented shell path
- **THEN** compilation MUST fail
- **AND** the failure MUST identify that the requested API is outside the alloc-only surface

#### Scenario: Compile-fail verification is reviewable
ID: core.no-std-core-baseline.compile-fail-verification-is-reviewable

- **GIVEN** compile-fail fixtures under `crates/aspen-core/tests/ui/` that exercise shell imports without `std`
- **WHEN** their negative verification is saved for review
- **THEN** the exact fixture paths, commands, and stderr assertions SHALL be recorded under `openspec/changes/extend-no-std-foundation-and-wire/evidence/`
- **AND** the fixture set SHALL cover at least failed alloc-only imports of `aspen_core::{AppRegistry, NetworkTransport, SimulationArtifact, ContentDiscovery, DirectoryLayer}` plus failed alloc-only imports of `aspen_core::storage::SM_KV_TABLE` without the required shell features
- **AND** `openspec/changes/extend-no-std-foundation-and-wire/verification.md` SHALL identify which artifact proves each compile-fail expectation

#### Scenario: Std-dependent helpers require explicit opt-in
ID: core.no-std-core-baseline.std-dependent-helpers-require-explicit-opt-in

- **GIVEN** a consumer that needs runtime helpers such as transport integration, in-memory registries, simulation persistence, or Redb table definitions
- **WHEN** the consumer opts into the documented shell compatibility path by depending on `aspen-core-shell` (often aliased locally as `aspen-core`)
- **THEN** those helpers MUST become available only through that explicit opt-in boundary
- **AND** the alloc-only `aspen-core` surface MUST remain unchanged for consumers that do not opt in

#### Scenario: Std-gated shell APIs keep current public paths
ID: core.no-std-core-baseline.std-gated-shell-apis-keep-current-public-paths

- **GIVEN** a shell API currently exposed from `aspen_core::*`, including Redb-specific storage helpers such as `aspen_core::storage::SM_KV_TABLE`
- **WHEN** a consumer enables the required shell feature for this cut through the established `aspen-core-shell` alias pattern
- **THEN** that API MUST remain available at its current public path
- **AND** the change MUST gate that path rather than rename it

#### Scenario: Module-family boundary matches documented inventory
ID: core.no-std-core-baseline.module-family-boundary-matches-documented-inventory

- **GIVEN** the documented alloc-only families `circuit_breaker`, `cluster`, `constants` scalar and numeric exports, `crypto`, `error`, `hlc`, `kv`, alloc-safe `prelude`, `protocol`, `spec`, `storage` record types and codecs, `traits`, `types`, `vault`, `verified`, and alloc-safe optional `sql`
- **AND** the documented shell families `app_registry`, `context`, `simulation`, `transport`, Redb-specific storage table definitions, `utils`, `test_support`, runtime-only prelude additions, and duration convenience exports
- **AND** the documented optional `std`-gated path families `layer` and `global-discovery`
- **WHEN** the boundary is reviewed
- **THEN** the alloc-only families SHALL remain available without `std`
- **AND** the shell families plus optional `std`-gated path families SHALL remain gated or shell-only as documented for this cut

#### Scenario: Alloc-only storage surface excludes Redb table definitions
ID: core.no-std-core-baseline.alloc-only-storage-surface-excludes-redb-table-definitions

- **GIVEN** an alloc-only consumer of `aspen-core`
- **WHEN** it imports `aspen_core::storage`
- **THEN** alloc-safe record types such as `KvEntry` MUST remain available
- **AND** Redb table definitions such as `SM_KV_TABLE` MUST remain unavailable unless the consumer opts into the documented shell path through `aspen-core-shell`, preserving the shell-facing import `aspen_core::storage::SM_KV_TABLE`

#### Scenario: Shell alias path proof is reviewable
ID: core.no-std-core-baseline.shell-alias-path-proof-is-reviewable

- **GIVEN** the preserved shell-facing import `aspen_core::storage::SM_KV_TABLE` through the `aspen-core-shell` alias pattern
- **WHEN** that compatibility claim is verified for review
- **THEN** the exact commands and results for `cargo check -p aspen-core-shell`, `cargo check -p aspen-core-shell --features layer`, `cargo check -p aspen-core-shell --features global-discovery`, `cargo check -p aspen-core-shell --features sql`, and `cargo check --manifest-path crates/aspen-core/tests/fixtures/shell-alias-smoke/Cargo.toml` SHALL be saved under `openspec/changes/extend-no-std-foundation-and-wire/evidence/`
- **AND** the alias-form consumer fixture SHALL live at `crates/aspen-core/tests/fixtures/shell-alias-smoke/` and import `aspen_core::storage::SM_KV_TABLE` through the alias form `aspen-core = { package = "aspen-core-shell", ... }`
- **AND** `openspec/changes/extend-no-std-foundation-and-wire/verification.md` SHALL identify which artifact proves the alias-path claim

#### Scenario: Representative std consumers remain supported
ID: core.no-std-core-baseline.representative-std-consumers-remain-supported

- **GIVEN** the representative workspace consumers `crates/aspen-cluster`, `crates/aspen-client`, `crates/aspen-cli`, `crates/aspen-rpc-handlers`, and the root `aspen` bundle slice built with `--no-default-features --features node-runtime`
- **WHEN** they opt into the documented `std` compatibility path
- **THEN** they MUST continue to compile against `aspen-core`
- **AND** the verification matrix MUST keep those consumer slices green

#### Scenario: Compile-slice verification is reviewable
ID: core.no-std-core-baseline.compile-slice-verification-is-reviewable

- **GIVEN** the alloc-only crate, alloc-safe `sql`, the shell package `aspen-core-shell` with its relevant feature combinations, the smoke consumer, and representative std consumers
- **WHEN** their compile slices are verified for review
- **THEN** the exact commands and results for `cargo check -p aspen-core`, `cargo check -p aspen-core --no-default-features`, `cargo check -p aspen-core --no-default-features --features sql`, `cargo check -p aspen-core-shell`, `cargo check -p aspen-core-shell --features layer`, `cargo check -p aspen-core-shell --features global-discovery`, `cargo check -p aspen-core-shell --features sql`, `cargo check -p aspen-core-no-std-smoke`, `cargo check -p aspen-cluster`, `cargo check -p aspen-client`, `cargo check -p aspen-cli`, `cargo check -p aspen-rpc-handlers`, and `cargo check -p aspen --no-default-features --features node-runtime` SHALL be saved under `openspec/changes/extend-no-std-foundation-and-wire/evidence/`
- **AND** the saved smoke-consumer artifact SHALL prove alloc-only `aspen_core::storage::KvEntry` remains reachable after the storage split
- **AND** `openspec/changes/extend-no-std-foundation-and-wire/verification.md` SHALL identify which artifact proves each slice

