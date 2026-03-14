## ADDED Requirements

### Requirement: Issue federation tokens

The system SHALL allow cluster administrators to issue capability tokens for federation, signed by the cluster's long-lived Ed25519 key. Tokens SHALL be scoped to specific KV prefixes and optionally allow further delegation.

#### Scenario: Issue a read-only federation token

- **WHEN** an administrator issues a token for Cluster B with `Read{prefix: "_sys:nix-cache:"}`
- **THEN** the token SHALL have `issuer` = local cluster public key
- **AND** `audience` = Cluster B's public key
- **AND** the token SHALL be signed by the cluster's secret key

#### Scenario: Issue a delegatable federation token

- **WHEN** an administrator issues a token with `Read{prefix: "_sys:nix-cache:"}` and `Delegate` capability
- **THEN** Cluster B SHALL be able to create child tokens for other clusters
- **AND** child tokens SHALL be constrained to subsets of the parent's capabilities

#### Scenario: Issue token with custom lifetime

- **WHEN** an administrator issues a token with lifetime 7 days
- **THEN** the token's `expires_at` SHALL be `issued_at + 7 days`
- **AND** the token SHALL fail verification after expiry (plus clock skew tolerance)

### Requirement: Token refresh protocol

The system SHALL support refreshing near-expiry federation tokens. A refresh request SHALL present the current credential, and the issuing cluster SHALL respond with a fresh token carrying the same capabilities and a new expiry.

#### Scenario: Refresh a valid near-expiry token

- **WHEN** a cluster presents a credential with a token expiring within the refresh window
- **AND** the token's issuer is the receiving cluster
- **AND** the token has not been revoked
- **THEN** the issuing cluster SHALL return a new token with the same capabilities and audience
- **AND** the new token's `expires_at` SHALL be set to current time + original lifetime duration

#### Scenario: Refuse refresh for revoked token

- **WHEN** a cluster presents a credential for refresh
- **AND** any token in the credential's chain has been revoked
- **THEN** the issuing cluster SHALL reject the refresh with a revocation error
- **AND** no new token SHALL be issued

#### Scenario: Refuse refresh from wrong audience

- **WHEN** Cluster C presents Cluster B's token (audience = Cluster B) for refresh
- **THEN** the issuing cluster SHALL reject with `WrongAudience`
- **AND** no new token SHALL be issued

#### Scenario: Auto-refresh during active sync sessions

- **WHEN** a subscription is actively syncing and the credential's token is within 20% of expiry
- **THEN** the system SHALL automatically attempt a refresh before the next sync
- **AND** if refresh succeeds, the subscription SHALL continue with the new credential
- **AND** if refresh fails, the subscription SHALL enter `NeedsRefresh` state

### Requirement: Token revocation

The system SHALL support revoking federation tokens. Revocation SHALL be immediate for the issuing cluster and best-effort propagated to connected clusters via gossip.

#### Scenario: Revoke a token locally

- **WHEN** an administrator revokes a federation token by its hash
- **THEN** the token hash SHALL be added to the local `RevocationStore`
- **AND** any subsequent verification of that token SHALL fail with `TokenRevoked`
- **AND** any sync requests presenting that token SHALL be rejected

#### Scenario: Gossip revocation to connected clusters

- **WHEN** a token is revoked on the issuing cluster
- **THEN** the revocation hash SHALL be broadcast via the federation gossip topic
- **AND** connected clusters receiving the gossip SHALL add the hash to their local `RevocationStore`
- **AND** clusters not currently connected SHALL receive the revocation on next gossip sync

#### Scenario: Token expires naturally without explicit revocation

- **WHEN** a federation token reaches its `expires_at` time
- **AND** no refresh has been issued
- **THEN** the token SHALL fail verification with `TokenExpired`
- **AND** no explicit revocation action SHALL be required

### Requirement: Graceful degradation for unrevoked tokens

The system SHALL accept a bounded window (up to token lifetime, default 24h) where a revoked token may still be valid on clusters that have not received the revocation gossip. Operators SHALL be able to configure shorter lifetimes for higher-security scenarios.

#### Scenario: Disconnected cluster uses revoked token

- **WHEN** Cluster A revokes a token issued to Cluster B
- **AND** Cluster C (which accepted a delegation from B) is offline and has not received the revocation gossip
- **THEN** Cluster C MAY continue to present the credential until the token's `expires_at`
- **AND** once the token expires, access SHALL be denied regardless of revocation state

#### Scenario: Configurable token lifetime for security

- **WHEN** an administrator issues a token with lifetime 1 hour for a high-security prefix
- **THEN** the maximum revocation window SHALL be 1 hour
- **AND** the token SHALL require refresh every hour to maintain access

### Requirement: List and inspect federation tokens

The system SHALL allow administrators to list active federation tokens issued by the cluster and inspect their capabilities, audience, expiry, and delegation status.

#### Scenario: List issued tokens

- **WHEN** an administrator runs `federation tokens list`
- **THEN** the system SHALL display all active (non-expired, non-revoked) tokens issued by this cluster
- **AND** each entry SHALL show audience, capabilities, expiry, and delegation depth

#### Scenario: Inspect a specific token

- **WHEN** an administrator inspects a token by its hash
- **THEN** the system SHALL display the full token details including issuer, audience, capabilities, facts, proof chain status, and time until expiry
