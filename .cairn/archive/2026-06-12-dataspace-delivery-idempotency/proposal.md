## Why

Remote dataspace envelopes and job worker messages need replay safety across retries, reconnects, duplicate deliveries, and partial failures. Molten already records deterministic delivery logs, but it lacks a general operation-id, dedup, sequence-window, retry, and causal replay contract across local and remote SAM traffic.

## What Changes

- Add canonical operation id, delivery class, dedup ledger, sequence window, retry, and replay-protection records.
- Bind idempotency evidence into local dataspace turns, remote dataspace transport receipts, protocol messages, service lifecycle events, and job worker requests/results.
- Deduplicate at the actor/session/service/protocol scope before committing side effects.
- Emit receipts for first delivery, duplicate replay, stale sequence, gap, retry, and denial.

## Impact

This makes remote communication safe under at-least-once transport and restart/recovery. It is a prerequisite for production Iroh gossip/docs use, job workers, protocol sessions, and service supervision.
