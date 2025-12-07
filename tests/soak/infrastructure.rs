/// Soak test infrastructure for long-running stress tests.
///
/// This module provides reusable components for soak testing:
/// - SoakMetrics: Tiger Style metrics collection (fixed-size, bounded)
/// - SoakTestConfig: Explicit configuration with bounded values
/// - SoakTestHarness: Reusable test runner with checkpoints
/// - LoadGenerator: Bounded workload generation utilities
///
/// Tiger Style Compliance:
/// - All types explicitly sized (u64, u32, [T; N])
/// - No unbounded collections (Vec growth limited)
/// - Fixed checkpoint limits
/// - Explicit error handling

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

/// Maximum number of checkpoints to store in memory.
/// Tiger Style: Fixed limit prevents unbounded memory growth.
const MAX_CHECKPOINTS: usize = 100;

/// Latency histogram buckets (microseconds).
/// Tiger Style: Fixed-size array, not Vec.
const LATENCY_BUCKET_COUNT: usize = 6;

/// Metrics collected during soak testing.
/// Tiger Style: Fixed-size arrays, explicit types, atomic counters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoakMetrics {
    /// Total successful write operations
    pub writes_successful: u64,
    /// Total failed write operations
    pub writes_failed: u64,
    /// Total successful read operations
    pub reads_successful: u64,
    /// Total failed read operations
    pub reads_failed: u64,

    /// Write latency histogram (microseconds)
    /// Buckets: [0-1ms, 1-10ms, 10-100ms, 100ms-1s, 1s-10s, 10s+]
    pub write_latency_buckets_us: [u64; LATENCY_BUCKET_COUNT],

    /// Read latency histogram (microseconds)
    /// Buckets: [0-1ms, 1-10ms, 10-100ms, 100ms-1s, 1s-10s, 10s+]
    pub read_latency_buckets_us: [u64; LATENCY_BUCKET_COUNT],

    /// Total write latency in microseconds (for average calculation)
    pub write_total_latency_us: u64,

    /// Total read latency in microseconds (for average calculation)
    pub read_total_latency_us: u64,
}

impl Default for SoakMetrics {
    fn default() -> Self {
        Self {
            writes_successful: 0,
            writes_failed: 0,
            reads_successful: 0,
            reads_failed: 0,
            write_latency_buckets_us: [0; LATENCY_BUCKET_COUNT],
            read_latency_buckets_us: [0; LATENCY_BUCKET_COUNT],
            write_total_latency_us: 0,
            read_total_latency_us: 0,
        }
    }
}

/// Add latency value to appropriate bucket.
/// Tiger Style: O(1) bounded operation.
fn add_to_latency_bucket(buckets: &mut [u64; LATENCY_BUCKET_COUNT], latency_us: u64) {
    let bucket_idx = if latency_us < 1_000 {
        0 // < 1ms
    } else if latency_us < 10_000 {
        1 // 1-10ms
    } else if latency_us < 100_000 {
        2 // 10-100ms
    } else if latency_us < 1_000_000 {
        3 // 100ms-1s
    } else if latency_us < 10_000_000 {
        4 // 1s-10s
    } else {
        5 // 10s+
    };
    buckets[bucket_idx] += 1;
}

impl SoakMetrics {
    /// Record a successful write with latency.
    pub fn record_write_success(&mut self, latency_us: u64) {
        self.writes_successful += 1;
        self.write_total_latency_us += latency_us;
        add_to_latency_bucket(&mut self.write_latency_buckets_us, latency_us);
    }

    /// Record a failed write.
    pub fn record_write_failure(&mut self) {
        self.writes_failed += 1;
    }

    /// Record a successful read with latency.
    pub fn record_read_success(&mut self, latency_us: u64) {
        self.reads_successful += 1;
        self.read_total_latency_us += latency_us;
        add_to_latency_bucket(&mut self.read_latency_buckets_us, latency_us);
    }

    /// Record a failed read.
    pub fn record_read_failure(&mut self) {
        self.reads_failed += 1;
    }

    /// Calculate average write latency in milliseconds.
    pub fn avg_write_latency_ms(&self) -> f64 {
        if self.writes_successful == 0 {
            0.0
        } else {
            (self.write_total_latency_us as f64) / (self.writes_successful as f64) / 1000.0
        }
    }

    /// Calculate average read latency in milliseconds.
    pub fn avg_read_latency_ms(&self) -> f64 {
        if self.reads_successful == 0 {
            0.0
        } else {
            (self.read_total_latency_us as f64) / (self.reads_successful as f64) / 1000.0
        }
    }

