//! Per-shard metrics collection for automatic split/merge decisions.
//!
//! This module provides metrics tracking for each shard to enable data-driven
//! decisions about when to split or merge shards.
//!
//! # Metrics Collected
//!
//! - **size_bytes**: Estimated total size of keys and values
//! - **key_count**: Number of keys in the shard
//! - **read_count**: Read operations in current window
//! - **write_count**: Write operations in current window
//! - **qps**: Computed queries per second
//!
//! # Tiger Style
//!
//! - All counters use saturating arithmetic to prevent overflow
//! - Metrics are collected per-shard for isolation
//! - Fixed measurement windows prevent unbounded accumulation

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;

use parking_lot::RwLock;

use crate::router::ShardId;
use crate::topology::DEFAULT_MERGE_SIZE_BYTES;
use crate::topology::DEFAULT_SPLIT_QPS;
use crate::topology::DEFAULT_SPLIT_SIZE_BYTES;

/// Duration of the metrics measurement window.
pub const METRICS_WINDOW_DURATION: Duration = Duration::from_secs(60);

/// How often to check for split/merge triggers.
pub const METRICS_CHECK_INTERVAL: Duration = Duration::from_secs(10);

/// Atomic counters for a single shard's metrics.
///
/// Uses atomics for lock-free increments on the hot path.
#[derive(Debug, Default)]
pub struct ShardMetricsAtomic {
    /// Estimated size in bytes (updated on writes).
    size_bytes: AtomicU64,
    /// Total key count.
    key_count: AtomicU64,
    /// Read operations in current window.
    read_count: AtomicU64,
    /// Write operations in current window.
    write_count: AtomicU64,
    /// Window start time (Unix millis).
    window_start_ms: AtomicU64,
}

impl ShardMetricsAtomic {
    /// Create new metrics with the given start time.
    pub fn new(start_time_ms: u64) -> Self {
        Self {
            size_bytes: AtomicU64::new(0),
            key_count: AtomicU64::new(0),
            read_count: AtomicU64::new(0),
            write_count: AtomicU64::new(0),
            window_start_ms: AtomicU64::new(start_time_ms),
        }
    }

