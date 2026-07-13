## Context

The existing coordination queue removes the head at dequeue. Production work delivery requires a claim period and terminal acknowledgement, but adding those fields to the base queue would change its state machine and replay identity. This change introduces an explicit higher-level service.

## Decisions

### 1. Base FIFO and durable delivery remain separate contracts

**Choice:** Preserve the existing queue schema and enqueue/dequeue semantics. Add a versioned delivery extension with claim, ack, nack, expiry, retry, DLQ, and redrive operations.

**Rationale:** Existing callers and receipts must not silently acquire new timing, retention, or failure semantics.

### 2. Delivery transitions are pure and deterministic

**Choice:** The pure state machine consumes queue state, operation-id history, logical-time facts, policy, fencing/currentness evidence, and a request. It returns candidate state, outputs, effects, and receipt facts without I/O.

**Rationale:** Crash, retry, and stale-token behavior must be testable without a running consensus group.

### 3. Visibility is an admitted logical-time lease

**Choice:** Claims bind item ref, delivery id, consumer, attempt, fencing token, visibility deadline, consistency epoch, and service generation. Only admitted fabric time can create or expire a lease.

**Rationale:** Wall-clock reads and local timeout tasks would make replay and currentness ambiguous.

### 4. Ack and nack are fenced mutations

**Choice:** Ack, nack, extension, and redrive require the current delivery token, owner or delegated authority, operation id, and linearizable or equivalent currentness evidence. Stale, duplicate-conflicting, wrong-owner, expired, or prior-epoch tokens deny without mutation.

**Rationale:** Receiving an item ref does not authorize completion or queue mutation.

### 5. Retry and DLQ policy is explicit

**Choice:** Policy declares maximum attempts, retry classes, logical backoff schedule, ordering posture, DLQ target, retention, redrive authority, and poison-item handling. Unsupported or missing policy denies rather than choosing defaults.

**Rationale:** These choices are application semantics and operational risk controls.

### 6. Payloads remain outside consistency state

**Choice:** Delivery records contain bounded metadata and canonical payload refs. Content bytes remain in admitted content storage and are fetched only after execution admission.

**Rationale:** Consensus and queue logs must not become the ordinary large-payload path.

## Functional core / imperative shell split

- Pure core: claim/ack/nack/expiry/retry/DLQ/redrive transitions, attempt accounting, token/currentness validation, idempotency, ordering policy, state refs, and receipt payloads.
- Shell: propose transitions through consistency ports, persist state, arm logical timers, resolve payload refs, publish status assertions, invoke workers only after separate execution admission, and emit evidence.

## Dependencies

- System-extension runtime.
- Fabric consistency, durable-state, time, resource, observability, and simulation profiles.
- Existing coordination and delivery-idempotency primitives.

## Risks / Trade-offs

- At-least-once delivery can duplicate external effects. Require worker-level idempotency and retain explicit non-claims.
- Strict FIFO with retries can block progress. Ordering posture is explicit per queue profile.
- DLQs can become unreviewed retention sinks. Apply capacity, retention, visibility, and redrive policy.
