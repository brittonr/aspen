## MODIFIED Requirements

### Requirement: Federation Links

The system SHALL allow administrators to establish federation links between independent Aspen clusters. Links SHALL be authenticated via capability tokens presented during the federation handshake, replacing the previous iroh endpoint ID authentication.

#### Scenario: Establish federation link via token

- **WHEN** Cluster A issues a capability token to Cluster B
- **AND** Cluster B presents the token in a federation handshake
- **THEN** Cluster A SHALL verify the token signature, expiry, and audience
- **AND** upon successful verification, the federation link SHALL be established
- **AND** communication SHALL flow over iroh QUIC

#### Scenario: Asymmetric federation

- **WHEN** Cluster A issues a token to Cluster B but Cluster B does not issue a token to Cluster A
- **THEN** Cluster B MAY pull resources from A (within token's capability scope)
- **AND** Cluster A SHALL NOT pull resources from B (no credential)

#### Scenario: Legacy handshake fallback

- **WHEN** a remote cluster initiates a federation handshake without a credential
- **AND** the local cluster has the `federation-tokens` feature in optional mode
- **THEN** the system SHALL fall back to `TrustManager` trust level checks
- **AND** the system SHALL log a deprecation warning

### Requirement: Federation Policy

The system SHALL enforce configurable policies governing what resources are shared, with whom, and in which direction. Authorization SHALL be determined by the capabilities in the presented credential, not by cluster-level trust levels alone.

#### Scenario: Read-only federation enforced by token

- **WHEN** Cluster B holds a credential with only `Read{prefix: "_forge:repos:"}` capabilities
- **AND** Cluster B attempts a write-equivalent operation on Cluster A
- **THEN** the operation SHALL be rejected with `Unauthorized`

#### Scenario: Selective sharing via prefix scoping

- **WHEN** Cluster A issues a token to Cluster B with `Read{prefix: "_forge:repos:public:"}`
- **AND** Cluster B requests data under prefix `_forge:repos:private:`
- **THEN** the request SHALL be rejected because the prefix is not within the token's capability scope
- **AND** Cluster B SHALL only see resources under `_forge:repos:public:`

### Requirement: Federation Handshake

The federation sync protocol handshake SHALL include an optional `Credential` field. The handshake SHALL establish a session-scoped authorization context used for all subsequent sync requests on that connection.

#### Scenario: Handshake with credential

- **WHEN** a cluster initiates a federation handshake with a valid `Credential`
- **THEN** the receiving cluster SHALL verify the credential (offline, chain walk)
- **AND** store the verified capabilities as the session's authorization context
- **AND** respond with a successful handshake including its own identity

#### Scenario: Handshake with invalid credential

- **WHEN** a cluster initiates a federation handshake with an expired or invalid `Credential`
- **THEN** the receiving cluster SHALL reject the handshake
- **AND** close the connection
- **AND** no sync data SHALL be transferred

#### Scenario: Authorization check on sync requests

- **WHEN** a sync request (`ListResources`, `GetResourceState`, `SyncObjects`) is received
- **THEN** the handler SHALL check the session's credential capabilities against the requested prefix
- **AND** reject requests outside the credential's authorized scope with `Unauthorized`

### Requirement: TrustManager as derived state

The `TrustManager` SHALL derive trust levels from active credential state. A cluster with a valid, non-expired credential SHALL be considered `Trusted`. A cluster with a revoked credential SHALL be considered `Blocked`. A cluster with no credential SHALL be considered `Public`.

#### Scenario: Trust level derived from active credential

- **WHEN** Cluster B has an active federation credential from Cluster A
- **THEN** `TrustManager.trust_level(cluster_b_key)` SHALL return `Trusted`

#### Scenario: Trust level after credential expiry

- **WHEN** Cluster B's federation credential from Cluster A expires without refresh
- **THEN** `TrustManager.trust_level(cluster_b_key)` SHALL return `Public`

#### Scenario: Trust level after explicit revocation

- **WHEN** Cluster A revokes the credential issued to Cluster B
- **THEN** `TrustManager.trust_level(cluster_b_key)` SHALL return `Blocked`
