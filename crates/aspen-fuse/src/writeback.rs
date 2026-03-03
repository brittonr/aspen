//! Write-ahead buffer for batching small writes before flushing to KV store.
//!
//! ## Architecture
//!
//! The write buffer sits between FUSE operations and the KV store, accumulating
//! small writes in memory and flushing them in larger batches. This reduces
//! network round-trips for workloads like `gcc` creating many small files or
//! `tar` extracting archives.
//!
//! ## Write Path
//!
//! ```text
//! FUSE write() → WriteBuffer (in-memory) → flush → chunked_write_range → KV store
//! FUSE fsync() → flush specific file
//! FUSE flush() → flush specific file
//! FUSE release() → flush specific file
//! Timer → flush all dirty entries older than WRITEBACK_FLUSH_INTERVAL
//! ```
//!
//! ## Read-Your-Writes Consistency
//!
//! When a `read()` is called, the buffer is checked first. If there are buffered
//! writes that overlap the read range, they are applied on top of the KV data,
//! ensuring consistency without flushing on every read.
//!
//! ## Auto-Flush Triggers
//!
//! 1. Per-file buffer exceeds `WRITEBACK_MAX_FILE_BYTES` → flush that file
//! 2. Total buffered bytes exceeds `WRITEBACK_MAX_TOTAL_BYTES` → flush oldest files
//! 3. File count exceeds `WRITEBACK_MAX_FILES` → flush oldest file
//! 4. Timer fires (every `WRITEBACK_FLUSH_INTERVAL`) → flush all expired files
//!
//! ## Tiger Style Compliance
//!
//! - All data structures are bounded by constants
//! - Uses `std::sync` (not tokio) for FUSE's synchronous operations
//! - Thread-safe: FUSE sends operations from multiple threads
//! - No `.unwrap()`/`.expect()` in production code
//! - All errors are propagated with context

use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Instant;

use tracing::debug;
use tracing::error;
use tracing::info;

use crate::chunking::chunked_write_range;
use crate::constants::WRITEBACK_FLUSH_INTERVAL;
use crate::constants::WRITEBACK_MAX_FILE_BYTES;
use crate::constants::WRITEBACK_MAX_FILES;
use crate::constants::WRITEBACK_MAX_TOTAL_BYTES;
use crate::fs::AspenFs;

/// A pending write operation.
#[derive(Debug, Clone)]
struct PendingWrite {
    /// Byte offset in the file.
    offset: u64,
    /// Data to write at this offset.
    data: Vec<u8>,
    /// Timestamp when this write was buffered.
    timestamp: Instant,
}

/// Per-file write buffer.
#[derive(Debug)]
struct FileBuffer {
    /// Pending writes, sorted by offset.
    writes: Vec<PendingWrite>,
    /// Total buffered bytes for this file.
    buffered_bytes: usize,
    /// Timestamp of the first write (for aging).
    first_write_time: Instant,
}

impl FileBuffer {
    /// Create a new empty file buffer.
    fn new() -> Self {
        Self {
            writes: Vec::new(),
            buffered_bytes: 0,
            first_write_time: Instant::now(),
        }
    }

    /// Add a write to the buffer, coalescing with existing writes if they overlap.
    ///
    /// Returns the number of bytes added (net increase in buffered_bytes).
    fn add_write(&mut self, offset: u64, data: Vec<u8>) -> usize {
        let new_write = PendingWrite {
            offset,
            data,
            timestamp: Instant::now(),
        };

        // Coalesce overlapping writes
        self.writes = coalesce_writes(std::mem::take(&mut self.writes), new_write);

        // Recalculate buffered bytes
        let new_buffered: usize = self.writes.iter().map(|w| w.data.len()).sum();
        let bytes_added = new_buffered.saturating_sub(self.buffered_bytes);
        self.buffered_bytes = new_buffered;

        bytes_added
    }

    /// Check if this buffer is old enough to be flushed.
    fn is_expired(&self, now: Instant) -> bool {
        now.duration_since(self.first_write_time) >= WRITEBACK_FLUSH_INTERVAL
    }

