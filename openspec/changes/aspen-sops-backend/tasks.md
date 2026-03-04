## 1. Crate Scaffolding & Transit Client

- [x] 1.1 Create `crates/aspen-sops/Cargo.toml` with dependencies (aspen-client, aspen-client-api, aspen-transport, aes-gcm, hmac, sha2, zeroize, toml, toml_edit, clap, iroh, serde, base64, snafu, tracing, tokio). Feature flags: `keyservice` (tonic/prost), `age-fallback` (age), `full`.
- [x] 1.2 Add `aspen-sops` to workspace `Cargo.toml` members list
- [x] 1.3 Create `src/lib.rs` with module declarations: `client`, `encrypt`, `decrypt`, `edit`, `rotate`, `updatekeys`, `mac`, `metadata`, `format`, `error`, `constants`, `verified`
- [x] 1.4 Create `src/error.rs` with `SopsError` enum (snafu): `TransitConnect`, `TransitEncrypt`, `TransitDecrypt`, `FileRead`, `FileWrite`, `FileTooLarge`, `ParseFile`, `InvalidFormat`, `MacVerificationFailed`, `NoMatchingKeyGroup`, `EditorFailed`, `KeyServiceBind`, `ValueEncrypt`, `ValueDecrypt`, `InvalidCiphertext`, `Serialization`
- [x] 1.5 Create `src/constants.rs`: `MAX_SOPS_FILE_SIZE` (1 MB), `MAX_VALUE_COUNT` (10,000), `MAX_KEY_PATH_LENGTH` (512), `DEFAULT_TRANSIT_MOUNT` ("transit"), `DEFAULT_TRANSIT_KEY` ("sops-data-key"), `DEFAULT_SOCKET_PATH` ("/tmp/aspen-sops.sock"), `SOPS_VERSION` ("3.9.0")
- [x] 1.6 Create `src/client.rs` with `TransitClient` struct: `connect(cluster_ticket, mount)`, `generate_data_key(key_name, bits) -> (Zeroizing<Vec<u8>>, String, u32)`, `decrypt_data_key(key_name, ciphertext) -> Zeroizing<Vec<u8>>`, `rewrap_data_key(key_name, ciphertext) -> (String, u32)`, `encrypt_data(key_name, plaintext) -> (String, u32)`
- [x] 1.7 Write unit tests for `TransitClient` using mock/in-memory Transit store from `aspen-secrets`
- [x] 1.8 Verify `cargo build -p aspen-sops` compiles

## 2. SOPS Metadata Types

- [x] 2.1 Create `src/metadata.rs` with `AspenTransitRecipient` struct: `cluster_ticket: String`, `mount: String`, `name: String`, `enc: String`, `key_version: u32`. Derive `Serialize`, `Deserialize`.
- [x] 2.2 Add `SopsFileMetadata` struct: `age: Vec<AgeRecipient>`, `aspen_transit: Vec<AspenTransitRecipient>`, `lastmodified: String`, `mac: String`, `version: String`, `encrypted_regex: Option<String>`. Reuse `AgeRecipient` from `aspen-secrets`.
- [x] 2.3 Implement `SopsFileMetadata::has_aspen_transit() -> bool`, `find_aspen_recipient(cluster_ticket) -> Option<&AspenTransitRecipient>`, `add_aspen_recipient(recipient)`, `remove_aspen_recipient(cluster_ticket)`
- [x] 2.4 Write tests for metadata serialization roundtrip (TOML with `[[sops.aspen_transit]]` array-of-tables)
- [x] 2.5 Write tests for metadata with mixed key groups (age + aspen_transit coexisting)

## 3. MAC Computation (Verified Core)

- [x] 3.1 Create `src/verified/mod.rs` with `pub mod mac;`
- [x] 3.2 Create `src/verified/mac.rs` with pure functions: `compute_sops_mac(values: &[(String, String)], data_key: &[u8; 32]) -> [u8; 32]` using HMAC-SHA256 over sorted (key_path, value) pairs. `collect_value_paths(table: &toml::Value, prefix: &str) -> Vec<(String, String)>` to flatten a TOML tree into sorted key-value pairs.
- [x] 3.3 Create `src/mac.rs` shell: `verify_mac(encrypted_mac: &str, data_key: &[u8; 32], values: &[(String, String)]) -> Result<()>` — decrypts MAC, recomputes, compares in constant time. `encrypt_mac(data_key: &[u8; 32], values: &[(String, String)]) -> Result<String>` — computes MAC, encrypts as `ENC[AES256_GCM,...]`.
- [ ] 3.4 Create `verus/mac_spec.rs` with Verus specifications for `compute_sops_mac`: ensures deterministic output for same inputs, ensures different values produce different MACs (collision resistance modulo HMAC-SHA256).
- [x] 3.5 Write unit tests: MAC of empty values, MAC of single value, MAC stability (same input → same output), MAC changes when values change, MAC changes when key paths change

