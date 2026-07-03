# Tasks: proof-obligation-decomposition

## Phase 1: Obligation DTOs

- [x] [serial] r[molten.testing.proof_obligations.manifest] Define proof obligation and aggregate manifest DTOs.
- [x] [serial] r[molten.testing.proof_obligations.classes] Implement initial obligation classes for validation, canonicalization, admission, mutation boundary, replay, and fail-closed behavior.
- [x] [serial] r[molten.testing.proof_obligations.aggregate_gate] Add aggregate proof validation that recomputes required children from explicit inputs.

## Phase 2: Reporting and integration

- [x] [parallel] r[molten.testing.proof_obligations.traceability] Allow traceability evidence to point at aggregate proof manifests.
- [x] [parallel] r[molten.testing.proof_obligations.operator_summary] Render grouped obligation summaries for review.

## Phase 3: Hegel RS and fixtures

- [x] [parallel] r[molten.testing.proof_obligations.hegel_properties] Add Hegel RS property tests for stable sorting, missing child denial, duplicate child denial, and subject mismatch denial.
- [x] [parallel] r[molten.testing.proof_obligations.fixtures] Add positive and negative fixtures for complete and incomplete obligation graphs.
- [x] [serial] r[molten.testing.proof_obligations.docs] Document how broad workflow claims decompose into child obligations.
