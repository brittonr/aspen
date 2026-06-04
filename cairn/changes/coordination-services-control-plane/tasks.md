## Phase 1: Coordination records

- [x] [serial] r[molten.coordination_services_control_plane.manifest] Define coordination service manifest, request, receipt, fencing token, state snapshot, and status assertion DTOs.
- [x] [serial] r[molten.coordination_services_control_plane.operation_ids] Require operation-id/idempotency refs for mutating coordination requests.
- [x] [parallel] r[molten.coordination_services_control_plane.ledger_catalog] Classify coordination artifacts in ledger/catalog/MCP views.
- [x] [parallel] r[molten.coordination_services_control_plane.schema_constants] Export schema constants for coordination records.

## Phase 2: Raft-backed primitives

- [x] [serial] r[molten.coordination_services_control_plane.lock_fencing] Implement lock/lease acquire/release with monotonic fencing tokens and stale-token denial.
- [x] [serial] r[molten.coordination_services_control_plane.queue_semaphore] Implement FIFO queue and semaphore operations with capacity/resource denial receipts.
- [x] [parallel] r[molten.coordination_services_control_plane.rate_election_barrier] Implement rate-limit, election, and barrier primitives with deterministic state transitions.
- [x] [parallel] r[molten.coordination_services_control_plane.service_registry] Implement service registry pointer updates over the control-plane registry.

## Phase 3: Gates and dataspace reflection

- [x] [serial] r[molten.coordination_services_control_plane.authority_resource] Gate all operations through authority, policy, resource, idempotency, and Raft read/commit evidence.
- [x] [serial] r[molten.coordination_services_control_plane.status_assertions] Publish committed coordination outcomes as local dataspace assertions after apply.
- [x] [parallel] r[molten.coordination_services_control_plane.read_index] Serve coordination reads through read-index receipts by default.
- [x] [parallel] r[molten.coordination_services_control_plane.gc_retention] Pin active locks/tokens/queues/barriers against unsafe ledger GC.

## Phase 4: Tests

- [x] [serial] r[molten.coordination_services_control_plane.lock_tests] Test acquire/release, stale fencing token denial, duplicate idempotent acquire, and observer assertions.
- [x] [serial] r[molten.coordination_services_control_plane.queue_tests] Test FIFO dequeue, duplicate enqueue, queue overflow, and resource denial.
- [x] [parallel] r[molten.coordination_services_control_plane.service_registry_tests] Test service registry updates and read-index reads.
- [x] [parallel] r[molten.coordination_services_control_plane.property_tests] Add Hegel properties for fencing monotonicity, FIFO ordering, semaphore bounds, and no-actor-traffic invariant.
