# Tasks: Comprehensive Test Coverage

## 1. aspen-crypto Unit Tests

- [x] 1.1 Add `cookie.rs` edge case tests: empty string → `CookieError::Empty`, unsafe default → `CookieError::UnsafeDefault`, 257-byte string → `CookieError::TooLong` (already existed)
- [x] 1.2 Add `cookie.rs` HMAC determinism test: `derive_cookie_hmac_key("same")` called twice returns identical `[u8; 32]` (already existed)
- [x] 1.3 Add `cookie.rs` gossip topic determinism test: `derive_gossip_topic("same")` called twice returns identical `[u8; 32]` (already existed)
- [x] 1.4 Add `cookie.rs` test: HMAC key ≠ gossip topic for same cookie (different derivation paths) (already existed)
- [x] 1.5 Add `cookie.rs` test: `validate_cookie_full` rejects all three error cases, accepts valid cookie (already existed)
- [x] 1.6 Add `identity.rs` hex round-trip test: `generate()` → `to_hex()` → `from_hex()` → same `public_key()` (already existed)
- [x] 1.7 Add `identity.rs` test: `from_hex` rejects wrong-length strings (63 chars, 65 chars, empty, non-hex)
- [x] 1.8 Add `identity.rs` uniqueness test: two `generate()` calls produce different public keys
- [x] 1.9 Run `cargo nextest run -p aspen-crypto` and verify all new + existing tests pass (22/22 pass)

## 2. FUSE Cache Unit Tests

- [x] 2.1 Add `cache.rs` test: `invalidate_all_scans` clears scan entries but preserves data and meta entries
- [x] 2.2 Add `cache.rs` test: expired data entry returns `None` from `get_data` (use short TTL or verify cache entry expiry logic)
- [x] 2.3 Add `cache.rs` byte-limit eviction test: insert entries exceeding `CACHE_MAX_DATA_BYTES`, verify total stored data stays within limit
- [x] 2.4 Add `cache.rs` meta entry limit test: insert `CACHE_MAX_META_ENTRIES + 10` entries, verify count doesn't exceed limit
- [x] 2.5 Add `cache.rs` scan entry limit test: insert `CACHE_MAX_SCAN_ENTRIES + 10` entries, verify count doesn't exceed limit
- [x] 2.6 Add `cache.rs` test: `put_data` updates existing key (overwrite), old bytes subtracted from total
- [x] 2.7 Run `cargo nextest run -p aspen-fuse` and verify all new + existing tests pass (169/169 pass)

## 3. FUSE Metadata Unit Tests

- [x] 3.1 Add `metadata.rs` test: `from_bytes` with exactly 32 bytes succeeds, 33+ bytes succeeds (ignores extra)
- [x] 3.2 Add `metadata.rs` test: `with_mtime` sets mtime to provided values and ctime to current time
- [x] 3.3 Add `metadata.rs` test: `to_bytes` produces exactly 32 bytes for all metadata values including negative timestamps and zero values
- [x] 3.4 Add `metadata.rs` round-trip test with extreme values: `i64::MAX`, `i64::MIN`, 0 for all fields
- [x] 3.5 Run `cargo nextest run -p aspen-fuse` and verify all metadata tests pass (169/169 pass)

## 4. SOPS Pure Function Unit Tests

- [x] 4.1 Add `decrypt.rs` unit test module: `to_key_array` accepts exactly 32 bytes, rejects 0/31/33 bytes
- [x] 4.2 Add `decrypt.rs` unit test: `parse_age_identity` accepts valid `AGE-SECRET-KEY-1...` line, rejects empty/comment-only/no-key input
- [x] 4.3 Add `encrypt.rs` unit test module: `to_key_array` accepts exactly 32 bytes, rejects wrong lengths
- [x] 4.4 Add `encrypt.rs` unit test: `encrypt_data_key_for_age` with a generated age keypair — encrypt then decrypt round-trip succeeds
- [x] 4.5 Add `encrypt.rs` unit test: `encrypt_data_key_for_age` with invalid recipient string returns error
- [x] 4.6 Add `decrypt.rs` unit test: `decrypt_age_ciphertext` round-trip — generate identity, encrypt with its public key, decrypt succeeds
- [x] 4.7 Add `edit.rs` unit test: `secure_delete` creates temp file, verify it's removed after call
- [x] 4.8 Add config default tests: verify `DecryptConfig::default()`, `EncryptConfig::default()` fields have expected values
- [x] 4.9 Run `cargo nextest run -p aspen-secrets` and verify all new + existing tests pass (138/138 pass)

