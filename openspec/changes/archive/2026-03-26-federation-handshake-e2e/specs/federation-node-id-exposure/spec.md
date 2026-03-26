## ADDED Requirements

### Requirement: Health response includes iroh node ID

The `HealthResponse` SHALL include an `iroh_node_id` field containing the node's iroh public key as a base32-encoded string. The field SHALL be `None` only if the iroh endpoint is not yet bound.

#### Scenario: Health response contains iroh node ID after cluster init

- **WHEN** a node has completed startup and `cluster health --json` is called
- **THEN** the response JSON SHALL contain an `iroh_node_id` field with a non-empty base32 string

#### Scenario: Iroh node ID is usable for federation sync

- **WHEN** the `iroh_node_id` from cluster A's health response is passed to cluster B via `federation sync --peer <iroh_node_id>`
- **THEN** cluster B SHALL attempt a QUIC connection to cluster A using that public key

### Requirement: Federation sync uses iroh node ID for peer addressing

The `federation sync --peer` command SHALL accept the iroh node ID (base32 PublicKey) from the health response and use it as the target for the federation QUIC handshake.

#### Scenario: Federation handshake completes between two clusters

- **WHEN** cluster B calls `federation sync --peer <alice_iroh_node_id>` where the node ID was obtained from cluster A's health response
- **THEN** the federation protocol handler on cluster A SHALL receive the connection and complete the handshake, returning cluster A's identity and trust status
