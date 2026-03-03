//! Speculative prefetch/readahead for AspenFs.
//!
//! Reduces network round-trips by:
//! - Prefetching metadata for all entries after `readdir()`
//! - Prefetching the first 128 KB on file open
//! - Detecting sequential access patterns and prefetching ahead
//!
//! # Design
//!
//! - `AccessTracker`: Per-file state tracking read offsets and sequential patterns
//! - `Prefetcher`: Queue of pending prefetch requests, executes synchronously
//! - `PrefetchRequest`: Metadata or file data to speculatively load
//!
//! # Sequential Detection
//!
//! A read is sequential if:
//! ```text
//! offset == last_offset + last_size  (exactly continuing where we left off)
//! ```
//!
//! After 3 sequential reads, prefetch 512 KB ahead.
//!
//! # Tiger Style
//!
//! - Bounded: MAX_TRACKED_FILES limits memory usage
//! - Explicit TTL: Trackers expire after 30s
//! - Thread-safe: Uses std::sync (FUSE is synchronous)

use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::RwLock;
use std::time::Instant;

use tracing::debug;

use crate::cache::CachedMeta;
use crate::constants::PREFETCH_LOOKAHEAD_BYTES;
use crate::constants::PREFETCH_MAX_PENDING;
use crate::constants::PREFETCH_MAX_TRACKED_FILES;
use crate::constants::PREFETCH_READAHEAD_BYTES;
use crate::constants::PREFETCH_SEQUENTIAL_THRESHOLD;
use crate::constants::PREFETCH_TRACKER_TTL;
use crate::fs::AspenFs;
use crate::inode::EntryType;

/// Access pattern tracker for a single file.
#[derive(Debug, Clone)]
struct AccessTracker {
    /// Last read offset.
    last_offset: u64,
    /// Last read size.
    last_size: u32,
    /// Number of sequential reads detected.
    sequential_count: u32,
    /// Timestamp of last access.
    last_access: Instant,
}

impl AccessTracker {
    /// Create a new tracker for the first read.
    fn new(offset: u64, size: u32) -> Self {
        Self {
            last_offset: offset,
            last_size: size,
            sequential_count: 0,
            last_access: Instant::now(),
        }
    }

    /// Check if a read is sequential.
    fn is_sequential(&self, offset: u64) -> bool {
        offset == self.last_offset.saturating_add(self.last_size as u64)
    }

    /// Update tracker with a new read.
    fn update(&mut self, offset: u64, size: u32, is_sequential: bool) {
        self.last_offset = offset;
        self.last_size = size;
        self.last_access = Instant::now();
        if is_sequential {
            self.sequential_count = self.sequential_count.saturating_add(1);
        } else {
            self.sequential_count = 0;
        }
    }

    /// Check if tracker has expired.
    fn is_expired(&self) -> bool {
        self.last_access.elapsed() >= PREFETCH_TRACKER_TTL
    }
}

/// Prefetch request types.
#[derive(Debug, Clone)]
pub enum PrefetchRequest {
    /// Prefetch metadata for entries in a directory.
    DirMeta {
        /// Directory prefix.
        prefix: String,
        /// Entry names (relative to prefix).
        entries: Vec<String>,
    },
    /// Prefetch file data at a specific range.
    FileData {
        /// File key in KV store.
        key: String,
        /// Starting offset.
        offset: u64,
        /// Number of bytes to prefetch.
        size: u32,
    },
}

/// Prefetch statistics.
#[derive(Debug, Clone, Default)]
pub struct PrefetchStats {
    /// Number of files currently tracked for access patterns.
    pub tracked_files: usize,
    /// Number of pending prefetch requests.
    pub pending_requests: usize,
    /// Total sequential access detections.
    pub sequential_detections: u64,
}

/// Prefetch engine that speculatively loads data.
pub struct Prefetcher {
    /// Per-file access pattern trackers.
    trackers: RwLock<HashMap<String, AccessTracker>>,
    /// Queue of prefetch requests to execute.
    pending: Mutex<VecDeque<PrefetchRequest>>,
    /// Total sequential detections (for stats).
    sequential_detections: Mutex<u64>,
}