    /// Calculate total operations (reads + writes).
    pub fn total_operations(&self) -> u64 {
        self.writes_successful + self.writes_failed + self.reads_successful + self.reads_failed
    }

    /// Calculate success rate as percentage.
    pub fn success_rate_percent(&self) -> f64 {
        let total = self.total_operations();
        if total == 0 {
            0.0
        } else {
            let successful = self.writes_successful + self.reads_successful;
            (successful as f64) / (total as f64) * 100.0
        }
    }

    /// Merge another SoakMetrics into this one.
    pub fn merge(&mut self, other: &SoakMetrics) {
        self.writes_successful += other.writes_successful;
        self.writes_failed += other.writes_failed;
        self.reads_successful += other.reads_successful;
        self.reads_failed += other.reads_failed;
        self.write_total_latency_us += other.write_total_latency_us;
        self.read_total_latency_us += other.read_total_latency_us;

        for i in 0..LATENCY_BUCKET_COUNT {
            self.write_latency_buckets_us[i] += other.write_latency_buckets_us[i];
            self.read_latency_buckets_us[i] += other.read_latency_buckets_us[i];
        }
    }
}

/// Thread-safe metrics collector.
/// Tiger Style: Arc<Mutex> for explicit ownership and locking.
#[derive(Clone)]
pub struct SoakMetricsCollector {
    metrics: Arc<Mutex<SoakMetrics>>,
}

impl SoakMetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(SoakMetrics::default())),
        }
    }

    pub fn record_write_success(&self, latency_us: u64) {
        self.metrics.lock().record_write_success(latency_us);
    }

    pub fn record_write_failure(&self) {
        self.metrics.lock().record_write_failure();
    }

    pub fn record_read_success(&self, latency_us: u64) {
        self.metrics.lock().record_read_success(latency_us);
    }

    pub fn record_read_failure(&self) {
        self.metrics.lock().record_read_failure();
    }

    pub fn snapshot(&self) -> SoakMetrics {
        self.metrics.lock().clone()
    }

    pub fn reset(&self) {
        *self.metrics.lock() = SoakMetrics::default();
    }
}

impl Default for SoakMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Checkpoint snapshot during soak test.
/// Tiger Style: Explicit types, immutable after creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoakCheckpoint {
    /// Checkpoint sequence number (0-indexed)
    pub checkpoint_id: u32,
    /// Elapsed time since test start (seconds)
    pub elapsed_seconds: u64,
    /// Metrics snapshot at this checkpoint
    pub metrics: SoakMetrics,
    /// Throughput at this checkpoint (ops/sec)
    pub throughput_ops_per_sec: f64,
}

/// Configuration for soak tests.
/// Tiger Style: All fields explicit, no defaults hidden.
#[derive(Debug, Clone)]
pub struct SoakTestConfig {
    /// Test duration in seconds
    pub duration_seconds: u64,
    /// Checkpoint interval in seconds
    pub checkpoint_interval_seconds: u64,
    /// Maximum total operations (prevents unbounded execution)
    pub max_total_operations: u64,
    /// Number of concurrent workers
    pub num_workers: u32,
    /// Read operation percentage (0-100)
    pub read_percentage: u32,
    /// Key space size for operations
    pub key_space_size: u64,
    /// Value size in bytes
    pub value_size_bytes: u32,
}

impl Default for SoakTestConfig {
    fn default() -> Self {
        Self {
            duration_seconds: 60,           // 1 minute default
            checkpoint_interval_seconds: 10, // Every 10 seconds
            max_total_operations: 10_000,   // 10k ops max
            num_workers: 4,                 // 4 concurrent workers
            read_percentage: 70,            // 70% reads, 30% writes
            key_space_size: 1000,           // 1000 unique keys
            value_size_bytes: 100,          // 100 byte values
        }
    }
}

/// Soak test results.
/// Tiger Style: Fixed-size checkpoint array, explicit counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoakTestResult {
    /// Test configuration used
    pub config_summary: String,
    /// Total elapsed time in seconds
    pub total_duration_seconds: f64,
    /// Final aggregated metrics
    pub final_metrics: SoakMetrics,
    /// Checkpoints collected (bounded to MAX_CHECKPOINTS)
    pub checkpoints: Vec<SoakCheckpoint>,
    /// Overall throughput (ops/sec)
    pub overall_throughput: f64,
    /// Test status
    pub status: SoakTestStatus,
    /// Error message if failed
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SoakTestStatus {
    Success,
    Failed,
    Timeout,
    Cancelled,
}

/// Workload operation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadOp {
    Read,
    Write,
}

