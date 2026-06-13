## ADDED Requirements

### Requirement: Synit and SAM are non-normative references
r[molten.runtime_spine.synit_reference_boundary] Molten MUST treat Synit and the Syndicated Actor Model as non-normative design references for reactive dataspace semantics, assertions, retractions, object capabilities, service assertions, and tracing. Molten MUST NOT claim Synit wire protocol, sturdyref, PID1, service-manager, OID, service-schema, or configuration-scripting compatibility.

#### Scenario: Documentation cites Synit without compatibility claim
- GIVEN Molten design material cites Synit or the Syndicated Actor Model
- WHEN the material describes an adopted pattern
- THEN it states the Molten-specific envelope, policy, evidence, transport, storage, and configuration boundaries
- AND it does not claim Synit compatibility.

### Requirement: Actor turns are atomic
r[molten.runtime_spine.turn_semantics] Molten MUST process local actor events in turns where an actor receives one event, accumulates pending actions, applies deterministic validation and policy gates, and commits or discards pending actions as a unit. Pending messages, assertions, observations, retractions, and effect intents MUST remain invisible until commit.

#### Scenario: Successful turn commits pending actions
- GIVEN an actor turn that stages an assertion and a message
- WHEN all admission and transition checks pass
- THEN the runtime makes the assertion and message visible in the committed turn result.

#### Scenario: Failed turn rolls back pending actions
- GIVEN an actor turn stages pending actions and is denied or fails before commit
- WHEN the runtime rolls the turn back
- THEN no pending assertions, retractions, messages, or effect intents become committed runtime state.

### Requirement: Assertion lifetimes are owner-scoped
r[molten.runtime_spine.assertion_lifetimes] Molten dataspace assertions MUST be owned by an actor, session, facet, or admitted live reference. Owner cleanup, termination, revocation, or session close MUST retract owner-scoped assertions and observers before they remain visible as live state. Duplicate canonical assertions MAY be represented by owner sets or deterministic assertion refs, but visibility MUST depend on at least one live owner.

#### Scenario: Actor cleanup retracts owned assertions
- GIVEN an actor has asserted service presence into a dataspace
- WHEN the actor scope is cleaned up
- THEN the runtime retracts the actor's live assertions and observers.

#### Scenario: Duplicate assertion remains until last owner retracts
- GIVEN two owners assert the same canonical Preserves value into the same dataspace
- WHEN one owner retracts or terminates
- THEN the assertion remains visible while another live owner still maintains it.

### Requirement: Observe subscriptions are explicit assertions
r[molten.runtime_spine.observe_patterns] Molten MUST support explicit `Observe`-style subscription records or DTOs over the implemented Preserves pattern subset. An observer MUST receive matching current assertions, future matching assertions, and matching retractions until the subscription is retracted or the observer scope is cleaned up.

#### Scenario: Observe receives existing and future assertions
- GIVEN a dataspace contains an assertion matching an observer's pattern
- WHEN an actor registers an `Observe` subscription for that pattern
- THEN the observer receives the existing assertion and later future matching assertions.

#### Scenario: Observe retraction stops delivery
- GIVEN an active observer has a live subscription
- WHEN the observer scope or subscription is retracted
- THEN the dataspace stops delivering future matches scoped to that subscription.

### Requirement: Preserves pattern matching is deterministic and bounded
r[molten.runtime_spine.preserves_patterns] Molten MUST define a bounded deterministic Preserves pattern subset for dataspace routing and policy-visible matching. The completed initial subset includes exact canonical value matching and wildcard binding with deterministic binding order; richer record, array, dictionary, conjunction, negation, or extensible compound matching MAY be added only by future admitted extensions.

#### Scenario: Pattern match produces stable bindings
- GIVEN the same implemented Preserves pattern and candidate value on two nodes
- WHEN each node evaluates the match
- THEN both nodes produce the same success or failure result and the same ordered binding sequence.

#### Scenario: Unsupported compound pattern denies
- GIVEN a pattern outside the admitted bounded subset
- WHEN routing or policy-visible matching evaluates it
- THEN the match is denied or rejected before it controls side effects.

### Requirement: Capabilities attenuate dataspace and message authority
r[molten.runtime_spine.capability_attenuation] Molten capabilities MUST attenuate authority over messages, assertions, subscriptions, and reference introduction through Molten policy/authority gates before delivery or publication. The completed scope supports scoped allow/deny authority contexts and live refs; rewrite/filter transforms require explicit future rule evidence before they can alter delivered values.

#### Scenario: Attenuation denies disallowed assertion
- GIVEN a live dataspace reference is scoped to an admitted capability
- WHEN an actor attempts to publish or observe outside that scope
- THEN the runtime denies before the assertion or subscription becomes visible.

#### Scenario: Rewrite requires explicit rule evidence
- GIVEN an actor requests message or assertion rewriting through attenuation
- WHEN no admitted rewrite rule evidence is present
- THEN Molten does not infer a rewrite from the capability alone.

### Requirement: Gatekeeper resolution emits live refs
r[molten.runtime_spine.gatekeeper_resolver] Molten MUST provide a gatekeeper resolver pattern that converts admitted long-lived credentials, UCANs, tickets, invites, or authority contexts into live scoped references with attenuation, expiry or revocation conditions, and evidence refs.

#### Scenario: Credential resolves to live scoped reference
- GIVEN a valid authority context that grants scoped access to a resource
- WHEN an actor submits it to the gatekeeper resolver
- THEN the resolver returns a live reference scoped to the admitted capability, attenuation, expiry, and receipt evidence.

