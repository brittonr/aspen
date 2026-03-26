## ADDED Requirements

### Requirement: CLI federation sync command

The system SHALL provide a `federation sync` CLI subcommand that triggers a one-shot federation sync pull from a specified remote cluster peer.

#### Scenario: Successful sync pull

- **WHEN** the user runs `aspen-cli federation sync --peer <node-id>`
- **THEN** the CLI SHALL send a `FederationSyncPeer` RPC request to the local cluster
- **AND** the local cluster SHALL connect to the remote peer via iroh QUIC at `/aspen/federation/1`
- **AND** the cluster SHALL perform a federation handshake with its cluster identity
- **AND** the cluster SHALL query the remote peer's resource state
- **AND** the CLI SHALL display the discovered refs and resource metadata

#### Scenario: Remote peer unreachable

- **WHEN** the user runs `aspen-cli federation sync --peer <node-id>` and the peer is unreachable
- **THEN** the CLI SHALL display an error message indicating the connection failed
- **AND** the exit code SHALL be non-zero

### Requirement: FederationSyncPeer RPC request

The client RPC API SHALL include a `FederationSyncPeer` request that accepts a remote peer's node ID and returns the federation handshake result and resource state from the remote cluster.

#### Scenario: RPC returns remote resource state

- **WHEN** the RPC handler receives a `FederationSyncPeer` request with a valid peer node ID
- **THEN** the handler SHALL connect to the remote peer using the node's iroh endpoint
- **AND** the handler SHALL perform a federation handshake
- **AND** the handler SHALL query `GetResourceState` for discovered resources
- **AND** the response SHALL include the remote cluster's identity, trust status, and resource heads
