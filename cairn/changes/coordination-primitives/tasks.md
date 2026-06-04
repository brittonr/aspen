## Phase 1: Coordination model

- [ ] [serial] r[molten.coordination.primitive_model] Define canonical DTOs for locks, elections, queues, semaphores, rate limiters, counters/sequences, barriers, and service registry.
- [ ] [serial] r[molten.coordination.dataspace_interface] Define dataspace assertions for coordination demand, grants, denials, expiry, release, and service readiness.
- [ ] [parallel] r[molten.coordination.no_default_mailbox] Document and enforce that coordination queues do not replace ordinary actor mailboxes or dataspace traffic.
- [ ] [parallel] r[molten.coordination.receipts] Emit receipts for acquire, grant, deny, renew, release, expire, enqueue, dequeue, ack, nack, DLQ, and cleanup.

## Phase 2: First primitives

- [ ] [serial] r[molten.coordination.lock_fencing] Specify and implement local/mock distributed locks with monotonic fencing tokens and lease expiry.
- [ ] [serial] r[molten.coordination.queue_visibility] Specify and implement local/mock queues with visibility timeout, ack/nack, retry, and DLQ.
- [ ] [parallel] r[molten.coordination.service_registry] Specify service registry assertions for demand, ready, health, exposed refs, and discovery.
- [ ] [parallel] r[molten.coordination.rate_limit_semaphore] Specify semaphore and rate-limiter semantics with resource governance integration.

## Phase 3: Backends and verification

- [ ] [serial] r[molten.coordination.raft_contract] Define Raft/control-plane backend contract for linearizable coordination state.
- [ ] [parallel] r[molten.coordination.replay_backend] Add replay backend that injects recorded coordination decisions for deterministic playback.
- [ ] [parallel] r[molten.coordination.trellis_predicates] Add bounded predicates for fencing monotonicity, mutual exclusion, queue visibility, and no-stale-token acceptance.
- [ ] [parallel] r[molten.coordination.integration_tests] Add tests where actors request locks/queue work through dataspace assertions and observe coordination results.

## Phase 4: Property tests

- [ ] [serial] r[molten.coordination.lock_tests] Add tests for mutual exclusion, fencing token monotonicity, expiry, release, and stale token rejection.
- [ ] [serial] r[molten.coordination.queue_tests] Add tests for visibility timeout, ack/nack, retry, DLQ, and dedup operation ids.
- [ ] [parallel] r[molten.coordination.no_raft_ordinary_traffic_tests] Add tests or static checks that ordinary actor messages do not route through coordination/Raft by default.
- [ ] [parallel] r[molten.coordination.property_tests] Add Hegel property tests for mutual exclusion, queue delivery invariants, and deterministic backend replay.