#### Scenario: Revoked credential denies resolution
- GIVEN a revocation applies to a credential, context, delegation, key, or capability
- WHEN gatekeeper resolution runs
- THEN the resolver denies or invalidates the live reference and records diagnostic evidence.

### Requirement: Live references have cleanup semantics
r[molten.runtime_spine.reference_lifetimes] Molten live references to local actors, dataspaces, protocol sessions, consensus resources, blob capabilities, and host resources MUST have explicit lifetime, revocation, and cleanup semantics. Cleanup MUST retract dependent assertions, subscriptions, pending operations, or handles where implemented.

#### Scenario: Session close cleans live references
- GIVEN a transport or protocol session introduced live references
- WHEN the session closes or is revoked
- THEN references scoped only to that session become invalid and dependent assertions, subscriptions, or pending operations are cleaned up.

### Requirement: Service dependencies are dataspace evidence
r[molten.runtime_spine.service_dependency_assertions] Molten MUST represent service demand, readiness, dependency, failure, completion, restart, shutdown, and exposed service references as canonical service runtime or supervision evidence. Demand-driven startup MUST wait for dependency readiness and emit receipt-backed diagnostics for missing, denied, or cyclic dependencies.

#### Scenario: Dependency delays service start
- GIVEN a service demand depends on another service readiness assertion
- WHEN the dependency is not ready
- THEN the runtime withholds startup and emits wait or deny diagnostics without asserting readiness.

#### Scenario: Service readiness publishes state
- GIVEN a demanded service starts with satisfied dependencies
- WHEN readiness checks pass
- THEN the runtime emits a readiness assertion and lifecycle/status evidence.

### Requirement: Supervision is logical, not OS parentage
r[molten.runtime_spine.supervision_tree] Molten MUST model logical supervision relationships independently from OS process parentage or adapter-specific process trees. Supervision evidence MUST bind failure markers, lifecycle receipts, monitor notifications, restart decisions, cleanup receipts, and diagnostics without granting service authority by itself.

#### Scenario: Supervised adapter process is logical child
- GIVEN an adapter process has an OS parent unrelated to Molten's logical supervisor
- WHEN Molten emits supervision evidence
- THEN the service appears under its logical supervisor in Molten evidence regardless of OS parentage.

### Requirement: Demand drives startup and shutdown
r[molten.runtime_spine.demand_driven_startup] Molten MUST use explicit service demand and dependency evidence to start, keep alive, restart, or shut down services without relying on hardcoded service graphs. Shutdown and cleanup remain receipt-backed and policy/resource-gated.

#### Scenario: Removing demand allows shutdown
- GIVEN a service has no remaining demand or reverse-dependency evidence
- WHEN shutdown is admitted
- THEN the runtime may stop the service and retract or clean up service state evidence.

### Requirement: Interaction tracing is canonical evidence
r[molten.runtime_spine.interaction_tracing] Molten MUST represent committed turns, actor lifecycle events, assertions, retractions, messages, policy decisions, runtime predicate receipts, service turn contexts, replay divergence records, choreography transitions, consensus events, and associated receipt refs as canonical Preserves evidence where those events cross runtime or audit boundaries.

#### Scenario: Turn emits trace context
- GIVEN a local runtime or service turn commits an assertion or message
- WHEN trace or report evidence is emitted
- THEN the evidence identifies actor or service context, committed actions, state refs, and receipt or policy evidence refs.

#### Scenario: Protocol and consensus events are traceable
- GIVEN a choreography endpoint transition or Raft-backed consensus commit is exposed to runtime tracing
- WHEN the event is recorded
- THEN the trace evidence identifies protocol/session or consensus group/term/index metadata with associated refs.

### Requirement: Trace inspection is evidence-only
r[molten.runtime_spine.trace_rendering] Molten SHOULD expose inspection, summary, replay, or export surfaces for canonical trace/report records. Rendered summaries MUST remain non-normative views and MUST NOT replace canonical receipts, policy gates, or replay validation.

#### Scenario: Operator inspects filtered trace evidence
- GIVEN runtime or service trace/report evidence exists
- WHEN an operator filters or renders it by actor, service, protocol, or consensus metadata
- THEN the output is derived from canonical evidence and does not grant authority.

### Requirement: SAM runtime tests cover implemented surfaces
r[molten.runtime_spine.sam_integration_tests] Molten MUST include tests for implemented SAM-style surfaces, including turn rollback, assertion cleanup, Observe delivery and retraction behavior, authority attenuation denial, gatekeeper resolution, service dependency startup, supervision cleanup, and trace/report emission.

#### Scenario: Assertion lifecycle integration test
- GIVEN an observer and asserting actor in a local dataspace
- WHEN the actor asserts and then its scope is cleaned up
- THEN tests show the assertion visibility and cleanup evidence follow the owner lifecycle.

### Requirement: SAM runtime properties are bounded
r[molten.runtime_spine.sam_property_tests] Molten SHOULD use bounded Hegel/property tests for implemented assertion, retraction, subscription, owner-lifetime, service dependency, and runtime predicate invariants.

#### Scenario: Generated assertion ownership preserves visibility invariant
- GIVEN a generated bounded sequence of assertion owners, duplicate assertions, retractions, and owner cleanup
- WHEN the runtime predicate model evaluates visibility
- THEN an assertion is visible exactly when at least one live owner still maintains that canonical assertion.
