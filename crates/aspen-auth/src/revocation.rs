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
use aspen_core::KeyValueStore;
use aspen_core::ReadRequest;
use aspen_core::ScanRequest;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;

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
        Ok(result.ok().and_then(|r| r.kv).is_some())
    }

    async fn load_all(&self) -> Result<Vec<[u8; 32]>> {
        let scan_result = self
            .kv
            .scan(ScanRequest {
                prefix: REVOCATION_PREFIX.to_string(),
                limit_results: Some(MAX_REVOCATION_LIST_SIZE),
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
