## Context

The local runtime is actor/dataspace-oriented and may host Goblins-style vats, Wasm actors, Steel scripts, services, protocol sessions, and job stages. Each needs lifecycle state and failure containment. BEAM/OTP and Synit are useful prior art, but Molten must define its own evidence-bearing semantics.

## Goals

- Define lifecycle states and transitions for runtime entities.
- Make failures, restarts, and cleanup deterministic and traceable.
- Support links, monitors, and supervisors as policy-controlled relationships.
- Represent service demand/readiness/failure through dataspace assertions.
- Clean up assertions, subscriptions, live refs, resources, and handler bindings on termination or authority loss.
- Keep adapter side effects behind committed turns and admission.

## Non-Goals

- Do not claim OTP compatibility or BEAM distribution semantics.
- Do not implement every OTP restart strategy in the first milestone.
- Do not hide crashes behind automatic restart without receipts.
- Do not allow supervisors to bypass policy/capability gates.

## Lifecycle states

Initial states:

- `declared`
- `spawning`
- `starting`
- `ready`
- `degraded`
- `stopping`
- `stopped`
- `failed`
- `restarting`
- `cleaned`

Transitions are canonical events with cause, policy refs, resource refs, and receipts.

## Turn failure

Actor turn failure discards pending actions and transactional vat deltas. Already-committed prior turns remain. Adapter effects should occur only after admission/commit ordering is explicit; effects that cannot be rolled back must be modeled as one-shot or compensating operations.

## Links, monitors, and supervisors

A link propagates failure according to policy. A monitor observes lifecycle changes without implying control. A supervisor owns restart policy, resource budget, and cleanup obligations for children. Restart strategies should include at least `never`, `one_for_one`, and bounded restart with logical-time windows.

## Service lifecycle assertions

Service state is represented through dataspace assertions: demand, starting, ready, failed, dependency needed, dependency ready, exposed reference, restart requested, and stop requested. Retraction of demand can trigger graceful shutdown under policy.

## Cleanup

Cleanup retracts assertions/subscriptions, revokes live refs, releases resource grants, closes handler bindings, drains mailboxes according to policy, and emits receipts. Cleanup must be idempotent.

## Open Questions

- Which restart strategies are needed before remote execution and job DAGs land?
- Should lifecycle state for protocol sessions be owned by choreography or supervision?
- How should one-shot external effects be reported to supervisors on failure?
