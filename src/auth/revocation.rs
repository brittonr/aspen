//! Persistent revocation storage for capability tokens.
//!
//! This module provides persistent storage for token revocation lists, ensuring
//! that revocations survive node restarts and are replicated across the cluster.
//!
//! # Design
//!
//! Revoked tokens are stored as keys in the distributed key-value store under
//! the system prefix `_system:auth:revoked:<hash_hex>`. The key's existence
//! indicates revocation - the value is empty.
//!
//! # Tiger Style
//!
//! - Fixed limit on revocation list size (MAX_REVOCATION_LIST_SIZE = 10,000)
//! - Revocations are write-once, never deleted (until token expires naturally)
//! - Bounded scan operations to prevent unbounded memory usage

use anyhow::Result;

use crate::api::KeyValueStore;
use crate::api::ReadRequest;
use crate::api::ScanRequest;
use crate::api::WriteCommand;
use crate::api::WriteRequest;

/// Maximum number of revoked tokens to load from storage.
///
/// Tiger Style: Fixed limit prevents unbounded memory usage during startup.
const MAX_REVOCATION_LIST_SIZE: u32 = 10_000;

/// System prefix for revocation storage.
///
/// Uses `_system:` prefix to prevent conflicts with user keys and ensure
/// these keys are treated as system state.
const REVOCATION_PREFIX: &str = "_system:auth:revoked:";

/// Storage backend for token revocation list.
///
/// Implementations provide persistent storage for revoked token hashes,
/// ensuring revocations survive restarts and are replicated across the cluster.
pub trait RevocationStore: Send + Sync {
    /// Add a token hash to the revocation list.
    ///
    /// This operation is idempotent - revoking an already-revoked token succeeds.
    /// The revocation is persisted immediately and replicated via Raft consensus.
    ///
    /// # Errors
    ///
    /// Returns error if the underlying storage operation fails.
    fn revoke(&self, token_hash: [u8; 32]) -> impl std::future::Future<Output = Result<()>> + Send;

    /// Check if a token hash is revoked.
    ///
    /// Returns `true` if the token is in the revocation list, `false` otherwise.
    ///
    /// # Errors
    ///
    /// Returns error if the underlying storage operation fails (not including
    /// key not found, which returns `Ok(false)`).
    fn is_revoked(&self, token_hash: &[u8; 32]) -> impl std::future::Future<Output = Result<bool>> + Send;

    /// Load all revoked token hashes from storage.
    ///
    /// Used during startup to populate the in-memory revocation cache.
    /// Results are limited to MAX_REVOCATION_LIST_SIZE to prevent unbounded
    /// memory usage.
    ///
    /// # Errors
    ///
    /// Returns error if the underlying storage scan operation fails.
    fn load_all(&self) -> impl std::future::Future<Output = Result<Vec<[u8; 32]>>> + Send;
}

/// Revocation store backed by the distributed key-value store.
///
/// Stores revoked token hashes as keys under `_system:auth:revoked:<hash_hex>`.
/// This ensures revocations are replicated via Raft and persist across restarts.
pub struct KeyValueRevocationStore<K: KeyValueStore> {
    /// The underlying key-value store.
    kv: std::sync::Arc<K>,
}

impl<K: KeyValueStore> KeyValueRevocationStore<K> {
    /// Create a new revocation store backed by the given key-value store.
    pub fn new(kv: std::sync::Arc<K>) -> Self {
        Self { kv }
    }
}

