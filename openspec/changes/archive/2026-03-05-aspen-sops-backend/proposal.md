## Why

Aspen already has a full Transit secrets engine (encrypt/decrypt, key rotation, data key generation) and a SOPS file decryptor using age. But there's a gap: SOPS users can't use Aspen as their key management backend. Today, SOPS supports AWS KMS, GCP KMS, Azure Key Vault, HashiCorp Vault Transit, age, and PGP — but not Aspen. This means operators managing Aspen clusters still need an external KMS for secrets at rest, defeating the self-hosted goal.

`aspen-sops` closes this gap by making Aspen's Transit engine a first-class SOPS backend. Instead of depending on cloud KMS or a separate Vault deployment, operators encrypt SOPS files with keys managed by Aspen's distributed, Raft-replicated Transit engine. This is the final piece for Aspen's self-hosted secrets story: the cluster manages its own encryption keys for its own configuration files.

This also directly supports the self-hosting goal: Aspen CI jobs that need secrets (deploy keys, API tokens, signing keys) can decrypt SOPS files using the same Aspen cluster that runs the CI pipeline — no external KMS dependency.

## What Changes

- **New crate `aspen-sops`**: Standalone binary and library that implements SOPS encrypt/decrypt operations using Aspen's Transit engine as the key management backend. Communicates with the Aspen cluster via Iroh QUIC (using the existing client API). No HTTP/gRPC for Aspen communication.

- **SOPS key service gRPC bridge**: A local gRPC server (Unix socket) implementing SOPS's `KeyService` protocol, so the standard Go `sops` binary can use Aspen Transit as a backend via `--keyservice unix:///path/to/socket`. This is the *only* gRPC in the project — it's a compatibility bridge to the external SOPS ecosystem, not internal Aspen communication.

- **Native SOPS operations**: Full Rust implementation of SOPS encrypt/decrypt/edit/rotate that talks directly to Aspen Transit via Iroh. No dependency on the Go `sops` binary for native mode.

- **SOPS metadata extension**: New `aspen_transit` key group in SOPS metadata (alongside existing `age`, `kms`, `gcp_kms`, etc.) identifying the Aspen Transit key used for encryption.

- **CLI**: `aspen-sops encrypt`, `aspen-sops decrypt`, `aspen-sops edit`, `aspen-sops rotate`, `aspen-sops keyservice` subcommands.

- **Integration with existing code**: Extends `aspen-secrets/src/sops/decryptor.rs` to support Aspen Transit as a key source in addition to age. The existing age-based decryption continues to work unchanged.

## Capabilities

### New Capabilities

**Native SOPS Operations (no Go binary needed)**:

- `aspen-sops encrypt` — Encrypt a TOML/YAML/JSON file using Aspen Transit data key, compatible with SOPS file format
- `aspen-sops decrypt` — Decrypt a SOPS-encrypted file using Aspen Transit
- `aspen-sops edit` — Decrypt to temp file, open in `$EDITOR`, re-encrypt on save
- `aspen-sops rotate` — Re-encrypt data key with new Transit key version (after key rotation)
- `aspen-sops updatekeys` — Add/remove key groups (add Aspen Transit, keep age as backup)

**SOPS Key Service Bridge**:

- `aspen-sops keyservice` — Start a gRPC key service on a Unix socket
- Bridges SOPS `Encrypt(data_key) → Transit.encrypt(key_name, data_key)` and `Decrypt(encrypted_data_key) → Transit.decrypt(key_name, ciphertext)`
- Allows: `sops --keyservice unix:///tmp/aspen-sops.sock decrypt secrets.sops.toml`

**Key Management via Transit**:

- Data key generation via `SecretsTransitDatakey` RPC (envelope encryption)
- Key rotation via `SecretsTransitRotateKey` RPC
- Uses AES-256-GCM for value encryption (SOPS standard), Transit key wraps the data key

### Modified Capabilities

- `aspen-secrets` SOPS decryptor gains Aspen Transit as a key source alongside age
- `SecretsManager::new()` can accept a Transit-backed identity in addition to age identity

### Unchanged

- All existing age-based SOPS decryption
- Transit engine API and storage
- KV, PKI, and mount registry in `aspen-secrets`

## Risks

- **gRPC dependency**: Adding `tonic` (gRPC) is new for the project. Mitigated: it's isolated to the key service bridge binary and only used for SOPS ecosystem compatibility, not internal Aspen communication.
- **SOPS file format compatibility**: SOPS's encrypted value format (`ENC[AES256_GCM,data:...,iv:...,tag:...]`) and MAC calculation must be byte-identical. Mitigated: existing `decryptor.rs` already handles the format correctly; encryption is the inverse.
- **Network dependency for decrypt**: Unlike age (local key file), Aspen Transit requires cluster connectivity. Mitigated: support multiple key groups (Aspen Transit + age fallback) so offline decryption works with age.

## Out of Scope

- Replacing existing age-based decryption in `aspen-secrets` (it stays as-is)
- SOPS support for AWS KMS, GCP KMS, Azure KV (use the Go binary with the key service bridge for those)
- YAML/JSON encrypted_regex or encrypted_suffix support in the native Rust implementation (TOML only for v1; YAML/JSON as follow-up)
- Web UI for secrets management
