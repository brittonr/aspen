## ADDED Requirements

### Requirement: Generic capability tokens do not replace membership admission
r[molten.capability_token.no_generic_membership] Molten MUST NOT allow generic capability tokens or proofsets to replace Raft membership preflight, quorum-safety predicate receipts, read-index evidence, or membership commit receipts.

#### Scenario: Membership token supports request only
- GIVEN a peer presents a capability token permitting it to request Raft membership preflight
- WHEN membership admission evaluates the peer
- THEN the token can satisfy only the request authority input
- AND separate membership preflight, quorum-safety, and commit evidence remain required before membership changes.
