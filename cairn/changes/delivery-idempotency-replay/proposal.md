## Why

Molten will deliver envelopes through local dataspaces, Iroh gossip/docs/blobs, choreography sessions, remote execution, typed storage handlers, and job DAG stages. Retries, duplicate messages, partial failures, and replay can otherwise cause duplicated effects or inconsistent state.

## What Changes

- Define delivery semantics for messages, assertions, effect requests, storage mutations, remote sync, and choreography operations.
- Add canonical idempotency keys, dedup windows, sequence numbers, causal links, and replay bounds.
- Require side-effecting operations to declare whether they are idempotent, transactional, compensating, or one-shot.
- Emit receipts for delivery attempt, dedup hit, accepted commit, retry, timeout, cancellation, and replay rejection.
- Integrate delivery/idempotency with deterministic playback, effect handlers, remote sync, storage, choreography, and upgrade sessions.

## Impact

This prevents retries and duplicated deliveries from duplicating external side effects. The first milestone can add idempotency keys and dedup windows to local dataspace messages and effect requests.
