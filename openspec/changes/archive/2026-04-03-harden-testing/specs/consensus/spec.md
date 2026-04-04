## ADDED Requirements

### Requirement: Split-brain heal madsim scenario

The system SHALL include a madsim test that partitions a cluster into two groups, allows both sides to elect leaders, then heals the partition and verifies the cluster converges to a single leader with no data loss.

#### Scenario: Partition and heal with writes on both sides

- **WHEN** a 5-node cluster is partitioned into {1,2} and {3,4,5}
- **AND** writes are attempted on both sides
- **THEN** only the majority side {3,4,5} SHALL accept writes
- **AND** when the partition heals, nodes 1 and 2 SHALL catch up to the majority's log
- **AND** all committed writes SHALL be readable from any node

#### Scenario: Minority side rejects writes

- **WHEN** a client submits writes to a node in the minority partition
- **THEN** those writes SHALL fail (no leader in minority)
- **AND** after partition heal, the minority nodes SHALL rejoin as followers

### Requirement: Slow follower madsim scenario

The system SHALL include a madsim test where one follower processes AppendEntries at 100x slower rate than other followers, verifying the cluster continues operating and the slow follower eventually catches up.

#### Scenario: Cluster remains available with slow follower

- **WHEN** node 3 in a 5-node cluster has 100x latency on AppendEntries processing
- **THEN** the cluster SHALL continue accepting writes (nodes 1,2,4,5 form majority)
- **AND** node 3 SHALL eventually catch up when the slowdown is removed

#### Scenario: Slow follower does not block commits

- **WHEN** 100 writes are submitted while node 3 is slow
- **THEN** all 100 writes SHALL be committed without waiting for node 3
- **AND** the commit latency SHALL not be affected by node 3's slowness

### Requirement: Snapshot during membership change madsim scenario

The system SHALL include a madsim test that triggers a snapshot while a learner is being added to the cluster, verifying the snapshot completes and the learner receives consistent state.

#### Scenario: Learner added during snapshot

- **WHEN** a snapshot is in progress on the leader
- **AND** a new learner is added to the cluster
- **THEN** the learner SHALL receive either the in-progress snapshot or a new one
- **AND** the learner's state SHALL be consistent with the leader's committed state

#### Scenario: Membership change during snapshot transfer

- **WHEN** a snapshot is being transferred to a follower
- **AND** a membership change is proposed
- **THEN** the snapshot transfer SHALL complete
- **AND** the membership change SHALL be applied after the snapshot is installed

### Requirement: Clock-skewed TTL madsim scenario

The system SHALL include a madsim test where nodes have divergent clocks and verify that TTL-based operations (lock expiry, lease renewal) behave correctly despite clock skew.

#### Scenario: Lock expiry with clock skew

- **WHEN** node A acquires a lock with TTL 10 seconds
- **AND** node B's clock is 5 seconds ahead of node A's clock
- **THEN** the lock expiry SHALL be determined by the leader's clock
- **AND** the lock SHALL NOT be considered expired on node B before the leader considers it expired

#### Scenario: Lease renewal under clock skew

- **WHEN** a lease is renewed with a new TTL
- **AND** the renewing node's clock is behind the leader's clock
- **THEN** the leader SHALL compute the new deadline using its own clock
- **AND** the lease SHALL remain valid for the full TTL from the leader's perspective