## 4. SOPS Value Encryption/Decryption

- [x] 4.1 Create `src/format/mod.rs` with `SopsFormat` enum (`Toml`, `Yaml`, `Json`), `detect_format(path: &Path) -> SopsFormat`, format dispatch
- [x] 4.2 Create `src/format/toml.rs` with `encrypt_toml_values(table: &mut toml_edit::DocumentMut, data_key: &[u8; 32], encrypted_regex: Option<&str>) -> Result<Vec<(String, String)>>` — walks TOML tree, encrypts string/integer/float/bool values to `ENC[AES256_GCM,...]` format, returns collected (path, plaintext) pairs for MAC. Skips the `[sops]` table.
- [x] 4.3 Add `decrypt_toml_values(table: &mut toml_edit::DocumentMut, data_key: &[u8; 32]) -> Result<Vec<(String, String)>>` — walks TOML tree, decrypts `ENC[...]` values back to plaintext, returns collected pairs for MAC verification.
- [x] 4.4 Add helpers: `encrypt_sops_value(plaintext: &str, data_key: &[u8; 32], value_type: &str) -> Result<String>` produces `ENC[AES256_GCM,data:...,iv:...,tag:...,type:<type>]`. `decrypt_sops_value(encrypted: &str, data_key: &[u8; 32]) -> Result<String>` parses and decrypts (reuse logic from `aspen-secrets/src/sops/decryptor.rs`).
- [x] 4.5 Write tests: encrypt then decrypt roundtrip for string, integer, float, boolean values. Verify `ENC[AES256_GCM,...]` format matches Go SOPS output structure.
- [x] 4.6 Write test: `encrypted_regex` only encrypts matching values, leaves others plaintext
- [x] 4.7 Create stub `src/format/yaml.rs` and `src/format/json.rs` with `unimplemented!("YAML/JSON support planned for v2")` — gated behind future feature flag

## 5. Encrypt Operation

- [x] 5.1 Create `src/encrypt.rs` with `encrypt_file(config: &EncryptConfig) -> Result<String>`
- [x] 5.2 Add `EncryptConfig` struct: `input_path: PathBuf`, `cluster_ticket: String`, `transit_key: String`, `transit_mount: String`, `age_recipients: Vec<String>`, `encrypted_regex: Option<String>`, `in_place: bool`
- [x] 5.3 Implement age recipient support: if `age_recipients` is non-empty, also encrypt data key for each age recipient (using `age::Encryptor`) and add `[[sops.age]]` entries
- [x] 5.4 Write integration test: encrypt a sample TOML file, verify output has `[sops]` section with `aspen_transit` metadata, verify all values are `ENC[...]` format
- [x] 5.5Write integration test: encrypt with both Aspen Transit and age recipients, verify both key groups in metadata

## 6. Decrypt Operation

- [x] 6.1 Create `src/decrypt.rs` with `decrypt_file(config: &DecryptConfig) -> Result<String>`
- [x] 6.2 Add `DecryptConfig` struct: `input_path: PathBuf`, `cluster_ticket: Option<String>`, `output_path: Option<PathBuf>`, `extract_path: Option<String>`, `age_identity: Option<PathBuf>`
- [x] 6.3 Implement `--extract` support: parse dotted path (e.g., `secrets.strings.api_key`), return only that value
- [x] 6.4 Write integration test: encrypt then decrypt roundtrip, verify output matches original
- [ ] 6.5 Write integration test: decrypt with age fallback when Transit is unavailable
- [x] 6.6 Write integration test: MAC verification failure on tampered file (modify an encrypted value, verify decrypt fails)

## 7. Rotate & UpdateKeys Operations

- [x] 7.1 Create `src/rotate.rs` with `rotate_file(config: &RotateConfig) -> Result<String>`
- [x] 7.2 Create `src/updatekeys.rs` with `update_keys(config: &UpdateKeysConfig) -> Result<String>`
- [ ] 7.3 Write test: rotate after Transit key rotation — verify `key_version` incremented, values unchanged
- [ ] 7.4 Write test: updatekeys — add age recipient to Transit-only file, verify both key groups present

