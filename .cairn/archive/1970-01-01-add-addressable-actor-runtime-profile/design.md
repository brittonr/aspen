## Context

Molten has actor and vat primitives, durable stores, a generation-fenced system-extension lifecycle, explicit scheduling, supervised recovery, and active work on durable delivery. These mechanisms lack one reviewed composition for a keyed actor that can release runtime resources while preserving selected durable facts.

Rivet Actors are a design reference for keyed addressability, persistent state, durable queues, timers, and wake-on-demand. Their implementation, service claims, benchmark values, and TypeScript API are not compatibility targets.

## Decisions

### Decision: Implement an actor system-extension profile

**Choice:** Define the addressable actor as a versioned system-extension profile over existing fabric ports. Do not add mailbox, persistence, scheduler, transport, or policy shortcuts to Molten core.

**Rationale:** The profile owns product semantics while each common mechanism keeps its current claim boundary and test surface.

### Decision: Separate durable and runtime survival

**Choice:** Make every supported fact explicit in a survival matrix. Durable state, admitted mailbox records, completed semantic events, and selected checkpoints can survive. Processes, open streams, live sessions, in-flight deltas, and uncommitted callback state do not survive unless a later profile proves a narrower behavior.

**Rationale:** An explicit matrix prevents a successful wake from becoming an unsupported process or conversation continuity claim.

### Decision: Fence every wake and callback

**Choice:** Bind actor key, placement assignment, extension generation, lifecycle sequence, and wake reason to every wake plan. Stale work preserves state and emits no effects.

**Rationale:** Delayed messages, timers, and callbacks must not reactivate a replaced actor generation.

### Decision: Preserve uncertainty after external effects

**Choice:** If an effect can have occurred but no terminal evidence exists, recovery records an unknown outcome and requires explicit policy or operator action. It does not retry automatically.

**Rationale:** Durable orchestration does not make external effects exactly once.

## Risks / Trade-offs

- The composition can duplicate existing lifecycle vocabulary unless every state maps to the system-extension FSM.
- Hibernation policy can become an implicit resource or pricing policy. The profile must use explicit reviewed limits and no cost claims.
- Delivery, checkpoint, and placement evidence can disagree after failure. Recovery must fail closed or expose bounded uncertainty.
- The Rivet source must be pinned to an exact reviewed revision before implementation and added to the repository reference list.
