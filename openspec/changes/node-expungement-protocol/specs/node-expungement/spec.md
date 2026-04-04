## ADDED Requirements

### Requirement: Persistent expungement marker

When a node is expunged, it MUST record `ExpungedMetadata { epoch, removed_by }` in persistent storage. This marker survives reboots and is permanent.

#### Scenario: Node receives Expunged message

- **WHEN** a node receives `Expunged(epoch)` from a peer or the Raft leader
- **THEN** it records `ExpungedMetadata` in redb, zeroizes its trust shares, and enters a terminal state

#### Scenario: Expunged node reboots

- **WHEN** an expunged node restarts
- **THEN** it loads the expunged marker from redb and remains in the expunged state without attempting to rejoin the cluster

### Requirement: Protocol rejection on expungement

An expunged node MUST reject all trust protocol messages and Raft RPCs.

#### Scenario: Expunged node receives GetShare

- **WHEN** an expunged node receives a `GetShare` request
- **THEN** it drops the message and logs a warning (does not respond with a share)

#### Scenario: Expunged node receives Raft AppendEntries

- **WHEN** an expunged node receives a Raft RPC
- **THEN** it rejects the RPC with an error indicating expungement

### Requirement: Peer-enforced expungement

When a node receives a request from a peer not in the current configuration, it MUST respond with `Expunged(current_epoch)` instead of the requested data.

#### Scenario: Removed node requests a share

- **WHEN** a node in configuration at epoch 5 receives `GetShare(epoch=4)` from a peer not in the epoch 5 member set
- **THEN** it responds with `Expunged(5)` and does not send the share

#### Scenario: Removed node requests during stale config

- **WHEN** a node that was removed at epoch 3 sends a request to a peer at epoch 5
- **THEN** the peer sends `Expunged(5)`, and the requesting node records its expungement

### Requirement: Share zeroization on expungement

When a node is expunged, it MUST immediately zeroize all trust shares in persistent storage.

#### Scenario: Shares destroyed on expunge

- **WHEN** a node records its expungement
- **THEN** all entries in the `trust_shares` redb table are overwritten with zeros then deleted

### Requirement: Factory reset for re-addition

An expunged node MUST NOT be re-addable to the cluster without a full data directory wipe.

#### Scenario: Attempt to re-add expunged node

- **WHEN** an operator tries to add a node that has an expunged marker
- **THEN** the node refuses to join and returns an error indicating factory reset is required

#### Scenario: Re-add after factory reset

- **WHEN** an operator wipes the data directory of an expunged node and starts it fresh
- **THEN** the node generates a new Iroh key and can be added as a new member

### Requirement: CLI expungement command

An `aspen-cli cluster expunge <node-id>` command MUST exist for operator-initiated removal.

#### Scenario: Operator expunges a node

- **WHEN** an operator runs `aspen-cli cluster expunge 3 --confirm`
- **THEN** the node is removed from Raft membership, trust reconfiguration is triggered, and `Expunged` is sent to the removed node
