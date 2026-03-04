## 1. Merge SOPS Library Code from `aspen-sops` into `aspen-secrets`

- [x] 1.1 Add `sops` feature flag to `crates/aspen-secrets/Cargo.toml` with deps: `toml_edit`, `serde_yaml`, `serde_json`, `chrono`, `aspen-client`, `aspen-transport`, `regex`, `subtle`, `clap` (only for re-export). Add `sops` to `full` feature.
- [x] 1.2Create `crates/aspen-secrets/src/sops/format/` directory. Move `aspen-sops/src/format/mod.rs` → `sops/format/mod.rs`, `toml.rs` → `sops/format/toml.rs`, `yaml.rs` → `sops/format/yaml.rs`, `json.rs` → `sops/format/json.rs`. Gate behind `#[cfg(feature = "sops")]`.
- [x] 1.3Extract shared SOPS value encrypt/decrypt into `crates/aspen-secrets/src/sops/format/common.rs`: move `encrypt_sops_value()`, `decrypt_sops_value()`, `is_sops_encrypted()` from `decryptor.rs`. Update both `decryptor.rs` and `format/toml.rs` to call `format::common::*`.
- [x] 1.4Move `aspen-sops/src/encrypt.rs` → `crates/aspen-secrets/src/sops/encrypt.rs`. Gate behind `#[cfg(feature = "sops")]`. Update imports to use crate-local modules.
- [x] 1.5Move `aspen-sops/src/decrypt.rs` → `crates/aspen-secrets/src/sops/decrypt.rs` (the Transit-based decrypt). Gate behind `#[cfg(feature = "sops")]`. The existing `decryptor.rs` (age-based) stays unchanged.
- [x] 1.6Move `aspen-sops/src/edit.rs` → `crates/aspen-secrets/src/sops/edit.rs`. Gate behind `#[cfg(feature = "sops")]`.
- [x] 1.7Move `aspen-sops/src/rotate.rs` → `crates/aspen-secrets/src/sops/rotate.rs`. Gate behind `#[cfg(feature = "sops")]`.
- [x] 1.8Move `aspen-sops/src/updatekeys.rs` → `crates/aspen-secrets/src/sops/updatekeys.rs`. Gate behind `#[cfg(feature = "sops")]`.
- [x] 1.9Move `aspen-sops/src/mac.rs` → `crates/aspen-secrets/src/sops/mac.rs`. Gate behind `#[cfg(feature = "sops")]`.
- [x] 1.10Move `aspen-sops/src/metadata.rs` → `crates/aspen-secrets/src/sops/metadata.rs`. Gate behind `#[cfg(feature = "sops")]`.
- [x] 1.11Move `aspen-sops/src/client.rs` → `crates/aspen-secrets/src/sops/client.rs`. Gate behind `#[cfg(feature = "sops")]`.
- [x] 1.12Move `aspen-sops/src/constants.rs` → merge into `crates/aspen-secrets/src/constants.rs` (add SOPS-specific constants from `aspen-sops` that aren't already present).
- [x] 1.13Move `aspen-sops/src/verified/mac.rs` → `crates/aspen-secrets/src/verified/mac.rs` (already exists from earlier work — reconcile if both have content).
- [x] 1.14Update `crates/aspen-secrets/src/sops/mod.rs`: add `#[cfg(feature = "sops")]` module declarations for `encrypt`, `decrypt` (Transit-based), `edit`, `rotate`, `updatekeys`, `mac`, `metadata`, `client`, `format`. Add re-exports.
- [x] 1.15Update `crates/aspen-secrets/src/lib.rs`: add `#[cfg(feature = "sops")]` re-exports for new SOPS types (`EncryptConfig`, `DecryptConfig`, `TransitClient`, `SopsFileMetadata`, etc.).
- [x] 1.16Move `aspen-sops/src/error.rs` variants into `crates/aspen-secrets/src/error.rs` — add `SopsError`-equivalent variants (`TransitConnect`, `TransitEncrypt`, `MacVerificationFailed`, `EditorFailed`, etc.) to existing `SecretsError` enum. Gate behind `#[cfg(feature = "sops")]`.
- [x] 1.17Verify `cargo build -p aspen-secrets --features sops` compiles.
- [x] 1.18Verify `cargo nextest run -p aspen-secrets --features sops` — existing tests still pass.

## 2. Slim Down `aspen-sops` to CLI-Only Wrapper

- [x] 2.1Replace `aspen-sops/Cargo.toml` deps: remove all crypto/format deps (`aes-gcm`, `hmac`, `sha2`, `toml_edit`, `serde_yaml`, `regex`, `subtle`, `blake3`, `zeroize`). Add `aspen-secrets = { path = "../aspen-secrets", features = ["sops"] }`.
- [x] 2.2Delete library source files from `aspen-sops/src/`: `encrypt.rs`, `decrypt.rs`, `edit.rs`, `rotate.rs`, `updatekeys.rs`, `mac.rs`, `metadata.rs`, `client.rs`, `constants.rs`, `format/`, `verified/`, `error.rs`.
- [x] 2.3Update `aspen-sops/src/lib.rs`: replace module declarations with `pub use aspen_secrets::sops::*` re-exports.
- [x] 2.4Update `aspen-sops/src/main.rs` and `cli.rs`: change imports from `crate::` to `aspen_secrets::sops::` for all library types.
- [x] 2.5Keep `aspen-sops/src/keyservice/` (gRPC bridge) in `aspen-sops` — it's a binary-specific concern, not library code. Update its imports to `aspen_secrets::sops::*`.
- [x] 2.6Verify `cargo build --bin aspen-sops` compiles.
- [x] 2.7Verify `cargo nextest run -p aspen-sops` — existing tests still pass.

## 3. Move Cookie Validation into `aspen-secrets`

- [x] 3.1Create `crates/aspen-secrets/src/cookie/mod.rs` with `validate_cookie()`, `validate_cookie_safety()`, `UNSAFE_DEFAULT_COOKIE`, `derive_gossip_topic()`. Port from `aspen-cluster/src/validation.rs`.
- [x] 3.2Create `crates/aspen-secrets/src/verified/cookie.rs` with pure validation functions (deterministic, no I/O).
- [x] 3.3Add `CookieError` variants to `SecretsError` (or create a separate `CookieError` type): `CookieEmpty`, `CookieUnsafeDefault`.
- [x] 3.4Update `crates/aspen-secrets/src/lib.rs`: add `pub mod cookie` and re-exports.
- [x] 3.5Update `aspen-cluster/src/validation.rs`: replace function bodies with calls to `aspen_secrets::cookie::*`. Keep old function signatures as pub wrappers for backward compat.
- [x] 3.6Add `aspen-secrets` dependency to `aspen-cluster/Cargo.toml` (if not already present).
- [x] 3.7Write tests in `aspen-secrets` for cookie validation: empty rejected, unsafe default rejected, valid cookie accepted, derive_gossip_topic deterministic.
- [x] 3.8Verify `cargo nextest run -p aspen-cluster` — existing cookie validation tests still pass.

## 4. Centralize Node Identity Key Lifecycle

- [x] 4.1Create `crates/aspen-secrets/src/identity/mod.rs` with `NodeIdentityProvider` struct.
- [x] 4.2Implement `NodeIdentityProvider::from_hex(hex: &str)` — parses hex-encoded iroh SecretKey. Port logic from `aspen-cluster/src/config/iroh_config.rs`.
- [x] 4.3Implement `NodeIdentityProvider::generate()` — wraps `iroh::SecretKey::generate()` with proper RNG.
- [x] 4.4Implement `NodeIdentityProvider::load_or_generate(path: &Path)` — reads from file or generates new key, writes to file if generated. Port from scattered config loading code.
- [x] 4.5Implement `NodeIdentityProvider::public_key()` and `NodeIdentityProvider::secret_key()` accessors.
- [x] 4.6Add `#[cfg(feature = "global-discovery")]` method `dht_signing_key()` — port `iroh_secret_to_signing_key()` from `aspen-dht-discovery/src/hash.rs`.
- [x] 4.7Update `crates/aspen-secrets/src/lib.rs`: add `pub mod identity` and re-exports.
- [x] 4.8Write tests: `from_hex` roundtrip, `generate` produces valid key, `load_or_generate` creates file, `load_or_generate` reads existing file, `dht_signing_key` deterministic.
- [x] 4.9Update `aspen-cluster/src/config/iroh_config.rs`: use `NodeIdentityProvider::from_hex()` for secret_key parsing. Add backward-compat wrapper.
- [x] 4.10Update `aspen-dht-discovery/src/hash.rs`: replace `iroh_secret_to_signing_key()` with call to `NodeIdentityProvider::dht_signing_key()` (or keep standalone fn that delegates).
- [x] 4.11Verify `cargo nextest run -p aspen-cluster` and `cargo nextest run -p aspen-dht-discovery` pass.

## 5. Move Nix Cache Signing Key Logic into `aspen-secrets`

- [x] 5.1Create `crates/aspen-secrets/src/nix_cache/mod.rs` with `NixCacheKeyManager` struct. Parameterized over `SecretsBackend`.
- [x] 5.2Implement `create_signing_key(cache_name)` — wraps Transit `create_key()` with Ed25519 type. Port from `aspen-secrets-handler/src/handler/nix_cache.rs::handle_nix_cache_create_key`.
- [x] 5.3Implement `get_public_key(cache_name)` — wraps Transit `get_key()` + extract public key. Port from `handle_nix_cache_get_public_key`.
- [x] 5.4Implement `rotate_signing_key(cache_name)` — wraps Transit `rotate_key()`. Port from `handle_nix_cache_rotate_key`.
- [x] 5.5Implement `delete_signing_key(cache_name)` — wraps Transit `delete_key()`. Port from `handle_nix_cache_delete_key`.
- [x] 5.6Implement `list_signing_keys()` — wraps Transit `list_keys()`. Port from `handle_nix_cache_list_keys`.
- [x] 5.7Update `crates/aspen-secrets/src/lib.rs`: add `pub mod nix_cache` and re-exports.
- [x] 5.8Update `aspen-secrets-handler/src/handler/nix_cache.rs`: replace inline Transit logic with calls to `aspen_secrets::nix_cache::NixCacheKeyManager`. Handler becomes pure dispatch.
- [x] 5.9Write tests in `aspen-secrets`: create/get/rotate/delete key lifecycle using `InMemorySecretsBackend`.
- [x] 5.10Verify `cargo nextest run -p aspen-secrets-handler` — existing nix cache tests still pass.

## 6. Unify SOPS Value Helpers (Eliminate Duplication)

- [x] 6.1In `crates/aspen-secrets/src/sops/format/common.rs`: ensure `encrypt_sops_value()` and `decrypt_sops_value()` are the canonical implementations.
- [x] 6.2Update `crates/aspen-secrets/src/sops/decryptor.rs`: replace inline `decrypt_sops_value()` with call to `format::common::decrypt_sops_value()`. Remove the local function.
- [x] 6.3Update `crates/aspen-secrets/src/sops/decryptor.rs`: replace inline `is_sops_encrypted()` with call to `format::common::is_sops_encrypted()`. Remove the local function.
- [x] 6.4Update `crates/aspen-secrets/src/sops/format/toml.rs`: ensure it calls `common::encrypt_sops_value()` and `common::decrypt_sops_value()` instead of having its own copies.
- [x] 6.5Verify `decrypt_toml_value()` in `decryptor.rs` (age path) and `decrypt_toml_values()` in `format/toml.rs` (Transit path) both use `common::decrypt_sops_value()`.
- [x] 6.6Run full SOPS test suite: `cargo nextest run -p aspen-secrets --features sops` — all decryption and encryption tests pass with shared helpers.

## 7. Constants and Error Consolidation

- [x] 7.1Merge SOPS constants from `aspen-sops/src/constants.rs` into `crates/aspen-secrets/src/constants.rs`: `MAX_SOPS_FILE_SIZE`, `MAX_VALUE_COUNT`, `MAX_KEY_PATH_LENGTH`, `DEFAULT_TRANSIT_MOUNT`, `DEFAULT_TRANSIT_KEY`, `DEFAULT_SOCKET_PATH`, `SOPS_VERSION`. Skip any that duplicate existing constants.
- [x] 7.2Add compile-time assertions for new constants (matching Tiger Style pattern in existing `constants.rs`).
- [x] 7.3Move cookie validation constants to `constants.rs`: `MAX_COOKIE_LENGTH` (new, e.g., 256 bytes), add assertion.
- [x] 7.4Verify no duplicate constant names between old and new constants. Rename if conflicts exist.

## 8. Update Workspace Feature Wiring

- [x] 8.1In workspace root `Cargo.toml`: add `aspen-secrets/sops` to the `sops` feature chain. Ensure `full` feature includes `aspen-secrets/full`.
- [x] 8.2In `aspen-rpc-handlers/Cargo.toml`: ensure `secrets` feature pulls `aspen-secrets/full` (or at minimum `sops` + `nix-cache`).
- [x] 8.3In `aspen-sops/Cargo.toml`: the `keyservice` feature stays local (gRPC bridge). `age-fallback` maps to `aspen-secrets/sops` + `aspen-secrets` age dep.
- [x] 8.4Verify `cargo build --features full` compiles the entire workspace.
- [x] 8.5Verify `cargo build -p aspen-sops --features full` compiles.

## 9. Lint, Format, and Test

- [x] 9.1Run `nix run .#rustfmt` — fix any formatting issues from moved code.
- [x] 9.2Run `cargo clippy -p aspen-secrets --features full --all-targets -- --deny warnings` — fix all warnings.
- [x] 9.3Run `cargo clippy -p aspen-sops --all-targets -- --deny warnings` — fix all warnings.
- [x] 9.4Run `cargo clippy -p aspen-cluster --all-targets -- --deny warnings` — fix all warnings from re-export changes.
- [x] 9.5Run `cargo nextest run -p aspen-secrets --features full` — all tests pass.
- [x] 9.6Run `cargo nextest run -p aspen-sops` — all tests pass.
- [x] 9.7Run `cargo nextest run -p aspen-cluster` — all tests pass.
- [x] 9.8Run `cargo nextest run -p aspen-secrets-handler` — all tests pass.
- [x] 9.9Run `cargo nextest run -p aspen-auth` — all tests pass (re-exports work).
- [x] 9.10Run `cargo nextest run --workspace` — full workspace passes.

## 10. Documentation

- [x] 10.1Update `crates/aspen-secrets/src/lib.rs` module doc: describe the expanded scope (SOPS, cookie, identity, nix-cache).
- [x] 10.2Add doc comments to each new module (`sops/encrypt.rs`, `cookie/mod.rs`, `identity/mod.rs`, `nix_cache/mod.rs`) explaining what was moved and why.
- [x] 10.3Update `AGENTS.md` Key Modules section to reflect that `aspen-secrets` is now the unified crypto/secrets crate.
- [x] 10.4Add deprecation comments to re-export wrappers in `aspen-cluster/src/validation.rs` pointing to `aspen_secrets::cookie`.
