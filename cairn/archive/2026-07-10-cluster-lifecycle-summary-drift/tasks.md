# Tasks: cluster-lifecycle-summary-drift

## Phase 1: Summary core

- [x] [serial] r[molten.testing.cluster_lifecycle_summary_drift.receipt_summary] Define a pure cluster lifecycle drift summary over canonical lifecycle receipt fields.
- [x] [parallel] r[molten.testing.cluster_lifecycle_summary_drift.negatives] Add diagnostics for changed child refs, node ordering drift, field-kind drift, undeclared volatile fields, ambient state, and rendered-output-only success.

## Phase 2: Rerun gate

- [x] [serial] r[molten.testing.cluster_lifecycle_summary_drift.receipt_summary] Add a focused command or Nix check that runs equivalent lifecycle workflows in two fresh roots and compares summaries.
- [x] [parallel] r[molten.testing.cluster_lifecycle_summary_drift.negatives] Add positive fixtures for stable lifecycle and already-running paths plus negative fixtures for semantic drift and stale variance declarations.

## Phase 3: Validation

- [x] [parallel] r[molten.testing.cluster_lifecycle_summary_drift.receipt_summary] Document the summary fields and allowed variance reasons.
- [x] [serial] r[molten.testing.cluster_lifecycle_summary_drift.negatives] Run focused drift tests and update traceability coverage.