impl Default for Prefetcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Prefetcher {
    /// Create a new prefetcher.
    pub fn new() -> Self {
        Self {
            trackers: RwLock::new(HashMap::new()),
            pending: Mutex::new(VecDeque::new()),
            sequential_detections: Mutex::new(0),
        }
    }

    /// Record a read access and detect sequential patterns.
    ///
    /// Returns a prefetch request if sequential access detected.
    pub fn record_read(&self, key: &str, offset: u64, size: u32) -> Option<PrefetchRequest> {
        let Ok(mut trackers) = self.trackers.write() else {
            return None;
        };

        // Enforce tracker limit by evicting expired entries
        if trackers.len() >= PREFETCH_MAX_TRACKED_FILES {
            trackers.retain(|_, tracker| !tracker.is_expired());
        }

        // If still over limit, evict oldest entry
        if trackers.len() >= PREFETCH_MAX_TRACKED_FILES {
            let oldest_key = trackers.iter().min_by_key(|(_, t)| t.last_access).map(|(k, _)| k.clone());
            if let Some(key_to_remove) = oldest_key {
                trackers.remove(&key_to_remove);
            }
        }

        let tracker = trackers.entry(key.to_string()).or_insert_with(|| AccessTracker::new(offset, size));

        // Check if this read is sequential
        let is_sequential = tracker.is_sequential(offset);
        tracker.update(offset, size, is_sequential);

        // If we've detected enough sequential reads, prefetch ahead
        if tracker.sequential_count >= PREFETCH_SEQUENTIAL_THRESHOLD {
            if let Ok(mut detections) = self.sequential_detections.lock() {
                *detections = detections.saturating_add(1);
            }

            let prefetch_offset = offset.saturating_add(size as u64);
            debug!(
                key = %key,
                offset = prefetch_offset,
                size = PREFETCH_LOOKAHEAD_BYTES,
                "sequential access detected, prefetching ahead"
            );

            return Some(PrefetchRequest::FileData {
                key: key.to_string(),
                offset: prefetch_offset,
                size: PREFETCH_LOOKAHEAD_BYTES,
            });
        }

        None
    }

    /// Queue metadata prefetch for directory entries.
    ///
    /// Called after readdir returns entries.
    pub fn queue_dir_prefetch(&self, prefix: &str, entries: &[String]) {
        let Ok(mut pending) = self.pending.lock() else {
            return;
        };

        // Enforce pending limit
        if pending.len() >= PREFETCH_MAX_PENDING {
            debug!("prefetch queue full, dropping old requests");
            // Drop oldest requests
            while pending.len() >= PREFETCH_MAX_PENDING && !pending.is_empty() {
                pending.pop_front();
            }
        }

        debug!(
            prefix = %prefix,
            count = entries.len(),
            "queueing directory metadata prefetch"
        );

        pending.push_back(PrefetchRequest::DirMeta {
            prefix: prefix.to_string(),
            entries: entries.to_vec(),
        });
    }

    /// Execute all pending prefetch requests against the filesystem.
    ///
    /// Populates the ReadCache with results.
    /// This is synchronous — called from FUSE threads.
    pub fn execute_pending(&self, fs: &AspenFs) {
        let mut requests = {
            let Ok(mut pending) = self.pending.lock() else {
                return;
            };
            let drained: Vec<_> = pending.drain(..).collect();
            drained
        };

        // Process up to PREFETCH_MAX_PENDING requests to bound execution time
        requests.truncate(PREFETCH_MAX_PENDING);

        for request in requests {
            match request {
                PrefetchRequest::DirMeta { prefix, entries } => {
                    self.execute_dir_meta_prefetch(fs, &prefix, &entries);
                }
                PrefetchRequest::FileData { key, offset, size } => {
                    self.execute_file_data_prefetch(fs, &key, offset, size);
                }
            }
        }
    }

