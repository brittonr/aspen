# Runtime spine: remote assertion ownership delta

## ADDED Requirements

### Requirement: Remote assertions are session-owned
r[molten.runtime_spine.remote_assertion_ownership] Molten MUST own a delivered remote assertion in the receiving session scope, record that owning session ref in the applied assertion record, retract session-owned assertions on session close or disconnect before the session identity can be reused, deny an envelope whose declared owner is unknown or from a closed session, and MUST NOT resurrect a closed session's assertions from replay.

#### Scenario: Disconnect retracts the peer's facts
- GIVEN a peer session that asserted a fact which the local runtime applied
- WHEN the session closes or disconnects
- THEN the assertion retracts through owner-scope cleanup and matching observers receive the retraction.

#### Scenario: Late delivery denies
- GIVEN an envelope that declares an owner whose session already closed
- WHEN the envelope is admitted
- THEN it denies before staging and no state changes.

#### Scenario: Reconnect requires re-assertion
- GIVEN a peer that reconnects with a new session identity
- WHEN the local runtime replays the previous session's deliveries
- THEN no assertion from the closed session reappears and the peer must assert again.

#### Scenario: Owner is readable from evidence
- GIVEN an applied remote assertion record
- WHEN an operator reads the record
- THEN the owning session ref is present and matches the session that delivered it.
