## ADDED Requirements

### Requirement: Self-contained credential type

The system SHALL provide a `Credential` type that bundles a `CapabilityToken` with its full delegation proof chain. The credential SHALL be verifiable offline without any server-side state or network calls.

#### Scenario: Credential with root token (no delegation)

- **WHEN** Cluster A issues a token directly to Cluster B (depth 0)
- **THEN** the credential SHALL contain the token and an empty proofs array
- **AND** verification SHALL succeed by checking `token.issuer` against trusted roots

#### Scenario: Credential with delegation chain

- **WHEN** Cluster A issues a token to Cluster B, and Cluster B delegates a subset to Cluster C
- **THEN** Cluster C's credential SHALL contain its leaf token and `proofs: [token_from_A_to_B]`
- **AND** verification SHALL walk the chain: leaf → parent → root
- **AND** each level SHALL verify signature, expiry, and capability attenuation

#### Scenario: Credential with max delegation depth

- **WHEN** a delegation chain reaches `MAX_DELEGATION_DEPTH` (8) levels
- **THEN** the credential SHALL be verifiable with all 8 parent tokens in the proofs array
- **AND** attempting to delegate further SHALL fail with `DelegationTooDeep`

### Requirement: Offline credential verification

The system SHALL verify credentials using only cryptographic operations on the credential contents. No server-side cache lookup, no network call, and no database query SHALL be required for verification.

#### Scenario: Verify credential from unknown cluster

- **WHEN** Cluster C presents a credential to Cluster A
- **AND** Cluster A has never communicated with Cluster C before
- **AND** the credential's proof chain roots at a token issued by Cluster A
- **THEN** Cluster A SHALL verify the credential successfully using only the chain contents
- **AND** no prior state about Cluster C SHALL be required

#### Scenario: Reject credential with broken chain

- **WHEN** a credential is presented where `token.proof` does not match the hash of `proofs[0]`
- **THEN** verification SHALL fail with `ParentTokenRequired` or equivalent chain error
- **AND** no access SHALL be granted

#### Scenario: Reject credential with capability escalation

- **WHEN** a delegated token in the chain has capabilities not contained by its parent
- **THEN** verification SHALL fail with `CapabilityEscalation`
- **AND** the specific escalated capability SHALL be identified in the error

#### Scenario: Reject credential with expired token in chain

- **WHEN** any token in the proof chain has expired (accounting for clock skew tolerance)
- **THEN** verification SHALL fail with `TokenExpired`
- **AND** the expired token's position in the chain SHALL be identifiable

### Requirement: Capability attenuation across delegation levels

Each delegated token in the chain SHALL have capabilities that are a subset of its parent token's capabilities. The `Capability::contains()` check SHALL be applied at each delegation level.

#### Scenario: Prefix narrowing across delegation

- **WHEN** Cluster A issues `Read{prefix: "_sys:nix-cache:"}` to Cluster B
- **AND** Cluster B delegates `Read{prefix: "_sys:nix-cache:narinfo:"}` to Cluster C
- **THEN** the chain SHALL verify because `"_sys:nix-cache:narinfo:".starts_with("_sys:nix-cache:")`

#### Scenario: Capability type narrowing across delegation

- **WHEN** Cluster A issues `Full{prefix: "_forge:"}` to Cluster B
- **AND** Cluster B delegates `Read{prefix: "_forge:repos:"}` to Cluster C
- **THEN** the chain SHALL verify because `Full` contains `Read` for narrower prefix

#### Scenario: Delegation cannot add capabilities

- **WHEN** Cluster B holds `Read{prefix: "_sys:nix-cache:"}` from Cluster A
- **AND** Cluster B attempts to delegate `Read{prefix: "_forge:repos:"}` to Cluster C
- **THEN** the delegation SHALL be rejected because `_forge:repos:` is not within `_sys:nix-cache:`

#### Scenario: Delegation cannot add Delegate capability

- **WHEN** Cluster A issues a token to Cluster B without `Delegate` capability
- **AND** Cluster B attempts to delegate to Cluster C
- **THEN** the delegation SHALL fail with `DelegationNotAllowed`

### Requirement: Credential wire encoding

The `Credential` type SHALL be serializable via postcard for wire transmission and BLAKE3-hashable for content addressing. The encoding SHALL be bounded by `MAX_DELEGATION_DEPTH` × max token size.

#### Scenario: Encode and decode credential

- **WHEN** a credential with a 3-level chain is serialized via postcard
- **THEN** deserializing the bytes SHALL produce an identical credential
- **AND** the serialized size SHALL be less than `MAX_DELEGATION_DEPTH * MAX_TOKEN_SIZE`

#### Scenario: Credential in federation handshake

- **WHEN** a federation handshake message includes a `Credential`
- **THEN** the credential SHALL be transmitted as a single postcard-encoded field
- **AND** the receiving side SHALL deserialize and verify before accepting the handshake

### Requirement: Facts field on capability token

The `CapabilityToken` SHALL support an optional `facts` field containing arbitrary key-value metadata. Facts SHALL be informational and SHALL NOT affect authorization decisions.

#### Scenario: Token with federation metadata facts

- **WHEN** a token is issued with facts `[("sync_interval", "60"), ("region", "us-east")]`
- **THEN** the facts SHALL be carried in the token and available after verification
- **AND** authorization checks SHALL produce the same result with or without facts

#### Scenario: Backward compatibility of facts field

- **WHEN** a token serialized without a `facts` field is deserialized by a system expecting it
- **THEN** deserialization SHALL succeed with an empty facts array
- **AND** the token SHALL verify and authorize identically to before

## MODIFIED Requirements

### Requirement: Self-contained credential type

The system SHALL provide a `Credential` type that bundles a `CapabilityToken` with its full delegation proof chain. The credential SHALL be verifiable offline without any server-side state or network calls. The federation sync client SHALL send the credential in the `Handshake` request when one is available, populating the existing `credential: Option<Credential>` field that was previously always `None`.

#### Scenario: Credential with root token (no delegation)

- **WHEN** Cluster A issues a token directly to Cluster B (depth 0)
- **THEN** the credential SHALL contain the token and an empty proofs array
- **AND** verification SHALL succeed by checking `token.issuer` against trusted roots

#### Scenario: Credential with delegation chain

- **WHEN** Cluster A issues a token to Cluster B, and Cluster B delegates a subset to Cluster C
- **THEN** Cluster C's credential SHALL contain its leaf token and `proofs: [token_from_A_to_B]`
- **AND** verification SHALL walk the chain: leaf → parent → root
- **AND** each level SHALL verify signature, expiry, and capability attenuation

#### Scenario: Credential with max delegation depth

- **WHEN** a delegation chain reaches `MAX_DELEGATION_DEPTH` (8) levels
- **THEN** the credential SHALL be verifiable with all 8 parent tokens in the proofs array
- **AND** attempting to delegate further SHALL fail with `DelegationTooDeep`

#### Scenario: Sync client populates credential field

- **WHEN** `connect_to_cluster` is called with a `Credential`
- **THEN** the `Handshake` request SHALL include `credential: Some(cred)`
- **AND** the handler SHALL verify it and store it in `session_credential`
