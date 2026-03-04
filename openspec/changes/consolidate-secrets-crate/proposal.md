## Why

Secrets and crypto functionality is scattered across ~10 crates. There are two independent SOPS implementations (`aspen-secrets/src/sops/decryptor.rs` and `aspen-sops/`), HMAC key derivation in `aspen-auth`, cluster cookie validation in `aspen-cluster`, node identity key loading in `aspen-cluster/config`, Nix cache signing key management in `aspen-secrets-handler`, and DHT signing key derivation in `aspen-dht-discovery`. Every crate independently pulls in crypto dependencies (`aes-gcm`, `blake3`, `ed25519-dalek`, `hmac`, `sha2`, `rand`, `age`).

This fragmentation creates three problems:

1. **Duplicate code**: Two SOPS implementations parse `ENC[AES256_GCM,...]` values, two HMAC key derivation paths, multiple independent `SecretKey::generate()` call sites.
2. **Inconsistent security patterns**: Some code uses constant-time comparison, some doesn't. Some uses `zeroize`, some doesn't. No single audit surface.
3. **Dependency sprawl**: 15+ crates pull `blake3`, 10+ pull `rand`, 5+ pull `aes-gcm` — each with their own version. A crypto library upgrade touches dozens of `Cargo.toml` files.

`aspen-secrets` already owns the Vault-like engines (KV, Transit, PKI) and SOPS file decryption. Making it the single owner of all crypto primitives and secret lifecycle management reduces the audit surface to one crate and eliminates duplication.

## What Changes

- **Merge `aspen-sops` library code into `aspen-secrets`**: The encrypt/decrypt/edit/rotate/mac/format/metadata/verified modules move into `aspen-secrets::sops`. The CLI binary stays as `aspen-sops` but becomes a thin wrapper over `aspen-secrets` library functions. Eliminates the duplicate SOPS value encryption/decryption code.

- **Move HMAC auth primitives from `aspen-auth`**: `hmac_auth.rs` (HMAC-SHA256 challenge-response) and `verified_auth.rs` (`derive_hmac_key`, `constant_time_compare`, `is_challenge_valid`) move to `aspen-secrets::hmac`. `aspen-auth` re-exports from `aspen-secrets` for backward compatibility.

- **Move cluster cookie management from `aspen-cluster`**: `validate_cookie()`, `validate_cookie_safety()`, `UNSAFE_DEFAULT_COOKIE`, and the cookie-to-HMAC-key derivation move to `aspen-secrets::cookie`. The cookie is a shared secret — its validation and derived key material belong with secrets.

- **Centralize node identity key lifecycle**: A new `aspen-secrets::identity` module provides `NodeIdentityProvider` — loads, generates, and derives keys from the node's `iroh::SecretKey`. Replaces scattered `SecretKey::generate()` and hex-parsing across `aspen-cluster/config`, `aspen-dht-discovery`, `aspen-net`, etc.

- **Move Nix cache signing key logic into `aspen-secrets`**: The key management functions from `aspen-secrets-handler/src/handler/nix_cache.rs` move into `aspen-secrets::nix_cache`. The RPC handler stays thin — dispatches to `aspen-secrets` functions.

## Capabilities

### New Capabilities

- `aspen-secrets::sops::encrypt_file()` — full SOPS encryption using Transit (was only in `aspen-sops`)
- `aspen-secrets::sops::rotate_file()` — SOPS key rotation (was only in `aspen-sops`)
- `aspen-secrets::sops::edit_file()` — decrypt-edit-reencrypt workflow (was only in `aspen-sops`)
- `aspen-secrets::identity::NodeIdentityProvider` — centralized node key management
- `aspen-secrets::cookie::CookieValidator` — cluster cookie validation + derived key material

### Modified Capabilities

- `aspen-sops` binary becomes a thin CLI over `aspen-secrets::sops` (library code removed)
- `aspen-auth::hmac_auth` re-exports from `aspen-secrets::hmac` (API unchanged)
- `aspen-auth::verified_auth` re-exports from `aspen-secrets::verified::auth` (API unchanged)
- `aspen-secrets-handler` Nix cache handler calls `aspen-secrets::nix_cache` functions
- `aspen-cluster` cookie validation calls `aspen-secrets::cookie` functions

### Unchanged

- All KV, Transit, PKI engine APIs
- Token building and verification (`aspen-auth` capability logic)
- RPC handler registration and dispatch
- Plugin signing (still uses `PluginSignatureInfo` from `aspen-plugin-api`)
- TrustedPeersRegistry (stays in `aspen-transport`)

## Risks

- **Dependency cycle**: `aspen-secrets` importing from `aspen-auth` while `aspen-auth` re-exports from `aspen-secrets`. Mitigated: only pure crypto primitives move; `aspen-auth` depends on `aspen-secrets` (not vice versa). The HMAC/cookie modules have zero dependency on auth token types.
- **Compile time impact**: Making `aspen-secrets` larger increases incremental rebuild time for anything that depends on it. Mitigated: the moved code is mostly leaf-level crypto with few transitive deps. Feature-gate the `sops` module behind the existing `secrets` feature.
- **Migration churn**: Re-export wrappers in `aspen-auth` and `aspen-cluster` maintain backward compat, but downstream code should eventually update imports. Mitigated: re-exports are zero-cost and can stay indefinitely.

## Out of Scope

- Migrating KV/Transit engines to WASM plugin (separate change)
- Plugin signing implementation (not yet built)
- Changing the `SecretsBackend` trait or storage format
- HTTP/gRPC API changes (Aspen has no HTTP API)
- Merging `aspen-auth` entirely into `aspen-secrets` (auth policy != crypto primitives)
