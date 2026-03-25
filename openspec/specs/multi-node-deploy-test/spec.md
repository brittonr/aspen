## ADDED Requirements

### Requirement: 3-node cluster formation

The VM test SHALL boot 3 separate NixOS VMs, each running an `aspen-node` instance with distinct node IDs (1, 2, 3) and deterministic iroh secret keys. Node 1 SHALL initialize the cluster, add nodes 2 and 3 as learners, and promote all to voters. The cluster SHALL report healthy with 3 members before proceeding.

#### Scenario: Cluster forms successfully

- **WHEN** 3 VMs are booted and node 1 runs `cluster init`, `cluster add-learner` for nodes 2 and 3, and `cluster change-membership 1 2 3`
- **THEN** `cluster health` reports healthy with 3 voter members

#### Scenario: KV replication works across nodes

- **WHEN** a KV write is performed on the leader
- **THEN** the value is readable from any node in the cluster

### Requirement: Forge push and CI build on multi-node cluster

The test SHALL create a Forge repo on the 3-node cluster, push source code, and wait for the CI pipeline to complete. The CI build SHALL produce a Nix store path with a runnable binary.

#### Scenario: CI pipeline completes on 3-node cluster

- **WHEN** source is pushed to a Forge repo on the 3-node cluster with CI auto-trigger enabled
- **THEN** the CI pipeline completes with status "success" and produces at least one output path

### Requirement: Rolling deploy across 3 nodes

The test SHALL deploy the CI-built binary to all 3 nodes using script-level rolling deploy: stop each node process, restart with the new binary, and wait for health before proceeding to the next node. Followers (nodes 2 and 3) SHALL be upgraded before the leader (node 1).

#### Scenario: Follower-first upgrade ordering

- **WHEN** rolling deploy is initiated on the 3-node cluster
- **THEN** nodes 2 and 3 are stopped and restarted before node 1

#### Scenario: Cluster remains available during follower upgrades

- **WHEN** node 2 is stopped for upgrade while nodes 1 and 3 are running
- **THEN** the cluster still accepts KV writes (quorum of 2/3 maintained)

#### Scenario: All nodes running new binary after deploy

- **WHEN** rolling deploy completes for all 3 nodes
- **THEN** all 3 node processes are running with the CI-built binary, cluster health reports healthy with 3 members, and a KV round-trip (write + read) succeeds

### Requirement: State persistence across restart

Raft state, KV data, and Forge repos SHALL survive node restarts during deploy. Data written before deploy SHALL be readable after all nodes are restarted with the new binary.

#### Scenario: KV data persists through deploy

- **WHEN** a KV key is written before deploy and all 3 nodes are restarted during deploy
- **THEN** the key is readable from the cluster after deploy completes

### Requirement: Cluster re-election after leader restart

When the leader (node 1) is stopped for upgrade, the remaining 2 nodes SHALL elect a new leader. After node 1 restarts with the new binary, it SHALL rejoin the cluster as a follower or win re-election, and the cluster SHALL return to 3 healthy voters.

#### Scenario: New leader elected during leader upgrade

- **WHEN** node 1 (leader) is stopped for upgrade while nodes 2 and 3 are running with the new binary
- **THEN** nodes 2 and 3 elect a new leader and the cluster continues accepting writes