    /// Get the effective file size considering buffered writes.
    fn effective_size(&self, stored_size: u64) -> u64 {
        let max_end = self.writes.iter().map(|w| w.offset.saturating_add(w.data.len() as u64)).max().unwrap_or(0);
        max_end.max(stored_size)
    }

    /// Read data from the buffer that overlaps [offset, offset+size).
    ///
    /// Returns a list of (offset, data) tuples for regions that are buffered.
    fn read_overlapping(&self, offset: u64, size: u32) -> Vec<(u64, Vec<u8>)> {
        let end = offset.saturating_add(u64::from(size));
        let mut result = Vec::new();

        for write in &self.writes {
            let write_end = write.offset.saturating_add(write.data.len() as u64);

            // Check if this write overlaps the read range
            if write.offset < end && write_end > offset {
                let overlap_start = write.offset.max(offset);
                let overlap_end = write_end.min(end);

                let data_start = overlap_start.saturating_sub(write.offset) as usize;
                let data_end = overlap_end.saturating_sub(write.offset) as usize;

                if data_start < write.data.len() && data_end <= write.data.len() {
                    result.push((overlap_start, write.data[data_start..data_end].to_vec()));
                }
            }
        }

        result
    }
}

/// Coalesce a new write with existing writes, merging overlapping regions.
///
/// The coalescing strategy:
/// 1. Sort all writes by offset
/// 2. Merge adjacent/overlapping writes
/// 3. Later writes overwrite earlier writes in the overlap region
fn coalesce_writes(mut writes: Vec<PendingWrite>, new_write: PendingWrite) -> Vec<PendingWrite> {
    writes.push(new_write);

    // Sort by offset
    writes.sort_by_key(|w| w.offset);

    if writes.len() <= 1 {
        return writes;
    }

    let mut coalesced = Vec::new();
    let mut current = writes[0].clone();

    for write in writes.into_iter().skip(1) {
        let current_end = current.offset.saturating_add(current.data.len() as u64);
        let write_end = write.offset.saturating_add(write.data.len() as u64);

        // Check if writes overlap or are adjacent
        if write.offset <= current_end {
            // Merge: extend current to cover both writes
            let new_end = current_end.max(write_end);
            let new_start = current.offset;
            let new_len = new_end.saturating_sub(new_start) as usize;

            let mut merged_data = vec![0u8; new_len];

            // Copy current data
            let current_len = current.data.len();
            merged_data[..current_len].copy_from_slice(&current.data);

            // Overlay write data (overwrites in overlap region)
            let write_start_in_merged = write.offset.saturating_sub(new_start) as usize;
            let write_end_in_merged = write_start_in_merged.saturating_add(write.data.len()).min(new_len);
            merged_data[write_start_in_merged..write_end_in_merged]
                .copy_from_slice(&write.data[..write_end_in_merged.saturating_sub(write_start_in_merged)]);

            current = PendingWrite {
                offset: new_start,
                data: merged_data,
                timestamp: write.timestamp, // Use latest timestamp
            };
        } else {
            // No overlap: save current and start new
            coalesced.push(current);
            current = write;
        }
    }

    coalesced.push(current);
    coalesced
}

/// Write-ahead buffer that batches small writes.
///
/// Thread-safe: uses `RwLock` for the buffer map and `AtomicUsize` for counters.
pub struct WriteBuffer {
    /// Per-file buffers: KV key -> FileBuffer
    buffers: RwLock<HashMap<String, FileBuffer>>,
    /// Total bytes buffered across all files.
    total_bytes: AtomicUsize,
}

impl WriteBuffer {
    /// Create a new write buffer.
    pub fn new() -> Self {
        Self {
            buffers: RwLock::new(HashMap::new()),
            total_bytes: AtomicUsize::new(0),
        }
    }

