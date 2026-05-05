# typed-nickel-contracts Specification

## Purpose
TBD - created by archiving change type-nickel-contract-boundaries. Update Purpose after archive.
## Requirements
### Requirement: Typed Nickel Contract Registry [r[typed-nickel-contracts.registry]]

Aspen MUST maintain a registry of typed Nickel contract families that identifies each contract's source of truth, owner, generated artifact path, validation command, and freshness gate.

#### Scenario: Registry classifies contract ownership [r[typed-nickel-contracts.registry.classifies-ownership]]

- GIVEN a contract family such as CI config, deploy protocol, dogfood receipt, node config, test harness manifest, feature profile, trust policy, or snix executor policy
- WHEN a maintainer inspects the registry
- THEN the registry MUST state whether the family is `rust-derived` or `nickel-authored`
- AND it MUST name the Rust module or Nickel file that owns the source schema

#### Scenario: Registry rejects unclassified schema drift [r[typed-nickel-contracts.registry.rejects-unclassified-drift]]

- GIVEN a generated Nickel contract changes without a matching source-of-truth change
- WHEN the freshness checker runs
- THEN the checker MUST fail with the contract family and stale artifact path

### Requirement: Rust-Derived Nickel Contracts [r[typed-nickel-contracts.rust-derived]]

Aspen MUST generate Nickel contracts from Rust-owned schema-bearing structs and enums when Rust owns a serialized, persisted, or operator-facing evidence/protocol shape.

#### Scenario: Rust DTO generates a Nickel contract [r[typed-nickel-contracts.rust-derived.generates-contract]]

- GIVEN a selected Rust struct or enum derives the schema metadata required by Aspen's generator
- WHEN the Nickel contract generation command runs
- THEN the generated Nickel contract MUST include record fields, enum alternatives, optionality, defaults when known, documentation when available, and bounded collection/string contracts when modeled

#### Scenario: Rust and Nickel remain fresh [r[typed-nickel-contracts.rust-derived.freshness]]

- GIVEN a selected Rust DTO field, enum variant, serde rename, default, or bound changes
- WHEN the generation freshness check runs without updating the generated Nickel artifact
- THEN the check MUST fail before tests or release packaging can treat the contract as current

#### Scenario: Generated contracts validate serialized evidence [r[typed-nickel-contracts.rust-derived.validates-serialized-evidence]]

- GIVEN Rust serializes an operator-facing receipt or protocol DTO to JSON
- WHEN that JSON is imported or checked through the generated Nickel contract
- THEN valid serialized values MUST pass and malformed, missing, out-of-range, or unknown-field values MUST fail

### Requirement: Nickel-Authored Runtime Config Contracts [r[typed-nickel-contracts.nickel-authored]]

Aspen MUST keep human-authored modular configuration in Nickel when defaults, merge semantics, documentation, and environment/profile overlays are the primary interface, and Rust MUST consume only validated exported values from those contracts.

#### Scenario: Nickel config exports validated data for Rust [r[typed-nickel-contracts.nickel-authored.exports-validated-data]]

- GIVEN a CI pipeline, node profile, feature bundle, test suite manifest, patchbay scenario, snix executor policy, or trust/bootstrap policy is Nickel-authored
- WHEN the config is exported for Rust consumption
- THEN Nickel typecheck and contract validation MUST run before Rust uses the exported data for runtime side effects

#### Scenario: Nickel modules keep helper fields out of runtime input [r[typed-nickel-contracts.nickel-authored.not-exported-helpers]]

- GIVEN a Nickel module uses local/helper fields, derived defaults, or documentation metadata
- WHEN the module is exported
- THEN helper fields marked `not_exported` MUST NOT appear in the Rust-consumed output

### Requirement: Operator Receipt Contracts [r[typed-nickel-contracts.operator-receipts]]

Dogfood and native CI receipt schemas MUST have typed Nickel contracts that are generated from the Rust receipt structs that own canonical serialization.

#### Scenario: Dogfood receipt contract validates run evidence [r[typed-nickel-contracts.operator-receipts.dogfood]]

