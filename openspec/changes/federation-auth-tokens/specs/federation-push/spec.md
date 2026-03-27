## MODIFIED Requirements

### Requirement: Push requires explicit authorization

The federation push handler SHALL verify that the remote peer has explicit push authorization for the target resource. Authorization SHALL be granted by either TrustManager trust level OR a session credential containing a `FederationPush` capability matching the target resource. Checking credential _presence_ without verifying its capabilities SHALL NOT be sufficient.

#### Scenario: Push accepted from trusted peer without credential

- **WHEN** a remote peer is in the TrustManager's trusted set
- **AND** the peer does not present a credential
- **THEN** the push SHALL be accepted (backwards compatible)

#### Scenario: Push accepted from untrusted peer with valid FederationPush credential

- **WHEN** a remote peer is not in the TrustManager's trusted set (Public trust level)
- **AND** the peer presents a credential with `FederationPush { repo_prefix }` matching the target resource
- **THEN** the push SHALL be accepted

#### Scenario: Push rejected from untrusted peer with pull-only credential

- **WHEN** a remote peer presents a credential with only `FederationPull` capabilities
- **AND** the peer is not in the TrustManager's trusted set
- **THEN** the push SHALL be rejected with `unauthorized`
- **AND** the error message SHALL indicate push capability is required

#### Scenario: Push rejected from blocked peer regardless of credential

- **WHEN** a remote peer is in the TrustManager's blocked set
- **AND** the peer presents a valid credential with `FederationPush`
- **THEN** the push SHALL be rejected