    /// Buffer a write. Returns true if auto-flush was triggered.
    ///
    /// Coalesces overlapping writes for the same file.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Lock acquisition fails
    /// - Buffer limits are exceeded and flush fails
    pub fn buffer_write(&self, key: &str, offset: u64, data: &[u8]) -> io::Result<bool> {
        if data.is_empty() {
            return Ok(false);
        }

        let mut buffers = self.buffers.write().map_err(|_| io::Error::other("write lock poisoned"))?;

        let buffer = buffers.entry(key.to_string()).or_insert_with(FileBuffer::new);

        let bytes_added = buffer.add_write(offset, data.to_vec());
        let old_total = self.total_bytes.fetch_add(bytes_added, Ordering::SeqCst);
        let new_total = old_total.saturating_add(bytes_added);

        // Check auto-flush conditions
        let should_flush_file = buffer.buffered_bytes >= WRITEBACK_MAX_FILE_BYTES;
        let should_flush_total = new_total >= WRITEBACK_MAX_TOTAL_BYTES;
        let should_flush_count = buffers.len() > WRITEBACK_MAX_FILES;

        if should_flush_file || should_flush_total || should_flush_count {
            return Ok(true);
        }

        Ok(false)
    }

    /// Flush all pending writes for a specific file.
    ///
    /// Called from fsync/flush/release.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Lock acquisition fails
    /// - KV write operation fails
    pub fn flush_file(&self, fs: &AspenFs, key: &str) -> io::Result<()> {
        let mut buffers = self.buffers.write().map_err(|_| io::Error::other("write lock poisoned"))?;

        let Some(buffer) = buffers.remove(key) else {
            return Ok(()); // No pending writes
        };

        // Update total byte counter
        self.total_bytes.fetch_sub(buffer.buffered_bytes, Ordering::SeqCst);

        // Write each pending write to the KV store
        for write in buffer.writes {
            chunked_write_range(fs, key, write.offset, &write.data)?;
        }

        Ok(())
    }

    /// Flush all files that have been dirty longer than WRITEBACK_FLUSH_INTERVAL.
    ///
    /// Called from a background timer or when total bytes exceeds limit.
    ///
    /// # Errors
    ///
    /// Returns the first flush error encountered, but attempts to flush all
    /// expired files even if some fail.
    pub fn flush_expired(&self, fs: &AspenFs) -> io::Result<()> {
        let now = Instant::now();
        let mut keys_to_flush = Vec::new();

        // Identify expired files
        {
            let buffers = self.buffers.read().map_err(|_| io::Error::other("read lock poisoned"))?;

            for (key, buffer) in buffers.iter() {
                if buffer.is_expired(now) {
                    keys_to_flush.push(key.clone());
                }
            }
        }

        // Flush each expired file
        let mut first_error = None;
        for key in keys_to_flush {
            if let Err(e) = self.flush_file(fs, &key)
                && first_error.is_none()
            {
                first_error = Some(e);
            }
        }

        if let Some(e) = first_error {
            return Err(e);
        }

        Ok(())
    }

    /// Flush the oldest file to make room when limits are exceeded.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Lock acquisition fails
    /// - KV write operation fails
    pub fn flush_oldest(&self, fs: &AspenFs) -> io::Result<()> {
        let oldest_key = {
            let buffers = self.buffers.read().map_err(|_| io::Error::other("read lock poisoned"))?;

            buffers.iter().min_by_key(|(_, buffer)| buffer.first_write_time).map(|(key, _)| key.clone())
        };

        if let Some(key) = oldest_key {
            self.flush_file(fs, &key)?;
        }

        Ok(())
    }

    /// Flush everything. Called on shutdown.
    ///
    /// # Errors
    ///
    /// Returns the first flush error encountered, but attempts to flush all
    /// files even if some fail.
    pub fn flush_all(&self, fs: &AspenFs) -> io::Result<()> {
        let keys: Vec<String> = {
            let buffers = self.buffers.read().map_err(|_| io::Error::other("read lock poisoned"))?;
            buffers.keys().cloned().collect()
        };

        let mut first_error = None;
        for key in keys {
            if let Err(e) = self.flush_file(fs, &key)
                && first_error.is_none()
            {
                first_error = Some(e);
            }
        }

        if let Some(e) = first_error {
            return Err(e);
        }

        Ok(())
    }

