## Context

With `shamir-cluster-secret` providing a root secret and `secret-rotation-on-membership-change` providing epoch-based rotation, the final piece is using derived keys to encrypt actual application secrets stored in redb. Currently the secrets engine writes plaintext to redb tables that are fully replicated across all Raft members.

## Goals / Non-Goals

**Goals:**

- Encrypt all secrets engine values (KV data, Transit key material, PKI private keys) at rest in redb
- Use authenticated encryption (ChaCha20Poly1305) with per-value nonces
- Derive the encryption key from the cluster secret via HKDF, scoped to epoch
- Cache the derived key in memory after cluster secret reconstruction; drop on epoch rotation
- Re-encrypt all secrets when the epoch changes (key rotation)
- Maintain transparent read/write semantics — callers don't know about encryption

**Non-Goals:**

- Encrypting the Raft log itself (only secrets engine data)
- Encrypting non-secret KV data (application data remains plaintext)
- Full disk encryption (out of scope — this is application-level encryption)
- Hardware-backed key storage (HSM/TPM)

## Decisions

**Encryption at the secrets engine layer, not the storage layer.** Only secrets engine tables are encrypted. Application KV data (`kv_data` table) remains plaintext. This keeps the performance impact minimal and avoids complicating Raft log replay.

**Per-value nonce via node_id + counter.** Each encrypted value includes a 12-byte nonce: 4 bytes node_id + 8 bytes monotonic counter. This avoids nonce reuse across nodes without coordination. The counter is persisted in redb and incremented on each write.

**Lazy reconstruction.** The secrets-at-rest key is not reconstructed at boot during ordinary startup. It's reconstructed on first secrets engine access, which triggers share collection. If the cluster doesn't have quorum, secrets are unavailable (not corrupt — just locked). The one exception is startup recovery for an interrupted epoch migration: if a persisted re-encryption checkpoint or stale lower-epoch ciphertext remains, Aspen may reconstruct the key at boot solely to finish background re-encryption recovery.

**Re-encryption on epoch change.** When `secret-rotation-on-membership-change` commits a new epoch, a background task re-encrypts all secrets with the new derived key. During re-encryption, reads use the old key for not-yet-migrated values and the new key for migrated ones. A version byte prefix on each encrypted value indicates which epoch's key was used. Re-encryption progress is checkpointed so a fresh provider/watcher on startup can resume migration without waiting for another epoch change.

**Prefix format:** `[magic: 4 bytes (AENC)][version: u8][epoch: u64][nonce: 12 bytes][ciphertext+tag]`. The 4-byte magic header enables unambiguous detection of encrypted values during migration from plaintext storage. The version byte allows future format changes. The epoch identifies which derived key to use for decryption. Detection is two-phase: (1) try to parse as an envelope — if parsing fails (wrong magic, bad version, too short), treat as legacy plaintext; (2) if parsing succeeds, decrypt — if decryption fails (Poly1305 tag mismatch, unknown epoch), raise an authentication error. This preserves tamper detection for real envelopes while allowing legacy plaintext to pass through.

## Risks / Trade-offs

- **Availability impact**: Secrets are unavailable until quorum is reached and the key is reconstructed. This is the security/availability trade-off. Non-secret KV data remains available.
- **Re-encryption cost**: On epoch change, every secret must be re-read and re-written. For large secrets stores this could take seconds. Acceptable — membership changes are rare.
- **Memory exposure**: The derived key lives in process memory. `zeroize` helps but a core dump could capture it. The window is smaller than plaintext secrets (one 32-byte key vs potentially megabytes of secrets).
- **Backup complexity**: Backups of redb are encrypted. Restoring requires quorum to be able to decrypt. Document this operational requirement.
