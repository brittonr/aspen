//! Metrics for hook execution.
//!
//! Provides execution metrics tracking for monitoring and debugging.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::Duration;

use parking_lot::RwLock;
use serde::Deserialize;
use serde::Serialize;

/// Metrics for hook handler executions.
#[derive(Debug)]
pub struct ExecutionMetrics {
    /// Per-handler metrics.
    handlers: Arc<RwLock<HashMap<String, HandlerMetrics>>>,

    /// Global counters.
    global: GlobalMetrics,
}

impl Default for ExecutionMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionMetrics {
    /// Create a new metrics instance.
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            global: GlobalMetrics::new(),
        }
    }

    /// Record a successful execution.
    pub fn record_success(&self, handler_name: &str, duration: Duration) {
        self.global.successes.fetch_add(1, Ordering::Relaxed);
        self.global.add_latency(duration);

        // Update per-handler metrics synchronously using parking_lot
        let mut handlers = self.handlers.write();
        let metrics = handlers.entry(handler_name.to_string()).or_default();
        metrics.record_success(duration);
    }

    /// Record a failed execution.
    pub fn record_failure(&self, handler_name: &str, duration: Duration) {
        self.global.failures.fetch_add(1, Ordering::Relaxed);
        self.global.add_latency(duration);

        let mut handlers = self.handlers.write();
        let metrics = handlers.entry(handler_name.to_string()).or_default();
        metrics.record_failure(duration);
    }

    /// Record a dropped event (concurrency limit reached).
    pub fn record_dropped(&self, handler_name: &str) {
        self.global.dropped.fetch_add(1, Ordering::Relaxed);

        let mut handlers = self.handlers.write();
        let metrics = handlers.entry(handler_name.to_string()).or_default();
        metrics.dropped.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a job submission.
    pub fn record_job_submitted(&self, handler_name: &str) {
        self.global.jobs_submitted.fetch_add(1, Ordering::Relaxed);

        let mut handlers = self.handlers.write();
        let metrics = handlers.entry(handler_name.to_string()).or_default();
        metrics.jobs_submitted.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a job submission failure.
    pub fn record_job_submit_failure(&self, handler_name: &str) {
        self.global.job_submit_failures.fetch_add(1, Ordering::Relaxed);

        let mut handlers = self.handlers.write();
        let metrics = handlers.entry(handler_name.to_string()).or_default();
        metrics.job_submit_failures.fetch_add(1, Ordering::Relaxed);
    }

    /// Get a snapshot of all metrics.
    pub fn snapshot(&self) -> MetricsSnapshot {
        let handlers = self.handlers.read();
        let handler_snapshots: HashMap<String, HandlerMetricsSnapshot> =
            handlers.iter().map(|(k, v)| (k.clone(), v.snapshot())).collect();

        MetricsSnapshot {
            global: self.global.snapshot(),
            handlers: handler_snapshots,
        }
    }

    /// Get global metrics snapshot.
    pub fn global_snapshot(&self) -> GlobalMetricsSnapshot {
        self.global.snapshot()
    }

    /// Reset all metrics.
    pub fn reset(&self) {
        self.global.reset();
        self.handlers.write().clear();
    }
}

/// Global metrics counters.
#[derive(Debug)]
struct GlobalMetrics {
    /// Total successful executions.
    successes: AtomicU64,
    /// Total failed executions.
    failures: AtomicU64,
    /// Total dropped events.
    dropped: AtomicU64,
    /// Total jobs submitted.
    jobs_submitted: AtomicU64,
    /// Total job submission failures.
    job_submit_failures: AtomicU64,
    /// Total latency in microseconds (for averaging).
    total_latency_us: AtomicU64,
    /// Number of latency samples.
    latency_samples: AtomicU64,
}

impl GlobalMetrics {
    fn new() -> Self {
        Self {
            successes: AtomicU64::new(0),
            failures: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
            jobs_submitted: AtomicU64::new(0),
            job_submit_failures: AtomicU64::new(0),
            total_latency_us: AtomicU64::new(0),
            latency_samples: AtomicU64::new(0),
        }
    }

    fn add_latency(&self, duration: Duration) {
        self.total_latency_us.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
        self.latency_samples.fetch_add(1, Ordering::Relaxed);
    }

    fn snapshot(&self) -> GlobalMetricsSnapshot {
        let samples = self.latency_samples.load(Ordering::Relaxed);
        let total_us = self.total_latency_us.load(Ordering::Relaxed);
        let avg_latency_us = total_us.checked_div(samples).unwrap_or(0);

        GlobalMetricsSnapshot {
            successes: self.successes.load(Ordering::Relaxed),
            failures: self.failures.load(Ordering::Relaxed),
            dropped: self.dropped.load(Ordering::Relaxed),
            jobs_submitted: self.jobs_submitted.load(Ordering::Relaxed),
            job_submit_failures: self.job_submit_failures.load(Ordering::Relaxed),
            avg_latency_us,
        }
    }

    fn reset(&self) {
        self.successes.store(0, Ordering::Relaxed);
        self.failures.store(0, Ordering::Relaxed);
        self.dropped.store(0, Ordering::Relaxed);
        self.jobs_submitted.store(0, Ordering::Relaxed);
        self.job_submit_failures.store(0, Ordering::Relaxed);
        self.total_latency_us.store(0, Ordering::Relaxed);
        self.latency_samples.store(0, Ordering::Relaxed);
    }
}

/// Per-handler metrics.
#[derive(Debug)]
pub struct HandlerMetrics {
    /// Successful executions.
    successes: AtomicU64,
    /// Failed executions.
    failures: AtomicU64,
    /// Dropped events.
    dropped: AtomicU64,
    /// Jobs submitted (for job-based handlers).
    jobs_submitted: AtomicU64,
    /// Job submission failures.
    job_submit_failures: AtomicU64,
    /// Total latency in microseconds.
    total_latency_us: AtomicU64,
    /// Number of latency samples.
    latency_samples: AtomicU64,
}

impl HandlerMetrics {
    /// Create new handler metrics.
    pub fn new() -> Self {
        Self {
            successes: AtomicU64::new(0),
            failures: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
            jobs_submitted: AtomicU64::new(0),
            job_submit_failures: AtomicU64::new(0),
            total_latency_us: AtomicU64::new(0),
            latency_samples: AtomicU64::new(0),
        }
    }

    /// Record a successful execution.
    pub fn record_success(&self, duration: Duration) {
        self.successes.fetch_add(1, Ordering::Relaxed);
        self.total_latency_us.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
        self.latency_samples.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a failed execution.
    pub fn record_failure(&self, duration: Duration) {
        self.failures.fetch_add(1, Ordering::Relaxed);
        self.total_latency_us.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
        self.latency_samples.fetch_add(1, Ordering::Relaxed);
    }

    /// Get a snapshot of the metrics.
    pub fn snapshot(&self) -> HandlerMetricsSnapshot {
        let samples = self.latency_samples.load(Ordering::Relaxed);
        let total_us = self.total_latency_us.load(Ordering::Relaxed);
        let avg_latency_us = total_us.checked_div(samples).unwrap_or(0);

        HandlerMetricsSnapshot {
            successes: self.successes.load(Ordering::Relaxed),
            failures: self.failures.load(Ordering::Relaxed),
            dropped: self.dropped.load(Ordering::Relaxed),
            jobs_submitted: self.jobs_submitted.load(Ordering::Relaxed),
            job_submit_failures: self.job_submit_failures.load(Ordering::Relaxed),
            avg_latency_us,
        }
    }
}

impl Default for HandlerMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of all metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Global metrics.
    pub global: GlobalMetricsSnapshot,
    /// Per-handler metrics.
    pub handlers: HashMap<String, HandlerMetricsSnapshot>,
}

/// Snapshot of global metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalMetricsSnapshot {
    /// Total successful executions.
    pub successes: u64,
    /// Total failed executions.
    pub failures: u64,
    /// Total dropped events.
    pub dropped: u64,
    /// Total jobs submitted.
    pub jobs_submitted: u64,
    /// Total job submission failures.
    pub job_submit_failures: u64,
    /// Average latency in microseconds.
    pub avg_latency_us: u64,
}

