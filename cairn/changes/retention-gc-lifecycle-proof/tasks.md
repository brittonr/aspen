# Tasks: retention-gc-lifecycle-proof

## Phase 1: Lifecycle decision core

- [ ] [serial] r[molten.retention_gc_lifecycle_proof.ordered_chain] Define a pure plan/apply/execute/audit lifecycle validator over parsed retention GC evidence.
- [ ] [parallel] r[molten.retention_gc_lifecycle_proof.drift_no_mutation] Add explicit no-mutation evidence for drift, denied recomputation, and missing admission cases.
- [ ] [parallel] r[molten.retention_gc_lifecycle_proof.execution_scope] Harden exact execution scope checks for subsystem, object, action, class, receipt, and tombstone refs.

## Phase 2: Positive and negative tests

- [ ] [parallel] r[molten.retention_gc_lifecycle_proof.ordered_chain] Add a passing plan→apply→execute→audit proof trace.
- [ ] [parallel] r[molten.retention_gc_lifecycle_proof.drift_no_mutation] Add negative tests for plan drift, missing authority, incomplete reference index, stale remote clearance, and missing apply ref.
- [ ] [parallel] r[molten.retention_gc_lifecycle_proof.execution_scope] Add negative tests for apply/execution scope mismatch and missing tombstone evidence.

## Phase 3: Evidence and validation

- [ ] [serial] r[molten.retention_gc_lifecycle_proof.ordered_chain] r[molten.retention_gc_lifecycle_proof.drift_no_mutation] r[molten.retention_gc_lifecycle_proof.execution_scope] Bind canonical proof refs and run `cargo test retention`.