## 8. Edit Operation

- [x] 8.1 Create `src/edit.rs` with `edit_file(config: &EditConfig) -> Result<()>`
- [ ] 8.2 Write test: mock editor that modifies a value, verify re-encrypted output reflects change
- [ ] 8.3 Write test: mock editor that makes no changes, verify file is unchanged

## 9. CLI Binary

- [x] 9.1 Create `src/cli.rs` with clap derive structs: `Cli` (top-level), `Commands` enum (`Encrypt`, `Decrypt`, `Edit`, `Rotate`, `UpdateKeys`, `Keyservice`). Each variant has the flags from the design doc.
- [x] 9.2 Create `src/main.rs`: parse CLI, initialize tracing, dispatch to operation functions. Handle `ASPEN_CLUSTER_TICKET` env var fallback for `--cluster-ticket`.
- [x] 9.3 Add `[[bin]] name = "aspen-sops"` to `Cargo.toml`
- [x] 9.4 Write CLI smoke test: `aspen-sops --help` exits 0, `aspen-sops encrypt --help` shows expected flags
- [x] 9.5 Verify `cargo build --bin aspen-sops` compiles

## 10. gRPC Key Service Bridge (feature-gated)

- [ ] 10.1 Add SOPS KeyService protobuf definition to `src/keyservice/keyservice.proto` (Encrypt/Decrypt RPCs with Key message containing `encrypted_key` bytes)
- [ ] 10.2 Add `build.rs` with `tonic_build::compile_protos("src/keyservice/keyservice.proto")` gated behind `keyservice` feature
- [ ] 10.3 Create `src/keyservice/mod.rs` with `start_keyservice(config: &KeyserviceConfig) -> Result<()>`: bind Unix socket, serve gRPC
- [ ] 10.4 Create `src/keyservice/bridge.rs` with `AspenKeyServiceBridge` implementing SOPS `KeyService` trait: `encrypt()` → `transit_client.encrypt_data()`, `decrypt()` → `transit_client.decrypt_data_key()`
- [ ] 10.5 Write integration test: start key service on Unix socket, send gRPC encrypt/decrypt requests, verify roundtrip
- [ ] 10.6 Write test: key service graceful shutdown on SIGTERM

## 11. Integration with aspen-secrets

- [ ] 11.1 Add `AspenTransitIdentity` variant to `aspen-secrets/src/sops/decryptor.rs` `decrypt_data_key()` function — try Aspen Transit alongside age
- [x] 11.2Extend `SecretsConfig` with optional `transit_cluster_ticket: Option<String>` and `transit_key_name: Option<String>` fields for Transit-based SOPS decryption at node startup
- [ ] 11.3 Write test: `decrypt_secrets_file()` with Aspen Transit metadata succeeds when Transit client is available
- [ ] 11.4 Write test: `decrypt_secrets_file()` falls back to age when Transit is unavailable

## 12. SOPS Compatibility Tests

- [x] 12.1 Create `tests/sops_compatibility.rs` with golden test: encrypt a known plaintext file, verify the output structure matches SOPS expectations (has `[sops]` table, all values are `ENC[AES256_GCM,...]`, MAC present)
- [ ] 12.2 Create test: decrypt a file encrypted by Go SOPS (with age) using `aspen-sops` — verify interop with existing SOPS files
- [ ] 12.3 Create test: encrypt with `aspen-sops`, decrypt with Go SOPS (via key service bridge) — full roundtrip interop
- [ ] 12.4 Create test: multi-key-group file (age + aspen_transit) — Go SOPS can decrypt with age, `aspen-sops` can decrypt with Transit
- [x] 12.5 Create test: `encrypted_regex` — only matched values encrypted, others plaintext
- [x] 12.6Verify `cargo nextest run -p aspen-sops` — all tests pass

## 13. Documentation & Feature Flag Wiring

- [ ] 13.1 Add `sops` feature flag to workspace `Cargo.toml` and `aspen-rpc-handlers` that pulls in `aspen-sops`
- [ ] 13.2 Add `aspen-sops` to the `full` feature set
- [x] 13.3Write `crates/aspen-sops/README.md`: usage examples for encrypt/decrypt/edit/rotate/keyservice, configuration, multi-key-group setup, CI integration example
- [x] 13.4Add `docs/sops.md` design document explaining the architecture, security model, and operational guide
- [x] 13.5Verify `cargo clippy -p aspen-sops --all-targets -- --deny warnings` passes
- [x] 13.6Verify `nix run .#rustfmt` passes
