//! LRU eviction for the execution cache.
//!
//! Background eviction scans `_exec_cache:` entries, removes expired ones,
//! then evicts least-recently-used entries when total size exceeds the bound.

use tracing::debug;
use tracing::info;

use crate::constants::MAX_CACHE_STORAGE_BYTES;
use crate::index::CacheKvStore;
use crate::index::ExecCacheIndex;
use crate::types::CacheEntry;
use crate::types::CacheKey;

/// Result of an eviction run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvictionResult {
    /// Number of expired entries removed.
    pub expired_removed: u32,
    /// Number of LRU entries evicted.
    pub lru_evicted: u32,
    /// Total entries remaining after eviction.
    pub remaining: u32,
}

/// Run one eviction pass on the cache index.
///
/// 1. Scans all entries.
/// 2. Removes entries past their TTL.
/// 3. If total estimated size exceeds `max_storage_bytes`, evicts the least-recently-used entries
///    until below the limit.
///
/// Returns the eviction result.
pub fn run_eviction<S: CacheKvStore>(
    index: &ExecCacheIndex<S>,
    now_ms: u64,
    max_storage_bytes: u64,
) -> crate::Result<EvictionResult> {
    let all_entries = index.scan_all(u32::MAX)?;

    let mut expired_removed = 0u32;
    let mut live_entries: Vec<(CacheKey, CacheEntry)> = Vec::new();

    // Phase 1: remove expired entries
    for (key, entry) in &all_entries {
        if entry.is_expired(now_ms) {
            let _ = index.delete(key);
            expired_removed = expired_removed.saturating_add(1);
            debug!(cache_key = %key, "evicted expired entry");
        } else {
            live_entries.push((*key, entry.clone()));
        }
    }

    // Phase 2: LRU eviction if over storage limit
    let mut lru_evicted = 0u32;
    let mut total_size = estimate_total_size(&live_entries);

    if total_size > max_storage_bytes {
        // Sort by last_accessed_ms ascending (oldest first)
        live_entries.sort_by_key(|(_, entry)| entry.last_accessed_ms);

        while total_size > max_storage_bytes {
            let Some((key, entry)) = live_entries.first() else {
                break;
            };
            let entry_size = estimate_entry_size(entry);
            let _ = index.delete(key);
            total_size = total_size.saturating_sub(entry_size);
            lru_evicted = lru_evicted.saturating_add(1);
            debug!(cache_key = %key, last_accessed_ms = entry.last_accessed_ms, "evicted LRU entry");
            live_entries.remove(0);
        }
    }

    let remaining = live_entries.len() as u32;

    if expired_removed > 0 || lru_evicted > 0 {
        info!(expired_removed, lru_evicted, remaining, "eviction pass complete");
    }

    Ok(EvictionResult {
        expired_removed,
        lru_evicted,
        remaining,
    })
}

/// Estimate the total storage size of a set of cache entries.
///
/// Uses output file sizes as the primary metric since the KV index
/// entries are small (~200 bytes each).
fn estimate_total_size(entries: &[(CacheKey, CacheEntry)]) -> u64 {
    entries.iter().map(|(_, entry)| estimate_entry_size(entry)).sum()
}

/// Estimate storage for a single entry: sum of output file sizes + index overhead.
fn estimate_entry_size(entry: &CacheEntry) -> u64 {
    let output_size: u64 = entry.outputs.iter().map(|o| o.size_bytes).sum();
    // ~256 bytes for the index entry itself
    output_size.saturating_add(256)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::DEFAULT_TTL_MS;
    use crate::index::test_store::InMemoryKvStore;
    use crate::types::OutputMapping;

    fn make_entry(created_at_ms: u64, last_accessed_ms: u64, output_size: u64) -> CacheEntry {
        CacheEntry {
            exit_code: 0,
            stdout_hash: [0; 32],
            stderr_hash: [0; 32],
            outputs: vec![OutputMapping {
                path: "out".into(),
                hash: [0; 32],
                size_bytes: output_size,
            }],
            created_at_ms,
            ttl_ms: DEFAULT_TTL_MS,
            last_accessed_ms,
            child_keys: vec![],
        }
    }

    #[test]
    fn evicts_expired_entries() {
        let index = ExecCacheIndex::new(InMemoryKvStore::new());
        let now = 200_000u64;

        // Expired entry (created at 1000, TTL 500)
        let key1 = CacheKey([0x01; 32]);
        let mut entry1 = make_entry(1000, 1000, 100);
        entry1.ttl_ms = 500;
        index.put(&key1, &entry1).unwrap();

        // Live entry
        let key2 = CacheKey([0x02; 32]);
        index.put(&key2, &make_entry(now, now, 100)).unwrap();

        let result = run_eviction(&index, now, MAX_CACHE_STORAGE_BYTES).unwrap();
        assert_eq!(result.expired_removed, 1);
        assert_eq!(result.lru_evicted, 0);
        assert_eq!(result.remaining, 1);

        // key1 gone, key2 still there
        assert!(index.get(&key1, now).unwrap().is_none());
        assert!(index.get(&key2, now).unwrap().is_some());
    }

    #[test]
    fn lru_eviction_removes_oldest_first() {
        let index = ExecCacheIndex::new(InMemoryKvStore::new());
        let now = 100_000u64;

        // 3 entries, each ~1KB output
        let key_old = CacheKey([0x01; 32]);
        let key_mid = CacheKey([0x02; 32]);
        let key_new = CacheKey([0x03; 32]);

        index.put(&key_old, &make_entry(now, 1000, 1024)).unwrap(); // oldest access
        index.put(&key_mid, &make_entry(now, 5000, 1024)).unwrap();
        index.put(&key_new, &make_entry(now, 9000, 1024)).unwrap(); // newest access

        // Set max to allow only ~2 entries
        let max = 2 * (1024 + 256) + 1;
        let result = run_eviction(&index, now, max).unwrap();

        assert_eq!(result.lru_evicted, 1);
        assert_eq!(result.remaining, 2);
        // Oldest should be evicted
        assert!(index.get(&key_old, now).unwrap().is_none());
        assert!(index.get(&key_mid, now).unwrap().is_some());
        assert!(index.get(&key_new, now).unwrap().is_some());
    }

    #[test]
    fn no_eviction_when_under_limit() {
        let index = ExecCacheIndex::new(InMemoryKvStore::new());
        let now = 100_000u64;

        let key = CacheKey([0x01; 32]);
        index.put(&key, &make_entry(now, now, 100)).unwrap();

        let result = run_eviction(&index, now, MAX_CACHE_STORAGE_BYTES).unwrap();
        assert_eq!(result.expired_removed, 0);
        assert_eq!(result.lru_evicted, 0);
        assert_eq!(result.remaining, 1);
    }
}
