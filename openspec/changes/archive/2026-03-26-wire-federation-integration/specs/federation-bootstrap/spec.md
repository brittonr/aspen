## ADDED Requirements

### Requirement: Federation services start at boot when configured

The system SHALL create and start federation services during node bootstrap when a cluster secret key is present in the node configuration. When no cluster secret key is configured, federation SHALL be disabled and no federation ALPN handler SHALL be registered.

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

### Requirement: Federation ALPN registration follows RouterBuilder pattern

The `RouterBuilder` SHALL provide a `federation()` method that accepts a `FederationProtocolHandler` and registers it at the `/aspen/federation/1` ALPN, following the same pattern as `auth_raft()`, `client()`, `blobs()`, and other protocol registrations.

#### Scenario: Register federation handler via RouterBuilder

- **WHEN** a caller invokes `builder.federation(handler)`
- **THEN** the handler SHALL be registered at ALPN `/aspen/federation/1`
- **AND** the method SHALL log registration at info level
- **AND** the method SHALL return the builder for fluent chaining

### Requirement: Federation bootstrap creates identity deterministically

The `ClusterIdentity` created during bootstrap SHALL be deterministic given the same secret key. The identity SHALL persist across node restarts when the same configuration is used.

#### Scenario: Same key produces same identity

- **WHEN** two nodes boot with the same `cluster_secret_key`
- **THEN** both SHALL produce the same `ClusterIdentity` with the same public key

#### Scenario: Identity persists across restarts

- **WHEN** a node restarts with the same configuration
- **THEN** the cluster identity public key SHALL be identical to the previous run

### Requirement: TrustManager initialized from config

The `TrustManager` created during bootstrap SHALL be pre-populated with all cluster public keys listed in `trusted_clusters` configuration, each at `TrustLevel::Trusted`.

#### Scenario: Trusted clusters from config

- **WHEN** the configuration lists public keys in `trusted_clusters`
- **THEN** the `TrustManager` SHALL contain each key at `TrustLevel::Trusted`
- **AND** unlisted cluster keys SHALL have `TrustLevel::Public`

#### Scenario: Empty trusted clusters

- **WHEN** the configuration has an empty `trusted_clusters` list
- **THEN** the `TrustManager` SHALL be created with no pre-trusted clusters
- **AND** all remote clusters SHALL have `TrustLevel::Public`

### Requirement: Federation handler shutdown

When the node shuts down, federation resources SHALL be released cleanly. The `FederationProtocolHandler` connection semaphore SHALL reject new connections during shutdown.

#### Scenario: Clean shutdown

- **WHEN** the node receives a shutdown signal
- **THEN** the federation handler SHALL stop accepting new connections
- **AND** in-progress sync streams SHALL be allowed to complete or timeout
- **AND** no federation resources SHALL leak
