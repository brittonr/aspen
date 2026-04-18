//! Background re-encryption of secrets after epoch rotation.
//!
//! When the cluster secret rotates (membership change), all stored secrets
//! must be re-encrypted with the new epoch's derived key. This module
//! provides the re-encryption logic with checkpoint support so it can
//! resume after crashes.
//!
//! The re-encryption process:
//! 1. Scan all secrets values in the KV store
//! 2. For each value, check if it's encrypted with the old epoch
//! 3. Decrypt with old key, re-encrypt with new key, write back
//! 4. Checkpoint progress after each key
//! 5. When done, remove old epoch key from the encryption context

use async_trait::async_trait;

use crate::encryption::SecretsEncryption;
use crate::envelope::EncryptedValue;

/// Trait for accessing the secrets store during re-encryption.
///
/// Abstracts the storage layer so re-encryption logic doesn't depend
/// on the specific KV implementation.
#[async_trait]
pub trait ReencryptionStore: Send + Sync {
    /// Scan keys with the given prefix, starting after `after_key` if provided.
    ///
    /// Returns up to `batch_size` key-value pairs.
    async fn scan_secrets(
        &self,
        prefix: &str,
        after_key: Option<&str>,
        batch_size: u32,
    ) -> Result<Vec<(String, Vec<u8>)>, ReencryptionError>;

    /// Write a re-encrypted value back to storage.
    async fn write_reencrypted(&self, key: &str, value: &[u8]) -> Result<(), ReencryptionError>;

    /// Save checkpoint: last processed key for a table.
    async fn save_checkpoint(&self, table_name: &str, last_key: &str) -> Result<(), ReencryptionError>;

    /// Load checkpoint: get last processed key for a table, if any.
    async fn load_checkpoint(&self, table_name: &str) -> Result<Option<String>, ReencryptionError>;

    /// Clear checkpoint for a table (re-encryption complete).
    async fn clear_checkpoint(&self, table_name: &str) -> Result<(), ReencryptionError>;
}

/// Errors during re-encryption.
#[derive(Debug, snafu::Snafu)]
pub enum ReencryptionError {
    /// Storage error during scan/write.
    #[snafu(display("storage error: {reason}"))]
    Storage { reason: String },

    /// Decryption failed for a value during re-encryption.
    #[snafu(display("decryption failed for key '{key}': {reason}"))]
    Decryption { key: String, reason: String },
}

/// Result of a re-encryption run.
#[derive(Debug)]
pub struct ReencryptionResult {
    /// Number of keys re-encrypted.
    pub keys_reencrypted: u64,
    /// Number of keys skipped (already at target epoch or not encrypted).
    pub keys_skipped: u64,
    /// Whether re-encryption completed for all keys.
    pub is_complete: bool,
}

/// Maximum number of keys to process in a single batch.
const REENCRYPTION_BATCH_SIZE: u32 = 100;
/// Maximum number of batches to process in a single reencryption pass.
/// At 100 keys per batch, this allows up to 10 million keys.
const MAX_REENCRYPTION_BATCHES: u32 = 100_000;

