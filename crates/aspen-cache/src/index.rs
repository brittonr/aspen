//! Store path index for the Nix binary cache.
//!
//! This module provides the `CacheIndex` trait and implementation for
//! storing and retrieving cache entries from the Raft KV store.

use std::sync::Arc;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use aspen_core::KeyValueStore;
use aspen_core::kv::ReadConsistency;
use aspen_core::kv::ReadRequest;
use aspen_core::kv::WriteCommand;
use aspen_core::kv::WriteRequest;
use async_trait::async_trait;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::error::CacheError;
use crate::error::Result;
use crate::narinfo::CACHE_KEY_PREFIX;
use crate::narinfo::CACHE_STATS_KEY;
use crate::narinfo::CacheEntry;
use crate::narinfo::CacheStats;

// Tiger Style: Explicit bounds
/// Maximum store hash length in characters.
const MAX_STORE_HASH_LENGTH: usize = 64;
/// Minimum store hash length in characters.
const MIN_STORE_HASH_LENGTH: usize = 32;

/// Trait for Nix binary cache index operations.
#[async_trait]
pub trait CacheIndex: Send + Sync {
    /// Get a cache entry by store hash.
    async fn get(&self, store_hash: &str) -> Result<Option<CacheEntry>>;

    /// Put a cache entry into the index.
    async fn put(&self, entry: CacheEntry) -> Result<()>;

    /// Check if a store hash exists in the cache.
    async fn exists(&self, store_hash: &str) -> Result<bool>;

    /// Get cache statistics.
    async fn stats(&self) -> Result<CacheStats>;
}

/// Implementation of cache index backed by a KeyValueStore.
pub struct KvCacheIndex<KV: KeyValueStore + ?Sized> {
    kv: Arc<KV>,
}

impl<KV: KeyValueStore + ?Sized> KvCacheIndex<KV> {
    /// Create a new cache index backed by the given KV store.
    pub fn new(kv: Arc<KV>) -> Self {
        Self { kv }
    }

    /// Validate a store hash.
    fn validate_store_hash(store_hash: &str) -> Result<()> {
        if store_hash.len() < MIN_STORE_HASH_LENGTH {
            return Err(CacheError::InvalidStoreHash {
                hash: store_hash.to_string(),
                reason: format!("too short: {} chars (min: {})", store_hash.len(), MIN_STORE_HASH_LENGTH),
            });
        }

        if store_hash.len() > MAX_STORE_HASH_LENGTH {
            return Err(CacheError::InvalidStoreHash {
                hash: store_hash.to_string(),
                reason: format!("too long: {} chars (max: {})", store_hash.len(), MAX_STORE_HASH_LENGTH),
            });
        }

        // Nix store hashes are base32 (lowercase letters + digits, no 'e', 'o', 't', 'u')
        // or base16 (hex). We accept both.
        if !store_hash.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()) {
            return Err(CacheError::InvalidStoreHash {
                hash: store_hash.to_string(),
                reason: "must contain only lowercase letters and digits".to_string(),
            });
        }

        Ok(())
    }

    /// Update cache statistics.
    async fn update_stats<F>(&self, updater: F) -> Result<()>
    where F: FnOnce(&mut CacheStats) {
        // Read current stats
        let read_request = ReadRequest {
            key: CACHE_STATS_KEY.to_string(),
            consistency: ReadConsistency::Linearizable,
        };

        let mut stats = match self.kv.read(read_request).await {
            Ok(result) => {
                if let Some(kv_entry) = result.kv {
                    serde_json::from_str::<CacheStats>(&kv_entry.value).unwrap_or_default()
                } else {
                    CacheStats::new()
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to read cache stats, using defaults");
                CacheStats::new()
            }
        };

        // Apply update
        updater(&mut stats);
        stats.last_updated = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

        // Serialize to JSON string
        let json_value =
            serde_json::to_string(&stats).map_err(|e| CacheError::Serialization { message: e.to_string() })?;

        // Write back
        let write_request = WriteRequest {
            command: WriteCommand::Set {
                key: CACHE_STATS_KEY.to_string(),
                value: json_value,
            },
        };

        self.kv.write(write_request).await.map_err(|e| CacheError::KvStore { message: e.to_string() })?;

        Ok(())
    }
}

#[async_trait]
impl<KV: KeyValueStore + ?Sized> CacheIndex for KvCacheIndex<KV> {
    async fn get(&self, store_hash: &str) -> Result<Option<CacheEntry>> {
        Self::validate_store_hash(store_hash)?;

        let key = format!("{}{}", CACHE_KEY_PREFIX, store_hash);
        debug!(key = %key, "Looking up cache entry");

        let read_request = ReadRequest {
            key: key.clone(),
            consistency: ReadConsistency::Linearizable,
        };

        match self.kv.read(read_request).await {
            Ok(result) => {
                if let Some(kv_entry) = result.kv {
                    // Parse JSON from the value string
                    let entry = serde_json::from_str::<CacheEntry>(&kv_entry.value)
                        .map_err(|e| CacheError::Deserialization { message: e.to_string() })?;

                    debug!(
                        store_path = %entry.store_path,
                        blob_hash = %entry.blob_hash,
                        nar_size = entry.nar_size,
                        "Cache hit"
                    );

                    // Record hit in stats (best effort)
                    let _ = self.update_stats(|s| s.record_hit()).await;

                    Ok(Some(entry))
                } else {
                    debug!(store_hash = %store_hash, "Cache miss");

                    // Record miss in stats (best effort)
                    let _ = self.update_stats(|s| s.record_miss()).await;

                    Ok(None)
                }
            }
            Err(aspen_core::error::KeyValueStoreError::NotFound { .. }) => {
                // NotFound is a cache miss, not an error
                debug!(store_hash = %store_hash, "Cache miss");
                let _ = self.update_stats(|s| s.record_miss()).await;
                Ok(None)
            }
            Err(e) => Err(CacheError::KvStore { message: e.to_string() }),
        }
    }

