# SOPS Integration

Aspen provides native SOPS (Secrets OPerationS) support using its distributed
Transit engine as a key management backend.

## Overview

SOPS encrypts individual values in structured files (TOML, YAML, JSON) while
keeping keys in plaintext. Each value is encrypted with AES-256-GCM using a
data key, and the data key itself is encrypted (envelope encryption) by one or
more key management systems.

Aspen adds itself as a SOPS key management backend alongside existing options
(age, AWS KMS, GCP KMS, HashiCorp Vault).

## Architecture

```text
┌──────────────────────────────────────┐
│         aspen-sops CLI               │
│  (encrypt / decrypt / edit / rotate) │
└──────────────┬───────────────────────┘
               │ Iroh QUIC
┌──────────────▼───────────────────────┐
│         Aspen Cluster                │
│   Transit Engine (Raft-replicated)   │
│   - generate_data_key                │
│   - encrypt / decrypt / rewrap       │
└──────────────────────────────────────┘
```

## Envelope Encryption Flow

### Encrypt

1. Generate random 32-byte data key via Transit `generate_data_key`
2. Transit returns both plaintext and encrypted data key
3. Encrypt each value with AES-256-GCM using the plaintext data key
4. Compute HMAC-SHA256 MAC over all plaintext values
5. Store encrypted data key in `[sops.aspen_transit]` metadata
6. Zeroize plaintext data key from memory

### Decrypt

1. Read `[sops.aspen_transit]` metadata from file
2. Send encrypted data key to Transit `decrypt`
3. Verify MAC (HMAC-SHA256 over decrypted values)
4. Decrypt each `ENC[AES256_GCM,...]` value
5. Zeroize data key from memory

### Rotate

After a Transit key rotation, re-wrap the data key without changing values:

1. Decrypt data key with current Transit key version
2. Call Transit `rewrap` to re-encrypt with latest version
3. Update `key_version` in metadata
4. Values remain unchanged (same data key)

## SOPS Metadata Format

```toml
[sops]
lastmodified = "2026-03-04T18:00:00Z"
mac = "ENC[AES256_GCM,data:...,iv:...,tag:...,type:str]"
version = "3.9.0"

[[sops.aspen_transit]]
cluster_ticket = "aspen1q..."
mount = "transit"
name = "sops-data-key"
enc = "aspen:v3:c2VjcmV0IGRhdGEga2V5..."
key_version = 3

[[sops.age]]
recipient = "age1..."
enc = "-----BEGIN AGE ENCRYPTED FILE-----\n..."
```

## Multi-Key-Group Support

Files can have multiple key groups for redundancy:

- **Aspen Transit**: Primary, requires cluster connectivity
- **Age**: Offline fallback, uses local age identity file

Any single key group can decrypt the file independently.

## Security Model

- **Data key zeroization**: Plaintext data keys use `Zeroizing<Vec<u8>>` —
  automatically wiped from memory on drop
- **No key material on disk**: Only the Transit-encrypted data key is stored
  in the SOPS file
- **MAC verification**: Always verified before returning decrypted values to
  detect file tampering
- **Transit access control**: Governed by Aspen capability tokens

## Crate Structure

- `aspen-secrets::sops` — Library: encrypt/decrypt/edit/rotate/MAC/metadata
- `aspen-sops` — CLI binary wrapper (`aspen-sops encrypt/decrypt/edit/rotate`)
- `aspen-sops::keyservice` — gRPC bridge for Go `sops` binary compatibility
