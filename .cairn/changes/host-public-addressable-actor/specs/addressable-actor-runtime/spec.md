# Addressable Actor Runtime Delta

## ADDED Requirements

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

