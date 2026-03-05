# Tasks: Extract `aspen-crypto`

## 1. Create `aspen-crypto` crate

- [x] 1.1 Create `crates/aspen-crypto/Cargo.toml` with deps: blake3, hex, iroh, rand, tokio, thiserror. No aspen-* deps.
- [x] 1.2 Add `aspen-crypto` to workspace `Cargo.toml` members and `[workspace.dependencies]`.
- [x] 1.3 Move `aspen-secrets/src/cookie/mod.rs` → `aspen-crypto/src/cookie.rs`. Replace `SecretsError` refs with local `CookieError`.
- [x] 1.4 Move `aspen-secrets/src/identity/mod.rs` → `aspen-crypto/src/identity.rs`. Create `IdentityError` to replace `SecretsError` refs.
- [x] 1.5 Create `aspen-crypto/src/lib.rs` with `pub mod cookie; pub mod identity;`.
- [x] 1.6 Verify `cargo build -p aspen-crypto` and `cargo nextest run -p aspen-crypto`.

## 2. Update `aspen-secrets` to re-export

- [x] 2.1 Add `aspen-crypto` dep to `aspen-secrets/Cargo.toml`.
- [x] 2.2 Replace `aspen-secrets/src/cookie/mod.rs` body with `pub use aspen_crypto::cookie::*;`.
- [x] 2.3 Replace `aspen-secrets/src/identity/mod.rs` body with `pub use aspen_crypto::identity::*;`.
- [x] 2.4 Remove `SecretsError::ParseSigningKey` and `SecretsError::LoadIdentity` variants (now in `IdentityError`), or keep as wrappers. Kept as wrappers — used by `sops/provider.rs` and `sops/decryptor.rs` with different error context.
- [x] 2.5 Verify `cargo nextest run -p aspen-secrets --features full` — all 138 tests pass.

## 3. Update `aspen-auth`

- [x] 3.1 Add `aspen-crypto` dep to `aspen-auth/Cargo.toml`.
- [x] 3.2 Replace `derive_hmac_key` body with delegation to `aspen_crypto::cookie::derive_cookie_hmac_key`.
- [x] 3.3 Verify `cargo nextest run -p aspen-auth` — all 73 tests pass.

## 4. Update `aspen-cluster` cookie validation

- [x] 4.1 Add `aspen-crypto` dep to `aspen-cluster/Cargo.toml` (required, not optional).
- [x] 4.2 In `validation.rs`: replace `UNSAFE_DEFAULT_COOKIE` with `aspen_crypto::cookie::UNSAFE_DEFAULT_COOKIE`.
- [x] 4.3 In `validation.rs`: replace `validate_cookie` and `validate_cookie_safety` bodies with delegation to `aspen_crypto::cookie` (mapping `CookieError` → `ValidationError`).
- [x] 4.4 In `config/mod.rs`: update imports if needed.
- [x] 4.5 Verify `cargo nextest run -p aspen-cluster` — all tests pass.

## 5. Update `aspen-cluster` identity key management

- [x] 5.1 In `endpoint_manager.rs`: replace `load_secret_key_from_file` with `NodeIdentityProvider::from_hex` on file contents.
- [x] 5.2 `save_secret_key_to_file` uses `hex::encode(key.to_bytes())` directly — identical to `NodeIdentityProvider::to_hex()`. Kept as-is because the function also handles Unix file permissions (0600) which `NodeIdentityProvider::load_or_generate` doesn't support.
- [x] 5.3 Replace `generate_secret_key` with `NodeIdentityProvider::generate`.
- [x] 5.4 `resolve_secret_key` uses all the above delegates. Clean.
- [x] 5.5 Verify `cargo nextest run -p aspen-cluster` — all 209 tests pass (3 skipped).

## 6. Update `aspen-dht-discovery`

- [x] 6.1 No `aspen-crypto` dep needed — `iroh_secret_to_signing_key` operates directly on `iroh::SecretKey` bytes (2-line function, wrapping in `NodeIdentityProvider` adds no value per task note).
- [x] 6.2 Updated comment in `hash.rs` to reference `aspen_crypto::identity::NodeIdentityProvider` instead of `aspen_secrets::identity::NodeIdentityProvider`.
- [x] 6.3 Verify `cargo build -p aspen-dht-discovery` — clean.

## 7. Lint, test, format

- [x] 7.1 `nix run .#rustfmt` (pending)
- [x] 7.2 `cargo clippy -p aspen-crypto -- --deny warnings` — clean
- [x] 7.3 `cargo clippy -p aspen-secrets -- --deny warnings` — clean
- [x] 7.4 `cargo clippy -p aspen-auth -- --deny warnings` — clean
- [x] 7.5 `cargo clippy -p aspen-cluster -- --deny warnings` — clean (pre-existing test-only warnings unrelated to this change)
- [x] 7.6 `cargo nextest run -p aspen-crypto -p aspen-secrets -p aspen-auth -p aspen-cluster -p aspen-dht-discovery` — 436 tests pass, 3 skipped