    /// Execute metadata prefetch for directory entries.
    fn execute_dir_meta_prefetch(&self, fs: &AspenFs, prefix: &str, entries: &[String]) {
        for entry in entries {
            let key = if prefix.is_empty() {
                entry.clone()
            } else {
                format!("{}/{}", prefix.trim_end_matches('/'), entry)
            };

            // Try to read metadata
            match fs.read_metadata(&key) {
                Ok(Some(meta)) => {
                    // Get file size
                    let size = match crate::chunking::chunked_size(fs, &key) {
                        Ok(Some(sz)) => sz,
                        Ok(None) => 0,
                        Err(e) => {
                            debug!(key = %key, error = %e, "failed to get file size during prefetch");
                            continue;
                        }
                    };

                    // Populate meta cache
                    fs.cache.put_meta(key.clone(), CachedMeta {
                        entry_type: EntryType::File,
                        size,
                        mtime_secs: meta.mtime_secs,
                        mtime_nsecs: meta.mtime_nsecs,
                        ctime_secs: meta.ctime_secs,
                        ctime_nsecs: meta.ctime_nsecs,
                    });
                }
                Ok(None) => {
                    // No metadata stored, skip
                }
                Err(e) => {
                    debug!(key = %key, error = %e, "failed to read metadata during prefetch");
                }
            }
        }
    }

    /// Execute file data prefetch.
    fn execute_file_data_prefetch(&self, fs: &AspenFs, key: &str, offset: u64, size: u32) {
        match crate::chunking::chunked_read_range(fs, key, offset, size) {
            Ok(data) => {
                // Create a cache key that includes offset for range-specific caching
                let cache_key = format!("{}@{}", key, offset);
                fs.cache.put_data(cache_key, data);
                debug!(key = %key, offset, size, "prefetch completed");
            }
            Err(e) => {
                debug!(key = %key, offset, size, error = %e, "prefetch failed");
            }
        }
    }

    /// Execute readahead for a specific file.
    ///
    /// Called on open() to prefetch the start of the file.
    pub fn prefetch_file_start(&self, fs: &AspenFs, key: &str) {
        debug!(key = %key, size = PREFETCH_READAHEAD_BYTES, "prefetching file start");

        match crate::chunking::chunked_read_range(fs, key, 0, PREFETCH_READAHEAD_BYTES) {
            Ok(data) => {
                let cache_key = format!("{}@0", key);
                fs.cache.put_data(cache_key, data);
            }
            Err(e) => {
                debug!(key = %key, error = %e, "file start prefetch failed");
            }
        }
    }

    /// Drain expired trackers to prevent unbounded growth.
    pub fn cleanup_expired(&self) {
        let Ok(mut trackers) = self.trackers.write() else {
            return;
        };

        let before = trackers.len();
        trackers.retain(|_, tracker| !tracker.is_expired());
        let after = trackers.len();

        if before > after {
            debug!(removed = before - after, remaining = after, "cleaned up expired access trackers");
        }
    }

