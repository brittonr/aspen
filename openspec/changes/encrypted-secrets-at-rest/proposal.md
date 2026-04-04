## Why

Aspen's secrets engine (Transit keys, PKI private keys, KV secrets) stores all secret material in plaintext in redb. Every Raft member holds a full, unencrypted copy. Disk theft, backup exfiltration, or a single compromised node exposes every secret in the cluster. With the cluster root secret from `shamir-cluster-secret`, we can derive encryption keys and encrypt secrets data at rest — requiring quorum cooperation to decrypt.

## What Changes

- Derive a secrets-at-rest encryption key from the cluster root secret via HKDF with context `b"aspen-v1-secrets-at-rest"`
- Encrypt all values in the secrets engine (KV, Transit, PKI) before writing to redb
- Decrypt on read, using the derived key cached in memory after quorum reconstruction
- Use ChaCha20Poly1305 for authenticated encryption (same as trust-quorum's chain encryption)
- Each secret value gets a unique nonce (counter-based per node, or random)
- On secret rotation (membership change from `secret-rotation-on-membership-change`), re-encrypt all secrets with the new epoch's derived key

## Capabilities

### New Capabilities

- `secrets-encryption-at-rest`: Transparent encryption/decryption of secrets engine data in redb using cluster-secret-derived keys

### Modified Capabilities

## Impact

- `crates/aspen-secrets/` (or equivalent secrets engine module) — encrypt/decrypt wrapper around redb reads/writes
- `crates/aspen-raft/` — secrets-at-rest key cached in memory after cluster secret reconstruction; cleared on epoch rotation
- `crates/aspen-trust/` — new HKDF context constant, key re-derivation on epoch change
- Operational: cluster must have quorum to start serving secrets (can't decrypt without K nodes)
