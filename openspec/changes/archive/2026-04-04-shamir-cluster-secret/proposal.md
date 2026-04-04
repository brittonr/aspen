## Why

Aspen has no shared cluster secret. Every node in a Raft cluster holds a full copy of all secrets in plaintext via Raft replication. If a single node's disk is compromised, every secret is exposed. Oxide's trust-quorum solves this with Shamir's Secret Sharing — a root cluster secret is split into K-of-N shares so no single node can reconstruct it alone. This is foundational: encrypted secrets-at-rest, key rotation on membership change, and node expungement all depend on having a cluster root secret.

## What Changes

- Add `aspen-trust` crate implementing GF(2^8) Shamir secret sharing (split, reconstruct, share validation)
- Introduce a `ClusterSecret` type representing the root secret, created at cluster initialization
- Split the secret across cluster members with a configurable threshold (default: majority)
- Store each node's share in redb alongside Raft persistent state
- Derive purpose-specific keys from the root secret via HKDF-SHA3-256 (e.g., `"aspen-v1-secrets-at-rest"`, `"aspen-v1-transit-keys"`)
- Track share digests (SHA3-256) in cluster configuration for tamper detection
- Use `zeroize` and `secrecy` crates for memory safety of secret material

## Capabilities

### New Capabilities

- `shamir-secret-sharing`: GF(2^8) Shamir split/reconstruct with threshold configuration, share digest validation, and key derivation
- `cluster-secret-lifecycle`: Cluster root secret creation at init, share distribution, and reconstruction via quorum

### Modified Capabilities

## Impact

- New crate: `crates/aspen-trust/` with `src/shamir.rs`, `src/secret.rs`, `src/kdf.rs`
- `crates/aspen-raft/` — cluster init flow creates and distributes shares
- `crates/aspen-core/` — `ClusterSecret`, `ShareDigest`, `Threshold` types
- Dependencies: `chacha20poly1305`, `hkdf`, `sha3`, `zeroize`, `secrecy`, `subtle`
- Feature flag: `trust` (off by default, required for encrypted secrets)
