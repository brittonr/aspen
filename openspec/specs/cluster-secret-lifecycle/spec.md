## ADDED Requirements

### Requirement: Secret creation at cluster init

A cluster root secret MUST be created during `init_cluster`, and each initial member MUST persist only its assigned share by applying the committed trust-init request for the current epoch.

#### Scenario: Single-node cluster init

- **WHEN** a 1-node cluster is initialized with trust enabled
- **THEN** a root secret is created, a single share (K=1) is stored locally, and the share digest is recorded in cluster metadata

#### Scenario: Multi-node cluster init

- **WHEN** a 3-node cluster is initialized with trust enabled
- **THEN** the Raft leader creates a root secret, splits it into 3 shares with K=2, submits one committed trust-init request for epoch 1, and each node stores only its own assigned share when that request is applied locally

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

Share distribution MUST happen through Raft-committed application entries, not out-of-band storage writes, so share assignment is part of the consensus log and followers do not depend on leader-local side effects.

#### Scenario: Leader distributes shares

- **WHEN** the Raft leader creates shares during init
- **THEN** each node's share is included in the committed trust-init request together with the epoch digests

#### Scenario: Follower applies trust-init request

- **WHEN** a follower applies the committed trust-init request for its cluster epoch
- **THEN** it stores its own share in `trust_shares`, stores the epoch digests, and does not require a direct write from the leader
