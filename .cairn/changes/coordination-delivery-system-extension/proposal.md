## Why

Molten's base coordination primitive intentionally supports deterministic FIFO enqueue and destructive dequeue only. Visibility leases, acknowledgements, retries, delivery attempts, dead-letter queues, and redrive are policy-bearing delivery semantics and must not silently change the existing queue contract or become the default actor mailbox.

A separate system extension is needed to provide durable work-delivery behavior over the consistency, durable-state, time, fencing, resource, and evidence ports.

## What Changes

- Add a versioned coordination-delivery extension manifest and records distinct from the base FIFO queue schema.
- Add pure transitions for claim, visibility lease, ack, nack, lease expiry, attempt accounting, retry eligibility, dead-lettering, and authorized redrive.
- Use admitted logical time, operation ids, fencing tokens, and consistency currentness rather than wall-clock time or local stale reads.
- Keep payload bytes outside coordination state by storing canonical content refs and bounded delivery metadata.
- Make retry, backoff, maximum attempts, DLQ, redrive, ordering, and retention policy explicit and versioned.
- Add deterministic simulation and multiprocess tests for duplicate delivery, crash-after-claim, stale acknowledgement, partition, restart, expiry, and redrive.

## Impact

- **Files**: coordination delivery models and extension host, consistency/durable/time bindings, queue schemas, worker scheduling integration, operator readback, fixtures, and `cairn/specs/coordination/spec.md`.
- **Testing**: positive claim/ack and retry/DLQ flows plus negative stale token, wrong owner, duplicate ack, over-attempt, missing authority, local-stale read, and payload-inline tests.
- **Safety**: the extension provides bounded at-least-once-style delivery evidence; it does not claim exact-once effects, global ordering, transactionality with external systems, or authority from queue possession.
- **Licensing**: Aspen `main` queue semantics are requirements prior art only unless an explicit compatible license covers implementation reuse.
