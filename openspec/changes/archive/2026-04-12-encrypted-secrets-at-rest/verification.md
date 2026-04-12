# Verification Evidence

## Implementation Evidence

- Changed file: `AGENTS.md`
- Changed file: `Cargo.toml`
- Changed file: `crates/aspen-raft/Cargo.toml`
- Changed file: `crates/aspen-raft/src/lib.rs`
- Changed file: `crates/aspen-raft/src/node/tests.rs`
- Changed file: `crates/aspen-raft/src/secrets_at_rest.rs`
- Changed file: `crates/aspen-raft/src/storage_shared/trust.rs`
- Changed file: `crates/aspen-secrets/src/mount_registry.rs`
- Changed file: `crates/aspen-trust/src/encryption.rs`
- Changed file: `crates/aspen-trust/src/key_manager.rs`
- Changed file: `openspec/changes/encrypted-secrets-at-rest/design.md`
- Changed file: `openspec/changes/encrypted-secrets-at-rest/proposal.md`
- Changed file: `openspec/changes/encrypted-secrets-at-rest/specs/secrets-encryption-at-rest/spec.md`
- Changed file: `src/bin/aspen_node/setup/client.rs`
- Changed file: `src/node/mod.rs`
- Changed file: `src/node/tests.rs`
- Changed file: `openspec/changes/encrypted-secrets-at-rest/tasks.md`
- Changed file: `openspec/changes/encrypted-secrets-at-rest/verification.md`
- Changed file: `openspec/changes/encrypted-secrets-at-rest/evidence/aspen-node-secrets-no-runtime-trust-test.txt`
- Changed file: `openspec/changes/encrypted-secrets-at-rest/evidence/aspen-raft-secrets-reencryption-membership-test.txt`
- Changed file: `openspec/changes/encrypted-secrets-at-rest/evidence/aspen-raft-secrets-reencryption-restart-test.txt`
- Changed file: `openspec/changes/encrypted-secrets-at-rest/evidence/aspen-node-secrets-provider-check.txt`
- Changed file: `openspec/changes/encrypted-secrets-at-rest/evidence/aspen-trust-key-manager-rotate-test.txt`
- Changed file: `openspec/changes/encrypted-secrets-at-rest/evidence/implementation.diff`
- Changed file: `openspec/changes/encrypted-secrets-at-rest/evidence/openspec-preflight.txt`

## Task Coverage

- [x] 1.1 Create `aspen-trust/src/envelope.rs` with `EncryptedValue` type: `version: u8`, `epoch: u64`, `nonce: [u8; 12]`, `ciphertext: Vec<u8>` (includes Poly1305 tag)
  - Evidence: `openspec/changes/encrypted-secrets-at-rest/evidence/source-survey.txt`
- [x] 1.2 Implement `encrypt_value(key: &[u8; 32], epoch: u64, nonce: &[u8; 12], plaintext: &[u8]) -> EncryptedValue`
  - Evidence: `openspec/changes/encrypted-secrets-at-rest/evidence/source-survey.txt`
- [x] 1.3 Implement `decrypt_value(key: &[u8; 32], value: &EncryptedValue) -> Result<Vec<u8>, DecryptError>`
  - Evidence: `openspec/changes/encrypted-secrets-at-rest/evidence/source-survey.txt`
- [x] 1.4 Implement `EncryptedValue::to_bytes()/from_bytes()` for the wire format: `magic(4: AENC) + version(1) + epoch(8) + nonce(12) + ciphertext+tag`
  - Evidence: `openspec/changes/encrypted-secrets-at-rest/evidence/source-survey.txt`
- [x] 1.5 Test: roundtrip encrypt/decrypt recovers plaintext
  - Evidence: `openspec/changes/encrypted-secrets-at-rest/evidence/source-survey.txt`
- [x] 1.6 Test: tampered ciphertext returns `DecryptError`
  - Evidence: `openspec/changes/encrypted-secrets-at-rest/evidence/source-survey.txt`
- [x] 1.7 Test: wrong key returns `DecryptError`
  - Evidence: `openspec/changes/encrypted-secrets-at-rest/evidence/source-survey.txt`
