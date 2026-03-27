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