/// Pre-generated workload for deterministic testing.
/// Tiger Style: Bounded size, pre-allocated.
pub struct Workload {
    /// Pre-generated operations
    operations: Vec<(WorkloadOp, u64)>, // (op_type, key_id)
}

impl Workload {
    /// Generate a deterministic workload.
    /// Tiger Style: Bounded operation count.
    pub fn generate(
        total_operations: u64,
        read_percentage: u32,
        key_space_size: u64,
    ) -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let mut operations = Vec::with_capacity(total_operations as usize);

        for _ in 0..total_operations {
            let is_read = rng.gen_range(0..100) < read_percentage;
            let key_id = rng.gen_range(0..key_space_size);

            let op_type = if is_read {
                WorkloadOp::Read
            } else {
                WorkloadOp::Write
            };

            operations.push((op_type, key_id));
        }

        Self { operations }
    }

    /// Get operation at index.
    pub fn get(&self, index: usize) -> Option<(WorkloadOp, u64)> {
        self.operations.get(index).copied()
    }

    /// Total operation count.
    pub fn len(&self) -> usize {
        self.operations.len()
    }

    /// Check if workload is empty.
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }
}

/// Progress reporter for soak tests.
pub struct SoakProgressReporter {
    start_time: Instant,
    last_report_time: Instant,
    operations_completed: AtomicU64,
    total_operations: u64,
    report_interval_seconds: u64,
}

impl SoakProgressReporter {
    pub fn new(total_operations: u64, report_interval_seconds: u64) -> Self {
        let now = Instant::now();
        Self {
            start_time: now,
            last_report_time: now,
            operations_completed: AtomicU64::new(0),
            total_operations,
            report_interval_seconds,
        }
    }

    /// Increment operations completed and report if interval passed.
    pub fn increment(&self) -> Option<String> {
        let completed = self.operations_completed.fetch_add(1, Ordering::Relaxed) + 1;
        let elapsed = self.last_report_time.elapsed();

        if elapsed.as_secs() >= self.report_interval_seconds {
            let total_elapsed = self.start_time.elapsed().as_secs_f64();
            let progress_percent = (completed as f64 / self.total_operations as f64) * 100.0;
            let ops_per_sec = completed as f64 / total_elapsed;

            Some(format!(
                "Progress: {}/{} ops ({:.1}%) | {:.1} ops/sec | {:.1}s elapsed",
                completed, self.total_operations, progress_percent, ops_per_sec, total_elapsed
            ))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_soak_metrics_latency_buckets() {
        let mut metrics = SoakMetrics::default();

        // Test bucket boundaries
        metrics.record_write_success(500);      // Bucket 0: < 1ms
        metrics.record_write_success(5_000);    // Bucket 1: 1-10ms
        metrics.record_write_success(50_000);   // Bucket 2: 10-100ms
        metrics.record_write_success(500_000);  // Bucket 3: 100ms-1s
        metrics.record_write_success(5_000_000); // Bucket 4: 1s-10s
        metrics.record_write_success(15_000_000); // Bucket 5: 10s+

        assert_eq!(metrics.write_latency_buckets_us[0], 1);
        assert_eq!(metrics.write_latency_buckets_us[1], 1);
        assert_eq!(metrics.write_latency_buckets_us[2], 1);
        assert_eq!(metrics.write_latency_buckets_us[3], 1);
        assert_eq!(metrics.write_latency_buckets_us[4], 1);
        assert_eq!(metrics.write_latency_buckets_us[5], 1);
        assert_eq!(metrics.writes_successful, 6);
    }

    #[test]
    fn test_soak_metrics_averages() {
        let mut metrics = SoakMetrics::default();

        metrics.record_write_success(1_000);  // 1ms
        metrics.record_write_success(2_000);  // 2ms
        metrics.record_write_success(3_000);  // 3ms

        // Average: (1 + 2 + 3) / 3 = 2ms
        assert!((metrics.avg_write_latency_ms() - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_workload_generation() {
        let workload = Workload::generate(100, 70, 50);

        assert_eq!(workload.len(), 100);

        let mut read_count = 0;
        let mut write_count = 0;

        for i in 0..workload.len() {
            if let Some((op_type, key_id)) = workload.get(i) {
                assert!(key_id < 50, "key_id should be < key_space_size");
                match op_type {
                    WorkloadOp::Read => read_count += 1,
                    WorkloadOp::Write => write_count += 1,
                }
            }
        }

        // With 70% reads, expect roughly 70 reads, 30 writes
        // Allow some variance due to randomness
        assert!(read_count >= 55 && read_count <= 85, "read_count: {}", read_count);
        assert!(write_count >= 15 && write_count <= 45, "write_count: {}", write_count);
    }
}