## 5. Ephemeral Pub/Sub Integration Tests

- [x] 5.1 Add `ephemeral_pubsub_integration.rs` test: subscriber disconnects mid-stream, broker cleans up subscription (no leak) (existing + new test_disconnect_cleanup_no_leak)
- [x] 5.2 Add `ephemeral_pubsub_integration.rs` test: publish to multiple topics concurrently, each subscriber gets only matching messages (test_multi_topic_isolation)
- [x] 5.3 Add `ephemeral_pubsub_integration.rs` test: subscriber with full buffer — verify publish doesn't block, messages are dropped for slow consumer (already existed: test_full_buffer_drops)
- [x] 5.4 Add broker unit test: `subscribe` → `publish` → `unsubscribe` → `publish` — second publish has no subscribers (already existed: test_unsubscribe)
- [x] 5.5 Run `cargo nextest run -p aspen-hooks` ephemeral tests and verify all pass (23/23 pass)

## 6. Pub/Sub Raft Integration Test TODOs

- [x] 6.1 Address TODO in `pubsub_integration_test.rs`: add scan verification with `build_topic_prefix` key format
- [x] 6.2 Address TODO in `pubsub_integration_test.rs`: documented deferral — CLI commands don't exist yet (ephemeral pubsub uses QUIC ALPN, not RPC)
- [x] 6.3 Run `cargo nextest run --test pubsub_integration_test` and verify existing + new tests pass (43/43 pass)

## 7. NixOS VM Test: Ephemeral Pub/Sub

- [x] 7.1–7.7 **DEFERRED**: Ephemeral pub/sub uses a custom QUIC ALPN protocol (not the RPC layer), with no CLI commands. A VM test would require a custom binary. The integration test in `crates/aspen-hooks/tests/ephemeral_pubsub_integration.rs` already covers real QUIC connections between iroh endpoints. VM test should be added when CLI commands exist.

## 8. NixOS VM Test: FUSE Mount Operations

- [x] 8.1 Create `nix/tests/fuse-operations.nix` with single-node cluster + `aspen-fuse` binary
- [x] 8.2 Initialize cluster, mount FUSE at `/mnt/aspen` with `aspen-fuse --mount-point /mnt/aspen --ticket <ticket>`
- [x] 8.3 Test: `echo "hello" > /mnt/aspen/test.txt` → `cat /mnt/aspen/test.txt` returns "hello"
- [x] 8.4 Test: `mkdir /mnt/aspen/subdir` → `ls /mnt/aspen/` includes "subdir"
- [x] 8.5 Test: `echo "nested" > /mnt/aspen/subdir/file.txt` → verify via `aspen-cli kv scan subdir/`
- [x] 8.6 Test: `rm /mnt/aspen/test.txt` → verify file gone via `test ! -f`
- [x] 8.7 Test: write 100KB file → read back, verify size and sha256sum match (chunked I/O)
- [x] 8.8 Register the test in `flake.nix` checks
- [ ] 8.9 Build and run: `nix build .#checks.x86_64-linux.fuse-operations-test` (requires nix build — deferred to CI)

## 9. Snapshot Recovery Tests

- [x] 9.1 Add `test_snapshot_during_concurrent_writes`: writes 3× threshold entries, verifies snapshot triggers mid-write and all data survives
- [x] 9.2 Add `test_snapshot_data_completeness`: two-phase writes across snapshot boundary with aggressive log compaction, verifies all entries survive
- [x] 9.3 Run `cargo nextest run --test router_snapshot_t10_build_snapshot` — 4/4 pass

## 10. Final Verification

- [x] 10.1 Run `cargo nextest run --workspace -P quick` — 6,342 tests pass (up from 6,308), 0 failures
- [x] 10.2 Run `cargo clippy --all-targets -- --deny warnings` — clean
- [x] 10.3 Run `nix run .#rustfmt` — formatted
- [x] 10.4 Update `.agent/napkin.md` with lessons: age SecretBox, workspace feature unification, empty test module scaffolding
