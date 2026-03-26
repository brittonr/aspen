//! Execution cache index backed by a KV store.
//!
//! Cache entries are stored at `_exec_cache:{blake3_hex}` keys with JSON values.
//! TTL is checked on read — expired entries return `None`.

use tracing::debug;

use crate::constants::EXEC_CACHE_KEY_PREFIX;
use crate::error::ExecCacheError;
use crate::error::Result;
use crate::types::CacheEntry;
use crate::types::CacheKey;

/// KV store abstraction for the execution cache index.
///
/// Kept minimal to avoid pulling in the full `aspen-traits` dependency.
/// Implementors provide read/write/delete/scan over string keys and byte values.
pub trait CacheKvStore: Send + Sync {
    /// Read a value by key. Returns `None` if the key doesn't exist.
    fn read(&self, key: &str) -> std::result::Result<Option<Vec<u8>>, String>;

    /// Write a key-value pair.
    fn write(&self, key: &str, value: &[u8]) -> std::result::Result<(), String>;

    /// Delete a key.
    fn delete(&self, key: &str) -> std::result::Result<(), String>;

    /// Scan keys matching a prefix. Returns (key, value) pairs.
    fn scan(&self, prefix: &str, limit: u32) -> std::result::Result<Vec<(String, Vec<u8>)>, String>;
}

/// Execution cache index.
///
/// Provides get/put/delete over a KV store using `_exec_cache:` key prefix.
/// TTL is enforced on reads — expired entries are treated as cache misses.
pub struct ExecCacheIndex<S> {
    store: S,
}

impl<S: CacheKvStore> ExecCacheIndex<S> {
    /// Create a new cache index backed by the given KV store.
    pub fn new(store: S) -> Self {
        Self { store }
    }

    /// Look up a cache entry by key.
    ///
    /// Returns `None` if the entry doesn't exist or has expired.
    /// Updates `last_accessed_ms` on hit.
    pub fn get(&self, key: &CacheKey, now_ms: u64) -> Result<Option<CacheEntry>> {
        let kv_key = format!("{}{}", EXEC_CACHE_KEY_PREFIX, key.to_hex());

        let bytes = self.store.read(&kv_key).map_err(|message| ExecCacheError::KvStore { message })?;

        let Some(bytes) = bytes else {
            debug!(cache_key = %key, "cache miss: no entry");
            return Ok(None);
        };

        let mut entry = CacheEntry::from_json(&bytes)?;

        // TTL check
        if entry.is_expired(now_ms) {
            debug!(
                cache_key = %key,
                created_at_ms = entry.created_at_ms,
                ttl_ms = entry.ttl_ms,
                now_ms,
                "cache miss: entry expired"
            );
            // Clean up expired entry
            let _ = self.store.delete(&kv_key);
            return Ok(None);
        }

        // Update last_accessed_ms for LRU tracking
        entry.last_accessed_ms = now_ms;
        if let Ok(updated_bytes) = entry.to_json() {
            let _ = self.store.write(&kv_key, &updated_bytes);
        }

        debug!(cache_key = %key, exit_code = entry.exit_code, "cache hit");
        Ok(Some(entry))
    }

    /// Store a cache entry.
    pub fn put(&self, key: &CacheKey, entry: &CacheEntry) -> Result<()> {
        let kv_key = format!("{}{}", EXEC_CACHE_KEY_PREFIX, key.to_hex());
        let bytes = entry.to_json()?;

        self.store.write(&kv_key, &bytes).map_err(|message| ExecCacheError::KvStore { message })?;

        debug!(
            cache_key = %key,
            exit_code = entry.exit_code,
            output_count = entry.outputs.len(),
            "cache entry stored"
        );
        Ok(())
    }

    /// Delete a cache entry.
    pub fn delete(&self, key: &CacheKey) -> Result<()> {
        let kv_key = format!("{}{}", EXEC_CACHE_KEY_PREFIX, key.to_hex());

        self.store.delete(&kv_key).map_err(|message| ExecCacheError::KvStore { message })?;

        debug!(cache_key = %key, "cache entry deleted");
        Ok(())
    }

    /// Scan all cache entries (for eviction).
    ///
    /// Returns (CacheKey, CacheEntry) pairs. Does not filter expired entries —
    /// the caller handles eviction logic.
    pub fn scan_all(&self, limit: u32) -> Result<Vec<(CacheKey, CacheEntry)>> {
        let entries = self
            .store
            .scan(EXEC_CACHE_KEY_PREFIX, limit)
            .map_err(|message| ExecCacheError::KvStore { message })?;

        let mut results = Vec::with_capacity(entries.len());
        for (kv_key, bytes) in &entries {
            let hex = kv_key.strip_prefix(EXEC_CACHE_KEY_PREFIX).unwrap_or(kv_key);
            let Some(cache_key) = CacheKey::from_hex(hex) else {
                continue;
            };
            let Ok(entry) = CacheEntry::from_json(bytes) else {
                continue;
            };
            results.push((cache_key, entry));
        }
        Ok(results)
    }

    /// Get a reference to the underlying store.
    pub fn store(&self) -> &S {
        &self.store
    }
}