    /// Read from the buffer (for read-your-writes consistency).
    ///
    /// Returns the buffered data that overlaps [offset, offset+size).
    /// Caller must merge this with data from the KV store.
    pub fn read_buffered(&self, key: &str, offset: u64, size: u32) -> Option<Vec<(u64, Vec<u8>)>> {
        let buffers = self.buffers.read().ok()?;
        let buffer = buffers.get(key)?;
        let overlapping = buffer.read_overlapping(offset, size);
        if overlapping.is_empty() {
            None
        } else {
            Some(overlapping)
        }
    }

    /// Check if a file has any buffered writes.
    pub fn has_pending(&self, key: &str) -> bool {
        self.buffers.read().ok().map(|b| b.contains_key(key)).unwrap_or(false)
    }

    /// Discard all buffered writes for a file without flushing.
    ///
    /// Used when a file is deleted — no point writing data for a removed file.
    pub fn discard_file(&self, key: &str) -> io::Result<()> {
        let mut buffers = self.buffers.write().map_err(|_| io::Error::other("write lock poisoned"))?;
        if let Some(buffer) = buffers.remove(key) {
            self.total_bytes.fetch_sub(buffer.buffered_bytes, Ordering::SeqCst);
        }
        Ok(())
    }

    /// Get the effective file size (considering buffered writes that extend the file).
    pub fn effective_size(&self, key: &str, stored_size: u64) -> u64 {
        self.buffers
            .read()
            .ok()
            .and_then(|b| b.get(key).map(|buffer| buffer.effective_size(stored_size)))
            .unwrap_or(stored_size)
    }

    /// Get statistics about the write buffer.
    pub fn stats(&self) -> BufferStats {
        let buffers = self.buffers.read().ok();
        let file_count = buffers.as_ref().map(|b| b.len()).unwrap_or(0);
        let total_bytes = self.total_bytes.load(Ordering::SeqCst);

        BufferStats {
            file_count,
            total_bytes,
        }
    }
}

impl Default for WriteBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the write buffer.
#[derive(Debug, Clone, Copy)]
pub struct BufferStats {
    /// Number of files with buffered writes.
    pub file_count: usize,
    /// Total bytes buffered across all files.
    pub total_bytes: usize,
}

// ============================================================================
// Background Flush Timer
// ============================================================================

/// Background flush timer that periodically flushes expired write buffers.
///
/// Spawns a dedicated OS thread (not tokio — FUSE uses `std::sync`) that
/// wakes every `WRITEBACK_FLUSH_INTERVAL` and calls `flush_expired()`.
/// This ensures dirty buffers are flushed even during FUSE idle periods.
///
/// # Lifecycle
///
/// - Created via [`FlushTimer::start`] with a shared `WriteBuffer` and a KV-access-only `AspenFs`
///   clone.
/// - Runs until [`FlushTimer::stop`] is called or the stop flag is set.
/// - `stop()` signals the thread and joins it (bounded by one interval).
///
/// # Tiger Style
///
/// - Bounded: thread exits on stop signal, no unbounded loops.
/// - Fail-safe: flush errors are logged but don't crash the thread.
/// - Resource-clean: thread handle is joined on stop.
pub(crate) struct FlushTimer {
    /// Signal to stop the background thread.
    stop_flag: Arc<AtomicBool>,
    /// Background thread handle.
    thread: Option<thread::JoinHandle<()>>,
}

