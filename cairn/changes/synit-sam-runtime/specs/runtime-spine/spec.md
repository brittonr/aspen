## ADDED Requirements

### Requirement: Synit/SAM reference boundary
r[molten.runtime_spine.synit_reference_boundary] The system MUST treat Synit and the Syndicated Actor Model as non-normative design references for reactive dataspace semantics, assertions, retractions, object capabilities, and tracing, and MUST NOT claim Synit wire protocol, sturdyref, PID1, service-manager, or configuration-scripting compatibility.

#### Scenario: Documentation cites Synit without compatibility claim
r[molten.runtime_spine.synit_reference_boundary.no_compat]
- GIVEN Molten design material that cites Synit or the Syndicated Actor Model
- WHEN the material describes an adopted pattern
- THEN it states the Molten-specific envelope, policy, evidence, transport, and configuration boundaries rather than claiming Synit compatibility

### Requirement: Actor turn semantics
r[molten.runtime_spine.turn_semantics] The local runtime MUST process actor events in turns where an actor receives one event, accumulates pending actions, applies deterministic validation and policy gates, and commits or discards the pending actions as a unit.

#### Scenario: Successful turn commits pending actions
r[molten.runtime_spine.turn_semantics.commit]
- GIVEN an actor turn that produces an assertion and a message and all admission checks pass
- WHEN the turn completes successfully
- THEN the runtime makes the assertion and message visible to their targets in the same committed turn result

#### Scenario: Failed turn rolls back pending actions
r[molten.runtime_spine.turn_semantics.rollback]
- GIVEN an actor turn that produces pending actions and then fails, panics, or is denied by policy
- WHEN the turn aborts
- THEN none of the pending assertions, retractions, messages, or adapter effects become visible as committed runtime state

### Requirement: Assertion lifetimes and automatic retraction
r[molten.runtime_spine.assertion_lifetimes] Dataspace assertions MUST be owned by an actor, session, facet, or admitted live reference and MUST be automatically retracted when the owner terminates, crashes, disconnects, or loses the authority that justified the assertion.

#### Scenario: Actor crash retracts owned assertions
r[molten.runtime_spine.assertion_lifetimes.crash]
- GIVEN an actor that has asserted service presence into a dataspace
- WHEN the actor crashes or is stopped
- THEN the runtime retracts the actor's live assertions and propagates matching retractions to subscribers

#### Scenario: Duplicate assertion remains until last owner retracts
r[molten.runtime_spine.assertion_lifetimes.dedup]
- GIVEN two owners that assert the same canonical Preserves value into the same dataspace
- WHEN one owner retracts or terminates
- THEN observers do not see a full retraction until the final owner of that canonical assertion withdraws it

### Requirement: Observe-style subscription assertions
r[molten.runtime_spine.observe_patterns] The dataspace adapter MUST support `Observe`-style subscription assertions over Preserves patterns, where a subscriber receives matching current assertions, future matching assertions, and matching retractions until the subscription assertion is retracted.

#### Scenario: Observe receives existing and future assertions
r[molten.runtime_spine.observe_patterns.delivery]
- GIVEN a dataspace with an existing assertion matching a pattern
- WHEN an actor asserts an `Observe` subscription for that pattern
- THEN the subscriber receives the existing matching assertion and later receives future matching assertions

#### Scenario: Observe retraction stops delivery
r[molten.runtime_spine.observe_patterns.retract]
- GIVEN an active `Observe` subscription
- WHEN the subscription assertion is retracted
- THEN the dataspace stops delivering future matches to that observer and retracts forwarded assertions that were scoped to the subscription

### Requirement: Deterministic Preserves pattern language
r[molten.runtime_spine.preserves_patterns] The system MUST define a bounded, deterministic Preserves pattern language for dataspace routing and policy-visible matching, including wildcard/discard, literal values, record/array/dictionary matching, and deterministic binding order.

#### Scenario: Pattern match produces stable bindings
r[molten.runtime_spine.preserves_patterns.bindings]
- GIVEN the same Preserves pattern and candidate value on two nodes
- WHEN each node evaluates the match
- THEN both nodes produce the same success or failure result and the same ordered binding sequence

#### Scenario: Extra compound fields remain extensible
r[molten.runtime_spine.preserves_patterns.extensible]
- GIVEN a pattern that matches selected fields of a record, array, or dictionary
- WHEN a candidate value contains additional fields not mentioned by the pattern
- THEN the match may still succeed according to the declared extensibility rules for that compound kind

### Requirement: Capability attenuation for messages and assertions
r[molten.runtime_spine.capability_attenuation] Capabilities MUST be able to attenuate authority over messages, assertions, subscriptions, and reference introduction by filtering, rewriting, or denying Preserves values through Molten policy gates before delivery or publication.

#### Scenario: Attenuation denies disallowed assertion
r[molten.runtime_spine.capability_attenuation.denied_assertion]
- GIVEN a live dataspace reference attenuated to allow only assertions matching an admitted pattern
- WHEN an actor attempts to publish a non-matching assertion through that reference
- THEN the runtime denies or discards the assertion before it becomes visible and records policy evidence when required

#### Scenario: Attenuation rewrites admitted message
r[molten.runtime_spine.capability_attenuation.rewrite]
- GIVEN a capability attenuation that rewrites a matching message body into a narrower canonical form
- WHEN an actor sends a matching message through the capability
- THEN the delivered message body is the rewritten canonical Preserves value and the applied attenuation rule is available as evidence

### Requirement: Gatekeeper resolver
r[molten.runtime_spine.gatekeeper_resolver] The system MUST provide a gatekeeper resolver pattern that converts long-lived credentials, UCANs, tickets, or invites into live scoped references with explicit attenuation, expiry or revocation conditions, and evidence references.

