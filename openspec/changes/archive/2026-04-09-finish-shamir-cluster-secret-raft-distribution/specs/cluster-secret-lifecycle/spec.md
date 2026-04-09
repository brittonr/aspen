## MODIFIED Requirements

### Requirement: Secret creation at cluster init

A cluster root secret MUST be created during `init_cluster`, and each initial member MUST persist only its assigned share by applying the committed trust-init request for the current epoch.

#### Scenario: Single-node cluster init

- **WHEN** a 1-node cluster is initialized with trust enabled
- **THEN** a root secret is created, a single share (K=1) is stored locally, and the share digest is recorded in cluster metadata

#### Scenario: Multi-node cluster init

- **WHEN** a 3-node cluster is initialized with trust enabled
- **THEN** the Raft leader creates a root secret, splits it into 3 shares with K=2, submits one committed trust-init request for epoch 1, and each node stores only its own assigned share when that request is applied locally

### Requirement: Share distribution via Raft

Share distribution MUST happen through Raft-committed application entries, not out-of-band storage writes, so share assignment is part of the consensus log and followers do not depend on leader-local side effects.

#### Scenario: Leader distributes shares

- **WHEN** the Raft leader creates shares during init
- **THEN** each node's share is included in the committed trust-init request together with the epoch digests

#### Scenario: Follower applies trust-init request

- **WHEN** a follower applies the committed trust-init request for its cluster epoch
- **THEN** it stores its own share in `trust_shares`, stores the epoch digests, and does not require a direct write from the leader
