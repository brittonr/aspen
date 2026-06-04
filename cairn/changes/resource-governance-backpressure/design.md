## Context

Policy gates decide whether actions are allowed, but Molten also needs to decide how much work is allowed. Resource governance turns operational limits into first-class policy and evidence. Backpressure prevents overload from turning into nondeterministic failure.

## Goals

- Represent resource grants and consumption canonically.
- Enforce deterministic local backpressure rules.
- Bound actor turns, mailboxes, assertions, memory, storage, blobs, network, remote sync, and adapter effects.
- Make Wasm, Steel, native actors, and job stages subject to comparable budgets.
- Emit traces and receipts for operational decisions.
- Allow policies to grant, attenuate, revoke, and renew resource budgets.

## Non-Goals

- Do not promise perfect CPU accounting for every native Rust operation in the first milestone.
- Do not let resource grants imply data access authority.
- Do not rely on OS scheduler fairness as semantic fairness.
- Do not silently drop messages without traceable policy.

## Resource model

Resource grant records should include:

- subject identity and scope,
- resource kind,
- amount/rate/burst/window,
- start/expiry/revocation refs,
- parent grant or quota pool,
- policy refs,
- evidence refs.

Resource kinds include turn budget, CPU/fuel, memory, mailbox slots, assertion count, blob bytes, storage bytes, network bytes/messages, remote fetch count, effect request count, trace bytes, and job-stage slots.

## Backpressure semantics

When a resource would exceed its limit, the runtime chooses a deterministic admitted response:

- enqueue within bounded queue,
- reject with denial receipt,
- suspend/delay using logical time,
- request budget renewal,
- cancel lower-priority work,
- signal supervisor,
- shed load according to declared policy.

Queue ordering and shedding rules must be canonical and traceable.

## Execution budgets

Wasmtime actors use fuel/epoch/deadline mechanisms where available. Steel scripts and native actor turns use cooperative operation budgets and runtime checkpoints. Long-running work must yield at deterministic boundaries or be denied/cancelled.

## Fairness

Fairness is implemented by deterministic scheduler policy, not OS timing. Scheduler keys may include budget class, priority, logical time, and sequence. Starvation prevention should be policy-defined and replayable.

## Integration

Effect manifests declare resource requirements. Handler profiles may have stricter limits for tests. Job DAG planners use resource budgets for placement and fusion. Remote peers apply local quotas before accepting sync or execution. Supervision receives overload/failure assertions.

## Open Questions

- Which resource units are stable enough for first receipts: abstract turns/fuel/bytes/messages rather than wall-clock CPU?
- How should budget renewal interact with capabilities and revocation?
- Should priority inversion be handled in scheduler policy or supervision policy?