    async fn put(&self, entry: CacheEntry) -> Result<()> {
        Self::validate_store_hash(&entry.store_hash)?;

        let key = entry.kv_key();
        let nar_size = entry.nar_size;

        // Serialize to JSON string
        let json_value =
            serde_json::to_string(&entry).map_err(|e| CacheError::Serialization { message: e.to_string() })?;

        info!(
            key = %key,
            store_path = %entry.store_path,
            blob_hash = %entry.blob_hash,
            nar_size = entry.nar_size,
            "Adding cache entry"
        );

        let write_request = WriteRequest {
            command: WriteCommand::Set { key, value: json_value },
        };

        self.kv.write(write_request).await.map_err(|e| CacheError::KvStore { message: e.to_string() })?;

        // Update stats
        let _ = self.update_stats(|s| s.record_entry_added(nar_size)).await;

        Ok(())
    }

    async fn exists(&self, store_hash: &str) -> Result<bool> {
        Self::validate_store_hash(store_hash)?;

        let key = format!("{}{}", CACHE_KEY_PREFIX, store_hash);

        let read_request = ReadRequest {
            key,
            consistency: ReadConsistency::Linearizable,
        };

        match self.kv.read(read_request).await {
            Ok(result) => Ok(result.kv.is_some()),
            Err(aspen_core::error::KeyValueStoreError::NotFound { .. }) => Ok(false),
            Err(e) => Err(CacheError::KvStore { message: e.to_string() }),
        }
    }

    async fn stats(&self) -> Result<CacheStats> {
        let read_request = ReadRequest {
            key: CACHE_STATS_KEY.to_string(),
            consistency: ReadConsistency::Linearizable,
        };

        match self.kv.read(read_request).await {
            Ok(result) => {
                if let Some(kv_entry) = result.kv {
                    serde_json::from_str::<CacheStats>(&kv_entry.value)
                        .map_err(|e| CacheError::Deserialization { message: e.to_string() })
                } else {
                    Ok(CacheStats::new())
                }
            }
            Err(e) => Err(CacheError::KvStore { message: e.to_string() }),
        }
    }
}

/// Parse a Nix store path and extract the hash component.
///
/// # Examples
///
/// ```
/// use aspen_cache::parse_store_path;
///
/// let (hash, name) = parse_store_path("/nix/store/abc123def456-hello-2.10").unwrap();
/// assert_eq!(hash, "abc123def456");
/// assert_eq!(name, "hello-2.10");
/// ```
pub fn parse_store_path(path: &str) -> Result<(String, String)> {
    const NIX_STORE_PREFIX: &str = "/nix/store/";

    if !path.starts_with(NIX_STORE_PREFIX) {
        return Err(CacheError::InvalidStorePath {
            reason: format!("path must start with '{NIX_STORE_PREFIX}'"),
        });
    }

    let remainder = &path[NIX_STORE_PREFIX.len()..];
    let Some(dash_pos) = remainder.find('-') else {
        return Err(CacheError::InvalidStorePath {
            reason: "path must contain hash-name format".to_string(),
        });
    };

    let hash = &remainder[..dash_pos];
    let name = &remainder[dash_pos + 1..];

    if hash.is_empty() {
        return Err(CacheError::InvalidStorePath {
            reason: "hash component is empty".to_string(),
        });
    }

    if name.is_empty() {
        return Err(CacheError::InvalidStorePath {
            reason: "name component is empty".to_string(),
        });
    }

    Ok((hash.to_string(), name.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_store_path() {
        let (hash, name) = parse_store_path("/nix/store/abc123-hello").unwrap();
        assert_eq!(hash, "abc123");
        assert_eq!(name, "hello");

        let (hash, name) = parse_store_path("/nix/store/abc123def456ghij-hello-2.10").unwrap();
        assert_eq!(hash, "abc123def456ghij");
        assert_eq!(name, "hello-2.10");
    }

    #[test]
    fn test_parse_store_path_invalid() {
        assert!(parse_store_path("/usr/bin/hello").is_err());
        assert!(parse_store_path("/nix/store/nohash").is_err());
        assert!(parse_store_path("/nix/store/-noname").is_err());
        assert!(parse_store_path("/nix/store/hash-").is_err());
    }

    #[test]
    fn test_validate_store_hash() {
        // Valid hashes
        assert!(
            KvCacheIndex::<aspen_testing::DeterministicKeyValueStore>::validate_store_hash(
                "abcdefghijklmnopqrstuvwxyz012345"
            )
            .is_ok()
        );

        // Too short
        assert!(KvCacheIndex::<aspen_testing::DeterministicKeyValueStore>::validate_store_hash("abc").is_err());

        // Contains uppercase
        assert!(
            KvCacheIndex::<aspen_testing::DeterministicKeyValueStore>::validate_store_hash(
                "ABCDEF12345678901234567890123456"
            )
            .is_err()
        );
    }
}
