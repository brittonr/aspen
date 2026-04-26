//! Store path index for the Nix binary cache.
//!
//! This module provides the `CacheIndex` trait and implementation for
//! storing and retrieving cache entries from the Raft KV store.

#[cfg(feature = "kv-index")]
use std::sync::Arc;
#[cfg(feature = "kv-index")]
use std::time::SystemTime;
#[cfg(feature = "kv-index")]
use std::time::UNIX_EPOCH;

#[cfg(feature = "kv-index")]
use aspen_kv_types::ReadConsistency;
#[cfg(feature = "kv-index")]
use aspen_kv_types::ReadRequest;
#[cfg(feature = "kv-index")]
use aspen_kv_types::WriteCommand;
#[cfg(feature = "kv-index")]
use aspen_kv_types::WriteRequest;
#[cfg(feature = "kv-index")]
use aspen_traits::KeyValueStore;
use async_trait::async_trait;
#[cfg(feature = "kv-index")]
use tracing::debug;
#[cfg(feature = "kv-index")]
use tracing::info;
#[cfg(feature = "kv-index")]
use tracing::warn;

use crate::error::CacheError;
use crate::error::Result;
#[cfg(feature = "kv-index")]
use crate::narinfo::CACHE_KEY_PREFIX;
#[cfg(feature = "kv-index")]
use crate::narinfo::CACHE_STATS_KEY;
use crate::narinfo::CacheEntry;
use crate::narinfo::CacheStats;

// Tiger Style: Explicit bounds
/// Maximum store hash length in characters.
const MAX_STORE_HASH_LENGTH: usize = 64;
/// Minimum store hash length in characters.
const MIN_STORE_HASH_LENGTH: usize = 32;

/// Validate a store hash.
///
/// Uses `nix_compat::nixbase32` validation: the hash must be within the
/// configured length bounds and use the Nix base32 alphabet.
pub fn validate_store_hash(store_hash: &str) -> Result<()> {
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

    if nix_compat::nixbase32::decode(store_hash.as_bytes()).is_err() {
        return Err(CacheError::InvalidStoreHash {
            hash: store_hash.to_string(),
            reason: "must contain only valid nixbase32 characters (0-9, a-z minus e,o,t,u)".to_string(),
        });
    }

    Ok(())
}

/// Look up cache entries by store hash.
#[async_trait]
pub trait CacheLookup: Send + Sync {
    /// Get a cache entry by store hash.
    async fn get(&self, store_hash: &str) -> Result<Option<CacheEntry>>;

    /// Check if a store hash exists in the cache.
    async fn exists(&self, store_hash: &str) -> Result<bool>;
}

/// Publish cache entries to the index.
#[async_trait]
pub trait CachePublish: Send + Sync {
    /// Put a cache entry into the index.
    async fn put(&self, entry: CacheEntry) -> Result<()>;
}

/// Read cache statistics.
#[async_trait]
pub trait CacheStatsProvider: Send + Sync {
    /// Get cache statistics.
    async fn stats(&self) -> Result<CacheStats>;
}

/// Composite trait preserving backward compatibility.
#[async_trait]
pub trait CacheIndex: CacheLookup + CachePublish + CacheStatsProvider + Send + Sync {}

/// Implementation of cache index backed by a KeyValueStore.
#[cfg(feature = "kv-index")]
pub struct KvCacheIndex<KV: KeyValueStore + ?Sized> {
    kv: Arc<KV>,
}

