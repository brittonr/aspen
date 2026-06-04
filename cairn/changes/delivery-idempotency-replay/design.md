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

Molten should distinguish:

- `ephemeral`: best-effort notification, no durable retry.
- `deduped`: duplicate requests return prior result/receipt within a window.
- `transactional`: commit or rollback within a turn/store transaction.
- `compensating`: requires explicit compensating action for rollback-like behavior.
- `one_shot_external`: cannot be retried safely without operator/policy approval.

Effect manifests and handler bindings declare the delivery class.

## Dedup and replay windows

Dedup ledgers track accepted operation ids, request hashes, response hashes, receipt refs, expiry, and storage/session scope. Replay windows reject stale, future, or duplicate sequences unless policy admits recovery. Duplicate with same request hash can return previous receipt; duplicate with conflicting request hash is a safety error.

## Retries and timeouts

Retries are runtime actions with their own traces. Retry schedules use logical time in deterministic profiles and recorded observations in record/replay profiles. Timeout does not imply remote non-execution; callers must rely on idempotency keys or reconciliation.

## Integration

Choreography messages use protocol/session/op indices and reject out-of-state duplicates. Typed storage writes use operation ids to prevent duplicate mutations. Remote artifact sync fetch/install uses closure and operation ids. Job DAG stages use memo keys and stage operation ids. Upgrade tasks are idempotent where possible and record irreversible one-shot actions.

## Open Questions

- Which dedup ledgers should be local Redb first vs Raft-backed control-plane state?
- What is the default dedup window per operation class?
- How should one-shot external effects be represented in upgrade rollback plans?