- GIVEN a dogfood run receipt JSON file
- WHEN the generated dogfood receipt Nickel contract validates it
- THEN it MUST require schema version, run id, command, timestamps, aggregate status, ordered stages, bounded artifacts, failure summaries, and publish/readback metadata when present

#### Scenario: CI receipt contract validates native CI evidence [r[typed-nickel-contracts.operator-receipts.ci]]

- GIVEN a native CI run receipt JSON value
- WHEN the generated CI receipt Nickel contract validates it
- THEN it MUST require schema `aspen.ci.run-receipt.v1`, run identity, repository/ref/commit identity, deterministic stages/jobs, job IDs when available, artifact metadata, status, and timestamps

### Requirement: CI and Deploy Configuration Contracts [r[typed-nickel-contracts.ci-deploy-config]]

Aspen CI and deploy configuration MUST expose typed Nickel contracts for pipeline stages, jobs, dependencies, executor kind, artifact inputs/outputs, cache policy, retry/timeouts, deploy hooks, statefulness, validation-only behavior, and deploy strategy fields.

#### Scenario: CI config rejects invalid dependency references [r[typed-nickel-contracts.ci-deploy-config.rejects-invalid-dependencies]]

- GIVEN a `.aspen/ci.ncl` pipeline references a missing stage, missing job, invalid artifact source, or cyclic dependency
- WHEN Nickel validation and Aspen's semantic config checks run
- THEN the pipeline MUST be rejected before jobs are enqueued

#### Scenario: Deploy contract preserves statefulness defaults [r[typed-nickel-contracts.ci-deploy-config.stateful-default]]

- GIVEN a deploy stage omits `stateful`
- WHEN the Nickel config is exported and parsed by Rust
- THEN the resulting deploy request MUST default to stateful lifecycle tracking for backwards compatibility

### Requirement: Node, Cluster, and Feature Profile Contracts [r[typed-nickel-contracts.node-profile-config]]

Aspen MUST provide typed Nickel contracts for node identity, cluster bootstrap topology, feature bundles, storage paths, transport/discovery policy, metrics/OTLP config, and trust/quorum references.

#### Scenario: Profile rejects invalid feature combinations [r[typed-nickel-contracts.node-profile-config.rejects-invalid-feature-combo]]

- GIVEN an operator selects a product profile such as minimal node, dogfood node, CI worker, forge-only node, snix cache gateway, or development cluster
- WHEN the feature bundle contract is evaluated
- THEN unsupported or contradictory feature combinations MUST fail with an operator-visible validation error

#### Scenario: Secret references are validated without embedding secrets [r[typed-nickel-contracts.node-profile-config.secret-references-only]]

- GIVEN node or trust/bootstrap configuration needs key, token, or capability material
- WHEN the Nickel contract validates the configuration
- THEN the config MUST use references, paths, handles, or capability names rather than raw bearer credential values

### Requirement: Test Harness and Fault Scenario Contracts [r[typed-nickel-contracts.test-fault-manifests]]

Aspen test harness and patchbay/network fault scenario manifests MUST use typed Nickel contracts for suite identity, execution layer, capabilities, isolation assumptions, timeout class, expected artifacts, fault dimensions, and convergence assertions.

#### Scenario: Generated inventory remains authoritative [r[typed-nickel-contracts.test-fault-manifests.generated-inventory]]

- GIVEN a suite or fault scenario manifest changes
- WHEN the harness inventory freshness check runs
- THEN generated Rust/Nix inventory outputs MUST be refreshed from Nickel and stale generated outputs MUST fail the check

#### Scenario: Fault scenarios declare bounded expectations [r[typed-nickel-contracts.test-fault-manifests.bounded-expectations]]

- GIVEN a patchbay or network scenario declares NAT, partition, latency, packet loss, or peer churn parameters
- WHEN the scenario is validated
- THEN the contract MUST enforce bounded parameter ranges and require explicit convergence or failure expectations

### Requirement: Snix, Trust, and Extraction Policy Contracts [r[typed-nickel-contracts.policy-contracts]]

