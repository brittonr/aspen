# Cluster Trust Quorum

Aspen's trust quorum distributes a 32-byte cluster root secret across N nodes using Shamir's Secret Sharing over GF(2^8). No single node holds the full secret. Reconstruction requires cooperation of K (threshold) nodes.

Inspired by [Oxide's trust-quorum](https://rfd.shared.oxide.computer/rfd/0398) and their `gfss` crate.

## Architecture

```
  ┌──────────────┐
  │ ClusterSecret│  32-byte root (exists only during init)
  │  (ephemeral) │
  └──────┬───────┘
         │ split_secret(K, N)
    ┌────┼────┬─────┐
    ▼    ▼    ▼     ▼
  Share Share Share Share   33 bytes each (1-byte x + 32-byte y)
  Node1 Node2 Node3 ...    stored in redb trust_shares table
         │
         │ HKDF-SHA3-256
         ▼
  ┌──────────────────┐
  │ Purpose Keys     │  derived with context + cluster_id + epoch
  │ • secrets-at-rest│
  │ • transit-keys   │
  │ • rack-secrets   │
  └──────────────────┘
```

## How It Works

### Initialization

1. During `cluster init --trust`, the Raft leader generates a random 32-byte `ClusterSecret`
2. The secret is split into N shares with threshold K (default: `(N/2) + 1`)
3. Each node's share is stored in the `trust_shares` redb table (epoch-keyed)
4. SHA3-256 digests of all shares are stored in `trust_digests` for tamper detection

### Reconstruction

When the cluster needs the root secret (e.g., to derive encryption keys):

1. Collect at least K shares from available nodes
2. Verify each share's SHA3-256 digest against stored digests
3. Reconstruct via Lagrange interpolation at x=0

### Key Derivation

Purpose-specific keys are derived from the root secret using HKDF-SHA3-256:

```
key = HKDF-SHA3-256(
    IKM = cluster_secret,
    info = context || cluster_id || epoch_be_bytes
)
```

Standard contexts:

- `b"aspen-v1-secrets-at-rest"` — encrypting secrets in redb
- `b"aspen-v1-transit-keys"` — encrypting inter-node secret transfer
- `b"aspen-v1-rack-secrets"` — physical topology isolation

## Security Properties

- **Information-theoretic security**: Fewer than K shares reveal nothing about the secret (not computationally hard — mathematically impossible)
- **Constant-time operations**: All secret comparisons use `subtle::ConstantTimeEq`
- **Memory safety**: All secret material implements `Zeroize` and `ZeroizeOnDrop`
- **Tamper detection**: SHA3-256 digests catch corrupted or modified shares

## Crate Structure

- `crates/aspen-trust/src/gf256.rs` — GF(2^8) field arithmetic
- `crates/aspen-trust/src/shamir.rs` — Split/reconstruct, Share type, digest
- `crates/aspen-trust/src/secret.rs` — ClusterSecret, Threshold
- `crates/aspen-trust/src/kdf.rs` — HKDF key derivation + context constants
- `crates/aspen-trust/verus/shamir_spec.rs` — Verus formal specs for threshold bounds

## Feature Flag

All trust functionality is behind the `trust` feature flag:

- `aspen-trust` — always compiled (standalone library)
- `aspen-raft --features trust` — enables redb trust tables and init integration
- `aspen --features trust` — top-level feature, included in `full`

## Secrets At Rest Encryption

With the `trust` feature enabled, all secrets engine data (KV, Transit key material, PKI private keys) is encrypted at rest using keys derived from the cluster secret.

### How It Works

1. **Lazy reconstruction**: On first secrets access, the node collects shares from K peers, reconstructs the cluster secret, and derives the at-rest encryption key via HKDF
2. **Transparent encryption**: `AspenSecretsBackend` wraps all writes with ChaCha20-Poly1305 and unwraps on read. Callers see only plaintext
3. **Wire format**: `[AENC magic (4)] [version (1)] [epoch (8)] [nonce (12)] [ciphertext + Poly1305 tag]`
4. **Nonce uniqueness**: 12-byte nonces = `[node_id (4)] [counter (8)]`, counter persisted in `trust_nonce_counter` redb table

### Epoch Rotation

When membership changes trigger `secret-rotation-on-membership-change`:

1. New secret is split and distributed via Raft
2. Each node derives the new at-rest key from the new secret
3. A background re-encryption task scans all secrets and re-encrypts with the new key
4. During re-encryption, reads check the epoch byte to select the correct key
5. After completion, the old epoch's key is zeroized

### Operational Requirements

- **Quorum required**: Secrets are unavailable until K nodes are reachable for share collection. Non-secret KV data remains available
- **Backup/restore**: Redb backups contain ciphertext. Restoring requires quorum to reconstruct the decryption key
- **Re-encryption cost**: On epoch change, every secret is re-read and re-written. For large stores this takes seconds. Membership changes are rare, so this is acceptable
- **Memory exposure**: One 32-byte derived key in process memory (zeroized on drop). Smaller exposure than the alternative of megabytes of plaintext secrets

### Modules

- `crates/aspen-trust/src/envelope.rs` — `EncryptedValue` wire format, encrypt/decrypt
- `crates/aspen-trust/src/encryption.rs` — `SecretsEncryption` context, multi-epoch key management
- `crates/aspen-trust/src/nonce.rs` — Counter-based nonce generation
- `crates/aspen-trust/src/key_manager.rs` — Lazy reconstruction, epoch rotation lifecycle
- `crates/aspen-trust/src/reencrypt.rs` — Background re-encryption with checkpoint resume
- `crates/aspen-secrets/src/backend.rs` — `AspenSecretsBackend` with transparent encrypt/decrypt
- `crates/aspen-secrets/src/mount_registry.rs` — Propagates encryption context to all engine backends

## Limitations

- **No VSS**: We trust the Raft leader (not Byzantine fault tolerant)
- **No HSM integration**: Shares are stored in software

## References

- [Oxide RFD 398: Trust Quorum](https://rfd.shared.oxide.computer/rfd/0398)
- [Shamir's Secret Sharing (Wikipedia)](https://en.wikipedia.org/wiki/Shamir%27s_secret_sharing)
- [GF(2^8) Field Arithmetic](https://en.wikipedia.org/wiki/Finite_field_arithmetic#Rijndael's_(AES)_finite_field)
