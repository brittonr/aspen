## MODIFIED Requirements

### Requirement: Sync client sends credential in handshake

The federation sync client SHALL send a stored credential in the `Handshake` request when one is available for the target cluster. If no credential is stored, the handshake SHALL send `credential: None` (backwards compatible).

The credential SHALL be loaded from KV at key `_sys:fed:token:received:<cluster_key>` where `<cluster_key>` is the hex-encoded public key of the target cluster. All federation connection paths — orchestrator sync, forge pull handlers, forge push handlers, and federation git operations — SHALL perform this lookup.

#### Scenario: Client sends stored credential

- **WHEN** the sync orchestrator initiates a connection to cluster B
- **AND** a credential for cluster B exists in KV at `_sys:fed:token:received:<cluster_b_key>`
- **THEN** the handshake request SHALL include the credential

#### Scenario: Client connects without credential

- **WHEN** the sync orchestrator initiates a connection to cluster B
- **AND** no credential exists for cluster B
- **THEN** the handshake request SHALL have `credential: None`
- **AND** the connection SHALL proceed (public resources remain accessible)

#### Scenario: Forge handler sends stored credential

- **WHEN** a forge federation handler (pull, push, or git clone) connects to a remote cluster
- **AND** a credential exists in KV for that cluster
- **THEN** the handshake request SHALL include the credential

#### Scenario: Credential lookup failure does not block connection

- **WHEN** the KV lookup for `_sys:fed:token:received:<key>` fails (e.g., store unavailable)
- **THEN** the connection SHALL proceed with `credential: None`
- **AND** a warning SHALL be logged