    /// Get stats about prefetch activity.
    pub fn stats(&self) -> PrefetchStats {
        let tracked_files = self.trackers.read().map(|t| t.len()).unwrap_or(0);
        let pending_requests = self.pending.lock().map(|p| p.len()).unwrap_or(0);
        let sequential_detections = self.sequential_detections.lock().map(|d| *d).unwrap_or(0);

        PrefetchStats {
            tracked_files,
            pending_requests,
            sequential_detections,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a test filesystem with some data.
    fn setup_fs_with_data() -> AspenFs {
        let fs = AspenFs::new_in_memory(1000, 1000);

        // Write some test files
        fs.kv_write("testfile", b"hello world").expect("write failed");
        fs.kv_write("dir/file1", b"content1").expect("write failed");
        fs.kv_write("dir/file2", b"content2").expect("write failed");

        // Write metadata
        let meta = crate::metadata::FileMetadata {
            mtime_secs: 1000,
            mtime_nsecs: 0,
            ctime_secs: 1000,
            ctime_nsecs: 0,
        };
        fs.kv_write("testfile.meta", &meta.to_bytes()).expect("write failed");
        fs.kv_write("dir/file1.meta", &meta.to_bytes()).expect("write failed");
        fs.kv_write("dir/file2.meta", &meta.to_bytes()).expect("write failed");

        fs
    }

    #[test]
    fn test_sequential_detection() {
        let prefetcher = Prefetcher::new();

        // First read - no prefetch
        let result = prefetcher.record_read("testfile", 0, 100);
        assert!(result.is_none());

        // Second sequential read - no prefetch yet
        let result = prefetcher.record_read("testfile", 100, 100);
        assert!(result.is_none());

        // Third sequential read - no prefetch yet
        let result = prefetcher.record_read("testfile", 200, 100);
        assert!(result.is_none());

        // Fourth sequential read - NOW prefetch triggers
        let result = prefetcher.record_read("testfile", 300, 100);
        assert!(result.is_some());

        if let Some(PrefetchRequest::FileData { key, offset, size }) = result {
            assert_eq!(key, "testfile");
            assert_eq!(offset, 400); // 300 + 100
            assert_eq!(size, PREFETCH_LOOKAHEAD_BYTES);
        } else {
            panic!("Expected FileData prefetch request");
        }

        // Stats should reflect sequential detection
        let stats = prefetcher.stats();
        assert_eq!(stats.sequential_detections, 1);
    }

    #[test]
    fn test_random_access_no_prefetch() {
        let prefetcher = Prefetcher::new();

        // Random access pattern - should not trigger prefetch
        prefetcher.record_read("testfile", 0, 100);
        prefetcher.record_read("testfile", 500, 100); // Non-sequential
        prefetcher.record_read("testfile", 1000, 100); // Non-sequential
        let result = prefetcher.record_read("testfile", 200, 100); // Non-sequential

        assert!(result.is_none());

        let stats = prefetcher.stats();
        assert_eq!(stats.sequential_detections, 0);
    }

    #[test]
    fn test_dir_prefetch_queuing() {
        let prefetcher = Prefetcher::new();

        let entries = vec!["file1".to_string(), "file2".to_string(), "file3".to_string()];
        prefetcher.queue_dir_prefetch("dir", &entries);

        let stats = prefetcher.stats();
        assert_eq!(stats.pending_requests, 1);

        // Verify the request is queued
        let pending = prefetcher.pending.lock().unwrap();
        assert_eq!(pending.len(), 1);
        match &pending[0] {
            PrefetchRequest::DirMeta { prefix, entries: e } => {
                assert_eq!(prefix, "dir");
                assert_eq!(e.len(), 3);
            }
            _ => panic!("Expected DirMeta request"),
        }
    }

    #[test]
    fn test_execute_pending_populates_cache() {
        let fs = setup_fs_with_data();
        let prefetcher = Prefetcher::new();

        // Queue a directory prefetch
        let entries = vec!["file1".to_string(), "file2".to_string()];
        prefetcher.queue_dir_prefetch("dir", &entries);

        // Execute pending requests
        prefetcher.execute_pending(&fs);

        // Check that metadata is now in cache
        let meta1 = fs.cache.get_meta("dir/file1");
        assert!(meta1.is_some());
        let meta1 = meta1.unwrap();
        assert_eq!(meta1.entry_type, EntryType::File);
        assert_eq!(meta1.mtime_secs, 1000);

        let meta2 = fs.cache.get_meta("dir/file2");
        assert!(meta2.is_some());

        // Pending queue should be empty
        let stats = prefetcher.stats();
        assert_eq!(stats.pending_requests, 0);
    }

    #[test]
    fn test_prefetch_file_start() {
        let fs = setup_fs_with_data();
        let prefetcher = Prefetcher::new();

        // Prefetch the start of a file
        prefetcher.prefetch_file_start(&fs, "testfile");

        // Check that data is in cache (with offset 0)
        let cached = fs.cache.get_data("testfile@0");
        assert!(cached.is_some());
        let cached = cached.unwrap();
        assert_eq!(cached, b"hello world");
    }

    #[test]
    fn test_cleanup_expired_trackers() {
        let prefetcher = Prefetcher::new();

        // Create a tracker with a past timestamp
        {
            let mut trackers = prefetcher.trackers.write().unwrap();
            let mut tracker = AccessTracker::new(0, 100);
            // Simulate expiration by backdating
            tracker.last_access = Instant::now() - PREFETCH_TRACKER_TTL - std::time::Duration::from_secs(1);
            trackers.insert("expired_file".to_string(), tracker);

            // Add a fresh tracker
            trackers.insert("fresh_file".to_string(), AccessTracker::new(0, 100));
        }

        // Before cleanup
        let stats = prefetcher.stats();
        assert_eq!(stats.tracked_files, 2);

        // Cleanup
        prefetcher.cleanup_expired();

        // After cleanup - only fresh tracker remains
        let stats = prefetcher.stats();
        assert_eq!(stats.tracked_files, 1);

        let trackers = prefetcher.trackers.read().unwrap();
        assert!(trackers.contains_key("fresh_file"));
        assert!(!trackers.contains_key("expired_file"));
    }

    #[test]
    fn test_max_tracked_files() {
        let prefetcher = Prefetcher::new();

        // Add files up to the limit
        for i in 0..PREFETCH_MAX_TRACKED_FILES {
            prefetcher.record_read(&format!("file{}", i), 0, 100);
        }

        let stats = prefetcher.stats();
        assert_eq!(stats.tracked_files, PREFETCH_MAX_TRACKED_FILES);

        // Add one more - should evict oldest
        prefetcher.record_read("new_file", 0, 100);

        let stats = prefetcher.stats();
        assert!(stats.tracked_files <= PREFETCH_MAX_TRACKED_FILES);
    }

    #[test]
    fn test_stats_reporting() {
        let prefetcher = Prefetcher::new();

        // Initial stats
        let stats = prefetcher.stats();
        assert_eq!(stats.tracked_files, 0);
        assert_eq!(stats.pending_requests, 0);
        assert_eq!(stats.sequential_detections, 0);

        // Add some activity
        prefetcher.record_read("file1", 0, 100);
        prefetcher.record_read("file2", 0, 100);
        prefetcher.queue_dir_prefetch("dir", &vec!["a".to_string()]);

        let stats = prefetcher.stats();
        assert_eq!(stats.tracked_files, 2);
        assert_eq!(stats.pending_requests, 1);
    }

    #[test]
    fn test_pending_queue_limit() {
        let prefetcher = Prefetcher::new();

        // Queue many prefetch requests
        for i in 0..PREFETCH_MAX_PENDING + 10 {
            let entries = vec![format!("file{}", i)];
            prefetcher.queue_dir_prefetch(&format!("dir{}", i), &entries);
        }

        // Should not exceed limit
        let stats = prefetcher.stats();
        assert!(stats.pending_requests <= PREFETCH_MAX_PENDING);
    }

    #[test]
    fn test_sequential_count_resets_on_random_access() {
        let prefetcher = Prefetcher::new();

        // Build up sequential count
        prefetcher.record_read("testfile", 0, 100);
        prefetcher.record_read("testfile", 100, 100);
        prefetcher.record_read("testfile", 200, 100);

        // Random access - should reset count
        prefetcher.record_read("testfile", 1000, 100);

        // After reset, need 3 more sequential reads to trigger prefetch again
        prefetcher.record_read("testfile", 1100, 100); // count = 1
        prefetcher.record_read("testfile", 1200, 100); // count = 2
        let result = prefetcher.record_read("testfile", 1300, 100); // count = 3, triggers!
        assert!(result.is_some()); // Triggers on 3rd sequential read after reset

        let stats = prefetcher.stats();
        assert_eq!(stats.sequential_detections, 1); // Only 1 detection total
    }
}
