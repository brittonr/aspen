## ADDED Requirements

### Requirement: Gossip address override in Raft network factory

When creating a network client for a target node, the factory SHALL check the gossip-populated `peer_addrs` cache before using the `RaftMemberInfo` address. If the cache contains an address with the same endpoint ID but different socket addresses, the cache address SHALL be used.

#### Scenario: Restarted node has new port

- **WHEN** node 2 restarts and binds port 45678 (was 12345), and gossip delivers the new address to node 1's peer_addrs cache
- **THEN** node 1's `new_client(2, raft_member_info)` uses the gossip address (port 45678) instead of the stale Raft membership address (port 12345)

#### Scenario: Endpoint ID mismatch rejected

- **WHEN** the gossip cache contains an address for node 2 with a different endpoint ID than what Raft membership records
- **THEN** the factory SHALL use the Raft membership address and log a warning about the mismatch

#### Scenario: No gossip entry falls back to Raft membership

- **WHEN** the gossip cache has no entry for the target node
- **THEN** the factory SHALL use the `RaftMemberInfo` address (existing behavior)

### Requirement: Eager gossip announcement on startup

After a node completes gossip setup during bootstrap, it SHALL immediately broadcast a `PeerAnnouncement` with its current endpoint address. This MUST happen before the node begins processing Raft RPCs.

#### Scenario: Node restarts and announces within 5 seconds

- **WHEN** a node restarts with the same secret key (same endpoint ID) but a new port
- **THEN** other nodes in the cluster receive the updated address via gossip within 5 seconds of the restarted node's gossip setup completing

### Requirement: Cluster reconnection after node restart

A node that restarts with the same secret key SHALL rejoin the cluster and participate in Raft consensus without manual intervention. Other nodes SHALL route Raft RPCs to the restarted node's new address.

#### Scenario: Follower restart during rolling deploy

- **WHEN** a follower is stopped and restarted in a 3-node cluster
- **THEN** the cluster reaches quorum with the restarted follower within 30 seconds, and the restarted node receives replicated log entries

#### Scenario: Leader restart with re-election

- **WHEN** the leader is stopped and restarted in a 3-node cluster
- **THEN** a new leader is elected among the remaining nodes, and the restarted node rejoins as a follower receiving log entries from the new leader