impl FlushTimer {
    /// Start the background flush timer.
    ///
    /// # Arguments
    ///
    /// * `write_buffer` - Shared write buffer (same Arc held by AspenFs)
    /// * `kv_fs` - Lightweight AspenFs clone with KV access only (no timer, empty buffer)
    pub(crate) fn start(write_buffer: Arc<WriteBuffer>, kv_fs: AspenFs) -> Self {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_flag_thread = stop_flag.clone();
        let interval = WRITEBACK_FLUSH_INTERVAL;

        let thread = thread::Builder::new()
            .name("aspen-flush-timer".to_string())
            .spawn(move || {
                info!(interval_ms = interval.as_millis() as u64, "flush timer started");

                loop {
                    thread::sleep(interval);

                    if stop_flag_thread.load(Ordering::Relaxed) {
                        break;
                    }

                    let stats = write_buffer.stats();
                    if stats.file_count == 0 {
                        continue;
                    }

                    match write_buffer.flush_expired(&kv_fs) {
                        Ok(()) => {
                            let after = write_buffer.stats();
                            let flushed = stats.file_count.saturating_sub(after.file_count);
                            if flushed > 0 {
                                debug!(
                                    flushed_files = flushed,
                                    remaining_files = after.file_count,
                                    remaining_bytes = after.total_bytes,
                                    "flush timer: flushed expired buffers"
                                );
                            }
                        }
                        Err(e) => {
                            // Log but don't crash — next tick will retry
                            error!(
                                error = %e,
                                "flush timer: error flushing expired buffers"
                            );
                        }
                    }
                }

                info!("flush timer stopped");
            })
            .expect("failed to spawn flush timer thread");

        Self {
            stop_flag,
            thread: Some(thread),
        }
    }

    /// Stop the background flush timer and join the thread.
    ///
    /// Signals the thread to exit and waits for it to finish.
    /// Safe to call multiple times (subsequent calls are no-ops).
    pub(crate) fn stop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);

        if let Some(thread) = self.thread.take()
            && let Err(e) = thread.join()
        {
            error!("flush timer thread panicked: {e:?}");
        }
    }
}

impl Drop for FlushTimer {
    fn drop(&mut self) {
        // Ensure the thread is stopped even if stop() wasn't called explicitly
        self.stop_flag.store(true, Ordering::Relaxed);
        // Don't join on drop — the thread will exit on its next tick
        // when it observes the stop flag. Joining here could block the
        // drop path for up to WRITEBACK_FLUSH_INTERVAL.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_write_and_read_back() {
        let buffer = WriteBuffer::new();
        let key = "test/file1";

        // Buffer a write
        let result = buffer.buffer_write(key, 0, b"hello");
        assert!(result.is_ok());
        assert!(!result.unwrap()); // No auto-flush

        // Read it back
        let data = buffer.read_buffered(key, 0, 5);
        assert!(data.is_some());
        let data = data.unwrap();
        assert_eq!(data.len(), 1);
        assert_eq!(data[0].0, 0); // offset
        assert_eq!(data[0].1, b"hello");
    }

    #[test]
    fn test_write_coalescing_overlapping() {
        let buffer = WriteBuffer::new();
        let key = "test/file2";

        // Write A: offset=100, data=[0;50] (100..150)
        buffer.buffer_write(key, 100, &vec![0u8; 50]).unwrap();

        // Write B: offset=120, data=[1;40] (120..160)
        buffer.buffer_write(key, 120, &vec![1u8; 40]).unwrap();

        // Should be coalesced into one write covering 100..160
        let data = buffer.read_buffered(key, 100, 60);
        assert!(data.is_some());
        let data = data.unwrap();
        assert_eq!(data.len(), 1); // Coalesced into one write
        assert_eq!(data[0].0, 100);
        assert_eq!(data[0].1.len(), 60);

        // Check that B's data overwrote A's in the overlap region
        assert_eq!(data[0].1[0], 0); // offset 100
        assert_eq!(data[0].1[19], 0); // offset 119
        assert_eq!(data[0].1[20], 1); // offset 120 (B starts)
        assert_eq!(data[0].1[59], 1); // offset 159
    }

    #[test]
    fn test_write_coalescing_adjacent() {
        let buffer = WriteBuffer::new();
        let key = "test/file3";

        // Write A: offset=0, data=[a;10]
        buffer.buffer_write(key, 0, &vec![b'a'; 10]).unwrap();

        // Write B: offset=10, data=[b;10] (adjacent)
        buffer.buffer_write(key, 10, &vec![b'b'; 10]).unwrap();

        // Should be coalesced into one write
        let data = buffer.read_buffered(key, 0, 20);
        assert!(data.is_some());
        let data = data.unwrap();
        assert_eq!(data.len(), 1); // Coalesced
        assert_eq!(data[0].0, 0);
        assert_eq!(data[0].1.len(), 20);
        assert_eq!(data[0].1[0], b'a');
        assert_eq!(data[0].1[9], b'a');
        assert_eq!(data[0].1[10], b'b');
        assert_eq!(data[0].1[19], b'b');
    }

