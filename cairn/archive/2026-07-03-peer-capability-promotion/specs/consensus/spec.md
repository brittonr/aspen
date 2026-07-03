## ADDED Requirements

### Requirement: Generic peer promotion cannot grant Raft membership
r[molten.peer_promotion.raft_boundary] Molten MUST NOT allow generic peer capability promotion to grant Raft voter, non-voter, learner, control-plane membership, or linearizable-read roles without the separate Raft membership admission and read-index/read-capability gates.

#### Scenario: Promotion to learner denies outside membership path
- GIVEN a peer promotion grant requests promotion to a Raft learner role
- WHEN generic peer promotion validates the request
- THEN validation denies the generic promotion path
- AND diagnostics direct the operator to the Raft membership admission preflight.