/// Re-encrypt all secrets from `old_epoch` to the current epoch.
///
/// Processes keys in batches, checkpointing after each batch.
/// Can be called repeatedly to resume after interruption.
///
/// The `encryption` context must have both old and new epoch keys loaded.
pub async fn reencrypt_secrets(
    store: &dyn ReencryptionStore,
    encryption: &SecretsEncryption,
    old_epoch: u64,
    prefix: &str,
) -> Result<ReencryptionResult, ReencryptionError> {
    debug_assert!(old_epoch > 0, "epoch 0 is not a valid trust epoch");
    debug_assert!(!prefix.is_empty(), "reencryption prefix must not be empty");
    let table_name = prefix;
    let mut keys_reencrypted: u64 = 0;
    let mut keys_skipped: u64 = 0;

    // Resume from checkpoint if one exists
    let mut after_key = store.load_checkpoint(table_name).await?;

    let mut batches_processed: u32 = 0;
    loop {
        debug_assert!(batches_processed < MAX_REENCRYPTION_BATCHES, "reencryption exceeded maximum batches");
        batches_processed = batches_processed.saturating_add(1);
        if batches_processed >= MAX_REENCRYPTION_BATCHES {
            break;
        }

        let batch = store.scan_secrets(prefix, after_key.as_deref(), REENCRYPTION_BATCH_SIZE).await?;

        if batch.is_empty() {
            break;
        }

        for (key, stored_bytes) in &batch {
            // Check if this value needs re-encryption
            match EncryptedValue::from_bytes(stored_bytes) {
                Ok(envelope) if envelope.epoch == old_epoch => {
                    // Decrypt with old key, re-encrypt with new key
                    let plaintext =
                        encryption.unwrap_read(stored_bytes).map_err(|e| ReencryptionError::Decryption {
                            key: key.clone(),
                            reason: e.to_string(),
                        })?;

                    let (new_ciphertext, _counter) =
                        encryption.wrap_write(&plaintext).map_err(|e| ReencryptionError::Decryption {
                            key: key.clone(),
                            reason: e.to_string(),
                        })?;

                    store.write_reencrypted(key, &new_ciphertext).await?;
                    keys_reencrypted = keys_reencrypted.saturating_add(1);
                }
                _ => {
                    // Already at current epoch, not encrypted, or parse failed (plaintext)
                    keys_skipped = keys_skipped.saturating_add(1);
                }
            }

            after_key = Some(key.clone());
        }

        // Checkpoint after each batch
        if let Some(ref last) = after_key {
            store.save_checkpoint(table_name, last).await?;
        }

        // If we got fewer than batch_size, we've reached the end
        if (batch.len() as u32) < REENCRYPTION_BATCH_SIZE {
            break;
        }
    }

    // Clear checkpoint — re-encryption is complete
    store.clear_checkpoint(table_name).await?;

    Ok(ReencryptionResult {
        keys_reencrypted,
        keys_skipped,
        is_complete: true,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    use super::*;

    /// In-memory store for testing re-encryption.
    // Lock order: data -> checkpoints.
    struct TestStore {
        data: Mutex<BTreeMap<String, Vec<u8>>>,
        checkpoints: Mutex<BTreeMap<String, String>>,
    }

    impl TestStore {
        fn new() -> Self {
            Self {
                data: Mutex::new(BTreeMap::new()),
                checkpoints: Mutex::new(BTreeMap::new()),
            }
        }

        fn insert(&self, key: &str, value: Vec<u8>) {
            self.data.lock().unwrap().insert(key.to_string(), value);
        }

        fn get(&self, key: &str) -> Option<Vec<u8>> {
            self.data.lock().unwrap().get(key).cloned()
        }
    }

    #[async_trait]
    impl ReencryptionStore for TestStore {
        async fn scan_secrets(
            &self,
            prefix: &str,
            after_key: Option<&str>,
            batch_key_count: u32,
        ) -> Result<Vec<(String, Vec<u8>)>, ReencryptionError> {
            let data = self.data.lock().unwrap();
            let batch_limit_keys = usize::try_from(batch_key_count).unwrap_or(usize::MAX);
            let iter = data
                .iter()
                .filter(|(k, _)| k.starts_with(prefix))
                .filter(|(k, _)| match after_key {
                    Some(after) => k.as_str() > after,
                    None => true,
                })
                .take(batch_limit_keys)
                .map(|(k, v)| (k.clone(), v.clone()));
            Ok(iter.collect())
        }

        async fn write_reencrypted(&self, key: &str, value: &[u8]) -> Result<(), ReencryptionError> {
            self.data.lock().unwrap().insert(key.to_string(), value.to_vec());
            Ok(())
        }

        async fn save_checkpoint(&self, table_name: &str, last_key: &str) -> Result<(), ReencryptionError> {
            self.checkpoints.lock().unwrap().insert(table_name.to_string(), last_key.to_string());
            Ok(())
        }

        async fn load_checkpoint(&self, table_name: &str) -> Result<Option<String>, ReencryptionError> {
            Ok(self.checkpoints.lock().unwrap().get(table_name).cloned())
        }

        async fn clear_checkpoint(&self, table_name: &str) -> Result<(), ReencryptionError> {
            self.checkpoints.lock().unwrap().remove(table_name);
            Ok(())
        }
    }

    fn make_encryption(secret: &[u8; 32], epoch: u64) -> SecretsEncryption {
        SecretsEncryption::new(secret, b"test-cluster", epoch, 1, 0)
    }

    #[tokio::test]
    async fn test_reencrypt_epoch_change() {
        let secret1 = [0xAAu8; 32];
        let secret2 = [0xBBu8; 32];

        // Create epoch 1 encryption and encrypt some values
        let enc1 = make_encryption(&secret1, 1);
        let store = TestStore::new();

        let (ct1, _) = enc1.wrap_write(b"secret-one").unwrap();
        let (ct2, _) = enc1.wrap_write(b"secret-two").unwrap();
        let (ct3, _) = enc1.wrap_write(b"secret-three").unwrap();

        store.insert("secrets/a", ct1);
        store.insert("secrets/b", ct2);
        store.insert("secrets/c", ct3);

        // Create epoch 2 encryption with both keys
        let mut enc2 = make_encryption(&secret2, 2);
        // Add epoch 1 key so we can decrypt old values
        let old_key = crate::kdf::derive_key(&secret1, crate::kdf::CONTEXT_SECRETS_AT_REST, b"test-cluster", 1);
        enc2.add_epoch_key(1, old_key);

        // Re-encrypt
        let result = reencrypt_secrets(&store, &enc2, 1, "secrets/").await.unwrap();

        assert_eq!(result.keys_reencrypted, 3);
        assert_eq!(result.keys_skipped, 0);
        assert!(result.is_complete);

        // Verify all values are now at epoch 2
        for key in ["secrets/a", "secrets/b", "secrets/c"] {
            let stored = store.get(key).unwrap();
            let epoch = SecretsEncryption::peek_epoch(&stored).unwrap();
            assert_eq!(epoch, 2, "key {key} should be at epoch 2");
        }

        // Verify values are still correct
        let a = enc2.unwrap_read(&store.get("secrets/a").unwrap()).unwrap();
        assert_eq!(&a, b"secret-one");

        // Checkpoint should be cleared
        assert!(store.load_checkpoint("secrets/").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_reencrypt_skips_already_migrated() {
        let secret1 = [0xAAu8; 32];
        let secret2 = [0xBBu8; 32];

        let enc1 = make_encryption(&secret1, 1);
        let mut enc2 = make_encryption(&secret2, 2);
        let old_key = crate::kdf::derive_key(&secret1, crate::kdf::CONTEXT_SECRETS_AT_REST, b"test-cluster", 1);
        enc2.add_epoch_key(1, old_key);

        let store = TestStore::new();

        // One old-epoch value
        let (ct_old, _) = enc1.wrap_write(b"old-secret").unwrap();
        store.insert("secrets/old", ct_old);

        // One already-at-epoch-2 value
        let (ct_new, _) = enc2.wrap_write(b"new-secret").unwrap();
        store.insert("secrets/new", ct_new);

        // One plaintext value (not encrypted)
        store.insert("secrets/plain", b"just-plaintext".to_vec());

        let result = reencrypt_secrets(&store, &enc2, 1, "secrets/").await.unwrap();
        assert_eq!(result.keys_reencrypted, 1); // only the old one
        assert_eq!(result.keys_skipped, 2); // new + plaintext
    }

    #[tokio::test]
    async fn test_reencrypt_resume_from_checkpoint() {
        let secret1 = [0xAAu8; 32];
        let secret2 = [0xBBu8; 32];

        let enc1 = make_encryption(&secret1, 1);
        let mut enc2 = make_encryption(&secret2, 2);
        let old_key = crate::kdf::derive_key(&secret1, crate::kdf::CONTEXT_SECRETS_AT_REST, b"test-cluster", 1);
        enc2.add_epoch_key(1, old_key);

        let store = TestStore::new();

        let (ct1, _) = enc1.wrap_write(b"s1").unwrap();
        let (ct2, _) = enc1.wrap_write(b"s2").unwrap();
        let (ct3, _) = enc1.wrap_write(b"s3").unwrap();
        store.insert("secrets/a", ct1);
        store.insert("secrets/b", ct2.clone());
        store.insert("secrets/c", ct3.clone());

        // Simulate a crash after processing "secrets/a"
        // Set checkpoint and manually re-encrypt "a"
        store.save_checkpoint("secrets/", "secrets/a").await.unwrap();
        let (new_ct1, _) = enc2.wrap_write(b"s1").unwrap();
        store.insert("secrets/a", new_ct1);

        // Resume — should skip "a" and process "b" and "c"
        let result = reencrypt_secrets(&store, &enc2, 1, "secrets/").await.unwrap();
        assert_eq!(result.keys_reencrypted, 2); // b + c
        assert!(result.is_complete);

        // All values at epoch 2
        for key in ["secrets/a", "secrets/b", "secrets/c"] {
            let stored = store.get(key).unwrap();
            let epoch = SecretsEncryption::peek_epoch(&stored).unwrap();
            assert_eq!(epoch, 2, "key {key} should be at epoch 2");
        }
    }
}
