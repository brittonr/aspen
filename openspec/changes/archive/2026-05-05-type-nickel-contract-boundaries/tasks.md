## Specification Foundation

- [x] S1 Create the OpenSpec proposal, design, task plan, and delta spec for typed Nickel contract boundaries.

## Phase 1: Contract Registry and Source-of-Truth Classification

- [x] I1 Inventory existing Nickel and schema-bearing Rust surfaces: CI config, deploy protocol, dogfood/CI receipts, node config, test harness manifests, crate-extraction policy, feature bundles, trust/bootstrap policy, snix executor policy, and patchbay/network scenarios. Evidence: `docs/typed-nickel-contract-registry.md`, `schemas/typed-nickel-contract-registry.ncl`.
- [x] I1a Compare `../crunch/crunch` Nickel/Rust schema patterns and explicitly classify each reusable piece as vendored, adapted, or rejected before implementing Aspen contracts. Evidence: `schemas/typed-nickel-contract-registry.ncl` `crunch_prior_art`.
- [x] I2 Add a contract registry document or manifest that classifies each family as `rust-derived` or `nickel-authored`, names its owning Rust module or Nickel file, and records generation/check commands. Evidence: `schemas/typed-nickel-contract-registry.ncl`, `scripts/check-typed-nickel-contract-registry.py`.
- [x] I3 Document explicit non-candidates: Raft behavior, protocol discriminant ownership, cryptographic internals, secret values, and hot-path runtime constants. Evidence: `docs/typed-nickel-contract-registry.md`, `schemas/typed-nickel-contract-registry.ncl` `non_candidates`.

## Phase 2: Rust-to-Nickel Generation

- [x] I4 Implement or extend a generator that maps selected Serde/schema-bearing Rust structs/enums into Nickel contracts. Evidence: `scripts/generate-typed-nickel-contracts.py`, `openspec/changes/type-nickel-contract-boundaries/evidence/rust-derived-receipt-contracts-verification.md`.
- [x] I5 Generate Nickel contracts for operator-facing receipt/protocol DTOs where Rust owns serialized shape, starting with dogfood/CI receipts and deploy protocol DTOs. Evidence: `schemas/dogfood-run-receipt.ncl`, `schemas/ci-run-receipt.ncl`, `schemas/deploy-protocol.ncl`, `openspec/changes/type-nickel-contract-boundaries/evidence/rust-derived-receipt-contracts-verification.md`, `openspec/changes/type-nickel-contract-boundaries/evidence/deploy-protocol-contract-verification.md`.
- [x] I6 Add freshness checks that fail when generated Nickel contracts differ from current Rust type/schema metadata. Evidence: `scripts/generate-typed-nickel-contracts.py --check`, negative mutation in `openspec/changes/type-nickel-contract-boundaries/evidence/rust-derived-receipt-contracts-verification.md`.

## Phase 3: Nickel-Authored Config Contracts

- [x] I7 Expand CI pipeline and deploy config contracts with typed stages/jobs, dependencies, artifacts, retry/timeouts, cache policy, deploy statefulness, and validation-only behavior. Evidence: `crates/aspen-ci/src/config/schema/ci_schema.ncl`, `scripts/check-typed-nickel-contract-fixtures.py`, `openspec/changes/type-nickel-contract-boundaries/evidence/nickel-authored-contracts-verification.md`.
- [x] I8 Expand node/cluster/profile contracts for identity, bootstrap topology, feature bundles, storage paths, transport/discovery, metrics/OTLP, and trust/quorum references. Evidence: `crates/aspen-nickel/src/schema/node_config.ncl`, `schemas/feature-bundles.ncl`, `openspec/changes/type-nickel-contract-boundaries/evidence/nickel-authored-contracts-verification.md`.
- [x] I9 Expand test harness and patchbay/network manifests with capabilities, isolation assumptions, timeout classes, expected artifacts, fault dimensions, and convergence assertions. Evidence: `test-harness/schema.ncl`, `test-harness/generated/inventory.json`, `scripts/check-typed-nickel-contract-fixtures.py`.
- [x] I10 Add typed Nickel contracts for snix/build executor policy and trust/bootstrap policy that validate references and bounds without embedding secrets. Evidence: `schemas/snix-build-executor-policy.ncl`, `schemas/trust-bootstrap-policy.ncl`, `scripts/check-typed-nickel-contract-fixtures.py`.
- [x] I11 Deepen crate-extraction policy contracts for readiness state, dependency rails, publication metadata, no-std/alloc/std class, and required evidence. Evidence: `docs/crate-extraction/policy.ncl`, `openspec/changes/type-nickel-contract-boundaries/evidence/crate-extraction-readiness.md`.

## Phase 4: Verification

- [x] V1 Run `openspec validate type-nickel-contract-boundaries --strict --json`. Evidence: `openspec/changes/type-nickel-contract-boundaries/evidence/nickel-authored-contracts-verification.md`.
- [x] V2 Run Nickel typecheck/export checks for every touched `.ncl` file and generated contract. Evidence: `scripts/check-typed-nickel-contract-fixtures.py`, `openspec/changes/type-nickel-contract-boundaries/evidence/nickel-authored-contracts-verification.md`.
- [x] V3 Run generator freshness checks and prove a negative mutation is detected. Evidence: `scripts/generate-typed-nickel-contracts.py --check`, fixture negative exports in `scripts/check-typed-nickel-contract-fixtures.py`, `openspec/changes/type-nickel-contract-boundaries/evidence/rust-derived-receipt-contracts-verification.md`.
- [x] V4 Run focused Rust round-trip/schema tests for Rust-derived contract families. Evidence: `cargo test -p aspen-ci test_deploy_protocol_schema_snapshot`, `cargo nextest run -p aspen-ci test_deploy_protocol_schema_snapshot`, `cargo test -p aspen-client-api ci_receipt_schema_and_status_labels_are_documented --features ci`.
- [x] V5 Run `git diff --check`. Evidence: final verification command output.
