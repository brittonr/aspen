# Tasks: service-supervision-restart-cleanup-proof

## Phase 1: Demand and dependency gates

- [x] [serial] r[molten.service_state_machine_proof.dependency_wait_no_start] Add tests proving missing or stale dependency readiness produces dependency-wait lifecycle evidence and no actor/service start side effects.
- [x] [parallel] r[molten.service_state_machine_proof.dependency_wait_no_start] Add positive tests proving ready dependencies and explicit evidence produce service lifecycle pass receipts and owned readiness assertions.

## Phase 2: Restart bounds and monitor order

- [x] [serial] r[molten.service_state_machine_proof.bounded_restart_trace] Add bounded restart trace tests covering restart within budget, budget exhaustion denial, and replay identity.
- [x] [parallel] r[molten.service_state_machine_proof.bounded_restart_trace] Add tests proving monitor notification refs are emitted in deterministic order and bind lifecycle failure refs.

## Phase 3: Cleanup proof

- [x] [serial] r[molten.service_state_machine_proof.cleanup_idempotence] Add tests proving service cleanup retracts owned assertions/resources and repeated cleanup leaves state unchanged.
- [x] [parallel] r[molten.service_state_machine_proof.cleanup_idempotence] Add negative tests for stale cleanup evidence, missing ownership refs, and cleanup attempts that would remove non-owned state.

## Phase 4: Validation

- [x] [serial] r[molten.service_state_machine_proof.dependency_wait_no_start] r[molten.service_state_machine_proof.bounded_restart_trace] r[molten.service_state_machine_proof.cleanup_idempotence] Add traceability evidence and run `cargo test service`.
