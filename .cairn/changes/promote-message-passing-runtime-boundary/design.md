# Design: Promote message passing to the Molten runtime boundary

## Context

Molten has the required pieces for message-oriented deterministic simulation. `fabric_simulation` already schedules message delivery. The transport layer already compares live and deterministic traces. System extensions already consume typed callbacks and emit effect plans.

The current contracts still expose several callback categories and transport lifecycle concepts without one shared message-boundary invariant. The implementation also uses source-text boundary tests that cannot detect aliases or vendor types reliably.

## Decisions

### Decision: Define state owners at existing semantic boundaries

**Choice:** Treat each system-extension service instance, vat turn owner, admitted application state machine, and fabric provider state machine as a state owner when it has exclusive transition authority.

A pure helper or local object inside one owner does not create another message boundary.

**Rationale:** The rule follows semantic authority rather than module or process layout.

### Decision: Use one canonical boundary-message contract

**Choice:** A boundary message is an owned, bounded, versioned application value. Its selected protocol contract records every behavior-affecting fact.

These facts can include source owner, destination owner, protocol and schema refs, service generation, logical sequence, correlation, causation, payload ref and byte count, logical time, deadline, authority refs, resource refs, and prior operation identity.

Not every message requires every field. The versioned protocol contract declares the required subset and bounds.

**Rationale:** One giant envelope would burden local events. Contract-selected fields preserve explicitness without forcing unrelated semantics.

### Decision: Treat every callback input as a message

**Choice:** Initialize, start, request, message, stream-open, stream-event, timer, health, checkpoint, recover, drain, shutdown, and effect-completion inputs all use the canonical callback envelope.

The callback kind is a message discriminant. `StreamOpen` and `StreamEvent` do not grant a stream handle. They carry logical identifiers, finite state, bounded owned payload values or refs, and declared lifecycle facts.

**Rationale:** Existing callback categories remain useful while their delivery becomes one scheduler-visible communication primitive.

### Decision: Keep connections and runtime handles in adapters

**Choice:** Live Iroh, socket, channel, client, executor, task, and borrowed-buffer values remain private to shell or adapter implementations.

Adapters validate framing, copy or materialize bounded payload bytes, construct canonical messages, and retain transport-specific diagnostics separately.

Cores can model logical sessions, streams, retries, and uncertainty only from explicit messages. They cannot query adapter liveness or transport-local ordering.

**Rationale:** Connections remain efficient implementation mechanisms without becoming hidden application state.

### Decision: Make the deterministic scheduler the sole simulation chooser

**Choice:** Deterministic simulation selects runnable work, message delivery, timer firing, storage completion, process lifecycle completion, fault activation, authority changes, resource outcomes, and other modeled nondeterministic completions through one bounded scheduler.

Every selected item has a canonical position, eligible-set identity, chosen alternative, virtual time, and replay behavior.

An adapter cannot invoke the core directly during simulation. It must enqueue the corresponding canonical message or completion event.

**Rationale:** Message passing helps only when delivery and completion order remain explicit.

### Decision: Preserve same-core and adapter parity

**Choice:** Live and deterministic compositions use the same state-transition artifact, message schemas, callback dispatcher, effect-plan types, and application state types.

Only shell adapters and top-level scheduling differ. Shared conformance compares canonical application traces before adapter-specific diagnostics.

Declared transport differences remain profile facts. They cannot silently change base application meaning.

**Rationale:** A mock-only simulator can reproduce expected outputs while missing production defects.

### Decision: Add compiler-backed Octet admission

**Choice:** After Octet publishes the message-boundary mechanism, Molten pins that immutable revision through Nix and selects strict architecture admission for declared core and adapter scopes.

The policy declares state owners, message types, transition paths, effect plans, live-handle providers, shell scopes, adapter scopes, and composition roots.

Existing source-text boundary tests remain defense in depth until compiler-backed coverage and negative fixtures replace their unique value.

**Rationale:** Compiler type facts catch aliases, nested generics, associated types, and vendor handles that string scanning misses.

### Decision: Migrate active changes through compatibility review

**Choice:** Review each active Cairn that introduces connection wake, stream callbacks, retries, client sessions, shared state, or new adapters.

For example, the addressable actor profile must model a connection wake as a canonical transport or lifecycle message. It cannot wake by inspecting a live connection object.

Each affected package receives a narrow update or explicit compatibility note before this change archives.

**Rationale:** A new invariant fails if active roadmap work can immediately bypass it.

### Decision: Preserve the evidence claim ladder

**Choice:** Static Octet evidence, pure-model evidence, deterministic whole-system evidence, multiprocess live evidence, host-chaos evidence, and VM or hardware evidence remain separate roles.

A passing message boundary is required for selected simulation profiles. It does not replace stronger runtime evidence.

**Rationale:** Message passing is a major determinism lever, not a whole-system proof.

## Required Flow

```text
transport or host observation
  -> shell adapter with private live handles
  -> owned canonical inbound message
  -> scheduler-visible delivery
  -> state-owner transition core
  -> next state + outbound messages + effect plans
  -> authority and resource admission
  -> shell adapter execution
  -> canonical completion or lifecycle message
```

## Adversarial Audit

The implementation must reject these cases:

- `CallbackKind::StreamOpen` carries an Iroh connection or borrowed stream;
- a message contains `Arc<Mutex<_>>`, sender, receiver, client, endpoint, executor, or task handle;
- an adapter calls a core transition directly during deterministic simulation;
- reconnect changes retry identity or ordering without a message;
- a connection close is treated as proof that a message did not commit;
- live and deterministic adapters use different callback or state types;
- a mock service replaces the admitted extension core;
- a timer, storage completion, process exit, authority change, or resource result bypasses the scheduler;
- a state owner reads wall time, entropy, process state, or adapter state;
- an active Cairn preserves connection-triggered semantics outside the message boundary;
- static evidence is presented as replay or production proof.

## Validation

Positive tests cover local and cross-process messages, logical stream events, timers, storage completions, retries, effect completions, shutdown, same-core replay, and live/simulation differential traces.

Negative tests cover handle escape, nested handle wrappers, borrowed buffers, callback bypass, shared mutable state, hidden retry state, scheduler bypass, same-core drift, adapter-specific semantic drift, stale generations, overload, cancellation, and uncertain delivery.

Run focused core tests before and after implementation. Then run strict Clippy, Octet, Cairn gates, Tracey coverage, and the smallest relevant Nix checks.