impl GlobalMetricsSnapshot {
    /// Get total executions (successes + failures).
    pub fn total_executions(&self) -> u64 {
        self.successes + self.failures
    }

    /// Get success rate as a percentage (0.0 - 100.0).
    pub fn success_rate(&self) -> f64 {
        let total = self.total_executions();
        if total == 0 {
            100.0
        } else {
            (self.successes as f64 / total as f64) * 100.0
        }
    }

    /// Get average latency as a Duration.
    pub fn avg_latency(&self) -> Duration {
        Duration::from_micros(self.avg_latency_us)
    }
}

/// Snapshot of per-handler metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandlerMetricsSnapshot {
    /// Successful executions.
    pub successes: u64,
    /// Failed executions.
    pub failures: u64,
    /// Dropped events.
    pub dropped: u64,
    /// Jobs submitted.
    pub jobs_submitted: u64,
    /// Job submission failures.
    pub job_submit_failures: u64,
    /// Average latency in microseconds.
    pub avg_latency_us: u64,
}

impl HandlerMetricsSnapshot {
    /// Get total executions.
    pub fn total_executions(&self) -> u64 {
        self.successes + self.failures
    }

    /// Get success rate as a percentage.
    pub fn success_rate(&self) -> f64 {
        let total = self.total_executions();
        if total == 0 {
            100.0
        } else {
            (self.successes as f64 / total as f64) * 100.0
        }
    }

