## ADDED Requirements

### Requirement: Peer roles resolve capability tokens at use time
r[molten.capability_token.peer_roles] Molten MUST require subscriber, publisher, relay, sync participant, job worker, node-control operator, and peer promotion roles to reference capability tokens or proofsets that are resolved by capability admission at use time.

#### Scenario: Publisher role requires write token admission
- GIVEN a peer session records a requested publisher role for one topic
- WHEN the peer attempts to publish
- THEN Molten resolves the referenced write token or proofset for that peer/session/topic
- AND publish denies if capability admission does not pass.

### Requirement: Capability validation is reproducible
r[molten.capability_token.validation] Molten SHOULD validate the capability-token spine with focused capability tests, peer-session tests, subscriber tests, promotion tests, authority tests, consensus boundary tests, formatting, and Cairn validation before archiving.

#### Scenario: Transport-as-token regression fails validation
- GIVEN a regression accepts a transport receipt as capability authority
- WHEN focused capability-token validation runs
- THEN the negative transport-as-token fixture fails
- AND the change cannot complete until transport receipts are classified as evidence only.
