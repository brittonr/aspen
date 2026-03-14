## ADDED Requirements

### Requirement: Cluster Nostr keypair generation

The system SHALL generate a secp256k1 keypair for the cluster's Nostr identity when the `nostr-relay` feature is first enabled. The keypair SHALL be stored persistently in the cluster configuration.

#### Scenario: First-time key generation

- **WHEN** a cluster enables the `nostr-relay` feature and no existing Nostr key is found
- **THEN** the system SHALL generate a new secp256k1 keypair and store it

#### Scenario: Key persistence across restarts

- **WHEN** a node restarts with `nostr-relay` enabled
- **THEN** the system SHALL load the previously generated Nostr keypair, not generate a new one

### Requirement: Nostr key stored alongside cluster identity

The Nostr secp256k1 keypair SHALL be stored in the same location as the `ClusterIdentity` Ed25519 keypair (cluster config). The Nostr key SHALL be optional — clusters without `nostr-relay` enabled SHALL NOT have one.

#### Scenario: Key accessible from cluster config

- **WHEN** the Nostr relay engine or a bridge plugin needs to sign an event
- **THEN** the cluster's Nostr secret key SHALL be accessible via the cluster configuration

#### Scenario: Key absent when feature disabled

- **WHEN** the `nostr-relay` feature is not enabled
- **THEN** no Nostr keypair SHALL be present in the cluster configuration

### Requirement: Nostr event signing

The system SHALL provide a function to sign Nostr events using the cluster's secp256k1 key. Signing SHALL follow NIP-01: SHA-256 hash of the serialized event array, signed with secp256k1 Schnorr signatures per BIP-340.

#### Scenario: Sign a Nostr event

- **WHEN** the bridge plugin constructs a NIP-34 event and requests signing
- **THEN** the system SHALL compute the event ID (SHA-256 of `[0, pubkey, created_at, kind, tags, content]`), sign the ID with the cluster's secp256k1 secret key using Schnorr, and set the `id`, `pubkey`, and `sig` fields

#### Scenario: Signed event verifiable by external clients

- **WHEN** a Nostr client receives an event signed by the cluster
- **THEN** the client SHALL be able to verify the signature using standard NIP-01 secp256k1 Schnorr verification

### Requirement: Nostr public key exposed via NIP-11

The cluster's Nostr public key (hex-encoded x-only secp256k1 pubkey) SHALL be included in the NIP-11 relay information document's `pubkey` field.

#### Scenario: Public key in relay info

- **WHEN** a client requests the NIP-11 relay information document
- **THEN** the `pubkey` field SHALL contain the 64-character hex-encoded x-only public key of the cluster's Nostr keypair

### Requirement: Key isolation from Ed25519 identity

The secp256k1 Nostr key SHALL be independent of the cluster's Ed25519 identity key. There SHALL be no derivation relationship between the two keys.

#### Scenario: Keys are independent

- **WHEN** both the Ed25519 cluster key and secp256k1 Nostr key exist
- **THEN** knowledge of one key SHALL NOT enable derivation of the other
