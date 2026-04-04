## ADDED Requirements

### Requirement: Transparent encryption of secrets data

All secrets engine values (KV, Transit, PKI) MUST be encrypted before writing to redb and decrypted on read, using a key derived from the cluster secret.

#### Scenario: Write a KV secret

- **WHEN** a KV secret is written via the secrets engine
- **THEN** the value stored in redb is encrypted with ChaCha20Poly1305 using the current epoch's derived key, and a plaintext read of the redb table reveals only ciphertext

#### Scenario: Read a KV secret

- **WHEN** a KV secret is read via the secrets engine
- **THEN** the ciphertext is decrypted with the appropriate epoch's derived key and the plaintext is returned to the caller

### Requirement: Authenticated encryption

Each encrypted value MUST use ChaCha20Poly1305 with a unique nonce, providing both confidentiality and integrity.

#### Scenario: Tampered ciphertext detected

- **WHEN** a byte in an encrypted secrets value is flipped in redb
- **THEN** decryption fails with an authentication error rather than returning corrupted plaintext

#### Scenario: Nonce uniqueness

- **WHEN** two values are encrypted by the same node in the same epoch
- **THEN** each uses a different nonce (monotonic counter per node)

### Requirement: Epoch-scoped key derivation

The encryption key MUST be derived via HKDF from the cluster secret with context `b"aspen-v1-secrets-at-rest"`, cluster_id, and epoch. Different epochs produce different keys.

#### Scenario: Key changes on epoch rotation

- **WHEN** the trust epoch changes from 1 to 2
- **THEN** new writes use a key derived from the epoch 2 secret, and the old key is dropped from memory after re-encryption completes

### Requirement: Lazy key reconstruction

The secrets-at-rest key MUST NOT be reconstructed at boot. It is reconstructed on first secrets engine access.

#### Scenario: Cluster below quorum at boot

- **WHEN** a node starts but the cluster has fewer than K nodes available
- **THEN** non-secrets KV data is fully accessible, but secrets engine reads return an "unavailable" error until quorum is reached

### Requirement: Re-encryption on epoch change

When the trust epoch changes, all secrets MUST be re-encrypted with the new epoch's derived key.

#### Scenario: Background re-encryption

- **WHEN** a new epoch is committed
- **THEN** a background task reads each secret, decrypts with the old key, re-encrypts with the new key, and writes back

#### Scenario: Mixed-epoch reads during re-encryption

- **WHEN** a secret is read during re-encryption
- **THEN** the epoch prefix in the stored value indicates which key to use, and decryption succeeds regardless of whether re-encryption has reached this value yet

### Requirement: Value format

Encrypted values MUST be stored as `[version: u8][epoch: u64][nonce: 12 bytes][ciphertext+tag]` to allow future format evolution and multi-epoch decryption.

#### Scenario: Format version check

- **WHEN** an encrypted value with an unknown version byte is read
- **THEN** decryption returns an error indicating unsupported format version