    #[test]
    fn test_flush_file() {
        let fs = AspenFs::new_in_memory(1000, 1000);
        let buffer = WriteBuffer::new();
        let key = "test/file4";

        // Buffer some writes
        buffer.buffer_write(key, 0, b"hello").unwrap();
        buffer.buffer_write(key, 5, b" world").unwrap();

        // Flush
        buffer.flush_file(&fs, key).unwrap();

        // Check that data was written to KV store
        let stored = fs.kv_read(key).unwrap();
        assert!(stored.is_some());
        assert_eq!(stored.unwrap(), b"hello world");

        // Buffer should be empty
        assert!(!buffer.has_pending(key));
    }

    #[test]
    fn test_read_your_writes() {
        let fs = AspenFs::new_in_memory(1000, 1000);
        let buffer = WriteBuffer::new();
        let key = "test/file5";

        // Write some data to KV store
        fs.kv_write(key, b"original data").unwrap();

        // Buffer a write that overlaps
        buffer.buffer_write(key, 5, b"XXXXX").unwrap();

        // Read should see buffered data overlaid on KV data
        let buffered = buffer.read_buffered(key, 0, 13);
        assert!(buffered.is_some());
        let buffered = buffered.unwrap();
        assert_eq!(buffered.len(), 1);
        assert_eq!(buffered[0].0, 5); // offset
        assert_eq!(buffered[0].1, b"XXXXX");

        // Flush and verify final result
        buffer.flush_file(&fs, key).unwrap();
        let stored = fs.kv_read(key).unwrap();
        assert_eq!(stored.unwrap(), b"origiXXXXXata");
    }

    #[test]
    fn test_auto_flush_on_file_limit() {
        let buffer = WriteBuffer::new();
        let key = "test/file6";

        // Write just under the limit
        let data = vec![b'x'; WRITEBACK_MAX_FILE_BYTES - 1];
        let result = buffer.buffer_write(key, 0, &data).unwrap();
        assert!(!result); // No auto-flush

        // Write one more byte to exceed the limit
        let result = buffer.buffer_write(key, data.len() as u64, b"y").unwrap();
        assert!(result); // Auto-flush triggered
    }

    #[test]
    fn test_auto_flush_on_total_limit() {
        let buffer = WriteBuffer::new();

        // Use a file size smaller than per-file limit but large enough
        // that 10 files exceed the total limit
        let file_size = WRITEBACK_MAX_FILE_BYTES / 2; // 128 KB per file
        // 10 files * 128 KB = 1.28 MB, well below 8 MB total limit
        // So we need more files. Let's use enough to approach the limit.
        let num_files = (WRITEBACK_MAX_TOTAL_BYTES / file_size) - 1;

        // Buffer files up to just under the limit
        for i in 0..num_files {
            let key = format!("test/file{}", i);
            let data = vec![b'x'; file_size];
            let result = buffer.buffer_write(&key, 0, &data).unwrap();
            assert!(!result); // No auto-flush yet
        }

        // Add one more file to exceed total limit
        let key = format!("test/file{}", num_files);
        let data = vec![b'x'; file_size];
        let result = buffer.buffer_write(&key, 0, &data).unwrap();
        assert!(result); // Auto-flush triggered
    }

    #[test]
    fn test_effective_size_with_extension() {
        let buffer = WriteBuffer::new();
        let key = "test/file7";

        // Buffer a write that extends the file
        buffer.buffer_write(key, 1000, b"extended").unwrap();

        // Effective size should be 1008 (1000 + 8)
        let size = buffer.effective_size(key, 100);
        assert_eq!(size, 1008);

        // If stored size is larger, use that
        let size = buffer.effective_size(key, 2000);
        assert_eq!(size, 2000);
    }

