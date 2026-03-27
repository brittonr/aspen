## ADDED Requirements

### Requirement: Federation-scoped capability variants

The auth system SHALL provide `FederationPull { repo_prefix }` and `FederationPush { repo_prefix }` capability variants. These capabilities SHALL authorize federation sync operations scoped to repositories whose federated ID matches the prefix.

#### Scenario: FederationPull authorizes pull for matching repo

- **WHEN** a credential contains `FederationPull { repo_prefix: "forge:" }`
- **AND** a pull request targets a resource with federated ID starting with `forge:`
- **THEN** the handler SHALL permit the pull

#### Scenario: FederationPull rejects pull for non-matching repo

- **WHEN** a credential contains `FederationPull { repo_prefix: "forge:org-a/" }`
- **AND** a pull request targets a resource with federated ID starting with `forge:org-b/`
- **THEN** the handler SHALL deny the pull with `ACCESS_DENIED`

#### Scenario: FederationPush authorizes push for matching repo

- **WHEN** a credential contains `FederationPush { repo_prefix: "" }` (all repos)
- **AND** a push request targets any resource
- **THEN** the handler SHALL permit the push

#### Scenario: Capability attenuation works for federation variants

- **WHEN** a parent token has `FederationPull { repo_prefix: "forge:" }`
- **AND** a child token delegates `FederationPull { repo_prefix: "forge:org-a/" }`
- **THEN** `contains()` SHALL return true (child is narrower)
- **AND** delegating `FederationPull { repo_prefix: "" }` SHALL fail (child is broader)

### Requirement: Sync client sends credential in handshake

The federation sync client SHALL send a stored credential in the `Handshake` request when one is available for the target cluster. If no credential is stored, the handshake SHALL send `credential: None` (backwards compatible).

#### Scenario: Client sends stored credential

- **WHEN** the sync orchestrator initiates a connection to cluster B
- **AND** a credential for cluster B exists in KV at `_sys:fed:token:received:<cluster_b_key>`
- **THEN** the handshake request SHALL include the credential

#### Scenario: Client connects without credential

- **WHEN** the sync orchestrator initiates a connection to cluster B
- **AND** no credential exists for cluster B
- **THEN** the handshake request SHALL have `credential: None`
- **AND** the connection SHALL proceed (public resources remain accessible)

### Requirement: Pull handler checks credential capabilities

The `check_resource_access` function SHALL check the session credential for a `FederationPull` capability matching the target resource before falling back to TrustManager. Blocked clusters SHALL always be denied regardless of credential.

#### Scenario: Credential-authorized pull on non-public resource

- **WHEN** a remote peer presents a credential with `FederationPull { repo_prefix: "forge:" }`
- **AND** the target resource has `FederationMode::AllowList`
- **AND** the peer is not in the TrustManager's trusted set
- **THEN** the pull SHALL succeed because the credential authorizes it

#### Scenario: Blocked peer denied despite valid credential

- **WHEN** a remote peer is in the TrustManager's blocked set
- **AND** the peer presents a valid credential
- **THEN** the pull SHALL be denied with `ACCESS_DENIED`

### Requirement: Push handler checks credential capabilities

The `handle_push_objects` function SHALL verify that the session credential contains a `FederationPush` capability matching the target resource's federated ID. Checking credential _presence_ alone SHALL NOT be sufficient.

#### Scenario: Push with matching FederationPush capability

- **WHEN** a remote peer presents a credential with `FederationPush { repo_prefix: "forge:" }`
- **AND** the push targets a resource with federated ID starting with `forge:`
- **THEN** the push SHALL be accepted

#### Scenario: Push rejected with wrong capability

- **WHEN** a remote peer presents a credential with `FederationPull { repo_prefix: "forge:" }` (pull, not push)
- **AND** the peer attempts a push
- **THEN** the push SHALL be rejected with `unauthorized`

#### Scenario: Push rejected with non-matching prefix

- **WHEN** a remote peer presents a credential with `FederationPush { repo_prefix: "forge:org-a/" }`
- **AND** the push targets a resource with federated ID starting with `forge:org-b/`
- **THEN** the push SHALL be rejected with `unauthorized`
