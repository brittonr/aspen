## Context

Distributed systems rarely deliver exactly once. Molten should avoid pretending otherwise. It can provide explicit at-most-once, at-least-once, idempotent, and transactional semantics at the runtime boundary, with canonical receipts and replay protection.

## Goals

- Make delivery guarantees explicit per operation kind.
- Prevent duplicate effect commits through idempotency keys and dedup ledgers.
- Bound replay using sequence windows, session ids, operation ids, and capability/policy refs.
- Make retries deterministic and auditable.
- Support choreography sequence checks and storage mutation dedup.

## Non-Goals

- Do not claim network-level exactly-once delivery.
- Do not make all operations idempotent automatically.
- Do not use Raft for ordinary actor message delivery.
- Do not silently ignore duplicates without evidence.

## Operation identity

Side-effecting operations should include:

- `operation_id`: canonical idempotency key scoped to actor/session/capability.
- `session_id`: protocol/job/transcript/replay scope.
- `sequence`: monotonic per sender/session where applicable.
- `cause`: triggering turn/envelope/effect id.
- `target`: actor/service/storage namespace/remote peer.
- `effect kind` and canonical request hash.
- policy/capability refs.

Operation identity is part of receipts and dedup ledgers.

## Delivery classes

This completed slice distinguishes the concrete evidence outcomes implemented by the delivery idempotency module:

- `first`: the next scoped sequence is admitted and the caller may commit the covered side effect.
- `duplicate`: the same operation/evidence returns the prior receipt or semantic result and suppresses a second side effect.
- `conflict`, `stale`, and `gap`: the delivery is denied before side effects.
- `retry`: a deterministic retry receipt points at the expected sequence boundary before side effects.
- lifecycle failure traces separately disclose `one_shot_external` effects without pretending they were rolled back.

Richer `ephemeral`, `transactional`, `compensating`, or manifest-declared delivery classes remain future extensions unless a specific effect manifest or handler profile admits them.

## Dedup and replay windows

Dedup ledgers track accepted operation ids, request hashes, response hashes, receipt refs, expiry, and storage/session scope. Replay windows reject stale, future, or duplicate sequences unless policy admits recovery. Duplicate with same request hash can return previous receipt; duplicate with conflicting request hash is a safety error.

## Retries and timeouts

Retries are runtime actions with their own traces. Retry schedules use logical time in deterministic profiles and recorded observations in record/replay profiles. Timeout does not imply remote non-execution; callers must rely on idempotency keys or reconciliation.

## Integration

Remote dataspace and node-control ingress use scoped operation ids and idempotency receipts before committing local side effects or durable enqueue operations. Coordination/control-plane commands derive operation ids for duplicate replay. Job DAG stages carry stage operation ids and memo keys as evidence, while worker execution still requires separate authority/provenance/source-gate checks. Choreography op-index integration, typed storage mutation dedup, remote artifact sync install dedup, and upgrade storage-migration idempotency are future explicit extensions; generic delivery receipts do not grant those subsystems authority or exactly-once semantics.

## Open Questions

- Which remaining subsystems should get first-class delivery idempotency next: typed storage writes, choreography op indices, remote artifact install, or upgrade storage migrations?
- Which dedup ledgers should stay local Redb and which should move to Raft-backed control-plane state?
- What is the default retention window per future manifest-declared delivery class?