Aspen MUST use typed Nickel contracts for snix/build executor policy, trust/quorum/bootstrap policy, and crate-extraction readiness policy when those policies are declarative, bounded, and operator- or maintainer-authored.

#### Scenario: Snix executor policy is fail-closed [r[typed-nickel-contracts.policy-contracts.snix]]

- GIVEN a snix/build executor policy configures upstream cache, sandbox backend, allowed substituters, artifact retention, upload target, or fallback policy
- WHEN the policy is validated
- THEN unsupported backends, unbounded retention, missing upload targets, or disallowed fallback settings MUST fail before build execution

#### Scenario: Trust quorum policy is bounded and secret-free [r[typed-nickel-contracts.policy-contracts.trust]]

- GIVEN a trust/bootstrap policy configures participant count, threshold, rotation, persistence backend, or recovery constraints
- WHEN the policy is validated
- THEN invalid thresholds, impossible participant counts, unsupported rotation modes, and embedded raw secret material MUST fail

#### Scenario: Crate extraction policy validates readiness evidence [r[typed-nickel-contracts.policy-contracts.extraction]]

- GIVEN a crate extraction candidate claims readiness
- WHEN the readiness policy contract and checker run
- THEN readiness state, dependency rails, publication metadata, no-std/alloc/std class, and required evidence files MUST be validated from the Nickel policy rather than duplicated in multiple unchecked scripts

### Requirement: Crunch Prior-Art Classification [r[typed-nickel-contracts.crunch-prior-art]]

Aspen MUST evaluate `../crunch/crunch` Nickel and Rust schema patterns before implementing typed Nickel contract families, and the contract registry MUST classify each reusable Crunch pattern as vendored, adapted, or rejected with rationale.

#### Scenario: Generic Nickel contract helpers are classified [r[typed-nickel-contracts.crunch-prior-art.generic-helpers]]

- GIVEN Crunch provides generic Nickel helper patterns such as `lib/contracts.ncl`, `lib/project.ncl`, `lib/project_outputs.ncl`, `builders/mk_derivation.ncl`, `lib/inventory.ncl`, and `lib/system_module.ncl`
- WHEN Aspen creates or updates its typed Nickel contract registry
- THEN the registry MUST state whether each helper family is vendored directly, adapted into Aspen-specific contracts, or rejected
- AND the rationale MUST distinguish reusable shape/default/topology contracts from Crunch-owned build semantics

#### Scenario: Rust/Nickel boundary patterns are classified [r[typed-nickel-contracts.crunch-prior-art.rust-boundaries]]

- GIVEN Crunch provides Rust-side schema and evidence patterns such as `crates/crunch-glue/src/types.rs`, `crates/crunch-project-core/src/manifest.rs`, `src/build_report.rs`, `src/operator_diagnostics.rs`, `src/witness_rebuild.rs`, and `crates/crunch-attestation-core/src/schema.rs`
- WHEN Aspen selects a `rust-derived` or `nickel-authored` contract family
- THEN the implementation plan MUST identify which Crunch Rust/Nickel boundary conventions are being reused
- AND it MUST reject direct reuse of Crunch derivation semantics, store-path hashing behavior, witness/bootstrap workflows, and runtime build behavior unless a later Aspen-specific OpenSpec explicitly adopts them

### Requirement: Non-Candidate Boundary [r[typed-nickel-contracts.non-candidates]]

Aspen MUST NOT move distributed behavior, cryptographic internals, wire discriminant compatibility ownership, raw secret material, hot-path runtime constants, or Crunch-owned runtime build semantics into Nickel contracts.

#### Scenario: Runtime behavior remains in Rust [r[typed-nickel-contracts.non-candidates.runtime-behavior]]

- GIVEN a proposed Nickel contract attempts to encode Raft state transitions, async orchestration behavior, token verification internals, or protocol enum discriminant assignment
- WHEN the contract boundary is reviewed
- THEN the proposal MUST be rejected or rewritten so Nickel validates only data shape/configuration/evidence and Rust retains behavior ownership
