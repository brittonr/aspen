## Why

Molten will deliver envelopes through local dataspaces, Iroh gossip/docs/blobs, choreography sessions, remote execution, typed storage handlers, and job DAG stages. Retries, duplicate messages, partial failures, and replay can otherwise cause duplicated effects or inconsistent state.

## What Changes

- Define delivery semantics for remote dataspace/node-control ingress, control-plane commands, job worker operation refs, and replay-bound diagnostics.
- Add canonical idempotency keys, dedup windows, sequence numbers, scoped causal intent, and replay bounds.
- Represent completed delivery outcomes as explicit evidence: first commit, duplicate suppression, conflict/stale/gap denial, retry-before-side-effects, and one-shot disclosure in lifecycle failure traces.
- Emit receipts for operation ids, delivery windows, dedup entries, accepted commits, dedup hits, conflicts, stale/gap rejections, and retry guidance.
- Document that typed storage mutation dedup, rich queue ack/nack/DLQ policies, choreography op-index integration, and upgrade/storage migration idempotency are future explicit extensions and are not implied by generic delivery receipts.

## Impact

This prevents retries and duplicated deliveries from duplicating covered side effects. The completed milestone adds idempotency keys and dedup windows for remote dataspace/node-control ingress plus reusable operation-id evidence for control-plane and job worker paths, while preserving evidence-only boundaries for authority, transport, provenance, policy, resource, source-gate, storage, choreography, and upgrade trust.
