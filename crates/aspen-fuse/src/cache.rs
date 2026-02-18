//! TTL-bounded read cache for FUSE filesystem operations.
//!
//! Caches KV reads, metadata lookups, and scan results to avoid redundant
//! network round-trips for repeated FUSE operations (e.g., `stat` after
//! `lookup`, sequential `read` calls on the same file).
//!
//! # Design
//!
//! Three independent caches with different TTLs and eviction policies:
//!
//! - **Data cache**: file contents by KV key, LRU-bounded by entry count and total bytes. TTL = 5s.
//! - **Meta cache**: file existence and type by path. TTL = 2s.
//! - **Scan cache**: readdir results by prefix. TTL = 1s.
//!
//! All caches use `std::sync::RwLock` for thread safety (FUSE operations
//! are synchronous and come from multiple threads).
//!
//! # Tiger Style
//!
//! - Bounded: entry count and byte limits enforced on every insert
//! - Explicit TTLs: stale data expires predictably
//! - Write-through invalidation: writes/deletes/renames purge affected entries

use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Duration;
use std::time::Instant;

use crate::constants::CACHE_DATA_TTL;
use crate::constants::CACHE_MAX_DATA_BYTES;
use crate::constants::CACHE_MAX_DATA_ENTRIES;
use crate::constants::CACHE_MAX_META_ENTRIES;
use crate::constants::CACHE_MAX_SCAN_ENTRIES;
use crate::constants::CACHE_META_TTL;
use crate::constants::CACHE_SCAN_TTL;
use crate::inode::EntryType;

/// A single cached value with an expiration time.
#[derive(Clone)]
struct CacheEntry<T> {
    value: T,
    expires_at: Instant,
}

impl<T> CacheEntry<T> {
    fn new(value: T, ttl: Duration) -> Self {
        Self {
            value,
            expires_at: Instant::now() + ttl,
        }
    }

    fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }
}

/// Cached metadata for a file or directory.
#[derive(Clone, Debug)]
pub struct CachedMeta {
    /// Entry type (file, directory, symlink).
    pub entry_type: EntryType,
    /// File size in bytes (0 for directories).
    pub size: u64,
    /// Stored mtime seconds (0 if no metadata stored).
    pub mtime_secs: i64,
    /// Stored mtime nanoseconds.
    pub mtime_nsecs: i64,
    /// Stored ctime seconds (0 if no metadata stored).
    pub ctime_secs: i64,
    /// Stored ctime nanoseconds.
    pub ctime_nsecs: i64,
}

/// Thread-safe TTL read cache for FUSE operations.
///
/// Wraps three separate caches (data, metadata, scan results) behind
/// `RwLock`s. All operations are non-blocking reads where possible,
/// with writes only on cache miss or invalidation.
pub struct ReadCache {
    /// Cached file data: KV key -> raw bytes.
    data: RwLock<HashMap<String, CacheEntry<Vec<u8>>>>,
    /// Total bytes stored in the data cache (tracked for byte-limit eviction).
    data_bytes: RwLock<usize>,
    /// Cached metadata: path -> (entry_type, size, timestamps).
    meta: RwLock<HashMap<String, CacheEntry<CachedMeta>>>,
    /// Cached scan results: prefix -> list of keys.
    scan: RwLock<HashMap<String, CacheEntry<Vec<String>>>>,
}

