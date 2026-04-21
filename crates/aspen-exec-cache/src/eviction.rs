//! LRU eviction for the execution cache.
//!
//! Background eviction scans `_exec_cache:` entries, removes expired ones,
//! then evicts least-recently-used entries when total size exceeds the bound.

use tracing::debug;
use tracing::info;

use crate::index::CacheKvStore;
use crate::index::ExecCacheIndex;
use crate::types::CacheEntry;
use crate::types::CacheKey;

/// Trait for unreferencing evicted blobs from the blob store GC.
///
/// When a cache entry is evicted, its output blobs should be unreferenced
/// so the blob store's garbage collector can reclaim the storage.
pub trait BlobGc: Send + Sync {
    /// Unreference a blob by hash, allowing GC to reclaim it.
    /// No-op if the blob is referenced elsewhere.
    fn unreference(&self, hash: &[u8; 32]) -> std::result::Result<(), String>;
}

/// No-op GC implementation (blobs are never unreferenced).
pub struct NoOpBlobGc;

impl BlobGc for NoOpBlobGc {
    fn unreference(&self, _hash: &[u8; 32]) -> std::result::Result<(), String> {
        Ok(())
    }
}

/// Parameters for one eviction run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvictionParams {
    /// Current wall-clock time in milliseconds since epoch.
    pub now_ms: u64,
    /// Maximum cache storage budget in bytes.
    pub max_storage_bytes: u64,
}

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
pub fn run_eviction<S: CacheKvStore, G: BlobGc>(
    index: &ExecCacheIndex<S>,
    params: EvictionParams,
    blob_gc: &G,
) -> crate::Result<EvictionResult> {
    // u32::MAX → usize cast: constants are bounded, never overflow
    #[allow(tigerstyle::platform_dependent_cast)]
    let all_entries = index.scan_all(u32::MAX)?;
    let (mut live_entries, expired_removed) = remove_expired_entries(index, &all_entries, params.now_ms, blob_gc)?;
    let lru_evicted = evict_lru_entries(index, &mut live_entries, params.max_storage_bytes, blob_gc)?;
    // live_entries.len() <= scan limit = u32::MAX, so cast is safe
    // live_entries.len() ≤ u32::MAX (bounded by scan limit)
    #[allow(tigerstyle::platform_dependent_cast)]
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

/// Remove entries past their TTL.
fn remove_expired_entries<S: CacheKvStore, G: BlobGc>(
    index: &ExecCacheIndex<S>,
    all_entries: &[(CacheKey, CacheEntry)],
    now_ms: u64,
    blob_gc: &G,
) -> crate::Result<(Vec<(CacheKey, CacheEntry)>, u32)> {
    let mut expired_removed = 0u32;
    let mut live_entries = Vec::with_capacity(all_entries.len());

    for (key, entry) in all_entries {
        if entry.is_expired(now_ms) {
            index.delete(key)?;
            unreference_entry_blobs(entry, blob_gc);
            expired_removed = expired_removed.saturating_add(1);
            debug!(cache_key = %key, "evicted expired entry");
            continue;
        }
        live_entries.push((*key, entry.clone()));
    }

    Ok((live_entries, expired_removed))
}

fn evict_lru_entries<S: CacheKvStore, G: BlobGc>(
    index: &ExecCacheIndex<S>,
    live_entries: &mut Vec<(CacheKey, CacheEntry)>,
    max_storage_bytes: u64,
    blob_gc: &G,
) -> crate::Result<u32> {
    if estimate_total_size_bytes(live_entries) <= max_storage_bytes {
        return Ok(0);
    }

    live_entries.sort_by_key(|(_, entry)| entry.last_accessed_ms);
    let removed_entries_count = count_lru_entries_to_remove(index, live_entries, max_storage_bytes, blob_gc)?;
    live_entries.drain(..removed_entries_count);
    // removed_entries_count <= live_entries.len() <= u32::MAX
    // removed_entries_count ≤ live_entries.len() ≤ u32::MAX
    #[allow(tigerstyle::platform_dependent_cast)]
    Ok(removed_entries_count as u32)
}

fn count_lru_entries_to_remove<S: CacheKvStore, G: BlobGc>(
    index: &ExecCacheIndex<S>,
    live_entries: &[(CacheKey, CacheEntry)],
    max_storage_bytes: u64,
    blob_gc: &G,
) -> crate::Result<usize> {
    let mut total_size_bytes = estimate_total_size_bytes(live_entries);
    let mut removed_entries_count = 0usize;

    for (key, entry) in live_entries {
        if total_size_bytes <= max_storage_bytes {
            break;
        }

        let entry_size_bytes = estimate_entry_size_bytes(entry);
        index.delete(key)?;
        unreference_entry_blobs(entry, blob_gc);
        total_size_bytes = total_size_bytes.saturating_sub(entry_size_bytes);
        removed_entries_count = removed_entries_count.saturating_add(1);
        debug!(cache_key = %key, last_accessed_ms = entry.last_accessed_ms, "evicted LRU entry");
    }

    Ok(removed_entries_count)
}

/// Unreference all blobs (stdout, stderr, outputs) for an evicted entry.
fn unreference_entry_blobs<G: BlobGc>(entry: &CacheEntry, blob_gc: &G) {
    log_unreference_result("stdout", &entry.stdout_hash, blob_gc.unreference(&entry.stdout_hash));
    log_unreference_result("stderr", &entry.stderr_hash, blob_gc.unreference(&entry.stderr_hash));
    for output in &entry.outputs {
        log_unreference_result("output", &output.hash, blob_gc.unreference(&output.hash));
    }
}

fn log_unreference_result(kind: &str, hash: &[u8; 32], result: std::result::Result<(), String>) {
    if let Err(error) = result {
        debug!(blob_kind = kind, blob_hash = ?hash, error, "failed to unreference evicted blob");
    }
}

/// Estimate the total storage size of a set of cache entries.
///
/// Uses output file sizes as the primary metric since the KV index
/// entries are small (~200 bytes each).
fn estimate_total_size_bytes(entries: &[(CacheKey, CacheEntry)]) -> u64 {
    entries.iter().map(|(_, entry)| estimate_entry_size_bytes(entry)).sum()
}

/// Storage parameters for estimating entry size.
#[derive(Debug, Clone, Copy, Default)]
struct EntrySizeParams {
    /// Sum of output file sizes in bytes.
    output_size_bytes: u64,
    /// Index entry overhead in bytes.
    index_overhead_bytes: u64,
}

/// Estimate storage for a single entry: sum of output file sizes + index overhead.
fn estimate_entry_size_bytes(entry: &CacheEntry) -> u64 {
    {
        let params = EntrySizeParams {
            output_size_bytes: entry.outputs.iter().map(|output| output.size_bytes).sum(),
            index_overhead_bytes: 256,
        };
        params.output_size_bytes.saturating_add(params.index_overhead_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::DEFAULT_TTL_MS;
    use crate::constants::MAX_CACHE_STORAGE_BYTES;
    use crate::index::test_store::InMemoryKvStore;
    use crate::types::OutputMapping;

    /// Parameters for constructing a test cache entry.
    #[derive(Debug, Clone, Copy)]
    struct MakeEntryParams {
        created_at_ms: u64,
        last_accessed_ms: u64,
        output_size_bytes: u64,
    }

    fn make_entry(params: MakeEntryParams) -> CacheEntry {
        CacheEntry {
            exit_code: 0,
            stdout_hash: [0; 32],
            stderr_hash: [0; 32],
            outputs: vec![OutputMapping {
                path: "out".into(),
                hash: [0; 32],
                size_bytes: params.output_size_bytes,
            }],
            created_at_ms: params.created_at_ms,
            ttl_ms: DEFAULT_TTL_MS,
            last_accessed_ms: params.last_accessed_ms,
            child_keys: vec![],
        }
    }

    #[test]
    fn evicts_expired_entries() {
        let index = ExecCacheIndex::new(InMemoryKvStore::new());
        let now = 200_000u64;

        // Expired entry (created at 1000, TTL 500)
        let key1 = CacheKey([0x01; 32]);
        let mut entry1 = make_entry(MakeEntryParams {
            created_at_ms: 1000,
            last_accessed_ms: 1000,
            output_size_bytes: 100,
        });
        entry1.ttl_ms = 500;
        index.put(&key1, &entry1).unwrap();

        // Live entry
        let key2 = CacheKey([0x02; 32]);
        index.put(&key2, &make_entry(MakeEntryParams {
            created_at_ms: now,
            last_accessed_ms: now,
            output_size_bytes: 100,
        })).unwrap();

        let result = run_eviction(
            &index,
            EvictionParams {
                now_ms: now,
                max_storage_bytes: MAX_CACHE_STORAGE_BYTES,
            },
            &NoOpBlobGc,
        )
        .unwrap();
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

        index.put(&key_old, &make_entry(MakeEntryParams {
            created_at_ms: now,
            last_accessed_ms: 1000,
            output_size_bytes: 1024,
        })).unwrap(); // oldest access
        index.put(&key_mid, &make_entry(MakeEntryParams {
            created_at_ms: now,
            last_accessed_ms: 5000,
            output_size_bytes: 1024,
        })).unwrap();
        index.put(&key_new, &make_entry(MakeEntryParams {
            created_at_ms: now,
            last_accessed_ms: 9000,
            output_size_bytes: 1024,
        })).unwrap(); // newest access

        // Set max to allow only ~2 entries
        let max = (2u64).saturating_mul(1024u64.saturating_add(256u64)).saturating_add(1);
        let result = run_eviction(
            &index,
            EvictionParams {
                now_ms: now,
                max_storage_bytes: max,
            },
            &NoOpBlobGc,
        )
        .unwrap();

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
        index.put(&key, &make_entry(MakeEntryParams {
            created_at_ms: now,
            last_accessed_ms: now,
            output_size_bytes: 100,
        })).unwrap();

        let result = run_eviction(
            &index,
            EvictionParams {
                now_ms: now,
                max_storage_bytes: MAX_CACHE_STORAGE_BYTES,
            },
            &NoOpBlobGc,
        )
        .unwrap();
        assert_eq!(result.expired_removed, 0);
        assert_eq!(result.lru_evicted, 0);
        assert_eq!(result.remaining, 1);
    }
}
