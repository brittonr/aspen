# Design: Unify service state assertions

## Context

Service readiness already flows through several paths: the lifecycle FSM, the system extension journal, the production
readiness module, and the service dependency predicate that checks demanded refs, force-run refs, readiness refs, and
dependency subsets. Each path carries its own vocabulary, and only the FSM owns transitions.

## Approach

Add one pure projection from committed service state to an assertion set:

- `started` when the FSM passed `starting`, or the extension phase passed `starting`.
- `ready` when the FSM reached `ready`, the extension reported ready, or the production readiness report passed.
- `complete` when a one-shot lifecycle reached a terminal success state, which the FSM names `stopped` today and the
  extension reports as a finished one-shot. The projection MUST distinguish a normal finish from a failure.
- `failed` when the FSM, extension, or readiness report failed.
- `up` derived from `ready` or `complete`.

The projection is a pure function over committed state, so it lands inside the existing turn boundary and needs no new
storage. The dependency predicate reads the projected set instead of a single enum value, so a dependent can require
`ready`, `complete`, or `up`. A user-defined value passes through unchanged and satisfies no requirement that names a
built-in state.

## Decisions

### Decision: Projection, not replacement

**Choice:** Keep the lifecycle FSM as the transition authority and derive assertions from it.

**Rationale:** The FSM already carries restart, degradation, and cleanup transitions with tests. Replacing it would move
transition logic into the service registry for no gain, and the manual's union model is about visibility, not about who
decides transitions.

### Decision: `complete` is a distinct state from `stopped`

**Choice:** Add `complete` and derive it from a normal one-shot finish.

**Rationale:** A dependent that waits for completion cannot express itself over `stopped`, which also covers an
intentional stop. The distinction is exactly what the manual adds.

## Risks / Trade-offs

- Three vocabularies remain in code, with one mapping table. Drift is possible, so the mapping table gets a focused
  test with a row per local value.
- A service can hold `started` and `ready` at once. Consumers that assumed one value per service must read the set.
- The projection is evidence-shaped state. It grants no authority and no placement decision.
