## Phase 1: Coordination model

- [x] [serial] r[molten.coordination.primitive_model] Define canonical DTOs for admitted initial primitives: locks, elections, queues, semaphores, rate limiters, barriers, and service registry.
- [x] [serial] r[molten.coordination.dataspace_interface] Define dataspace assertions for coordination demand, grants, denials, expiry, release, and service readiness.
- [x] [parallel] r[molten.coordination.no_default_mailbox] Document and enforce that coordination queues do not replace ordinary actor mailboxes or dataspace traffic.
- [x] [parallel] r[molten.coordination.receipts] Emit receipts for acquire, grant, deny, release, enqueue, dequeue, read, duplicate replay, capacity denial, and cleanup-relevant outcomes.

## Phase 2: First primitives

- [x] [serial] r[molten.coordination.lock_fencing] Specify and implement deterministic lock/election grants with monotonic fencing tokens, owner binding, duplicate replay, and stale-token denial.
- [x] [serial] r[molten.coordination.queue_visibility] Specify and implement explicit FIFO queues with enqueue/dequeue, duplicate replay, capacity/resource denial, and status assertions.
- [x] [parallel] r[molten.coordination.service_registry] Specify service registry assertions for demand, ready, health, exposed refs, and discovery.
- [x] [parallel] r[molten.coordination.rate_limit_semaphore] Specify semaphore and rate-limiter semantics with resource governance integration.

## Phase 3: Backends and verification

- [x] [serial] r[molten.coordination.raft_contract] Define Raft/control-plane backend contract for linearizable coordination state.
- [x] [parallel] r[molten.coordination.replay_backend] Add deterministic replay through operation-id prior receipt reuse and recorded evidence validation.
- [x] [parallel] r[molten.coordination.trellis_predicates] Add bounded checks/properties for fencing monotonicity, mutual exclusion, queue FIFO behavior, and no-stale-token acceptance.
- [x] [parallel] r[molten.coordination.integration_tests] Add tests where coordination requests publish status assertions and observers inspect coordination results.

## Phase 4: Property tests

- [x] [serial] r[molten.coordination.lock_tests] Add tests for mutual exclusion, fencing token monotonicity, duplicate replay, release, and stale token rejection.
- [x] [serial] r[molten.coordination.queue_tests] Add tests for FIFO ordering, duplicate enqueue replay, overflow/resource denial, and explicit queue status assertions.
- [x] [parallel] r[molten.coordination.no_raft_ordinary_traffic_tests] Add tests or static checks that ordinary actor messages do not route through coordination/Raft by default.
- [x] [parallel] r[molten.coordination.property_tests] Add Hegel property tests for mutual exclusion, queue delivery invariants, and deterministic backend replay.
