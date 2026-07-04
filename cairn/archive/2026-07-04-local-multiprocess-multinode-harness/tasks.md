# Tasks: local-multiprocess-multinode-harness

## Phase 1: Harness plan core

- [x] [parallel] r[molten.testing.multinode.local_multiprocess_harness] Define a pure local multiprocess plan model for node identities, state roots, local transport handles, commands, receipt expectations, and cleanup policy.
- [x] [parallel] r[molten.testing.multinode.process_isolation_cleanup] Add validation that rejects colliding state roots, colliding transport handles, missing cleanup policy, and missing expected receipt bindings.

## Phase 2: Imperative shell and fixtures

- [x] [serial] r[molten.testing.multinode.local_multiprocess_harness] Implement a thin shell that spawns isolated `molten node` processes, runs a cross-process control workflow, collects receipts, and emits a local multiprocess run receipt.
- [x] [serial] r[molten.testing.multinode.process_isolation_cleanup] Add crash, stale ticket, missing receipt, and orphaned-state negative fixtures with cleanup assertions.

## Phase 3: Evidence and validation

- [x] [parallel] r[molten.testing.multinode.local_multiprocess_harness] Document the local harness evidence scope and how it differs from deterministic simulation and NixOS VM evidence.
- [x] [serial] r[molten.testing.multinode.process_isolation_cleanup] Run focused local harness tests and `cairn validate --root .`, or record the blocker and next best check.