    /// Record a read operation.
    #[inline]
    pub fn record_read(&self) {
        self.read_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a write operation.
    #[inline]
    pub fn record_write(&self, key_len: usize, value_len: usize, is_new_key: bool) {
        self.write_count.fetch_add(1, Ordering::Relaxed);
        let entry_size = (key_len + value_len) as u64;
        self.size_bytes.fetch_add(entry_size, Ordering::Relaxed);
        if is_new_key {
            self.key_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record a delete operation.
    #[inline]
    pub fn record_delete(&self, key_len: usize, value_len: usize) {
        self.write_count.fetch_add(1, Ordering::Relaxed);
        let entry_size = (key_len + value_len) as u64;
        // Use fetch_sub with saturating behavior
        let old = self.size_bytes.load(Ordering::Relaxed);
        let new = old.saturating_sub(entry_size);
        self.size_bytes.store(new, Ordering::Relaxed);

        let old_count = self.key_count.load(Ordering::Relaxed);
        self.key_count.store(old_count.saturating_sub(1), Ordering::Relaxed);
    }

    /// Get estimated size in bytes.
    pub fn size_bytes(&self) -> u64 {
        self.size_bytes.load(Ordering::Relaxed)
    }

    /// Get key count.
    pub fn key_count(&self) -> u64 {
        self.key_count.load(Ordering::Relaxed)
    }

    /// Get read count for current window.
    pub fn read_count(&self) -> u64 {
        self.read_count.load(Ordering::Relaxed)
    }

    /// Get write count for current window.
    pub fn write_count(&self) -> u64 {
        self.write_count.load(Ordering::Relaxed)
    }

    /// Calculate queries per second based on current window.
    pub fn qps(&self, current_time_ms: u64) -> u32 {
        let window_start = self.window_start_ms.load(Ordering::Relaxed);
        let elapsed_secs = current_time_ms.saturating_sub(window_start) / 1000;
        if elapsed_secs == 0 {
            return 0;
        }
        let total = self.read_count() + self.write_count();
        (total / elapsed_secs) as u32
    }

    /// Reset the measurement window.
    pub fn reset_window(&self, current_time_ms: u64) {
        self.read_count.store(0, Ordering::Relaxed);
        self.write_count.store(0, Ordering::Relaxed);
        self.window_start_ms.store(current_time_ms, Ordering::Relaxed);
    }

    /// Check if shard should be split based on thresholds.
    pub fn should_split(&self, current_time_ms: u64) -> bool {
        self.size_bytes() > DEFAULT_SPLIT_SIZE_BYTES || self.qps(current_time_ms) > DEFAULT_SPLIT_QPS
    }

    /// Check if shard is eligible for merging.
    pub fn can_merge(&self) -> bool {
        self.size_bytes() < DEFAULT_MERGE_SIZE_BYTES
    }

    /// Get a snapshot of current metrics.
    pub fn snapshot(&self, current_time_ms: u64) -> MetricsSnapshot {
        MetricsSnapshot {
            size_bytes: self.size_bytes(),
            key_count: self.key_count(),
            read_count: self.read_count(),
            write_count: self.write_count(),
            qps: self.qps(current_time_ms),
            window_start_ms: self.window_start_ms.load(Ordering::Relaxed),
        }
    }
}

/// A point-in-time snapshot of shard metrics.
#[derive(Debug, Clone, Default)]
pub struct MetricsSnapshot {
    /// Estimated size in bytes.
    pub size_bytes: u64,
    /// Total key count.
    pub key_count: u64,
    /// Read operations in window.
    pub read_count: u64,
    /// Write operations in window.
    pub write_count: u64,
    /// Queries per second.
    pub qps: u32,
    /// When the window started.
    pub window_start_ms: u64,
}

/// Collector for all shard metrics in a node.
///
/// Thread-safe collection of per-shard metrics with efficient lookup.
#[derive(Debug)]
pub struct ShardMetricsCollector {
    /// Metrics for each shard, keyed by shard ID.
    shards: RwLock<HashMap<ShardId, Arc<ShardMetricsAtomic>>>,
    /// When the collector was created.
    created_at: Instant,
}

impl ShardMetricsCollector {
    /// Create a new metrics collector.
    pub fn new() -> Self {
        Self {
            shards: RwLock::new(HashMap::new()),
            created_at: Instant::now(),
        }
    }

    /// Register a shard for metrics collection.
    pub fn register_shard(&self, shard_id: ShardId) -> Arc<ShardMetricsAtomic> {
        let start_ms = self.current_time_ms();
        let metrics = Arc::new(ShardMetricsAtomic::new(start_ms));
        self.shards.write().insert(shard_id, Arc::clone(&metrics));
        metrics
    }

    /// Unregister a shard from metrics collection.
    pub fn unregister_shard(&self, shard_id: ShardId) {
        self.shards.write().remove(&shard_id);
    }

    /// Get metrics for a specific shard.
    pub fn get(&self, shard_id: ShardId) -> Option<Arc<ShardMetricsAtomic>> {
        self.shards.read().get(&shard_id).cloned()
    }

    /// Get or create metrics for a shard.
    pub fn get_or_create(&self, shard_id: ShardId) -> Arc<ShardMetricsAtomic> {
        // Fast path: check read lock first
        if let Some(metrics) = self.shards.read().get(&shard_id) {
            return Arc::clone(metrics);
        }
        // Slow path: create new metrics
        self.register_shard(shard_id)
    }

    /// Get snapshots of all shard metrics.
    pub fn all_snapshots(&self) -> HashMap<ShardId, MetricsSnapshot> {
        let current_ms = self.current_time_ms();
        self.shards.read().iter().map(|(&id, m)| (id, m.snapshot(current_ms))).collect()
    }

    /// Find shards that should be split.
    pub fn shards_to_split(&self) -> Vec<ShardId> {
        let current_ms = self.current_time_ms();
        self.shards.read().iter().filter(|(_, m)| m.should_split(current_ms)).map(|(&id, _)| id).collect()
    }

    /// Find shards that could be merged.
    pub fn shards_to_merge(&self) -> Vec<ShardId> {
        self.shards.read().iter().filter(|(_, m)| m.can_merge()).map(|(&id, _)| id).collect()
    }

    /// Reset all windows (called periodically).
    pub fn reset_all_windows(&self) {
        let current_ms = self.current_time_ms();
        for metrics in self.shards.read().values() {
            metrics.reset_window(current_ms);
        }
    }

    /// Get current time in milliseconds since collector creation.
    fn current_time_ms(&self) -> u64 {
        self.created_at.elapsed().as_millis() as u64
    }
}

impl Default for ShardMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atomic_metrics_read_write() {
        let metrics = ShardMetricsAtomic::new(0);

        metrics.record_read();
        metrics.record_read();
        assert_eq!(metrics.read_count(), 2);

        metrics.record_write(10, 100, true);
        assert_eq!(metrics.write_count(), 1);
        assert_eq!(metrics.size_bytes(), 110);
        assert_eq!(metrics.key_count(), 1);
    }

    #[test]
    fn test_atomic_metrics_delete() {
        let metrics = ShardMetricsAtomic::new(0);

        metrics.record_write(10, 100, true);
        metrics.record_write(10, 100, true);
        assert_eq!(metrics.key_count(), 2);
        assert_eq!(metrics.size_bytes(), 220);

        metrics.record_delete(10, 100);
        assert_eq!(metrics.key_count(), 1);
        assert_eq!(metrics.size_bytes(), 110);
    }

    #[test]
    fn test_atomic_metrics_qps() {
        let metrics = ShardMetricsAtomic::new(0);

        for _ in 0..100 {
            metrics.record_read();
        }
        for _ in 0..50 {
            metrics.record_write(1, 1, false);
        }

        // At 10 seconds, QPS = 150/10 = 15
        assert_eq!(metrics.qps(10_000), 15);
    }

    #[test]
    fn test_atomic_metrics_reset_window() {
        let metrics = ShardMetricsAtomic::new(0);

        metrics.record_read();
        metrics.record_write(10, 10, true);
        metrics.reset_window(60_000);

        assert_eq!(metrics.read_count(), 0);
        assert_eq!(metrics.write_count(), 0);
        // Size and key count should persist across window reset
        assert_eq!(metrics.size_bytes(), 20);
        assert_eq!(metrics.key_count(), 1);
    }

    #[test]
    fn test_collector_register_unregister() {
        let collector = ShardMetricsCollector::new();

        let metrics = collector.register_shard(0);
        metrics.record_read();

        assert!(collector.get(0).is_some());
        assert!(collector.get(1).is_none());

        collector.unregister_shard(0);
        assert!(collector.get(0).is_none());
    }

    #[test]
    fn test_collector_get_or_create() {
        let collector = ShardMetricsCollector::new();

        let m1 = collector.get_or_create(0);
        let m2 = collector.get_or_create(0);

        // Should return same Arc
        m1.record_read();
        assert_eq!(m2.read_count(), 1);
    }

    #[test]
    fn test_collector_all_snapshots() {
        let collector = ShardMetricsCollector::new();

        collector.register_shard(0);
        collector.register_shard(1);
        collector.get(0).unwrap().record_read();
        collector.get(1).unwrap().record_write(10, 10, true);

        let snapshots = collector.all_snapshots();
        assert_eq!(snapshots.len(), 2);
        assert_eq!(snapshots.get(&0).unwrap().read_count, 1);
        assert_eq!(snapshots.get(&1).unwrap().size_bytes, 20);
    }

    #[test]
    fn test_should_split_by_size() {
        let metrics = ShardMetricsAtomic::new(0);

        // Add enough data to trigger split
        for _ in 0..1_000_000 {
            metrics.record_write(32, 32, true); // 64 bytes each
        }

        // 64 million bytes > 64MB threshold
        assert!(metrics.should_split(10_000));
    }

    #[test]
    fn test_should_split_by_qps() {
        let metrics = ShardMetricsAtomic::new(0);

        // Generate high QPS
        for _ in 0..30_000 {
            metrics.record_read();
        }

        // 30000 ops / 10 seconds = 3000 QPS > 2500 threshold
        assert!(metrics.should_split(10_000));
    }

    #[test]
    fn test_can_merge() {
        let metrics = ShardMetricsAtomic::new(0);

        // Small shard
        metrics.record_write(100, 100, true);
        assert!(metrics.can_merge());

        // Large shard: need > 16MB to fail can_merge
        // 16 MB = 16,777,216 bytes. With 64 bytes per entry, need 262,144 entries
        for _ in 0..300_000 {
            metrics.record_write(32, 32, true); // 64 bytes each
        }
        // 300,000 * 64 = 19.2 MB > 16 MB threshold
        assert!(!metrics.can_merge());
    }
}
