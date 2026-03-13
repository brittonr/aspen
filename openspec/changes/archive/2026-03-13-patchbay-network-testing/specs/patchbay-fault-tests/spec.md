## ADDED Requirements

### Requirement: Network partition via region link break

The system SHALL survive a full network partition between two regions and recover when the link is restored, using patchbay's `break_region_link()` and `restore_region_link()`.

#### Scenario: Majority partition maintains quorum

- **WHEN** a 3-node cluster spans two regions (2 EU, 1 US) and the inter-region link is broken
- **THEN** the 2-node EU partition maintains quorum, continues accepting writes, and the isolated US node detects loss of leader within its election timeout

#### Scenario: Partition heals and lagging node catches up

- **WHEN** the inter-region link is restored after 30 seconds of partition
- **THEN** the previously isolated node rejoins the cluster, receives all missed log entries, and returns consistent KV reads within 30 seconds of restoration

#### Scenario: Minority partition rejects writes

- **WHEN** the single US node is isolated and a client attempts a KV write against it
- **THEN** the write fails or times out because the node cannot reach quorum

### Requirement: Latency injection on active cluster

The system SHALL continue operating under injected latency, with Raft heartbeats and KV replication adapting to increased round-trip times.

#### Scenario: 200ms latency does not cause false elections

- **WHEN** 200ms latency is added to all links in a 3-node cluster via `set_link_condition()`
- **THEN** the existing leader remains leader (no spurious election) and KV writes still succeed within 15 seconds

#### Scenario: Latency exceeding election timeout triggers re-election

- **WHEN** latency is increased to 5000ms (exceeding Raft election timeout) on the leader's link
- **THEN** followers trigger a new election and a new leader is elected from nodes with functional links

### Requirement: Packet loss on active cluster

The system SHALL tolerate moderate packet loss through Raft retransmission and iroh's QUIC reliability.

#### Scenario: 10% packet loss does not break replication

- **WHEN** 10% packet loss is applied to all links in a 3-node cluster
- **THEN** a batch of 50 KV writes completes (all values readable from all nodes) within 60 seconds

#### Scenario: 50% packet loss degrades but does not deadlock

- **WHEN** 50% packet loss is applied to all links
- **THEN** the cluster does not deadlock — either writes eventually succeed (with retries) or fail with a timeout error, and the cluster recovers to normal operation when loss is removed

### Requirement: Link down/up on individual device

The system SHALL handle a single node losing its network link and recovering, simulating a network interface flap.

#### Scenario: Link down on follower

- **WHEN** `device.link_down("eth0")` is called on a follower node
- **THEN** the remaining 2 nodes maintain quorum and continue accepting writes

#### Scenario: Link up restores follower

- **WHEN** the follower's link is restored via `device.link_up("eth0")`
- **THEN** the follower rejoins the cluster and catches up on missed log entries within 30 seconds

#### Scenario: Link down on leader triggers failover

- **WHEN** `device.link_down("eth0")` is called on the current leader
- **THEN** a new leader is elected from the remaining nodes within the election timeout and KV writes succeed against the new leader

### Requirement: Dynamic NAT change mid-operation

The system SHALL handle a router's NAT mode changing at runtime, simulating a network reconfiguration.

#### Scenario: NAT mode switch from Public to Home

- **WHEN** a router switches from `Nat::None` to `Nat::Home` via `router.set_nat_mode()` while the cluster is running
- **THEN** iroh re-establishes connectivity (via relay if needed) and KV operations resume within 60 seconds
