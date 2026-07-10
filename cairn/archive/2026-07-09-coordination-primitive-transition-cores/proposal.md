## Why

Coordination primitives already have deterministic receipts, property tests, and generated traces, but each primitive's semantic transition is still partly encoded inside mutation preparation helpers. Locks, queues, semaphores, rate limits, elections, barriers, and registry entries should each expose pure transition cores that make pass, deny, duplicate replay, and no-mutation behavior explicit.

## What Changes

- Define per-primitive transition cores for coordination services with state, request, manifest limits, idempotency facts, and guard evidence as inputs.
- Return transition decisions, next state or preserved state, token/output facts, status assertion facts, diagnostics, and receipt input facts without mutating runtime state.
- Treat duplicate operation-id replay as an explicit no-advance transition.
- Extend generated traces to cover the transition matrix and primitive-specific negative edges.

## Impact

Coordination behavior becomes easier to audit and reuse behind Raft/control-plane shells. The existing receipt and property-test model becomes table-driven, reducing the risk of a denial path accidentally preparing mutated state.