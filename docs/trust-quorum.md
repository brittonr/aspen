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

## Limitations

- **No proactive resharing**: Share redistribution on membership change is not yet implemented
- **No VSS**: We trust the Raft leader (not Byzantine fault tolerant)
- **No HSM integration**: Shares are stored in software
- **Single-node init only**: Multi-node share distribution via Raft log entries is planned

## References

- [Oxide RFD 398: Trust Quorum](https://rfd.shared.oxide.computer/rfd/0398)
- [Shamir's Secret Sharing (Wikipedia)](https://en.wikipedia.org/wiki/Shamir%27s_secret_sharing)
- [GF(2^8) Field Arithmetic](https://en.wikipedia.org/wiki/Finite_field_arithmetic#Rijndael's_(AES)_finite_field)
