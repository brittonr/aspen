# Tasks: Extract `aspen-crypto`

## 1. Create `aspen-crypto` crate

- [ ] 1.1 Create `crates/aspen-crypto/Cargo.toml` with deps: blake3, hex, iroh, rand, tokio, thiserror. No aspen-* deps.
- [ ] 1.2 Add `aspen-crypto` to workspace `Cargo.toml` members and `[workspace.dependencies]`.
- [ ] 1.3 Move `aspen-secrets/src/cookie/mod.rs` → `aspen-crypto/src/cookie.rs`. Replace `SecretsError` refs with local `CookieError`.
- [ ] 1.4 Move `aspen-secrets/src/identity/mod.rs` → `aspen-crypto/src/identity.rs`. Create `IdentityError` to replace `SecretsError` refs.
- [ ] 1.5 Create `aspen-crypto/src/lib.rs` with `pub mod cookie; pub mod identity;`.
- [ ] 1.6 Verify `cargo build -p aspen-crypto` and `cargo nextest run -p aspen-crypto`.

## 2. Update `aspen-secrets` to re-export

- [ ] 2.1 Add `aspen-crypto` dep to `aspen-secrets/Cargo.toml`.
- [ ] 2.2 Replace `aspen-secrets/src/cookie/mod.rs` body with `pub use aspen_crypto::cookie::*;`.
- [ ] 2.3 Replace `aspen-secrets/src/identity/mod.rs` body with `pub use aspen_crypto::identity::*;`.
- [ ] 2.4 Remove `SecretsError::ParseSigningKey` and `SecretsError::LoadIdentity` variants (now in `IdentityError`), or keep as wrappers.
- [ ] 2.5 Verify `cargo nextest run -p aspen-secrets --features full` — all 208 tests pass.

## 3. Update `aspen-auth`

- [ ] 3.1 Add `aspen-crypto` dep to `aspen-auth/Cargo.toml`.
- [ ] 3.2 Replace `derive_hmac_key` body with delegation to `aspen_crypto::cookie::derive_cookie_hmac_key`.
- [ ] 3.3 Verify `cargo nextest run -p aspen-auth` — all 73 tests pass.

## 4. Update `aspen-cluster` cookie validation

- [ ] 4.1 Add `aspen-crypto` dep to `aspen-cluster/Cargo.toml` (required, not optional).
- [ ] 4.2 In `validation.rs`: replace `UNSAFE_DEFAULT_COOKIE` with `aspen_crypto::cookie::UNSAFE_DEFAULT_COOKIE`.
- [ ] 4.3 In `validation.rs`: replace `validate_cookie` and `validate_cookie_safety` bodies with delegation to `aspen_crypto::cookie` (mapping `CookieError` → `ValidationError`).
- [ ] 4.4 In `config/mod.rs`: update imports if needed.
- [ ] 4.5 Verify `cargo nextest run -p aspen-cluster` — all tests pass.

## 5. Update `aspen-cluster` identity key management

- [ ] 5.1 In `endpoint_manager.rs`: replace `load_secret_key_from_file` with `NodeIdentityProvider::from_hex` on file contents.
- [ ] 5.2 Replace `save_secret_key_to_file` with `NodeIdentityProvider::to_hex` + write.
- [ ] 5.3 Replace `generate_secret_key` with `NodeIdentityProvider::generate`.
- [ ] 5.4 Simplify `resolve_secret_key` to use `NodeIdentityProvider` throughout.
- [ ] 5.5 Verify `cargo nextest run -p aspen-cluster` — all tests pass.

## 6. Update `aspen-dht-discovery`

- [ ] 6.1 Add `aspen-crypto` dep to `aspen-dht-discovery/Cargo.toml`.
- [ ] 6.2 In `hash.rs`: use `NodeIdentityProvider::new(secret_key).secret_key().to_bytes()` for key byte extraction (or just keep the 2-line function with a re-export comment — minimal value).
- [ ] 6.3 Verify `cargo build -p aspen-dht-discovery`.

## 7. Lint, test, format

- [ ] 7.1 `nix run .#rustfmt`
- [ ] 7.2 `cargo clippy -p aspen-crypto --all-targets -- --deny warnings`
- [ ] 7.3 `cargo clippy -p aspen-secrets --features full --all-targets -- --deny warnings`
- [ ] 7.4 `cargo clippy -p aspen-auth --all-targets -- --deny warnings`
- [ ] 7.5 `cargo clippy -p aspen-cluster --all-targets -- --deny warnings`
- [ ] 7.6 `cargo nextest run -p aspen-crypto -p aspen-secrets --features full -p aspen-auth -p aspen-cluster -p aspen-secrets-handler`