/// In-memory KV store for testing.
#[cfg(test)]
pub(crate) mod test_store {
    use std::collections::BTreeMap;
    use std::sync::RwLock;

    use super::CacheKvStore;

    pub struct InMemoryKvStore {
        data: RwLock<BTreeMap<String, Vec<u8>>>,
    }

    impl InMemoryKvStore {
        pub fn new() -> Self {
            Self {
                data: RwLock::new(BTreeMap::new()),
            }
        }
    }

    impl CacheKvStore for InMemoryKvStore {
        fn read(&self, key: &str) -> std::result::Result<Option<Vec<u8>>, String> {
            let data = self.data.read().map_err(|e| e.to_string())?;
            Ok(data.get(key).cloned())
        }

        fn write(&self, key: &str, value: &[u8]) -> std::result::Result<(), String> {
            let mut data = self.data.write().map_err(|e| e.to_string())?;
            data.insert(key.to_string(), value.to_vec());
            Ok(())
        }

        fn delete(&self, key: &str) -> std::result::Result<(), String> {
            let mut data = self.data.write().map_err(|e| e.to_string())?;
            data.remove(key);
            Ok(())
        }

        fn scan(&self, prefix: &str, limit: u32) -> std::result::Result<Vec<(String, Vec<u8>)>, String> {
            let data = self.data.read().map_err(|e| e.to_string())?;
            Ok(data
                .range(prefix.to_string()..)
                .take_while(|(k, _)| k.starts_with(prefix))
                .take(limit as usize)
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use test_store::InMemoryKvStore;

    use super::*;
    use crate::constants::DEFAULT_TTL_MS;

    fn make_entry(created_at_ms: u64, ttl_ms: u64) -> CacheEntry {
        CacheEntry {
            exit_code: 0,
            stdout_hash: [0x11; 32],
            stderr_hash: [0x22; 32],
            outputs: vec![],
            created_at_ms,
            ttl_ms,
            last_accessed_ms: created_at_ms,
            child_keys: vec![],
        }
    }

    #[test]
    fn get_returns_none_for_missing_key() {
        let index = ExecCacheIndex::new(InMemoryKvStore::new());
        let key = CacheKey([0xaa; 32]);
        assert!(index.get(&key, 1000).unwrap().is_none());
    }

    #[test]
    fn put_then_get_returns_entry() {
        let index = ExecCacheIndex::new(InMemoryKvStore::new());
        let key = CacheKey([0xbb; 32]);
        let entry = make_entry(1000, DEFAULT_TTL_MS);

        index.put(&key, &entry).unwrap();
        let result = index.get(&key, 1500).unwrap();
        assert!(result.is_some());
        let got = result.unwrap();
        assert_eq!(got.exit_code, 0);
        assert_eq!(got.stdout_hash, [0x11; 32]);
    }

    #[test]
    fn get_returns_none_for_expired_entry() {
        let index = ExecCacheIndex::new(InMemoryKvStore::new());
        let key = CacheKey([0xcc; 32]);
        let entry = make_entry(1000, 500); // expires at 1500

        index.put(&key, &entry).unwrap();

        // Within TTL
        assert!(index.get(&key, 1200).unwrap().is_some());

        // Past TTL — re-insert since expired entry was deleted
        index.put(&key, &make_entry(1000, 500)).unwrap();
        assert!(index.get(&key, 1501).unwrap().is_none());
    }

    #[test]
    fn get_updates_last_accessed_ms() {
        let index = ExecCacheIndex::new(InMemoryKvStore::new());
        let key = CacheKey([0xdd; 32]);
        let entry = make_entry(1000, DEFAULT_TTL_MS);

        index.put(&key, &entry).unwrap();
        let got = index.get(&key, 5000).unwrap().unwrap();
        assert_eq!(got.last_accessed_ms, 5000);
    }

    #[test]
    fn delete_removes_entry() {
        let index = ExecCacheIndex::new(InMemoryKvStore::new());
        let key = CacheKey([0xee; 32]);
        let entry = make_entry(1000, DEFAULT_TTL_MS);

        index.put(&key, &entry).unwrap();
        assert!(index.get(&key, 1500).unwrap().is_some());

        index.delete(&key).unwrap();
        assert!(index.get(&key, 1500).unwrap().is_none());
    }

    #[test]
    fn scan_all_returns_entries() {
        let index = ExecCacheIndex::new(InMemoryKvStore::new());

        let key1 = CacheKey([0x01; 32]);
        let key2 = CacheKey([0x02; 32]);
        index.put(&key1, &make_entry(1000, DEFAULT_TTL_MS)).unwrap();
        index.put(&key2, &make_entry(2000, DEFAULT_TTL_MS)).unwrap();

        let results = index.scan_all(100).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn stores_non_zero_exit_code() {
        let index = ExecCacheIndex::new(InMemoryKvStore::new());
        let key = CacheKey([0xff; 32]);
        let mut entry = make_entry(1000, DEFAULT_TTL_MS);
        entry.exit_code = 1;

        index.put(&key, &entry).unwrap();
        let got = index.get(&key, 1500).unwrap().unwrap();
        assert_eq!(got.exit_code, 1);
    }
}
