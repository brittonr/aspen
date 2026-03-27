## MODIFIED Requirements

### Requirement: Issue federation tokens

The system SHALL allow cluster administrators to issue capability tokens for federation, signed by the cluster's long-lived Ed25519 key. Tokens SHALL be scoped using `FederationPull` and `FederationPush` capability variants with repo prefix constraints. Issued tokens SHALL be stored in KV at `_sys:fed:token:issued:<audience_hex>` for tracking and revocation.

#### Scenario: Issue a pull-only federation token

- **WHEN** an administrator issues a token for Cluster B with `FederationPull { repo_prefix: "forge:" }`
- **THEN** the token SHALL have `issuer` = local cluster public key
- **AND** `audience` = Cluster B's public key
- **AND** the token SHALL be signed by the cluster's secret key
- **AND** the token SHALL be stored in KV at `_sys:fed:token:issued:<cluster_b_hex>`

#### Scenario: Issue a push+pull federation token

- **WHEN** an administrator issues a token with both `FederationPull` and `FederationPush` capabilities
- **THEN** the credential holder SHALL be authorized for both pull and push operations

#### Scenario: Issue a delegatable federation token

- **WHEN** an administrator issues a token with `FederationPull { repo_prefix: "forge:" }` and `Delegate` capability
- **THEN** Cluster B SHALL be able to create child tokens for other clusters
- **AND** child tokens SHALL be constrained to subsets of the parent's `FederationPull` scope

### Requirement: Credentials stored and auto-loaded for outbound connections

Received federation credentials SHALL be stored in KV at `_sys:fed:token:received:<issuer_hex>`. The sync orchestrator SHALL load stored credentials when initiating outbound federation connections.

#### Scenario: Store received credential

- **WHEN** a cluster receives a federation token from another cluster
- **AND** the credential passes verification
- **THEN** the credential SHALL be serialized and stored in KV

#### Scenario: Auto-load credential for outbound connection

- **WHEN** the sync orchestrator connects to a remote cluster
- **AND** a valid (non-expired) credential exists in KV for that cluster
- **THEN** the orchestrator SHALL load the credential and pass it to `connect_to_cluster`

#### Scenario: Skip expired credential

- **WHEN** the sync orchestrator finds a credential in KV for a remote cluster
- **AND** the credential's token has expired
- **THEN** the orchestrator SHALL connect without a credential (credential: None)
- **AND** the expired credential SHALL remain in KV until cleanup

### Requirement: Token revocation

The system SHALL support revoking federation tokens. Revocation SHALL be stored in KV via `RevocationStore`. Revoked tokens SHALL be rejected during credential verification on the issuing cluster.

#### Scenario: Revoke an issued token

- **WHEN** an administrator revokes a token by its BLAKE3 hash
- **THEN** subsequent handshakes presenting that token SHALL fail verification
- **AND** the revocation record SHALL be stored in KV

#### Scenario: Revoke affects active session

- **WHEN** a token is revoked while a federation session is active
- **AND** the remote peer attempts a new stream on the same connection
- **THEN** the handler SHALL reject the request (session credential check fails on next use)
