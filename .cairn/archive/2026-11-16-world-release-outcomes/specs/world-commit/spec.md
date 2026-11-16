# World Commit Delta

## MODIFIED Requirements

### Requirement: Dispatch consumes only committed current reservations

r[molten.world_promotion.dispatch] Molten MUST dispatch only from committed reservations after rechecking current capability, policy, semantic handler, adapter generation, and reservation ownership. Retries MUST reuse the same logical release identity. Every attempt record MUST carry an explicit succeeded, failed, or unknown outcome, and only a succeeded observation MAY be acknowledged or committed as proven external completion. A dispatcher error, or an interruption before the dispatcher result is recorded, MUST leave the attempt and its reservation recorded as uncertain.

#### Scenario: Reservation remains admitted

- GIVEN a committed reservation passes every current dispatch gate
- WHEN a dispatcher claims it
- THEN one attempt MAY run under the reservation identity

#### Scenario: Authority changed after promotion

- GIVEN promotion succeeded but current capability admission now denies
- WHEN dispatch admission runs
- THEN Molten MUST record a blocked reservation and MUST NOT execute the effect

#### Scenario: The release effect reports failure

- GIVEN a dispatcher observation reports that the release failed
- WHEN the attempt is classified, acknowledged, or committed as an observation
- THEN the attempt MUST record a failed outcome
- AND acknowledgement and observation commit MUST be denied

#### Scenario: The dispatcher returns an error

- GIVEN an attempt was recorded as attempting and the dispatcher returns an error
- WHEN the dispatch shell handles the error
- THEN the attempt and reservation MUST be recorded as uncertain with an unknown outcome before the error is returned
- AND retry with acknowledged duplicate risk and abandon with an acknowledged unknown outcome MUST be available from that state
- AND a failure to record the uncertain state MUST be returned together with the dispatcher error

#### Scenario: A stored attempt cannot express its outcome

- GIVEN a canonical attempt record uses the seven-field v1 layout or an unsupported outcome
- WHEN the record is parsed
- THEN parsing MUST fail rather than default the outcome

#### Scenario: Dispatch was interrupted before its result was recorded

- GIVEN a process stored an attempting attempt and stopped before recording the dispatcher result
- WHEN reconciliation opens the store
- THEN the attempt and its reservation MUST be recorded as uncertain with an unknown outcome and an observation receipt
- AND reconciliation MUST NOT retry the effect