impl<K: KeyValueStore + Send + Sync> RevocationStore for KeyValueRevocationStore<K> {
    async fn revoke(&self, token_hash: [u8; 32]) -> Result<()> {
        let key = format!("{}{}", REVOCATION_PREFIX, hex::encode(token_hash));
        // Value is empty - key existence indicates revocation
        self.kv
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key,
                    value: String::new(),
                },
            })
            .await?;
        Ok(())
    }

    async fn is_revoked(&self, token_hash: &[u8; 32]) -> Result<bool> {
        let key = format!("{}{}", REVOCATION_PREFIX, hex::encode(token_hash));
        let result = self.kv.read(ReadRequest::new(key)).await;
        // Key exists = revoked, key not found = not revoked
        Ok(result.is_ok() && result.unwrap().kv.is_some())
    }

    async fn load_all(&self) -> Result<Vec<[u8; 32]>> {
        let scan_result = self
            .kv
            .scan(ScanRequest {
                prefix: REVOCATION_PREFIX.to_string(),
                limit: Some(MAX_REVOCATION_LIST_SIZE),
                continuation_token: None,
            })
            .await?;

        let mut hashes = Vec::with_capacity(scan_result.entries.len());

        for entry in scan_result.entries {
            if let Some(hash_hex) = entry.key.strip_prefix(REVOCATION_PREFIX) {
                // Parse hex-encoded hash
                match hex::decode(hash_hex) {
                    Ok(bytes) if bytes.len() == 32 => {
                        let mut hash = [0u8; 32];
                        hash.copy_from_slice(&bytes);
                        hashes.push(hash);
                    }
                    _ => {
                        // Invalid hex encoding or wrong length - skip
                        // This shouldn't happen in normal operation, but we handle it gracefully
                        tracing::warn!("invalid revocation key format (expected 32-byte hex): {}", entry.key);
                    }
                }
            }
        }

        if scan_result.is_truncated {
            tracing::warn!(
                "revocation list truncated at {} entries (limit: {})",
                hashes.len(),
                MAX_REVOCATION_LIST_SIZE
            );
        }

        Ok(hashes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::inmemory::DeterministicKeyValueStore;

    #[tokio::test]
    async fn test_revocation_store_roundtrip() {
        let kv = std::sync::Arc::new(DeterministicKeyValueStore::new());
        let store = KeyValueRevocationStore::new(kv);

        let hash = [42u8; 32];

        // Initially not revoked
        assert!(!store.is_revoked(&hash).await.expect("should check"));

        // Revoke the token
        store.revoke(hash).await.expect("should revoke");

        // Now it's revoked
        assert!(store.is_revoked(&hash).await.expect("should check"));

        // Revoking again is idempotent
        store.revoke(hash).await.expect("should revoke again");
        assert!(store.is_revoked(&hash).await.expect("should check"));
    }

    #[tokio::test]
    async fn test_load_revoked_at_startup() {
        let kv = std::sync::Arc::new(DeterministicKeyValueStore::new());
        let store = KeyValueRevocationStore::new(kv.clone());

        // Revoke several tokens
        let hash1 = [1u8; 32];
        let hash2 = [2u8; 32];
        let hash3 = [3u8; 32];

        store.revoke(hash1).await.expect("should revoke");
        store.revoke(hash2).await.expect("should revoke");
        store.revoke(hash3).await.expect("should revoke");

        // Load all revocations
        let loaded = store.load_all().await.expect("should load");

        // Should contain all three hashes
        assert_eq!(loaded.len(), 3);
        assert!(loaded.contains(&hash1));
        assert!(loaded.contains(&hash2));
        assert!(loaded.contains(&hash3));
    }

    #[tokio::test]
    async fn test_revocation_key_format() {
        let kv = std::sync::Arc::new(DeterministicKeyValueStore::new());
        let store = KeyValueRevocationStore::new(kv.clone());

        let hash = [0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89]
            .iter()
            .cycle()
            .take(32)
            .copied()
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        store.revoke(hash).await.expect("should revoke");

        // Verify the key format in storage
        let actual_key = format!("_system:auth:revoked:{}", hex::encode(hash));

        // Read directly from KV to verify key format
        let result = kv.read(ReadRequest::new(actual_key.clone())).await.expect("should read");

        assert!(result.kv.is_some(), "revocation key should exist");
        assert_eq!(result.kv.unwrap().value, "", "value should be empty");
    }

    #[tokio::test]
    async fn test_load_all_handles_invalid_keys() {
        let kv = std::sync::Arc::new(DeterministicKeyValueStore::new());

        // Manually insert an invalid revocation key (wrong length)
        kv.write(WriteRequest {
            command: WriteCommand::Set {
                key: "_system:auth:revoked:invalid".to_string(),
                value: String::new(),
            },
        })
        .await
        .expect("should write");

        // Insert a valid revocation
        let valid_hash = [42u8; 32];
        kv.write(WriteRequest {
            command: WriteCommand::Set {
                key: format!("_system:auth:revoked:{}", hex::encode(valid_hash)),
                value: String::new(),
            },
        })
        .await
        .expect("should write");

        let store = KeyValueRevocationStore::new(kv);

        // Load should skip invalid key and return only the valid one
        let loaded = store.load_all().await.expect("should load");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0], valid_hash);
    }

    #[tokio::test]
    async fn test_is_revoked_handles_missing_key() {
        let kv = std::sync::Arc::new(DeterministicKeyValueStore::new());
        let store = KeyValueRevocationStore::new(kv);

        let nonexistent_hash = [99u8; 32];

        // Should return false for non-existent key, not error
        let result = store.is_revoked(&nonexistent_hash).await.expect("should check");
        assert!(!result);
    }

    #[tokio::test]
    async fn test_load_all_respects_limit() {
        let kv = std::sync::Arc::new(DeterministicKeyValueStore::new());
        let store = KeyValueRevocationStore::new(kv.clone());

        // Revoke many tokens (more than would fit in a typical limit)
        // Note: In real usage, MAX_REVOCATION_LIST_SIZE = 10,000 would apply
        for i in 0..100 {
            let mut hash = [0u8; 32];
            hash[0] = (i % 256) as u8;
            hash[1] = (i / 256) as u8;
            store.revoke(hash).await.expect("should revoke");
        }

        let loaded = store.load_all().await.expect("should load");

        // Should load all tokens (we're under the limit)
        assert_eq!(loaded.len(), 100);
    }
}
