## MODIFIED Requirements

### Requirement: Federation Links

The system SHALL allow administrators to establish federation links between independent Aspen clusters. Links SHALL be authenticated via capability tokens presented during the federation handshake, replacing the previous iroh endpoint ID authentication.

When the `commit-dag-federation` feature is enabled, federation links SHALL additionally carry commit provenance metadata, enabling the importing cluster to verify that received KV state was produced by legitimate Raft consensus on the source cluster.

The `FederationResourceResolver` SHALL return real resource state from cluster storage. For Forge repositories, the resolver SHALL read ref heads from KV and return git objects from blob storage. The resolver SHALL NOT return stub responses when storage is available.

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

#### Scenario: Federation with commit verification

- **WHEN** the `commit-dag-federation` feature is enabled on both clusters
- **AND** Cluster B imports KV entries from Cluster A
- **THEN** Cluster B SHALL verify commit chain hashes for entries that include commit metadata
- **AND** verified commits SHALL have `verified: true` in their provenance record
- **AND** unverifiable entries (no commit metadata) SHALL be imported normally with no provenance record

#### Scenario: Federation with mixed capability peers

- **WHEN** Cluster A has `commit-dag-federation` enabled
- **AND** Cluster B does NOT have `commit-dag-federation` enabled
- **THEN** Cluster A SHALL export commit metadata alongside regular entries
- **AND** Cluster B SHALL import all entries (including `_sys:commit:` entries) as regular KV data
- **AND** no verification SHALL occur on Cluster B (feature not enabled)
- **AND** data sync SHALL function correctly regardless of feature mismatch

#### Scenario: Resource resolver returns real Forge data

- **WHEN** a federation sync request arrives for a Forge repository's `FederatedId`
- **AND** the repository exists on the local cluster with federation enabled
- **THEN** `get_resource_state()` SHALL return current ref heads read from KV storage
- **AND** `sync_objects()` SHALL return git objects from blob storage
- **AND** `resource_exists()` SHALL return `true`

#### Scenario: Resource resolver handles missing resources

- **WHEN** a federation sync request arrives for a `FederatedId` that does not match any local resource
- **THEN** `get_resource_state()` SHALL return `FederationResourceError::NotFound`
- **AND** `resource_exists()` SHALL return `false`

#### Scenario: Resource resolver respects federation settings

- **WHEN** a federation sync request arrives for a Forge repository
- **AND** the repository has `FederationMode::Disabled`
- **THEN** `get_resource_state()` SHALL return `FederationResourceError::FederationDisabled`

#### Scenario: Cross-cluster Forge sync end-to-end

- **WHEN** Cluster A creates a Forge repository and federates it
- **AND** Cluster B connects to Cluster A via the federation sync protocol
- **AND** Cluster B requests the repository's resource state
- **THEN** Cluster B SHALL receive the ref heads that Cluster A has
- **AND** the ref names and hashes SHALL match exactly
