# Tasks: distributed-simulation-fault-plans

## Phase 1: Model and schema

- [ ] [serial] r[molten.testing.distributed_simulation.fault_plan_schema] Define canonical distributed topology, scheduler profile, seed, and fault-plan records.
- [ ] [serial] r[molten.testing.distributed_simulation.simulator_core] Add a pure deterministic simulation core over virtual time, explicit queues, peer state, and fault events.

## Phase 2: Receipts and invariants

- [ ] [parallel] r[molten.testing.distributed_simulation.run_receipts] Emit parseable distributed test run receipts with topology, seed, fault-plan, child evidence, decisions, diagnostics, replay status, and allowed variance refs.
- [ ] [parallel] r[molten.testing.distributed_simulation.property_invariants] Add Hegel/property or model tests for idempotency, transport-authority separation, deny-before-side-effects, and restart replay stability.

## Phase 3: Fixtures, validation, and docs

- [ ] [parallel] r[molten.testing.distributed_simulation.fixtures] Add positive and negative simulation fixtures for delay, drop, duplicate, reorder, partition, rejoin, stale evidence, and ambient-state drift.
- [ ] [serial] r[molten.testing.distributed_simulation.docs] Document how simulation evidence fits between unit tests, VM checks, and live soak evidence.
