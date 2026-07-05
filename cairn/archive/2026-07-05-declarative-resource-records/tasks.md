# Tasks: declarative-resource-records

## Phase 1: Resource DTOs and validation

- [x] [serial] r[molten.resource_model.canonical_resource_records] Define pure resource identity and metadata DTOs with canonical Preserves encoding, BLAKE3 identity refs, scope refs, generation, desired refs, observed refs, labels, annotations, and evidence refs.
- [x] [parallel] r[molten.resource_model.canonical_resource_records] Add positive fixtures for valid scoped resources and negative fixtures for malformed refs, duplicate names in scope, invalid labels, and non-canonical identity bytes.

## Phase 2: Status and lifecycle metadata

- [x] [serial] r[molten.resource_model.status_conditions_observed_generation] Implement status condition transition validation with observed generation, reason, severity, message, and evidence refs.
- [x] [parallel] r[molten.resource_model.status_conditions_observed_generation] Add positive condition-update fixtures and negative fixtures for stale observed generation, missing evidence refs, and condition updates that mutate desired state.
- [x] [serial] r[molten.resource_model.owner_refs_finalizers_gc] Implement owner-ref and finalizer eligibility checks for deletion and GC plans.
- [x] [parallel] r[molten.resource_model.owner_refs_finalizers_gc] Add positive deletion-ready fixtures and negative fixtures for missing finalizer receipts, live owners, dangling pins, and unauthorized GC authority.

## Phase 3: Documentation and validation

- [x] [serial] r[molten.resource_model.canonical_resource_records] Document the Molten resource model as Kubernetes-inspired but non-compatible (embedded in code docs, spec delta, and DTOs), and ran focused resource-model tests (18 positive/negative tests pass) plus `cairn validate --root .` (valid).