#[cfg(feature = "kv-index")]
impl<KV: KeyValueStore + ?Sized> KvCacheIndex<KV> {
    /// Create a new cache index backed by the given KV store.
    pub fn new(kv: Arc<KV>) -> Self {
        Self { kv }
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

#[cfg(feature = "kv-index")]
#[async_trait]
impl<KV: KeyValueStore + ?Sized> CacheLookup for KvCacheIndex<KV> {
    async fn get(&self, store_hash: &str) -> Result<Option<CacheEntry>> {
        validate_store_hash(store_hash)?;

        let key = format!("{}{}", CACHE_KEY_PREFIX, store_hash);
        debug!(key = %key, "Looking up cache entry");

        let read_request = ReadRequest {
            key: key.clone(),
            consistency: ReadConsistency::Linearizable,
        };

        match self.kv.read(read_request).await {
            Ok(result) => {
                if let Some(kv_entry) = result.kv {
                    let entry = serde_json::from_str::<CacheEntry>(&kv_entry.value)
                        .map_err(|e| CacheError::Deserialization { message: e.to_string() })?;

                    debug!(
                        store_path = %entry.store_path,
                        blob_hash = %entry.blob_hash,
                        nar_size = entry.nar_size,
                        "Cache hit"
                    );

                    let _ = self.update_stats(|s| s.record_hit()).await;
                    Ok(Some(entry))
                } else {
                    debug!(store_hash = %store_hash, "Cache miss");
                    let _ = self.update_stats(|s| s.record_miss()).await;
                    Ok(None)
                }
            }
            Err(aspen_kv_types::KeyValueStoreError::NotFound { .. }) => {
                debug!(store_hash = %store_hash, "Cache miss");
                let _ = self.update_stats(|s| s.record_miss()).await;
                Ok(None)
            }
            Err(e) => Err(CacheError::KvStore { message: e.to_string() }),
        }
    }

    async fn exists(&self, store_hash: &str) -> Result<bool> {
        validate_store_hash(store_hash)?;

        let key = format!("{}{}", CACHE_KEY_PREFIX, store_hash);

        let read_request = ReadRequest {
            key,
            consistency: ReadConsistency::Linearizable,
        };

        match self.kv.read(read_request).await {
            Ok(result) => Ok(result.kv.is_some()),
            Err(aspen_kv_types::KeyValueStoreError::NotFound { .. }) => Ok(false),
            Err(e) => Err(CacheError::KvStore { message: e.to_string() }),
        }
    }
}

#[cfg(feature = "kv-index")]
#[async_trait]
impl<KV: KeyValueStore + ?Sized> CachePublish for KvCacheIndex<KV> {
    async fn put(&self, entry: CacheEntry) -> Result<()> {
        validate_store_hash(&entry.store_hash)?;

        let key = entry.kv_key();
        let nar_size = entry.nar_size;

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

        let _ = self.update_stats(|s| s.record_entry_added(nar_size)).await;
        Ok(())
    }
}

#[cfg(feature = "kv-index")]
#[async_trait]
impl<KV: KeyValueStore + ?Sized> CacheStatsProvider for KvCacheIndex<KV> {
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

#[cfg(feature = "kv-index")]
impl<KV: KeyValueStore + ?Sized> CacheIndex for KvCacheIndex<KV> {}

/// Parse a Nix store path and extract the hash and name components.
///
/// Uses `nix_compat::store_path::StorePath::from_absolute_path()` for validation,
/// which checks hash characters (exactly 32 nixbase32 chars) and name constraints.
///
/// # Examples
///
/// ```
/// use aspen_cache::parse_store_path;
///
/// let (hash, name) = parse_store_path("/nix/store/w1sn8rsa8p38m4i6h0qkdpxalx2hsjdb-hello-2.12.1").unwrap();
/// assert_eq!(hash, "w1sn8rsa8p38m4i6h0qkdpxalx2hsjdb");
/// assert_eq!(name, "hello-2.12.1");
/// ```
pub fn parse_store_path(path: &str) -> Result<(String, String)> {
    use nix_compat::nixbase32;
    use nix_compat::store_path::StorePath;

    let store_path: StorePath<&str> = StorePath::from_absolute_path(path.as_bytes())
        .map_err(|e| CacheError::InvalidStorePath { reason: e.to_string() })?;

    let hash_str = nixbase32::encode(store_path.digest());
    let name = store_path.name().to_string();

    Ok((hash_str, name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_store_path() {
        // Valid nix store path with proper 32-char nixbase32 hash
        let (hash, name) = parse_store_path("/nix/store/w1sn8rsa8p38m4i6h0qkdpxalx2hsjdb-hello-2.12.1").unwrap();
        assert_eq!(hash, "w1sn8rsa8p38m4i6h0qkdpxalx2hsjdb");
        assert_eq!(name, "hello-2.12.1");

        let (hash, name) = parse_store_path("/nix/store/7gx4kiv5m0i7d7qkixq2cwzbr10lvxwc-glibc-2.38-27").unwrap();
        assert_eq!(hash, "7gx4kiv5m0i7d7qkixq2cwzbr10lvxwc");
        assert_eq!(name, "glibc-2.38-27");
    }

    #[test]
    fn test_parse_store_path_invalid() {
        // Not a /nix/store path
        assert!(parse_store_path("/usr/bin/hello").is_err());
        // No dash separator (invalid store path format)
        assert!(parse_store_path("/nix/store/nohash").is_err());
        // Hash too short
        assert!(parse_store_path("/nix/store/abc-hello").is_err());
    }

    #[test]
    fn test_parse_store_path_rejects_invalid_hash_chars() {
        // Uppercase chars are invalid in nix store hashes
        assert!(parse_store_path("/nix/store/ABCDEFGHIJKLMNOPQRSTUVWXYZ012345-hello").is_err());
        // 'e', 'o', 't', 'u' are not in the nixbase32 alphabet
        assert!(parse_store_path("/nix/store/eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee-hello").is_err());
    }

    #[test]
    fn test_validate_store_hash() {
        // Valid nixbase32 hash (32 chars from nix alphabet: 0-9, a-z minus e,o,t,u)
        assert!(validate_store_hash("w1sn8rsa8p38m4i6h0qkdpxalx2hsjdb").is_ok());

        // Too short
        assert!(validate_store_hash("abc").is_err());

        // Contains uppercase
        assert!(validate_store_hash("ABCDEF12345678901234567890123456").is_err());

        // Contains invalid nixbase32 char 'e'
        assert!(validate_store_hash("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee").is_err());
    }
}
