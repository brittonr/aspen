# Addressable Actor Runtime Delta

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
