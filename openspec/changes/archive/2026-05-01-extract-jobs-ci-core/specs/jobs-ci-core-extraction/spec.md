## ADDED Requirements

### Requirement: Core boundary inventory [r[jobs-ci-core-extraction.core-boundary-inventory]]
The jobs/CI core extraction SHALL classify current jobs and CI public surfaces before moving code so reviewers can distinguish reusable contracts from runtime shells.

#### Scenario: Public surface classified [r[jobs-ci-core-extraction.core-boundary-inventory.public-surface-classified]]
- GIVEN the public exports from `crates/aspen-jobs/src/lib.rs` and reusable CI/job protocol crates
- WHEN the extraction inventory is prepared
- THEN every public jobs export considered by the first slice SHALL be classified as reusable core, compatibility re-export, runtime shell, worker/executor adapter, or deferred orchestration
- AND the classification SHALL be saved as checked-in evidence under `openspec/changes/archive/2026-05-01-extract-jobs-ci-core/evidence/`

#### Scenario: Baseline dependencies recorded [r[jobs-ci-core-extraction.core-boundary-inventory.baseline-dependencies-recorded]]
- GIVEN `aspen-jobs`, `aspen-ci-core`, and `aspen-jobs-protocol` before the first implementation slice
- WHEN their dependency graphs are captured for review
- THEN the exact commands and results SHALL be saved as evidence
- AND the evidence SHALL identify which dependencies are forbidden from the reusable jobs/CI core default surface

### Requirement: Reusable core crate [r[jobs-ci-core-extraction.reusable-core-crate]]
The system SHALL provide a reusable jobs core crate for pure job contracts and deterministic helpers without depending on Aspen runtime shells or concrete worker/executor adapters.

#### Scenario: Core crate exists [r[jobs-ci-core-extraction.reusable-core-crate.core-crate-exists]]
- GIVEN a downstream consumer needs job IDs, priority, retry policy, schedule descriptors, job config/spec/result/status helpers, dependency state, and deterministic run-state helpers
- WHEN it depends on the reusable jobs core crate with default features
- THEN Cargo resolution SHALL succeed without requiring root `aspen`, handler crates, concrete transport crates, worker crates, executor crates, Redb storage, VM support, process execution, Nix/SNIX integration, or node bootstrap crates

#### Scenario: Core contracts verified [r[jobs-ci-core-extraction.reusable-core-crate.core-contracts-verified]]
- GIVEN the reusable core exposes serialized job model and deterministic transition helpers
- WHEN tests run for the core crate
- THEN serialization roundtrips and deterministic state-transition behavior SHALL be verified without invoking runtime services

### Requirement: Compatibility shell [r[jobs-ci-core-extraction.compatibility-shell]]
The runtime `aspen-jobs` crate SHALL remain the compatibility shell for existing public paths while delegating reusable contracts to the core crate.

#### Scenario: Existing paths continue [r[jobs-ci-core-extraction.compatibility-shell.existing-paths-continue]]
- GIVEN existing workspace consumers import reusable job types from `aspen_jobs::*`
- WHEN the core extraction slice is complete
- THEN those imports SHALL continue to compile through re-exports or compatible wrappers
- AND new code SHALL prefer canonical `aspen_jobs_core::*` imports where runtime services are not needed

#### Scenario: Workspace consumers still compile [r[jobs-ci-core-extraction.compatibility-shell.workspace-consumers-still-compile]]
- GIVEN the job/CI runtime crates, handlers, CLI, dogfood, and executor crates that consume jobs/CI APIs
- WHEN focused compatibility checks are run
- THEN each checked consumer SHALL compile or have a documented pre-existing blocker with command output saved as evidence

### Requirement: Downstream fixture [r[jobs-ci-core-extraction.downstream-fixture]]
The extraction SHALL include a downstream-style fixture that proves canonical reusable imports are sufficient for non-runtime jobs/CI consumers.

#### Scenario: Fixture uses canonical core [r[jobs-ci-core-extraction.downstream-fixture.fixture-uses-canonical-core]]
- GIVEN a fixture outside the workspace package graph depends only on reusable jobs/CI defaults
- WHEN it imports `aspen-jobs-core`, `aspen-ci-core`, and `aspen-jobs-protocol`
- THEN it SHALL compile without importing the runtime `aspen-jobs` crate or any handler/worker/executor crate
- AND saved metadata SHALL prove root `aspen` and runtime adapter crates are absent from the fixture graph

### Requirement: Runtime boundary [r[jobs-ci-core-extraction.runtime-boundary]]
Runtime-only jobs/CI services SHALL remain outside the reusable core default surface.

#### Scenario: Runtime shells excluded from core default [r[jobs-ci-core-extraction.runtime-boundary.runtime-shells-excluded-from-core-default]]
- GIVEN reusable jobs/CI defaults are used by a downstream consumer
- WHEN negative boundary checks attempt to import runtime-only services
- THEN imports from `aspen-jobs`, workers, handlers, root `aspen`, shell/VM/Nix executors, concrete transport, Redb storage, or process runtime APIs SHALL fail unless explicit runtime crates/features are selected

### Requirement: Reviewable evidence [r[jobs-ci-core-extraction.reviewable-evidence]]
The jobs/CI core extraction SHALL produce durable evidence linked from task and verification artifacts.

#### Scenario: Evidence linked from verification [r[jobs-ci-core-extraction.reviewable-evidence.evidence-linked-from-verification]]
- GIVEN implementation tasks are marked complete
- WHEN the change is prepared for review
- THEN `verification.md` SHALL cite checked-in evidence for inventory, dependency graphs, core tests, downstream fixture checks, negative boundary checks, readiness checker output, compatibility checks, and OpenSpec validation
