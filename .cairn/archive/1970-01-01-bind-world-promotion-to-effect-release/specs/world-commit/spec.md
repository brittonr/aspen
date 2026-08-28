# Molten World Commit Specification Delta

## Purpose

Make branch promotion and local effect eligibility one durable decision without claiming atomic external effect completion.

## ADDED Requirements

### Requirement: Promotion plans bind one candidate and intent closure

r[molten.world_promotion.plan] Molten MUST bind each promotion plan to the expected active head, candidate commit, exact effect-intent closure, branch policy, current authority inputs, and one operation identity.

#### Scenario: Candidate intent closure is complete

- GIVEN every candidate intent has one exact semantic identity and admitted release classification
- WHEN promotion planning runs
- THEN it MUST produce a complete bounded reservation plan

#### Scenario: Candidate contains an unclassified intent

- GIVEN one intent lacks a release, deny, simulate, or retain classification
- WHEN promotion planning runs
- THEN Molten MUST deny promotion planning as incomplete

### Requirement: Active-head movement and release reservation are locally atomic

r[molten.world_promotion.transaction] Molten MUST update the active head and publish the complete release-reservation set in one local transaction. A failed or stale transaction MUST leave both prior states unchanged.

#### Scenario: Promotion transaction commits

- GIVEN expected head, authority, policy, intent closure, and reservation identities remain current
- WHEN the local transaction commits
- THEN the candidate MUST become active and every admitted reservation MUST be durable

#### Scenario: Reservation insertion fails

- GIVEN one required reservation cannot enter the transaction
- WHEN promotion publication runs
- THEN the active head MUST remain unchanged and no partial reservation set may become dispatchable

### Requirement: Dispatch consumes only committed current reservations

r[molten.world_promotion.dispatch] Molten MUST dispatch only from committed reservations after rechecking current capability, policy, semantic handler, adapter generation, and reservation ownership. Retries MUST reuse the same logical release identity.

#### Scenario: Reservation remains admitted

- GIVEN a committed reservation passes every current dispatch gate
- WHEN a dispatcher claims it
- THEN one attempt MAY run under the reservation identity

#### Scenario: Authority changed after promotion

- GIVEN promotion succeeded but current capability admission now denies
- WHEN dispatch admission runs
- THEN Molten MUST record a blocked reservation and MUST NOT execute the effect

### Requirement: Uncertain outcomes require observation-first reconciliation

r[molten.world_promotion.reconciliation] Molten MUST classify publication and effect observations as not published, published, unknown, or conflicting before retry decisions. Unknown or conflicting outcomes MUST NOT trigger a blind new logical operation.

#### Scenario: Local commit result is unknown

- GIVEN the transaction returned no reliable completion result
- WHEN recovery begins
- THEN Molten MUST observe the durable operation identity before planning any repeat mutation

#### Scenario: External acknowledgment was lost

- GIVEN an effect may have completed but no acknowledgment is durable
- WHEN reconciliation runs
- THEN Molten MUST retain an uncertain state unless an admitted observation resolves it

### Requirement: Acknowledged observations use explicit successor commits

r[molten.world_promotion.observation_commit] Molten MUST bind an acknowledged effect observation to one recorded-effect transition from the promoted candidate to an explicit successor. This transition MUST NOT mutate the promoted commit or grant dispatch authority.

#### Scenario: Acknowledged observation becomes recorded history

- GIVEN one acknowledged attempt has an exact reservation and observation identity
- WHEN observation-commit planning runs
- THEN the transition MUST bind the promoted candidate as parent, the observation as input, and the supplied successor

#### Scenario: Observation is unknown or mismatched

- GIVEN an attempt is unacknowledged, lacks an observation, names another reservation, or keeps the same successor
- WHEN observation-commit planning runs
- THEN Molten MUST deny the transition before history publication

### Requirement: Promotion receipts state bounded non-claims

r[molten.world_promotion.non_claims] Promotion, reservation, attempt, and reconciliation receipts MUST distinguish local eligibility, dispatch attempts, observations, and external completion. They MUST NOT claim generic exactly-once execution or atomic external effects.

#### Scenario: Promotion succeeded without dispatch

- GIVEN the active head and reservations committed but no dispatcher ran
- WHEN the promotion receipt is inspected
- THEN it MUST report eligibility without claiming any external effect occurred

### Requirement: Promotion verification covers crash windows

r[molten.world_promotion.verification] Molten MUST test pre-commit, uncertain-commit, post-commit, pre-dispatch, post-dispatch, lost-acknowledgment, duplicate, conflict, denial, and overclaim paths.

#### Scenario: Focused promotion rail runs

- GIVEN deterministic fake adapters cover every declared crash boundary
- WHEN the promotion verification rail runs
- THEN it MUST report local atomicity and external-effect non-claims
