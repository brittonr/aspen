# Coordination delivery extension

## Purpose

The coordination delivery extension adds durable work claims, acknowledgements,
retries, dead-letter handling, and redrive. It does not change the accepted base
FIFO enqueue and dequeue contract.

The extension provides bounded at-least-once delivery facts. It does not prove
that an external effect occurred once.

## Architecture

The pure core is in `crates/molten-core/src/coordination_delivery/`. It owns:

- the exact delivery policy and manifest;
- canonical item, token, attempt, state, transition, and status values;
- deterministic claim, ack, nack, lease extension, expiry, retry, DLQ, redrive,
  and retention transitions;
- BLAKE3 identities; and
- worker-admission and operator-status projections.

The shell is in `src/coordination_delivery/`. It owns:

- capability-rooted Redb storage;
- compare-and-commit execution;
- unknown-outcome readback without blind retry;
- logical timer requests;
- bounded status publication;
- restart and multiprocess fixtures; and
- canonical Preserves receipts.

The shell consumes exact system-extension port-binding references for
consistency, durable state, logical time, resources, and observability. It does
not load those capabilities from ambient process state.

## Profile

`config/coordination-delivery/profile.ncl` is the reviewed source. The generated
projection is `config/coordination-delivery/generated/profile.json`.

The current profile binds:

- strict FIFO ordering;
- logical time only;
- fixed retry delay with no jitter;
- explicit attempt, ready, in-flight, retry, DLQ, metadata, and status bounds;
- separate completion, expiry, redrive, and retention authority references;
- retryable and poison failure classes; and
- the complete non-claim set.

The policy identity is
`blake3:05be03f3c3a2af25a8ba2f4f603b205b8105abdc15c61b4438518370b5e09d8a`.

Seven negative Nickel fixtures reject zero attempts, wall-clock expiry, inline
payloads, missing non-claims, capacity growth, retry jitter, and receipt-based
authority.

## Transition rules

A claim moves one eligible item to in-flight state. The token binds the queue,
item, consumer, attempt, cycle, fencing token, logical deadline, consistency
epoch, service generation, and policy.

Ack, nack, and extension require the current token. A different consumer also
needs the exact delegated completion authority reference.

Expiry needs the exact expiry authority reference and admitted logical time at
or after the deadline. A process timer or local-stale read cannot expire work.

Retry uses the accepted fabric-time retry planner. Attempt exhaustion and poison
classification move an item to the bounded DLQ. Redrive starts a new cycle but
retains prior attempt history.

DLQ cleanup needs a separate retention authority reference. A retention timer
or receipt does not authorize deletion by itself.

## Payload boundary

Delivery state contains content and metadata references only. It does not hold
large payload bytes or executables.

A claim does not authorize worker effects. The worker plan requires separate
content, provenance, authority, policy, resource, execution, and evidence facts.
Even an admitted worker plan does not claim that an external effect is exactly
once.

## Recovery and uncertainty

The local adapter stores one canonical state per queue. It compares the expected
state identity and revision inside one Redb transaction.

If commit outcome is unknown, the service reads the queue again. It classifies
the result as applied, not applied, or still unknown. It does not repeat the
commit blindly.

Timer and status failures do not rewrite a durable coordination commit. Their
failed or unknown observations stay explicit for operator reconciliation.

## Verification

Focused verification covers:

- policy, manifest, identity, and state validation;
- enqueue, claim, ack, delegated completion, nack, extension, expiry, retry,
  DLQ, redrive, cleanup, and duplicate replay;
- stale currentness, wrong owner, token drift, expired ack, unsupported failure,
  metadata overflow, and missing worker admission;
- restart and capability-rooted Redb reopen;
- unknown-before and unknown-after commit reconciliation;
- timer failure after durable commit;
- deterministic crash, partition, duplicate, authority-revocation, and resource
  fault inputs through existing simulation fault classes; and
- a real child-process stale-token and durable-recovery fixture.

The focused Octet and Nix checks inspect only this extension and its declared
boundary. Broad repository findings remain separate evidence.

## Non-claims

A delivery transition or receipt does not:

- grant authority;
- prove exactly-once external effects;
- prove payload correctness;
- prove global ordering;
- prove the store, broker, clock, or worker correct; or
- establish release eligibility.
