# Design: `aspen-crypto`

## Dependency Graph (after)

```
aspen-crypto          (NEW — no aspen deps, just blake3/hex/iroh/rand/tokio/thiserror)
  ├── aspen-auth      (adds dep on aspen-crypto, delegates derive_hmac_key)
  ├── aspen-cluster   (adds dep on aspen-crypto, replaces validation.rs + endpoint_manager.rs)
  ├── aspen-secrets   (adds dep on aspen-crypto, re-exports cookie/identity)
  └── aspen-dht-discovery (adds dep on aspen-crypto, uses identity for key bytes)
```

No cycles. `aspen-crypto` sits at the bottom of the tree with zero aspen-* dependencies.

## Modules

### `cookie`

Moved from `aspen-secrets::cookie`. No changes to API.

- `validate_cookie()`, `validate_cookie_safety()`, `validate_cookie_length()`, `validate_cookie_full()`
- `derive_cookie_hmac_key()` — BLAKE3 hash of cookie bytes → 32-byte HMAC key
- `derive_gossip_topic()` — BLAKE3 with domain separator → 32-byte topic ID
- `UNSAFE_DEFAULT_COOKIE`, `MAX_COOKIE_LENGTH`, `CookieError`

### `identity`

Moved from `aspen-secrets::identity`. No changes to API.

- `NodeIdentityProvider` — wraps `iroh::SecretKey`
- `from_hex()`, `generate()`, `load_or_generate()`, `public_key()`, `secret_key()`, `to_hex()`
- `SECRET_KEY_HEX_LENGTH`

## Downstream Changes

| Crate | Change |
|-------|--------|
| `aspen-auth` | Add `aspen-crypto` dep. `derive_hmac_key` delegates to `aspen_crypto::cookie::derive_cookie_hmac_key` |
| `aspen-cluster/validation.rs` | Replace `validate_cookie`, `validate_cookie_safety`, `UNSAFE_DEFAULT_COOKIE` with imports from `aspen_crypto::cookie` |
| `aspen-cluster/endpoint_manager.rs` | Replace `resolve_secret_key`, `load_secret_key_from_file`, `save_secret_key_to_file`, `generate_secret_key` with `NodeIdentityProvider` |
| `aspen-secrets` | Replace `cookie` and `identity` modules with `pub use aspen_crypto::{cookie, identity}` |
| `aspen-dht-discovery` | Add `aspen-crypto` dep. Use `NodeIdentityProvider` to get key bytes, then convert to mainline SigningKey locally |

## Error Types

`aspen-crypto` defines its own error types:

- `CookieError` (already exists in cookie module)
- `IdentityError` (new, replaces `SecretsError` variants for identity operations)

Downstream crates map these into their own error types as needed (e.g., `ValidationError::CookieEmpty` wraps `CookieError::Empty`).
