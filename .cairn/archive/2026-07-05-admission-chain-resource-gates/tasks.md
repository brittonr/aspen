# Tasks: admission-chain-resource-gates

## Phase 1: Admission chain model

- [x] [serial] r[molten.resource_admission.ordered_chain_receipts] Define pure admission inputs, ordered phase results, denial diagnostics, and canonical admission receipt records for create, update, status, delete, and reconcile-apply intents.
- [x] [parallel] r[molten.resource_admission.ordered_chain_receipts] Add positive fixtures for admitted create/update/delete plans and negative fixtures for malformed resources, missing policy refs, wrong scope, and out-of-order phase evidence.

## Phase 2: Defaulting, mutation, and status isolation

- [x] [serial] r[molten.resource_admission.mutation_requires_reviewed_rule] Implement deterministic defaulting and mutation validation that requires reviewed rule refs and records pre/post candidate refs.
- [x] [parallel] r[molten.resource_admission.mutation_requires_reviewed_rule] Add positive fixtures for reviewed defaulting/mutation and negative fixtures for unreviewed mutation, non-deterministic mutation claims, and changed authority-bearing metadata without rule evidence.
- [x] [serial] r[molten.resource_admission.status_subresource_isolated] Implement status-operation validation that permits condition/observed-state updates while denying desired-state changes.
- [x] [parallel] r[molten.resource_admission.status_subresource_isolated] Add positive status fixtures and negative fixtures for stale generation, desired-ref mutation, finalizer mutation, and missing observation evidence.

## Phase 3: Documentation and validation

- [x] [serial] r[molten.resource_admission.ordered_chain_receipts] Document the admission chain as Kubernetes-inspired but not webhook-compatible (embedded in DTO types and spec delta), and ran focused admission tests (8 positive/negative tests pass) plus `cairn validate --root .` (valid).
