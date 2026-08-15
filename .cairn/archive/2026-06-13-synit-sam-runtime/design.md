## Context

Molten already has a planned envelope spine and a local dataspace adapter. The next architectural question is what semantics the dataspace adapter should provide. Synit and the Syndicated Actor Model are strong prior art for a system-layer runtime built on reactive assertions, object capabilities, and Preserves schemas.

Molten should adopt the model-level ideas, not the whole Synit system. The adopted semantics must remain compatible with Molten's own boundaries:

- Preserves canonical representations at all communication boundaries.
- Basalt/UCAN, Nickel, Steel, Trellis, and Cairn for authority, policy, predicates, and receipts.
- Iroh transports and Redb stores as adapters.
- Trellis choreography and Trellis Raft as separate higher-level protocol/consensus layers.
- Pure core validation separated from effectful adapter execution.

## Goals

- Make local actor interaction reactive and state-oriented rather than only message-oriented.
- Give assertions explicit lifetimes and failure semantics.
- Define deterministic turn boundaries for actor effects.
- Use `Observe`-style subscriptions and Preserves patterns for dataspace routing.
- Support capability attenuation for both observation and invocation.
- Provide a gatekeeper resolver abstraction for upgrading long-lived authority into live scoped references.
- Express service lifecycle and dependency management as dataspace assertions.
- Record interactions as trace data that can be inspected, filtered, replayed, or rendered.

## Non-Goals

- Do not implement Synit as an operating system or replace Linux/systemd/PID1 in this change.
- Do not adopt Synit's ad-hoc configuration scripting language; Molten uses Nickel for declarative config and Steel for reviewed dynamic orchestration.
- Do not use Synit sturdyrefs as Molten's authority root; Molten uses Basalt/UCAN-backed capabilities and policy receipts.
- Do not require Synit's exact wire protocol, OID numbering, service schemas, or process supervision implementation.
- Do not make every dataspace assertion durable or replicated; durability and consensus remain explicit adapter/control-plane choices.

## Runtime Model

### Actors, entities, assertions, and messages

A Molten actor owns local state, entities or service handles, and outbound assertions. Assertions represent conversational state and may carry entity references, content references, capabilities, or evidence references. Messages remain useful for one-shot notifications, but durable conversational frames should be assertions so they can be retracted when their owner dies or loses authority.

### Turn semantics

An actor processes one event at a time. During a turn, it may produce pending actions:

- assert a value,
- retract a prior assertion,
- send a message,
- spawn or stop a local actor/service,
- request a policy-gated adapter effect,
- emit trace/evidence records.

Pending actions are not visible until the turn commits. If the turn fails, panics, is denied by policy, or violates deterministic validation, pending actions are discarded. Adapter side effects occur only after admission and commit ordering are explicit.

### Assertion lifetimes

Each assertion has an owner actor/session/facet and a stable handle within that owner. When the owner terminates, crashes, disconnects, or loses the capability that authorized the assertion, the runtime retracts all live assertions owned by that scope and propagates retractions to subscribers.

Duplicate assertions of the same canonical value within the same dataspace are deduplicated for observers, while reference counts or ownership sets are maintained internally so a value is only fully retracted when the last owner withdraws it.

### Observe patterns

A subscription is an assertion whose body expresses interest in other assertions:

```text
<Observe pattern observer>
```

Molten should define this as a canonical Preserves shape or equivalent typed DTO that lowers to Preserves. When an `Observe` assertion appears, the dataspace immediately forwards all matching current assertions to the observer and continues forwarding future assertions and retractions until the `Observe` assertion is itself retracted.

### Preserves patterns

Patterns should be bounded and indexable. The completed initial runtime predicate subset supports exact canonical value matching and wildcard binding with deterministic binding order. Literal/record/array/dictionary structural matching, bounded conjunction, negation, and extensible compound matching remain future admitted extensions unless a later slice adds the indexing and policy rules.

Pattern matching should be pure and deterministic. Pattern values are Preserves values, and routing decisions must not depend on Rust debug formatting, allocation identity, or nondeterministic map iteration.

### Capability attenuation

Capabilities should govern both behavior and observation:

- which messages may be sent,
- which assertions may be published,
- which `Observe` subscriptions may be established,
- which references may be introduced to another actor or remote peer.

Molten can borrow Synit's caveat/filter concept but route it through Molten policy:

```text
capability + attenuation pattern + requested assertion/message
        -> Basalt/UCAN + Nickel/Steel contract + Trellis bounded predicate
        -> admitted or denied with receipt evidence
```

The completed initial scope supports scoped allow/deny authority contexts and live refs. Unknown or invalid attenuation rules deny by default. Rewrite transforms must preserve canonical Preserves identity and require explicit future rule evidence that identifies the applied rule.

### Gatekeeper resolver

A gatekeeper is a service that resolves long-lived credentials into live scoped references. Inputs may be UCANs, invites, tickets, content-addressed grants, or other admitted credentials. Outputs are live references to actors, dataspaces, protocol sessions, Raft-backed resources, blob capabilities, or Wasmtime host resources.

Resolution must be policy-gated and auditable. A live reference has a scope, attenuation, expiry or revocation condition, and evidence reference. When the backing credential is revoked or expires, the runtime retracts assertions and subscriptions made through the live reference as needed.

### Service dependency assertions

Service lifecycle should be expressed through dataspace facts, for example:

- require service `S`,
- run service `S`,
- service `S` depends on state `T ready`,
- service `S` state is `started`, `ready`, `failed`, or `complete`,
- service `S` exposes service object/reference `R`,
- restart service `S`.

Molten's exact schema may differ from Synit's, but it should preserve the core model: demand and dependencies are assertions, readiness is asserted by services, and removal of demand leads to graceful shutdown or retraction.

### Interaction tracing

Every committed turn and significant adapter event should optionally emit canonical trace records. Trace records should include:

- timestamp or logical time if available through an admitted source,
- actor/entity/session ids,
- cause and parent turn ids,
- assertions, retractions, messages, and sync events,
- policy decisions and receipt references,
- choreography transition metadata,
- consensus group/term/index metadata when applicable.

Tracing is data, not logs only. Trace records should be Preserves values so they can be filtered, rendered, stored, hashed, or attached to receipts.

## Open Questions

- Which compound Preserves pattern forms should extend the current exact/wildcard subset first?
- Should duplicate assertion deduplication be global per dataspace or scoped by subject/facet for performance?
- How should turn rollback interact with adapter effects that require reservation before commit?
- Which live references should be first-class Rust values versus envelope-level Preserves references?
- Should trace timestamps use wall clock, logical clock, or both when wall-clock capability is absent?