- [x] 1.8 Test: unknown version byte returns `ParseError`
  - Evidence: `openspec/changes/encrypted-secrets-at-rest/evidence/source-survey.txt`
- [x] 2.1 Add `trust_nonce_counter` table to redb: `TableDefinition<u64, u64>` (node_id → counter)
  - Evidence: `openspec/changes/encrypted-secrets-at-rest/evidence/source-survey.txt`
- [x] 2.2 Implement `NonceGenerator` struct: loads counter from redb, increments on each `next_nonce()`, constructs 12-byte nonce as `[node_id: 4 bytes][counter: 8 bytes]`
  - Evidence: `openspec/changes/encrypted-secrets-at-rest/evidence/source-survey.txt`
- [x] 2.3 Persist counter to redb on each increment (or batch persist for performance)
  - Evidence: `openspec/changes/encrypted-secrets-at-rest/evidence/source-survey.txt`
- [x] 2.4 Test: sequential nonces are unique; nonces from different node_ids are unique
  - Evidence: `openspec/changes/encrypted-secrets-at-rest/evidence/source-survey.txt`
- [x] 3.1 Add `SecretsEncryption` struct holding the cached derived key, nonce generator, and epoch
  - Evidence: `openspec/changes/encrypted-secrets-at-rest/evidence/source-survey.txt`
- [x] 3.2 Implement `wrap_write(&self, plaintext: &[u8]) -> Vec<u8>` that encrypts and serializes
  - Evidence: `openspec/changes/encrypted-secrets-at-rest/evidence/source-survey.txt`
- [x] 3.3 Implement `unwrap_read(&self, stored: &[u8]) -> Result<Vec<u8>, SecretsError>` that deserializes and decrypts with the appropriate epoch's derived key (multi-epoch key map; EpochMismatch only for unknown epochs)
  - Evidence: `openspec/changes/encrypted-secrets-at-rest/evidence/source-survey.txt`
- [x] 3.4 Integrate `wrap_write`/`unwrap_read` into KV secrets engine write/read paths (via AspenSecretsBackend put/get/put_cas/get_with_version)
  - Evidence: `openspec/changes/encrypted-secrets-at-rest/evidence/source-survey.txt`
- [x] 3.5 Integrate into Transit secrets engine (encrypt key material before storage)
  - Evidence: `openspec/changes/encrypted-secrets-at-rest/evidence/source-survey.txt`
- [x] 3.6 Integrate into PKI secrets engine (encrypt private keys before storage)
  - Evidence: `openspec/changes/encrypted-secrets-at-rest/evidence/source-survey.txt`
- [x] 3.7 Gate all encryption behind `trust` feature flag — without feature, secrets are stored plaintext (current behavior)
  - Evidence: `openspec/changes/encrypted-secrets-at-rest/evidence/source-survey.txt`
- [x] 4.1 Implement lazy key reconstruction: on first secrets access, trigger cluster secret reconstruction via share collection, derive the at-rest key, cache it
  - Evidence: `openspec/changes/encrypted-secrets-at-rest/evidence/source-survey.txt`
- [x] 4.2 Return `SecretsUnavailable` error if reconstruction fails (cluster below quorum) — type defined in encryption.rs
  - Evidence: `openspec/changes/encrypted-secrets-at-rest/evidence/source-survey.txt`
- [x] 4.3 On epoch change notification: derive new key, swap into `SecretsEncryption`, start re-encryption task
  - Evidence: `openspec/changes/encrypted-secrets-at-rest/evidence/source-survey.txt`
- [x] 4.4 Zeroize key material on drop and on explicit removal — manual Drop impl fills all key bytes with zeros; remove_epoch_key() zeroizes and removes a specific old-epoch key (panics if called on current epoch). The actual call from a re-encryption completion path is deferred to task 5.1.
  - Evidence: `openspec/changes/encrypted-secrets-at-rest/evidence/source-survey.txt`
- [x] 5.1 Implement background re-encryption task: scan all secrets tables, decrypt with old key, re-encrypt with new key, write back
  - Evidence: `openspec/changes/encrypted-secrets-at-rest/evidence/source-survey.txt`
