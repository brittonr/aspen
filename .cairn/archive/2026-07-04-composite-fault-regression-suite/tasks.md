# Tasks: composite-fault-regression-suite

## Phase 1: Suite and promotion core

- [x] [parallel] r[molten.testing.distributed_simulation.composite_fault_regression_suite] Define a pure composite fault case model for case id, seed, topology, scheduler, fault plan, commands, invariant, replay status, expected decision, diagnostics, and caveats.
- [x] [parallel] r[molten.testing.distributed_simulation.generated_case_promotion_budget] Define promotion metadata for generated cases, including stable refs, profile eligibility, traceability coverage, retry policy, variance declarations, and cost budget.

## Phase 2: Fixture coverage

- [x] [serial] r[molten.testing.distributed_simulation.composite_fault_regression_suite] Add named fixtures for duplicate-after-restart, partition-with-stale-evidence, reorder-with-reconcile, crash-during-dispatch, and resource-pressure-during-quorum.
- [x] [serial] r[molten.testing.distributed_simulation.generated_case_promotion_budget] Add promotion readback or fixture metadata that records why each named case is included, deferred, or diagnostic-only.

## Phase 3: Positive and negative validation

- [x] [parallel] r[molten.testing.distributed_simulation.composite_fault_regression_suite] Add positive tests for deterministic replay and convergence where the composite case is expected to pass.
- [x] [parallel] r[molten.testing.distributed_simulation.composite_fault_regression_suite] Add negative tests for stale evidence, unauthorized transport, corrupted receipts, ambient drift, partitioned quorum, and resource pressure deny-before-side-effects behavior.
- [x] [serial] r[molten.testing.distributed_simulation.generated_case_promotion_budget] Run focused distributed simulation tests and the distributed CI gate, or record blockers and next best checks.