#### Scenario: Credential resolves to live scoped reference
r[molten.runtime_spine.gatekeeper_resolver.resolve]
- GIVEN a valid credential that grants access to a protocol session or dataspace resource
- WHEN an actor submits it to the gatekeeper resolver
- THEN the resolver returns a live reference scoped to the admitted resource, attenuation, expiry, and receipt evidence

#### Scenario: Revoked credential retracts dependent assertions
r[molten.runtime_spine.gatekeeper_resolver.revoke]
- GIVEN assertions or subscriptions made through a live reference derived from a credential
- WHEN the credential expires or is revoked
- THEN the runtime revokes the live reference and retracts dependent assertions or subscriptions that no longer have authority

### Requirement: Live reference lifetime cleanup
r[molten.runtime_spine.reference_lifetimes] Live references to local actors, dataspaces, protocol sessions, consensus resources, blob capabilities, and host resources MUST have explicit lifetime, revocation, and cleanup semantics.

#### Scenario: Session close cleans live references
r[molten.runtime_spine.reference_lifetimes.session_close]
- GIVEN a transport or protocol session that introduced live references
- WHEN the session closes
- THEN references scoped only to that session become invalid and their dependent assertions, subscriptions, or pending operations are cleaned up

### Requirement: Service dependency assertions
r[molten.runtime_spine.service_dependency_assertions] The runtime MUST support service lifecycle and dependency assertions for demand-driven startup, readiness, restart, failure, completion, and exposed service objects or references.

#### Scenario: Dependency delays service start
r[molten.runtime_spine.service_dependency_assertions.dependency]
- GIVEN a service `worker` that depends on service state `network ready`
- WHEN `worker` is required but `network ready` is not asserted
- THEN the runtime withholds `worker` startup until the dependency state is asserted or the requirement is withdrawn

#### Scenario: Service readiness publishes state
r[molten.runtime_spine.service_dependency_assertions.ready]
- GIVEN a service that has started and completed its readiness checks
- WHEN it becomes able to serve requests
- THEN it asserts a ready service state and may assert service object references for clients to discover

### Requirement: Logical supervision tree
r[molten.runtime_spine.supervision_tree] The runtime MUST model logical supervision relationships independently from OS process parentage or adapter-specific process trees.

#### Scenario: Supervised adapter process is logical child
r[molten.runtime_spine.supervision_tree.logical_child]
- GIVEN an adapter process whose OS parent is a launcher or PID 1 equivalent
- WHEN Molten represents runtime supervision
- THEN the service appears under its logical supervisor in the Molten supervision tree regardless of OS parentage

### Requirement: Demand-driven startup and shutdown
r[molten.runtime_spine.demand_driven_startup] The runtime MUST use service demand and dependency assertions to start, keep alive, restart, or shut down services without relying on hardcoded service graphs.

#### Scenario: Removing demand allows shutdown
r[molten.runtime_spine.demand_driven_startup.shutdown]
- GIVEN a service with no remaining `require-service` or `run-service` demand assertions
- WHEN dependencies and reverse-dependencies no longer require it
- THEN the runtime may gracefully shut down the service and retract its service state assertions

### Requirement: Canonical interaction tracing
r[molten.runtime_spine.interaction_tracing] The runtime MUST be able to emit canonical Preserves trace records for committed turns, actor lifecycle events, assertions, retractions, messages, policy decisions, choreography transitions, consensus events, and receipt references.

#### Scenario: Turn emits trace record
r[molten.runtime_spine.interaction_tracing.turn]
- GIVEN tracing is enabled for a local runtime
- WHEN an actor turn commits an assertion and a message
- THEN the runtime emits trace records that identify the actor, turn id, causal parent if any, committed actions, and receipt or policy evidence references

#### Scenario: Choreography and consensus events are traceable
r[molten.runtime_spine.interaction_tracing.protocol_consensus]
- GIVEN a choreography endpoint transition or Raft-backed consensus commit
- WHEN the event is exposed to the runtime tracing surface
- THEN the trace record identifies the protocol/session or consensus group/term/index metadata along with associated evidence references

### Requirement: Trace export and rendering surface
r[molten.runtime_spine.trace_rendering] The runtime SHOULD expose a trace inspection/export surface that can filter canonical trace records and support later sequence-diagram or interaction rendering tools.

#### Scenario: Operator exports filtered trace
r[molten.runtime_spine.trace_rendering.export]
- GIVEN a runtime with stored or streamed trace records
- WHEN an operator filters by actor id, protocol session id, or consensus group id
- THEN the inspection surface returns matching canonical trace records suitable for rendering or audit

### Requirement: SAM-style runtime integration tests
r[molten.runtime_spine.sam_integration_tests] The system MUST include integration tests for turn rollback, assertion auto-retraction, Observe delivery and retraction, attenuation deny/rewrite/admit, gatekeeper resolution, service dependency startup, and trace emission.

#### Scenario: Assertion lifecycle integration test
r[molten.runtime_spine.sam_integration_tests.assertion_lifecycle]
- GIVEN two actors and a local dataspace
- WHEN one actor observes a pattern and the other asserts then terminates a matching value
- THEN the observing actor receives the assertion followed by the automatic retraction

### Requirement: SAM-style property tests
r[molten.runtime_spine.sam_property_tests] The system MUST use Hegel property-based tests for generated assertion, retraction, subscription, and owner-lifetime sequences within supported bounds.

#### Scenario: Generated assertion ownership preserves visibility invariant
r[molten.runtime_spine.sam_property_tests.visibility]
- GIVEN a generated sequence of assertion owners, duplicate assertions, retractions, and owner terminations
- WHEN the dataspace model evaluates visible assertions
- THEN an assertion is visible exactly when at least one live owner still maintains that canonical assertion
