## ADDED Requirements

### Requirement: Secret creation at cluster init

A cluster root secret MUST be created and shares distributed to all initial members during `init_cluster`.

#### Scenario: Single-node cluster init

- **WHEN** a 1-node cluster is initialized with trust enabled
- **THEN** a root secret is created, a single share (K=1) is stored locally, and the share digest is recorded in cluster metadata

#### Scenario: Multi-node cluster init

- **WHEN** a 3-node cluster is initialized with trust enabled
- **THEN** the Raft leader creates a root secret, splits it into 3 shares with K=2, distributes shares to each member, and commits share digests to Raft state

### Requirement: Share persistence

Each node's share MUST be stored in a dedicated redb table (`trust_shares`) keyed by epoch.

#### Scenario: Node restarts

- **WHEN** a node restarts after receiving its share
- **THEN** the share is loaded from redb and available for reconstruction without re-distribution

#### Scenario: Share table isolation

- **WHEN** application KV data is read or scanned
- **THEN** the `trust_shares` table is never included in application-level results

### Requirement: Threshold configuration

The share threshold MUST default to `(N/2) + 1` (majority) and be configurable at cluster init time.

#### Scenario: Default threshold for 5-node cluster

- **WHEN** a 5-node cluster is initialized without explicit threshold
- **THEN** the threshold is set to 3

#### Scenario: Custom threshold

- **WHEN** a cluster is initialized with `--trust-threshold 4` for a 5-node cluster
- **THEN** the threshold is 4 and 4 shares are required for reconstruction

#### Scenario: Invalid threshold rejected

- **WHEN** a threshold of 0, or greater than N, is specified
- **THEN** cluster init fails with an error

### Requirement: Quorum reconstruction

The cluster secret MUST be reconstructable when at least K nodes are available and provide their shares.

#### Scenario: Reconstruct after node failure

- **WHEN** 1 node in a 3-node cluster (K=2) is down
- **THEN** the remaining 2 nodes can reconstruct the cluster secret

#### Scenario: Reconstruction fails below threshold

- **WHEN** fewer than K nodes are available
- **THEN** reconstruction returns an error indicating insufficient shares

### Requirement: Share distribution via Raft

Share distribution MUST happen through Raft-committed entries, not out-of-band channels, so that share assignment is part of the consensus log.

#### Scenario: Leader distributes shares

- **WHEN** the Raft leader creates shares during init
- **THEN** each node's share is included in a Raft log entry addressed to that node, and the node stores it upon applying the entry
