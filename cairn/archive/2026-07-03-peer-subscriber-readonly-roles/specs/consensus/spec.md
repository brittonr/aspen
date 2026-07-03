## ADDED Requirements

### Requirement: Subscriber peers are not Raft members
r[molten.peer_subscriber.raft_boundary] Molten MUST NOT treat subscriber, observer, or read-only peer roles as Raft voters, non-voters, learners, or linearizable read replicas without separate membership admission and read-index/read-capability evidence.

#### Scenario: Subscriber cannot serve linearizable read by role alone
- GIVEN a peer has a read-only subscription grant for control-plane status summaries
- WHEN a client asks that peer to serve a linearizable control-plane read
- THEN Molten denies unless separate read-index and read-capability evidence is present
- AND the subscriber role is not recorded as Raft membership.
