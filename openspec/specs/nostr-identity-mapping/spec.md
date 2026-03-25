## ADDED Requirements

### Requirement: npub to ed25519 keypair mapping

The system SHALL maintain a mapping from Nostr npub to a managed ed25519 keypair in the Raft KV store, encrypted with the cluster key.

#### Scenario: First authentication with a new npub

- **WHEN** a user authenticates with an npub that has no existing mapping
- **THEN** the system SHALL generate a new ed25519 keypair, encrypt and store it, and return a session bound to that npub

#### Scenario: Subsequent authentication with a known npub

- **WHEN** a user authenticates with an npub that already has a mapping
- **THEN** the system SHALL decrypt and use the existing ed25519 keypair

#### Scenario: Mapping persistence across node restarts

- **WHEN** a cluster node restarts
- **THEN** all npub → ed25519 mappings SHALL be available via Raft KV

### Requirement: Challenge-response authentication

The system SHALL authenticate npub ownership via a secp256k1 Schnorr signature over a server-provided challenge.

#### Scenario: Successful authentication

- **WHEN** a client sends its npub, receives a challenge, and returns a valid secp256k1 Schnorr signature over the challenge
- **THEN** the system SHALL issue a capability token carrying the npub and assigned ed25519 public key

#### Scenario: Invalid signature

- **WHEN** a client returns a signature that does not verify against the claimed npub
- **THEN** the system SHALL reject the authentication and return an error

#### Scenario: Challenge expiry

- **WHEN** a client attempts to respond to a challenge older than 60 seconds
- **THEN** the system SHALL reject the response and require a new challenge

### Requirement: Keypair encryption at rest

The stored ed25519 secret keys SHALL be encrypted using a key derived from the cluster's iroh secret key via BLAKE3 keyed hash.

#### Scenario: Key retrieval requires cluster membership

- **WHEN** a process without access to the cluster's iroh secret key reads the KV entry
- **THEN** the ed25519 secret key SHALL NOT be recoverable from the stored ciphertext
