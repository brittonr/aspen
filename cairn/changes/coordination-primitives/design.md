## Context

Aspen implements distributed locks, leader election, barriers, semaphores, counters, rate limiters, queues, service registry, and worker coordination over CAS/Raft. Molten needs similar functionality, but actors should still collaborate through dataspaces by default. Coordination primitives are services with stronger consistency requirements, not the whole runtime.

## Goals

- Provide strongly consistent coordination only where needed.
- Represent coordination demand, grants, denials, and state changes as dataspace assertions and receipts.
- Use fencing tokens for external side effects guarded by locks/leases.
- Make queues explicit work-distribution primitives with visibility timeout, ack/nack, and DLQ.
- Support deterministic local/mock handlers for tests and Raft-backed handlers for production control plane.
- Preserve no-Raft-for-ordinary-actor-traffic rule.

## Non-Goals

- Do not replace actor mailboxes with Raft queues.
- Do not require locks for normal actor collaboration.
- Do not make every dataspace assertion strongly consistent.
- Do not expose coordination grants without capabilities, resource budgets, and receipts.

## Primitive set

Initial primitives:

- `DistributedLock` with fencing token and lease.
- `LeaderElection` with lease renewal and fencing token.
- `Queue` with enqueue, dequeue, visibility timeout, ack, nack, retry, DLQ.
- `Semaphore` with bounded permits.
- `RateLimiter` with token bucket or fixed window.
- `Counter` and `SequenceGenerator` for monotonic ids.
- `Barrier` for N-party synchronization.
- `ServiceRegistry` for service instance registration, readiness, health, and discovery.

## Dataspace interface

Actors interact by assertions/effects, for example:

```text
<NeedLock resource requester lease_policy>
<LockHeld resource requester fencing_token expiry evidence>
<QueueItemAvailable queue item_id>
<ServiceReady service instance ref health>
```

The coordination service owns the linearizable backend and publishes admitted results. Retractions reflect expiry, release, revocation, failure, or cleanup.

## Fencing tokens

Any lock/lease/election grant that may guard external side effects carries a monotonically increasing fencing token. External adapters should reject stale tokens where possible. Tokens are receipt-backed and scoped to a resource.

## Queues

Queues are for explicit work distribution, not default actor mailboxes. Queue items have operation ids, visibility timeout, ack/nack, retry count, DLQ reason, and payload refs. Delivery/idempotency rules apply.

## Backend modes

- `local`: deterministic in-process for tests.
- `mock`: scripted grants/denials for transcripts.
- `raft_control_plane`: linearizable control-plane state.
- `replay`: inject recorded coordination decisions.

## Open Questions

- Which primitive should be implemented first after locks: queue or service registry?
- Should fencing token monotonicity be proved with Trellis/Verus-style predicates?
- Which coordination state belongs in Raft vs Redb local metadata?
