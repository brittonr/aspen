# Addressable Actor Runtime Specification

## Purpose

Defines the `addressable-actor-runtime` capability.

## Requirements

### Requirement: Addressable actors use one versioned composition profile

r[molten.addressable_actor.profile] Molten MUST define addressable actors as a versioned system-extension profile over canonical actor keys and admitted fabric ports. The profile MUST NOT add a second persistence, mailbox, scheduler, placement, transport, authority, or evidence core.

#### Scenario: A complete profile is admitted

- GIVEN an actor profile binds a canonical actor key, extension generation, placement, delivery, durable-state, time, resource, supervision, authority, and evidence profiles
- WHEN Molten validates the profile
- THEN Molten admits one bounded actor composition without changing the ownership of the selected fabric mechanisms

#### Scenario: An incomplete profile fails closed

- GIVEN an actor profile omits a required port or uses an unsupported profile version
- WHEN Molten validates the profile
- THEN Molten denies activation before actor code or external effects run

### Requirement: Actor lifecycle and wake transitions are explicit and fenced

r[molten.addressable_actor.lifecycle] Molten MUST model dormant, starting, running, draining, stopped, degraded, and recovery actor states with explicit transitions. Every wake and callback plan MUST bind the actor key, placement assignment, active generation, lifecycle sequence, and wake reason.

#### Scenario: An admitted message wakes a dormant actor

- GIVEN a dormant actor has a current placement and an admitted durable message
- WHEN the wake planner evaluates the message
- THEN it emits one generation-bound start and delivery plan

#### Scenario: A stale wake preserves state

- GIVEN a wake request targets an older placement or actor generation
- WHEN the wake planner evaluates the request
- THEN it denies the wake, preserves lifecycle state, and emits no actor effects

### Requirement: Survival across sleep and restart is explicit

r[molten.addressable_actor.survival] Molten MUST define a versioned survival matrix for durable state, mailbox records, completed semantic events, checkpoints, processes, streams, sessions, partial callbacks, and in-flight deltas. A restore result MUST NOT claim survival for a class that the selected profile marks runtime-only or unsupported.

#### Scenario: Durable facts restore

- GIVEN a valid checkpoint and admitted durable records match the active actor generation
- WHEN the actor wakes after runtime teardown
- THEN restore exposes only the durable classes admitted by the survival matrix

#### Scenario: Runtime-only facts do not survive

- GIVEN processes, streams, sessions, partial callbacks, or in-flight deltas existed before sleep
- WHEN the actor wakes
- THEN Molten reports those classes as not survived unless a separately reviewed profile provides matching evidence

### Requirement: Delivery and unknown outcomes remain bounded

r[molten.addressable_actor.delivery] Molten MUST consume the coordination-delivery extension for claim, lease, acknowledgement, retry, dead-letter, and redrive behavior. An external effect with no terminal evidence MUST become an explicit unknown outcome and MUST NOT be retried automatically.

#### Scenario: Completed delivery is acknowledged

- GIVEN an actor callback completed and its semantic result is durably committed
- WHEN delivery completion is evaluated
- THEN the matching fenced delivery can be acknowledged once

#### Scenario: Crash after effect remains uncertain

- GIVEN an external effect can have occurred before a crash and no terminal evidence exists
- WHEN recovery evaluates the delivery
- THEN it records an unknown outcome and requires explicit policy or operator action

### Requirement: Actor effects retain authority and evidence boundaries

r[molten.addressable_actor.authority] Every actor effect MUST pass current policy, capability, resource, placement, generation, and adapter admission before execution. Actor identity, wake success, transport identity, or checkpoint possession MUST NOT grant authority.

#### Scenario: Authority drift blocks a wake effect

- GIVEN an actor wake plan was valid but current authority or resource admission changed
- WHEN the shell prepares the effect
- THEN it denies before execution and records the changed admission fact

### Requirement: Positive and negative actor evidence is required

r[molten.addressable_actor.verification] The addressable actor profile MUST include deterministic positive and negative lifecycle fixtures, restart tests, simulation cases, and bounded operator evidence.

#### Scenario: Focused evidence passes

- GIVEN the implementation covers admitted sleep, wake, restore, delivery, drain, and denial paths
- WHEN the focused verification rail runs
- THEN it emits passing evidence with the survival matrix and non-claims visible

### Requirement: Public actor results distinguish semantic completion

r[molten.addressable_actor.public_result] The public actor result MUST distinguish applied, denied, stale, duplicate replay, progress and unknown. Applied MUST require a current durable semantic delivery-completion transition with terminal effect evidence. Durable lifecycle intent, queued work and successful callbacks alone MUST NOT imply durable semantic completion. Unknown effect, commit, or publication results MUST remain unknown and MUST NOT authorize retry.

#### Scenario: A wake is progress

- GIVEN a wake lifecycle plan was durably committed and native callbacks succeeded
- WHEN the application reads its result
- THEN the result is progress, not applied semantic completion

#### Scenario: An uncertain completion is not applied

- GIVEN a delivery-completion transition lacks current durable commit or terminal acknowledgement evidence
- WHEN the application reads its result
- THEN the result is unknown, not applied

#### Scenario: A completed request is replayed

- GIVEN a completed logical request produces duplicate-preserve without effects
- WHEN the application reads its result
- THEN duplicate replay is explicit and no second semantic commit or hidden live effect is implied

### Requirement: Public native actors retain existing owner authority

r[molten.addressable_actor.public_native_consumer] Aspen MUST compose its public keyed native consumer from the existing actor state shell, capability-rooted assignment persistence, native supervisor/journal, and coordination-delivery owner. Current committed placement, generation/incarnation, exact manifest/profile, resource usage and independent operator admission MUST be observed before protected effects. Dormant actors MUST NOT cause the delivery owner to fabricate a Running lifecycle. Sleeping MUST checkpoint then drain without terminal shutdown; rewake MUST recover the exact selected checkpoint under a fresh native incarnation. A result MUST be bound to the original successful materialized native receipt and durable journal history, not inferred from callback invocation or a state reference alone.

#### Scenario: A dormant actor receives admitted durable work

- GIVEN the native actor is Drained and its separate admitted delivery member is Running
- WHEN the real delivery owner commits and claims a message under exact current authority
- THEN the actor consumer restores its selected checkpoint, rewakes and delivers through its existing retained native work owner

#### Scenario: Currentness changes before a protected effect

- GIVEN a prepared actor effect names an old key, placement, generation or incarnation, or current capability is revoked
- WHEN the effect adapter observes actual current owners
- THEN no protected actor effect runs and the rejection remains explicit

#### Scenario: A drained native worker is lost before recovery

- GIVEN a native actor is Drained with ingress closed and a current generation/incarnation fence
- WHEN its native owner records actual process loss
- THEN the existing failure policy applies without executing application work, and a later admitted recovery persists a fresh closed-ingress incarnation before any callback

#### Scenario: Process reopening retains semantic result identity

- GIVEN an admitted counter request has a successful materialized receipt, terminal delivery and actor completed-event metadata
- WHEN the hosting process exits and the same rooted owners reopen
- THEN a public result observation retains the original result, event and commit identity without another callback, acknowledgement or semantic commit

#### Scenario: Unsupported survival fails closed

- GIVEN a recovery claim includes a runtime-only or unsupported class
- WHEN the public native consumer validates the claim
- THEN it rejects before protected materialization or callback effects and does not invent survival evidence
