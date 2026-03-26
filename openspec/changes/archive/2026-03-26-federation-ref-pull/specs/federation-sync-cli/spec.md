## MODIFIED Requirements

### Requirement: CLI federation sync command

The system SHALL provide a `federation sync` CLI subcommand that triggers a one-shot federation sync pull from a specified remote cluster peer. When the `--fetch` flag is provided, the system SHALL additionally fetch ref objects for each discovered resource and persist them locally.

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

#### Scenario: Sync with fetch flag

- **WHEN** the user runs `aspen-cli federation sync --peer <node-id> --fetch`
- **THEN** the CLI SHALL first perform the standard sync (discover refs)
- **AND** for each discovered resource, the CLI SHALL send a `FederationFetchRefs` RPC request
- **AND** the CLI SHALL display per-resource fetch results (fetched count, already present, errors)
