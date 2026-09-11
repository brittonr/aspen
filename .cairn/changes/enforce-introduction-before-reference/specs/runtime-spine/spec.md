# Runtime spine: reference introduction delta

## ADDED Requirements

### Requirement: Messages cannot introduce references
r[molten.runtime_spine.reference_introduction_rule] Molten MUST derive the introduced reference set of a session from that session's live assertions and established bootstrap references, MUST deny a message that carries a reference outside that set before delivery, and SHOULD refuse to send such a message.

#### Scenario: Assertion introduces a reference
- GIVEN a session holds a live assertion whose payload contains a reference
- WHEN a message from the same session carries that reference
- THEN the message is admitted and applied through the turn boundary.

#### Scenario: Unknown reference denies before delivery
- GIVEN a message whose payload carries a reference that no live session assertion introduced
- WHEN the receiving session admits the message
- THEN the message denies before delivery, the denial names the unknown reference, and no partial state commits.

#### Scenario: Retracting the introducing assertion withdraws the reference
- GIVEN a reference introduced only by one live assertion
- WHEN that assertion retracts
- THEN a later message that carries the reference denies again.

#### Scenario: Sender refuses an unintroduced reference
- GIVEN an outbound message that carries a reference outside the session's introduced set
- WHEN the envelope is built
- THEN the build refuses and emits denial evidence instead of sending the envelope.
