## MODIFIED Requirements

### Requirement: Federation services start at boot when configured

The system SHALL create and start federation services during node bootstrap when a cluster secret key is present in the node configuration. When no cluster secret key is configured, federation SHALL be disabled and no federation ALPN handler SHALL be registered.

The federation ALPN handler SHALL be reachable by remote peers over iroh QUIC, enabling cross-cluster federation sync connections.

#### Scenario: Node boots with federation configured

- **WHEN** the node configuration contains a `cluster_secret_key`
- **THEN** the bootstrap SHALL create a `ClusterIdentity` from the secret key
- **AND** the bootstrap SHALL create a `TrustManager` pre-populated with `trusted_clusters` from config
- **AND** the bootstrap SHALL create a `FederationProtocolHandler`
- **AND** the handler SHALL be registered on the iroh Router at ALPN `/aspen/federation/1`

#### Scenario: Node boots without federation configured

- **WHEN** the node configuration has no `cluster_secret_key`
- **THEN** the bootstrap SHALL skip federation initialization
- **AND** no federation ALPN handler SHALL be registered on the iroh Router
- **AND** the node SHALL operate normally for all non-federation functionality

#### Scenario: Invalid cluster secret key

- **WHEN** the node configuration contains a `cluster_secret_key` that is not valid 64-character hex
- **THEN** the bootstrap SHALL log an error with the validation failure reason
- **AND** the bootstrap SHALL skip federation initialization (same as unconfigured)
- **AND** the node SHALL continue to start without federation

#### Scenario: Remote peer connects to federation handler

- **WHEN** a remote cluster connects to this node's iroh endpoint at ALPN `/aspen/federation/1`
- **AND** the remote cluster performs a valid federation handshake
- **THEN** the handler SHALL respond with this cluster's signed identity
- **AND** the handler SHALL serve resource state and object sync requests

#### Scenario: Cross-cluster federation sync in NixOS VMs

- **WHEN** two independent Aspen clusters run in separate NixOS VMs
- **AND** both have federation enabled with different cluster keys
- **AND** cluster B connects to cluster A's iroh endpoint
- **THEN** cluster B SHALL successfully complete a federation handshake with cluster A
- **AND** cluster B SHALL receive cluster A's Forge repository ref heads