impl Default for ReadCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ReadCache {
    /// Create a new empty cache.
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
            data_bytes: RwLock::new(0),
            meta: RwLock::new(HashMap::new()),
            scan: RwLock::new(HashMap::new()),
        }
    }

    // =========================================================================
    // Data cache
    // =========================================================================

    /// Look up cached file data.
    ///
    /// Returns `Some(bytes)` on cache hit, `None` on miss or expiration.
    pub fn get_data(&self, key: &str) -> Option<Vec<u8>> {
        let cache = self.data.read().ok()?;
        let entry = cache.get(key)?;
        if entry.is_expired() {
            return None;
        }
        Some(entry.value.clone())
    }

    /// Insert file data into the cache.
    ///
    /// Evicts expired entries first, then LRU-evicts if over limits.
    pub fn put_data(&self, key: String, value: Vec<u8>) {
        let value_len = value.len();

        let Ok(mut cache) = self.data.write() else {
            return;
        };
        let Ok(mut total_bytes) = self.data_bytes.write() else {
            return;
        };

        // Evict expired entries
        let before = cache.len();
        cache.retain(|_, entry| !entry.is_expired());
        if cache.len() < before {
            // Recalculate total bytes after expiration eviction
            *total_bytes = cache.values().map(|e| e.value.len()).sum();
        }

        // Remove old entry if updating
        if let Some(old) = cache.remove(&key) {
            *total_bytes = total_bytes.saturating_sub(old.value.len());
        }

        // Evict entries if over limits (remove oldest entries)
        while (cache.len() >= CACHE_MAX_DATA_ENTRIES || *total_bytes + value_len > CACHE_MAX_DATA_BYTES)
            && !cache.is_empty()
        {
            // Find the entry closest to expiration (oldest)
            let oldest_key = cache.iter().min_by_key(|(_, e)| e.expires_at).map(|(k, _)| k.clone());
            if let Some(key) = oldest_key {
                if let Some(evicted) = cache.remove(&key) {
                    *total_bytes = total_bytes.saturating_sub(evicted.value.len());
                }
            } else {
                break;
            }
        }

        *total_bytes += value_len;
        cache.insert(key, CacheEntry::new(value, CACHE_DATA_TTL));
    }

    /// Invalidate a specific data cache entry.
    pub fn invalidate_data(&self, key: &str) {
        if let Ok(mut cache) = self.data.write()
            && let Some(removed) = cache.remove(key)
            && let Ok(mut total_bytes) = self.data_bytes.write()
        {
            *total_bytes = total_bytes.saturating_sub(removed.value.len());
        }
    }

    // =========================================================================
    // Metadata cache
    // =========================================================================

    /// Look up cached file metadata.
    pub fn get_meta(&self, path: &str) -> Option<CachedMeta> {
        let cache = self.meta.read().ok()?;
        let entry = cache.get(path)?;
        if entry.is_expired() {
            return None;
        }
        Some(entry.value.clone())
    }

    /// Insert file metadata into the cache.
    pub fn put_meta(&self, path: String, meta: CachedMeta) {
        let Ok(mut cache) = self.meta.write() else {
            return;
        };

        // Evict expired entries if over limit
        if cache.len() >= CACHE_MAX_META_ENTRIES {
            cache.retain(|_, entry| !entry.is_expired());
        }

        // If still over limit, evict oldest
        if cache.len() >= CACHE_MAX_META_ENTRIES {
            let oldest_key = cache.iter().min_by_key(|(_, e)| e.expires_at).map(|(k, _)| k.clone());
            if let Some(key) = oldest_key {
                cache.remove(&key);
            }
        }

        cache.insert(path, CacheEntry::new(meta, CACHE_META_TTL));
    }

    /// Invalidate a specific metadata cache entry.
    pub fn invalidate_meta(&self, path: &str) {
        if let Ok(mut cache) = self.meta.write() {
            cache.remove(path);
        }
    }

    // =========================================================================
    // Scan cache
    // =========================================================================

    /// Look up cached scan results.
    pub fn get_scan(&self, prefix: &str) -> Option<Vec<String>> {
        let cache = self.scan.read().ok()?;
        let entry = cache.get(prefix)?;
        if entry.is_expired() {
            return None;
        }
        Some(entry.value.clone())
    }

    /// Insert scan results into the cache.
    pub fn put_scan(&self, prefix: String, keys: Vec<String>) {
        let Ok(mut cache) = self.scan.write() else {
            return;
        };

        // Evict expired entries if over limit
        if cache.len() >= CACHE_MAX_SCAN_ENTRIES {
            cache.retain(|_, entry| !entry.is_expired());
        }

        // If still over limit, evict oldest
        if cache.len() >= CACHE_MAX_SCAN_ENTRIES {
            let oldest_key = cache.iter().min_by_key(|(_, e)| e.expires_at).map(|(k, _)| k.clone());
            if let Some(key) = oldest_key {
                cache.remove(&key);
            }
        }

        cache.insert(prefix, CacheEntry::new(keys, CACHE_SCAN_TTL));
    }

    // =========================================================================
    // Bulk invalidation
    // =========================================================================

    /// Invalidate all cache entries whose key starts with the given prefix.
    ///
    /// Used on directory rename/delete to purge stale children.
    pub fn invalidate_prefix(&self, prefix: &str) {
        if let Ok(mut cache) = self.data.write() {
            let keys_to_remove: Vec<String> = cache.keys().filter(|k| k.starts_with(prefix)).cloned().collect();
            if let Ok(mut total_bytes) = self.data_bytes.write() {
                for key in &keys_to_remove {
                    if let Some(removed) = cache.remove(key) {
                        *total_bytes = total_bytes.saturating_sub(removed.value.len());
                    }
                }
            }
        }
        if let Ok(mut cache) = self.meta.write() {
            cache.retain(|k, _| !k.starts_with(prefix));
        }
        if let Ok(mut cache) = self.scan.write() {
            cache.retain(|k, _| !k.starts_with(prefix));
        }
    }

    /// Invalidate all scan cache entries (used after any directory mutation).
    pub fn invalidate_all_scans(&self) {
        if let Ok(mut cache) = self.scan.write() {
            cache.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_cache_hit_and_miss() {
        let cache = ReadCache::new();
        assert!(cache.get_data("key1").is_none());

        cache.put_data("key1".into(), b"hello".to_vec());
        assert_eq!(cache.get_data("key1").unwrap(), b"hello");

        assert!(cache.get_data("key2").is_none());
    }

    #[test]
    fn data_cache_invalidation() {
        let cache = ReadCache::new();
        cache.put_data("key1".into(), b"value".to_vec());
        assert!(cache.get_data("key1").is_some());

        cache.invalidate_data("key1");
        assert!(cache.get_data("key1").is_none());
    }

    #[test]
    fn meta_cache_hit_and_miss() {
        let cache = ReadCache::new();
        assert!(cache.get_meta("path/to/file").is_none());

        cache.put_meta("path/to/file".into(), CachedMeta {
            entry_type: EntryType::File,
            size: 42,
            mtime_secs: 1000,
            mtime_nsecs: 0,
            ctime_secs: 1000,
            ctime_nsecs: 0,
        });
        let meta = cache.get_meta("path/to/file").unwrap();
        assert_eq!(meta.size, 42);
        assert_eq!(meta.entry_type, EntryType::File);
    }

    #[test]
    fn scan_cache_hit_and_miss() {
        let cache = ReadCache::new();
        assert!(cache.get_scan("prefix/").is_none());

        cache.put_scan("prefix/".into(), vec!["prefix/a".into(), "prefix/b".into()]);
        let keys = cache.get_scan("prefix/").unwrap();
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn prefix_invalidation_clears_matching_entries() {
        let cache = ReadCache::new();
        cache.put_data("dir/a".into(), b"a".to_vec());
        cache.put_data("dir/b".into(), b"b".to_vec());
        cache.put_data("other/c".into(), b"c".to_vec());
        cache.put_meta("dir/a".into(), CachedMeta {
            entry_type: EntryType::File,
            size: 1,
            mtime_secs: 0,
            mtime_nsecs: 0,
            ctime_secs: 0,
            ctime_nsecs: 0,
        });

        cache.invalidate_prefix("dir/");

        assert!(cache.get_data("dir/a").is_none());
        assert!(cache.get_data("dir/b").is_none());
        assert!(cache.get_data("other/c").is_some());
        assert!(cache.get_meta("dir/a").is_none());
    }

    #[test]
    fn data_cache_respects_entry_limit() {
        let cache = ReadCache::new();
        // Fill cache beyond limit
        for i in 0..CACHE_MAX_DATA_ENTRIES + 10 {
            cache.put_data(format!("key{i}"), vec![0u8; 16]);
        }
        // Cache should not exceed the limit
        let count = cache.data.read().unwrap().len();
        assert!(count <= CACHE_MAX_DATA_ENTRIES);
    }
}
