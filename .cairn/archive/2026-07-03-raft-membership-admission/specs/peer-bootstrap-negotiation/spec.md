## ADDED Requirements

### Requirement: Peer bootstrap separates Raft membership roles
r[molten.raft_membership_admission.peer_boundary] Molten MUST require a separate Raft/control-plane membership-change command and membership receipt before any peer receives a voter, non-voter, learner, or control-plane membership role.

#### Scenario: Peer agreement admits gossip but not Raft
- GIVEN a peer agreement admits a peer for a gossip topic, docs namespace, protocol session, or job pool
- WHEN the peer requests Raft/control-plane membership
- THEN peer bootstrap evidence is treated only as supporting input
- AND a separate membership preflight and commit path remains required.

### Requirement: Raft membership admission validation is reproducible
r[molten.raft_membership_admission.validation] Molten SHOULD validate membership admission changes with focused consensus tests, peer-bootstrap boundary tests, formatting, and Cairn validation before archiving.

#### Scenario: Boundary regression fails validation
- GIVEN a regression lets a peer agreement alone satisfy a Raft membership role
- WHEN focused membership admission validation runs
- THEN the negative peer-bootstrap boundary test fails
- AND the change cannot complete until separate membership admission is restored.
