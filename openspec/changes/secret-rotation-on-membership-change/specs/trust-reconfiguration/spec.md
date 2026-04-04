## ADDED Requirements

### Requirement: Secret rotation on membership change

A new cluster secret MUST be generated and shares redistributed whenever Raft membership changes (add voter, remove voter, joint consensus).

#### Scenario: Add a voter

- **WHEN** a new voter is added to a 3-node cluster
- **THEN** the leader collects K old shares, generates a new secret, creates 4 new shares (K=3), and distributes them to all members including the new one

#### Scenario: Remove a voter

- **WHEN** a voter is removed from a 5-node cluster
- **THEN** the leader collects K old shares, generates a new secret, creates 4 new shares (K=3), and the removed node's old share is useless for the new epoch

#### Scenario: Leader crash during reconfiguration

- **WHEN** the leader fails after collecting old shares but before committing the new configuration
- **THEN** the new leader detects the incomplete reconfiguration and restarts the process from scratch

### Requirement: Encrypted secret chain

Each epoch's configuration MUST carry all prior secrets encrypted with the current epoch's secret.

#### Scenario: Decrypt historical secret

- **WHEN** a node at epoch 3 needs to derive a key from the epoch 1 secret
- **THEN** it reconstructs the epoch 3 secret from shares, decrypts the chain to recover epoch 1 and epoch 2 secrets

#### Scenario: Chain integrity

- **WHEN** the encrypted secret chain is tampered with (bit flip)
- **THEN** decryption fails with an authenticated encryption error (ChaCha20Poly1305 tag mismatch)

### Requirement: Epoch alignment with Raft

Trust epochs MUST correspond to Raft log indices where membership+trust entries are committed.

#### Scenario: Epoch ordering

- **WHEN** two membership changes are committed at log indices 100 and 200
- **THEN** trust epochs are 100 and 200, and epoch 200's chain contains the secret from epoch 100

### Requirement: Share collection protocol

The leader MUST collect K shares from old configuration members before generating new shares.

#### Scenario: Sufficient shares available

- **WHEN** K of the old members respond to `GetShare(old_epoch)` requests
- **THEN** the leader reconstructs the old secret and proceeds to generate the new configuration

#### Scenario: Insufficient shares (timeout)

- **WHEN** fewer than K old members respond within the collection timeout
- **THEN** the reconfiguration fails and can be retried when more nodes are available

#### Scenario: Invalid share rejected

- **WHEN** a node responds with a share whose SHA3-256 digest does not match the stored digest
- **THEN** the share is rejected and the leader continues collecting from other nodes

### Requirement: Reconfiguration as sans-IO state machine

The reconfiguration coordinator MUST be implemented as a sans-IO state machine with states: `CollectingOldShares` → `Preparing` → `Committed`.

#### Scenario: State machine driven by shell

- **WHEN** the shell feeds share responses into the coordinator state machine
- **THEN** the state machine transitions states and produces outbound messages (GetShare requests, Raft proposals) without performing I/O directly
