# Tasks: cross-node-state-reconciliation-gate

## Phase 1: Reconciliation model

- [x] [parallel] r[molten.testing.multinode.cross_node_reconciliation_gate] Define a pure reconciliation input model for node summaries, topology ref, scenario fixture ref, required receipt refs, expected equality classes, allowed variance refs, and diagnostics.
- [x] [parallel] r[molten.testing.multinode.reconciliation_deny_drift] Define negative diagnostics for missing node evidence, stale ref, wrong topology, duplicate semantic commit, divergent queue, divergent ledger, undeclared variance, and log-only reconciliation.

## Phase 2: Gate implementation and fixtures

- [x] [serial] r[molten.testing.multinode.cross_node_reconciliation_gate] Implement reconciliation receipt construction and positive fixtures for converged simulation, local multiprocess, and VM evidence summaries.
- [x] [serial] r[molten.testing.multinode.reconciliation_deny_drift] Add negative fixtures proving drift and missing bindings deny before pass evidence.

## Phase 3: Documentation and validation

- [x] [parallel] r[molten.testing.multinode.cross_node_reconciliation_gate] Document expected equality classes and allowed variance declarations for multinode reconciliation.
- [x] [serial] r[molten.testing.multinode.reconciliation_deny_drift] Run focused reconciliation tests and `cairn validate --root .`, or record the blocker and next best check.