    /// Get average latency as a Duration.
    pub fn avg_latency(&self) -> Duration {
        Duration::from_micros(self.avg_latency_us)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_metrics() {
        let metrics = ExecutionMetrics::new();

        metrics.record_success("handler1", Duration::from_millis(10));
        metrics.record_success("handler1", Duration::from_millis(20));
        metrics.record_failure("handler1", Duration::from_millis(5));

        let snapshot = metrics.global_snapshot();
        assert_eq!(snapshot.successes, 2);
        assert_eq!(snapshot.failures, 1);
        assert_eq!(snapshot.total_executions(), 3);
    }

    #[test]
    fn test_handler_metrics() {
        let metrics = ExecutionMetrics::new();

        metrics.record_success("handler1", Duration::from_millis(10));
        metrics.record_failure("handler2", Duration::from_millis(5));

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.handlers.len(), 2);
        assert!(snapshot.handlers.contains_key("handler1"));
        assert!(snapshot.handlers.contains_key("handler2"));
    }

    #[test]
    fn test_success_rate_calculation() {
        let snapshot = GlobalMetricsSnapshot {
            successes: 8,
            failures: 2,
            dropped: 0,
            jobs_submitted: 0,
            job_submit_failures: 0,
            avg_latency_us: 1000,
        };

        assert_eq!(snapshot.success_rate(), 80.0);
        assert_eq!(snapshot.total_executions(), 10);
    }

    #[test]
    fn test_success_rate_zero_executions() {
        let snapshot = GlobalMetricsSnapshot {
            successes: 0,
            failures: 0,
            dropped: 0,
            jobs_submitted: 0,
            job_submit_failures: 0,
            avg_latency_us: 0,
        };

        // No executions should report 100% success rate
        assert_eq!(snapshot.success_rate(), 100.0);
    }

    #[test]
    fn test_metrics_reset() {
        let metrics = ExecutionMetrics::new();

        metrics.record_success("handler1", Duration::from_millis(10));

        let before = metrics.global_snapshot();
        assert_eq!(before.successes, 1);

        metrics.reset();

        let after = metrics.global_snapshot();
        assert_eq!(after.successes, 0);
    }

    #[test]
    fn test_metrics_serialization() {
        let snapshot = MetricsSnapshot {
            global: GlobalMetricsSnapshot {
                successes: 100,
                failures: 5,
                dropped: 2,
                jobs_submitted: 50,
                job_submit_failures: 1,
                avg_latency_us: 5000,
            },
            handlers: HashMap::new(),
        };

        let json = serde_json::to_string(&snapshot).unwrap();
        let decoded: MetricsSnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.global.successes, 100);
        assert_eq!(decoded.global.failures, 5);
    }
}