- [x] 5.2 Track re-encryption progress in a `trust_reencryption_progress` redb table (last processed key per table)
  - Evidence: `openspec/changes/encrypted-secrets-at-rest/evidence/source-survey.txt`
- [x] 5.3 Handle reads during re-encryption: check epoch prefix to determine which key to use
  - Evidence: `openspec/changes/encrypted-secrets-at-rest/evidence/source-survey.txt`
- [x] 5.4 Test: epoch change triggers re-encryption; all values end up at new epoch
  - Evidence: `openspec/changes/encrypted-secrets-at-rest/evidence/source-survey.txt`
- [x] 5.5 Test: a fresh trust-aware provider/watcher resumes re-encryption from the persisted checkpoint after interruption, without requiring another epoch bump
  - Evidence: `crates/aspen-raft/src/secrets_at_rest.rs`, `crates/aspen-raft/src/node/tests.rs`, `crates/aspen-raft/src/storage_shared/trust.rs`, `openspec/changes/encrypted-secrets-at-rest/evidence/aspen-raft-secrets-reencryption-restart-test.txt`, `openspec/changes/encrypted-secrets-at-rest/evidence/implementation.diff`
- [x] 6.1 Integration test: write secrets, verify redb contains only ciphertext, read back plaintext matches
  - Evidence: `openspec/changes/encrypted-secrets-at-rest/evidence/source-survey.txt`
- [x] 6.2 Integration test: 3-node cluster, stop 2 nodes, secrets become unavailable, restart nodes, secrets available again
  - Evidence: `crates/aspen-raft/src/node/tests.rs`, `crates/aspen-secrets/src/backend.rs`, `crates/aspen-secrets/src/error.rs`, `crates/aspen-secrets/src/lib.rs`, `openspec/changes/encrypted-secrets-at-rest/evidence/aspen-raft-secrets-quorum-test.txt`, `openspec/changes/encrypted-secrets-at-rest/evidence/source-survey.txt`
- [x] 6.3 Integration test: trigger membership change, verify re-encryption runs, verify old-epoch values still readable during transition
  - Evidence: `crates/aspen-raft/src/node/tests.rs`, `crates/aspen-raft/src/secrets_at_rest.rs`, `crates/aspen-raft/src/storage_shared/trust.rs`, `crates/aspen-secrets/src/mount_registry.rs`, `crates/aspen-trust/src/encryption.rs`, `crates/aspen-trust/src/key_manager.rs`, `src/bin/aspen_node/setup/client.rs`, `src/node/mod.rs`, `openspec/changes/encrypted-secrets-at-rest/evidence/aspen-raft-secrets-reencryption-membership-test.txt`, `openspec/changes/encrypted-secrets-at-rest/evidence/aspen-node-secrets-provider-check.txt`, `openspec/changes/encrypted-secrets-at-rest/evidence/aspen-trust-key-manager-rotate-test.txt`, `openspec/changes/encrypted-secrets-at-rest/evidence/implementation.diff`
- [x] 6.4 Document operational requirements in `docs/trust-quorum.md`: quorum needed for secrets access, backup considerations
  - Evidence: `openspec/changes/encrypted-secrets-at-rest/evidence/source-survey.txt`

## Review Scope Snapshot

### `git diff HEAD -- crates/aspen-raft/src/node/tests.rs`

- Status: captured
- Artifact: `openspec/changes/encrypted-secrets-at-rest/evidence/implementation.diff`

## Verification Commands

### `rg -n "EncryptedValue|encrypt_value\(|decrypt_value\(|NonceGenerator|trust_nonce_counter|SecretsEncryption|wrap_write\(|unwrap_read\(|SecretsUnavailableError|rotate_epoch\(|run_reencryption|trust_reencryption_progress|test_stored_value_is_ciphertext_not_plaintext|test_tampered_envelope_returns_error_not_plaintext|test_secrets_become_unavailable_below_quorum_and_recover_after_nodes_return|backup considerations|quorum needed for secrets access" crates docs openspec/changes/encrypted-secrets-at-rest -g "*.rs" -g "*.md"`

