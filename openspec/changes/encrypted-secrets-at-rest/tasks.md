## 1. Encryption Layer

- [x] 1.1 Create `aspen-trust/src/envelope.rs` with `EncryptedValue` type: `version: u8`, `epoch: u64`, `nonce: [u8; 12]`, `ciphertext: Vec<u8>` (includes Poly1305 tag)
- [x] 1.2 Implement `encrypt_value(key: &[u8; 32], epoch: u64, nonce: &[u8; 12], plaintext: &[u8]) -> EncryptedValue`
- [x] 1.3 Implement `decrypt_value(key: &[u8; 32], value: &EncryptedValue) -> Result<Vec<u8>, DecryptError>`
- [x] 1.4 Implement `EncryptedValue::to_bytes()/from_bytes()` for the wire format: `magic(4: AENC) + version(1) + epoch(8) + nonce(12) + ciphertext+tag`
- [x] 1.5 Test: roundtrip encrypt/decrypt recovers plaintext
- [x] 1.6 Test: tampered ciphertext returns `DecryptError`
- [x] 1.7 Test: wrong key returns `DecryptError`
- [x] 1.8 Test: unknown version byte returns `ParseError`

## 2. Nonce Management

- [x] 2.1 Add `trust_nonce_counter` table to redb: `TableDefinition<u64, u64>` (node_id → counter)
- [x] 2.2 Implement `NonceGenerator` struct: loads counter from redb, increments on each `next_nonce()`, constructs 12-byte nonce as `[node_id: 4 bytes][counter: 8 bytes]`
- [x] 2.3 Persist counter to redb on each increment (or batch persist for performance)
- [x] 2.4 Test: sequential nonces are unique; nonces from different node_ids are unique

## 3. Secrets Engine Integration

- [x] 3.1 Add `SecretsEncryption` struct holding the cached derived key, nonce generator, and epoch
- [x] 3.2 Implement `wrap_write(&self, plaintext: &[u8]) -> Vec<u8>` that encrypts and serializes
- [x] 3.3 Implement `unwrap_read(&self, stored: &[u8]) -> Result<Vec<u8>, SecretsError>` that deserializes and decrypts with the appropriate epoch's derived key (multi-epoch key map; EpochMismatch only for unknown epochs)
- [x] 3.4 Integrate `wrap_write`/`unwrap_read` into KV secrets engine write/read paths (via AspenSecretsBackend put/get/put_cas/get_with_version)
- [x] 3.5 Integrate into Transit secrets engine (encrypt key material before storage)
- [x] 3.6 Integrate into PKI secrets engine (encrypt private keys before storage)
- [x] 3.7 Gate all encryption behind `trust` feature flag — without feature, secrets are stored plaintext (current behavior)

## 4. Key Lifecycle

- [x] 4.1 Implement lazy key reconstruction: on first secrets access, trigger cluster secret reconstruction via share collection, derive the at-rest key, cache it
- [x] 4.2 Return `SecretsUnavailable` error if reconstruction fails (cluster below quorum) — type defined in encryption.rs
- [x] 4.3 On epoch change notification: derive new key, swap into `SecretsEncryption`, start re-encryption task
- [x] 4.4 Zeroize key material on drop and on explicit removal — manual Drop impl fills all key bytes with zeros; remove_epoch_key() zeroizes and removes a specific old-epoch key (panics if called on current epoch). The actual call from a re-encryption completion path is deferred to task 5.1.

## 5. Re-encryption on Epoch Change

- [x] 5.1 Implement background re-encryption task: scan all secrets tables, decrypt with old key, re-encrypt with new key, write back
- [x] 5.2 Track re-encryption progress in a `trust_reencryption_progress` redb table (last processed key per table)
- [x] 5.3 Handle reads during re-encryption: check epoch prefix to determine which key to use
- [x] 5.4 Test: epoch change triggers re-encryption; all values end up at new epoch
- [x] 5.5 Test: crash during re-encryption resumes from last checkpoint on restart

## 6. Testing and Documentation

- [x] 6.1 Integration test: write secrets, verify redb contains only ciphertext, read back plaintext matches
- [ ] 6.2 Integration test: 3-node cluster, stop 2 nodes, secrets become unavailable, restart nodes, secrets available again
- [ ] 6.3 Integration test: trigger membership change, verify re-encryption runs, verify old-epoch values still readable during transition
- [x] 6.4 Document operational requirements in `docs/trust-quorum.md`: quorum needed for secrets access, backup considerations
