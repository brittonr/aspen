## ADDED Requirements

### Requirement: Generate cache signing keypair

The system SHALL generate an Ed25519 signing keypair for the Nix binary cache. The keypair SHALL be stored in Raft KV at well-known keys and generated automatically on first use.

#### Scenario: First gateway startup

- **WHEN** the gateway starts and no signing key exists at `_sys:nix-cache:signing-key`
- **THEN** it SHALL generate a new Ed25519 keypair, store it in KV, and log the public key

#### Scenario: Subsequent gateway startup

- **WHEN** the gateway starts and a signing key already exists at `_sys:nix-cache:signing-key`
- **THEN** it SHALL load the existing keypair without generating a new one

### Requirement: Sign narinfo responses

The gateway SHALL sign every narinfo response with the cache signing key using the Nix fingerprint format.

#### Scenario: Signed narinfo

- **WHEN** the gateway serves a narinfo response
- **THEN** the response SHALL include a `Sig:` field containing `{cache-name}:{base64(ed25519_sign(fingerprint))}` where fingerprint = `1;{store_path};{nar_hash};{nar_size};{sorted_references}`

#### Scenario: Signature verification

- **WHEN** a Nix client receives a signed narinfo and has the cache's public key in `trusted-public-keys`
- **THEN** the client SHALL accept the store path as trusted

### Requirement: Distribute public key via RPC

The system SHALL expose the cache public key via an RPC endpoint so clients and CI workers can retrieve it.

#### Scenario: Query public key

- **WHEN** a client sends a `NixCacheGetPublicKey` RPC request
- **THEN** the system SHALL respond with the public key in Nix format (`{cache-name}:{base64(ed25519_public_key)}`)

#### Scenario: No key exists

- **WHEN** a client sends `NixCacheGetPublicKey` and no cache has been initialized
- **THEN** the system SHALL respond with an empty/absent key indication

### Requirement: Cache name configuration

The cache signing key SHALL be associated with a configurable cache name (default: `aspen-cache`). The name appears in signatures and public key distribution.

#### Scenario: Default cache name

- **WHEN** the gateway starts without `--cache-name`
- **THEN** it SHALL use `aspen-cache` as the cache name in signatures

#### Scenario: Custom cache name

- **WHEN** the gateway starts with `--cache-name my-org-cache`
- **THEN** it SHALL use `my-org-cache` in all signatures and public key responses
