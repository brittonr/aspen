# Trust/crypto/secrets serialization contract scope

## Stable compatibility contracts covered by this change

### `aspen-trust`

- `Share::to_bytes` / `Share::from_bytes`: 33-byte layout `[x, y[0], ..., y[31]]`; zero `x` remains rejected.
- `EncryptedValue::to_bytes` / `EncryptedValue::from_bytes`: `AENC` magic, version byte, big-endian epoch, 12-byte nonce, and ciphertext/tag suffix; wrong version and too-short inputs remain rejected.
- `TrustRequest` / `TrustResponse`: postcard bytes for existing variants are pinned so enum insertion/reordering is detected.
- `Threshold`: JSON scalar representation for the nonzero constructor path.
- `EncryptedSecretChain`: JSON field representation for deterministic persisted chain state.

### `aspen-secrets-core`

- KV persisted state/config: `SecretData`, `VersionMetadata`, `SecretMetadata`, and `KvConfig`.
- Transit persisted state/config: `KeyType`, `KeyVersion`, and `TransitKey`.
- PKI persisted state/config: `PkiKeyType`, `CertificateAuthority`, `PkiRole`, `CrlEntry`, `CrlState`, `PkiConfig`, and `PendingIntermediateCa`.

## Internal or deferred surfaces

- `aspen-secrets-core` request/response DTOs that currently do not derive serde remain service implementation convenience types, not serialization compatibility contracts. A future change must add serde derives and goldens before treating them as wire/persisted contracts.
- Runtime `aspen-secrets` stores, SOPS file IO, crypto execution, auth-runtime helpers, and handler adapters remain outside the core serialization contract.
- `aspen-trust` async service APIs remain behind `aspen-trust/async` and are not promoted by this evidence slice.

## Readiness decision

This evidence removes the known serialization/golden blocker for the currently classified reusable surfaces, but the aggregate family remains `workspace-internal` until a fresh owner/readiness review explicitly promotes it.