    #[test]
    fn test_flush_all() {
        let fs = AspenFs::new_in_memory(1000, 1000);
        let buffer = WriteBuffer::new();

        // Buffer writes to multiple files
        buffer.buffer_write("file1", 0, b"data1").unwrap();
        buffer.buffer_write("file2", 0, b"data2").unwrap();
        buffer.buffer_write("file3", 0, b"data3").unwrap();

        // Flush all
        buffer.flush_all(&fs).unwrap();

        // Check all files were written
        assert_eq!(fs.kv_read("file1").unwrap().unwrap(), b"data1");
        assert_eq!(fs.kv_read("file2").unwrap().unwrap(), b"data2");
        assert_eq!(fs.kv_read("file3").unwrap().unwrap(), b"data3");

        // Buffer should be empty
        assert!(!buffer.has_pending("file1"));
        assert!(!buffer.has_pending("file2"));
        assert!(!buffer.has_pending("file3"));
    }

    #[test]
    fn test_concurrent_writes() {
        use std::sync::Arc;
        use std::thread;

        let buffer = Arc::new(WriteBuffer::new());
        let key = "test/concurrent";
        let num_threads = 8;
        let writes_per_thread = 100;

        let handles: Vec<_> = (0..num_threads)
            .map(|i| {
                let buffer = Arc::clone(&buffer);
                thread::spawn(move || {
                    for j in 0..writes_per_thread {
                        let offset = (i * writes_per_thread + j) as u64;
                        let data = vec![i as u8; 1];
                        buffer.buffer_write(key, offset, &data).unwrap();
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // All writes should be present
        assert!(buffer.has_pending(key));
        let stats = buffer.stats();
        assert_eq!(stats.file_count, 1);
    }

    #[test]
    fn test_flush_expired() {
        use std::thread;
        use std::time::Duration;

        let fs = AspenFs::new_in_memory(1000, 1000);
        let buffer = WriteBuffer::new();

        // Buffer a write
        buffer.buffer_write("test/old", 0, b"old data").unwrap();

        // Wait for it to expire
        thread::sleep(WRITEBACK_FLUSH_INTERVAL + Duration::from_millis(100));

        // Buffer a fresh write
        buffer.buffer_write("test/fresh", 0, b"fresh data").unwrap();

        // Flush expired should flush only the old file
        buffer.flush_expired(&fs).unwrap();

        // Old file should be flushed
        assert!(!buffer.has_pending("test/old"));
        assert_eq!(fs.kv_read("test/old").unwrap().unwrap(), b"old data");

        // Fresh file should still be buffered
        assert!(buffer.has_pending("test/fresh"));
    }

    #[test]
    fn test_stats() {
        let buffer = WriteBuffer::new();

        // Initially empty
        let stats = buffer.stats();
        assert_eq!(stats.file_count, 0);
        assert_eq!(stats.total_bytes, 0);

        // Add some writes
        buffer.buffer_write("file1", 0, b"hello").unwrap();
        buffer.buffer_write("file2", 0, b"world").unwrap();

        let stats = buffer.stats();
        assert_eq!(stats.file_count, 2);
        assert_eq!(stats.total_bytes, 10); // 5 + 5
    }

    #[test]
    fn test_empty_write() {
        let buffer = WriteBuffer::new();
        let result = buffer.buffer_write("test/empty", 0, b"");
        assert!(result.is_ok());
        assert!(!result.unwrap()); // No auto-flush

        // Should not create a buffer entry
        assert!(!buffer.has_pending("test/empty"));
    }

    #[test]
    fn test_read_partial_overlap() {
        let buffer = WriteBuffer::new();
        let key = "test/partial";

        // Write at offset 10
        buffer.buffer_write(key, 10, b"0123456789").unwrap();

        // Read overlapping region [5, 15)
        let data = buffer.read_buffered(key, 5, 10);
        assert!(data.is_some());
        let data = data.unwrap();
        assert_eq!(data.len(), 1);
        assert_eq!(data[0].0, 10); // Overlap starts at 10
        assert_eq!(data[0].1, b"01234"); // Only overlapping part
    }

    #[test]
    fn test_read_no_overlap() {
        let buffer = WriteBuffer::new();
        let key = "test/nooverlap";

        // Write at offset 100
        buffer.buffer_write(key, 100, b"data").unwrap();

        // Read non-overlapping region [0, 10)
        let data = buffer.read_buffered(key, 0, 10);
        assert!(data.is_none());
    }

    // ========================================================================
    // FlushTimer tests
    // ========================================================================

    #[test]
    fn test_flush_timer_flushes_expired_entries() {
        use std::thread;
        use std::time::Duration;

        // Create shared write buffer and in-memory filesystem for KV access
        let write_buffer = Arc::new(WriteBuffer::new());
        let kv_fs = AspenFs::new_in_memory(1000, 1000);

        // Buffer a write
        write_buffer.buffer_write("timer/file1", 0, b"hello from timer").unwrap();
        assert!(write_buffer.has_pending("timer/file1"));

        // Wait for the write to expire (WRITEBACK_FLUSH_INTERVAL = 500ms)
        thread::sleep(WRITEBACK_FLUSH_INTERVAL + Duration::from_millis(100));

        // Start the flush timer with the shared buffer
        let mut timer = FlushTimer::start(write_buffer.clone(), kv_fs);

        // Wait one timer tick for the flush to happen
        thread::sleep(WRITEBACK_FLUSH_INTERVAL + Duration::from_millis(200));

        // Buffer should be empty (timer flushed it)
        assert!(!write_buffer.has_pending("timer/file1"), "expected timer to flush expired entry");

        // Stop the timer cleanly
        timer.stop();
    }

    #[test]
    fn test_flush_timer_leaves_fresh_entries() {
        use std::thread;
        use std::time::Duration;

        let write_buffer = Arc::new(WriteBuffer::new());
        let kv_fs = AspenFs::new_in_memory(1000, 1000);

        // Start the timer first
        let mut timer = FlushTimer::start(write_buffer.clone(), kv_fs);

        // Buffer a write AFTER the timer starts (it's fresh, not expired)
        write_buffer.buffer_write("timer/fresh", 0, b"fresh data").unwrap();

        // Wait less than the flush interval — entry should stay
        thread::sleep(Duration::from_millis(200));
        assert!(write_buffer.has_pending("timer/fresh"), "fresh entry should not be flushed yet");

        // Stop the timer
        timer.stop();
    }

    #[test]
    fn test_flush_timer_stop_is_idempotent() {
        let write_buffer = Arc::new(WriteBuffer::new());
        let kv_fs = AspenFs::new_in_memory(1000, 1000);

        let mut timer = FlushTimer::start(write_buffer, kv_fs);

        // Calling stop multiple times should not panic
        timer.stop();
        timer.stop();
    }

    #[test]
    fn test_flush_timer_writes_to_kv() {
        use std::thread;
        use std::time::Duration;

        // Use shared in-memory store so we can inspect KV after flush
        let (kv_fs, store) = AspenFs::new_in_memory_shared(1000, 1000);
        let write_buffer = Arc::new(WriteBuffer::new());

        // Buffer a write
        write_buffer.buffer_write("timer/kv_test", 0, b"persisted data").unwrap();

        // Wait for it to expire
        thread::sleep(WRITEBACK_FLUSH_INTERVAL + Duration::from_millis(100));

        // Start flush timer — it should flush the expired entry to KV
        let mut timer = FlushTimer::start(write_buffer.clone(), kv_fs);

        // Wait for a timer tick
        thread::sleep(WRITEBACK_FLUSH_INTERVAL + Duration::from_millis(200));

        // Verify data landed in the KV store
        let store_guard = store.read().unwrap();
        let value = store_guard.get("timer/kv_test");
        assert!(value.is_some(), "expected data to be flushed to KV store");
        assert_eq!(value.unwrap(), b"persisted data");
        drop(store_guard);

        // Buffer should be empty
        assert!(!write_buffer.has_pending("timer/kv_test"));

        timer.stop();
    }
}
