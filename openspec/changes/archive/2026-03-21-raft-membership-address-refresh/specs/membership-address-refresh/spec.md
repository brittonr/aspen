## ADDED Requirements

### Requirement: Gossip-triggered membership address update

When gossip discovers a peer with the same endpoint ID but different socket addresses compared to the current Raft membership, the leader updates the membership with the new addresses.

#### Scenario: Node restarts with new port

- **WHEN** a node restarts and gossip announces its new address (same endpoint ID, different socket addresses)
- **THEN** the leader calls `add_learner` with updated `RaftMemberInfo` containing the new socket addresses
- **THEN** subsequent `new_client()` calls receive the correct address from Raft membership without needing the gossip fallback

#### Scenario: Follower receives gossip announcement

- **WHEN** a follower node receives a gossip announcement with a changed peer address
- **THEN** it updates the local gossip cache (existing behavior) but does NOT attempt a Raft membership update
- **THEN** no error is logged

#### Scenario: Leader receives duplicate gossip announcement

- **WHEN** the leader receives a gossip announcement with the same address that was already submitted to `add_learner` within the debounce window (60 seconds)
- **THEN** the update is skipped to avoid redundant Raft log entries

#### Scenario: Address matches current membership

- **WHEN** gossip announces an address that matches what's already in the Raft membership
- **THEN** no `add_learner` call is made

### Requirement: Debounced updates

Address updates to Raft membership are debounced to prevent log spam during rapid gossip announcements.

#### Scenario: Multiple gossip ticks for same address

- **WHEN** gossip announces the same address for the same node multiple times within 60 seconds
- **THEN** only the first triggers an `add_learner` call

#### Scenario: Address changes again within debounce window

- **WHEN** a node restarts twice within 60 seconds (different addresses each time)
- **THEN** each distinct address triggers an `add_learner` call (the debounce key includes the address hash, not just the node ID)