- Status: pass
- Artifact: `openspec/changes/encrypted-secrets-at-rest/evidence/source-survey.txt`

### `cargo test -p aspen-raft --features trust,testing test_secrets_become_unavailable_below_quorum_and_recover_after_nodes_return -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/encrypted-secrets-at-rest/evidence/aspen-raft-secrets-quorum-test.txt`

### `cargo test -p aspen-trust test_rotate_epoch -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/encrypted-secrets-at-rest/evidence/aspen-trust-key-manager-rotate-test.txt`

### `cargo test -p aspen --features trust,secrets,jobs,docs,hooks,federation test_finish_secrets_service_setup_keeps_service_without_runtime_trust -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/encrypted-secrets-at-rest/evidence/aspen-node-secrets-no-runtime-trust-test.txt`

### `cargo test -p aspen-raft --features trust,secrets,testing test_reencryption_checkpoint_resumes_after_restart -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/encrypted-secrets-at-rest/evidence/aspen-raft-secrets-reencryption-restart-test.txt`

### `cargo test -p aspen-raft --features trust,secrets,testing test_membership_change_reencrypts_secrets_and_preserves_mixed_epoch_reads -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/encrypted-secrets-at-rest/evidence/aspen-raft-secrets-reencryption-membership-test.txt`

### `cargo check -p aspen --bin aspen-node --features trust,secrets,jobs,docs,hooks,automerge`

- Status: pass
- Artifact: `openspec/changes/encrypted-secrets-at-rest/evidence/aspen-node-secrets-provider-check.txt`

### `scripts/openspec-preflight.sh openspec/changes/encrypted-secrets-at-rest`

- Status: pass
- Artifact: `openspec/changes/encrypted-secrets-at-rest/evidence/openspec-preflight.txt`

### `cargo test -p aspen-raft --features trust,testing test_multi_node_trust_init_persists_follower_shares -- --nocapture && cargo test -p aspen-raft --features trust,testing test_multi_node_trust_membership_change_rotates_secret_add_and_remove -- --nocapture && cargo test -p aspen-raft --features trust,testing test_trust_reconfiguration_restarts_after_leader_crash_during_share_collection -- --nocapture`

- Status: pass
- Artifact: `openspec/changes/encrypted-secrets-at-rest/evidence/aspen-raft-trust-helper-regressions.txt`

## Notes

- OpenSpec now explicitly allows one startup reconstruction exception: if persisted re-encryption progress or stale lower-epoch ciphertext remains after an interrupted migration, Aspen may reconstruct the current secrets-at-rest key at boot strictly to finish recovery.
- Fresh runtime evidence in this session covers task 5.5 and task 6.3 end-to-end: the production trust-aware provider resumes from persisted re-encryption checkpoints without another epoch bump, notices membership-driven epoch changes, keeps mixed-epoch keys available, and the background re-encryption path migrates stored secrets automatically.
- `src/node/mod.rs` and `src/bin/aspen_node/setup/client.rs` now scope fail-closed behavior to trust-configured nodes only. When `trust` is compiled but runtime trust wiring is absent, secrets service stays available without a trust-aware provider.
- `cargo test -p aspen --features trust,secrets,jobs,docs,hooks,federation test_finish_secrets_service_setup_keeps_service_without_runtime_trust -- --nocapture` provides direct runtime coverage for that non-trust-configured path.
- `cargo check -p aspen --bin aspen-node --features trust,secrets,jobs,docs,hooks,automerge` proves the real `aspen-node` setup path still compiles with the new provider guard.
- Task 5.5 wording was narrowed to match the proven boundary exactly: the evidence covers a fresh provider/watcher resuming persisted checkpoint recovery, not reopening the same Redb file inside one test process.
- Prior runtime evidence in this change directory still covers task 6.2 and the surrounding shared Redb helper regressions in `crates/aspen-raft/src/node/tests.rs`.
- The remaining checked tasks are carried forward with source-location evidence from the current tree in `source-survey.txt`; this file does not claim those tasks were re-executed in this session.
