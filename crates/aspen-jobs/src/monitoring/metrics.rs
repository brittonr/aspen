//! Job execution metrics and aggregation.

use std::collections::HashMap;
use std::collections::VecDeque;
use std::time::Duration;

use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

use crate::error::Result;
use crate::job::JobId;

/// Job execution metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobMetrics {
    /// Job ID.
    pub job_id: JobId,
    /// Job type.
    pub job_type: String,
    /// Worker ID.
    pub worker_id: String,
    /// Node ID.
    pub node_id: String,
    /// Queue time (ms).
    pub queue_time_ms: u64,
    /// Execution time (ms).
    pub execution_time_ms: u64,
    /// Total time (ms).
    pub total_time_ms: u64,
    /// CPU usage (percentage).
    pub cpu_usage: f32,
    /// Memory usage (bytes).
    pub memory_bytes: u64,
    /// Network bytes sent.
    pub network_sent_bytes: u64,
    /// Network bytes received.
    pub network_recv_bytes: u64,
    /// Retry count.
    pub retry_count: u32,
    /// Success flag.
    pub success: bool,
    /// Error message if failed.
    pub error_message: Option<String>,
    /// Custom metrics.
    pub custom: HashMap<String, f64>,
}

/// Aggregated metrics over a time window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedMetrics {
    /// Time window start.
    pub window_start: DateTime<Utc>,
    /// Time window end.
    pub window_end: DateTime<Utc>,
    /// Total jobs processed.
    pub total_jobs: u64,
    /// Successful jobs.
    pub successful_jobs: u64,
    /// Failed jobs.
    pub failed_jobs: u64,
    /// Average queue time (ms).
    pub avg_queue_time_ms: f64,
    /// Average execution time (ms).
    pub avg_execution_time_ms: f64,
    /// P50 execution time (ms).
    pub p50_execution_time_ms: u64,
    /// P95 execution time (ms).
    pub p95_execution_time_ms: u64,
    /// P99 execution time (ms).
    pub p99_execution_time_ms: u64,
    /// Jobs per second.
    pub jobs_per_second: f64,
    /// Success rate.
    pub success_rate: f64,
    /// Top job types by count.
    pub top_job_types: Vec<(String, u64)>,
    /// Top error reasons.
    pub top_errors: Vec<(String, u64)>,
}

/// Metrics aggregator.
pub(crate) struct MetricsAggregator {
    pub(crate) total_count: u64,
    pub(crate) success_count: u64,
    pub(crate) execution_times: VecDeque<u64>,
    pub(crate) queue_times: VecDeque<u64>,
}

impl MetricsAggregator {
    pub(crate) fn new() -> Self {
        Self {
            total_count: 0,
            success_count: 0,
            execution_times: VecDeque::new(),
            queue_times: VecDeque::new(),
        }
    }

    pub(crate) fn add_metrics(&mut self, metrics: &JobMetrics) {
        self.total_count += 1;

        if metrics.success {
            self.success_count += 1;
        }

        // Keep bounded history
        while self.execution_times.len() >= 10000 {
            self.execution_times.pop_front();
        }
        self.execution_times.push_back(metrics.execution_time_ms);

        while self.queue_times.len() >= 10000 {
            self.queue_times.pop_front();
        }
        self.queue_times.push_back(metrics.queue_time_ms);
    }

    pub(crate) fn success_rate(&self) -> f64 {
        if self.total_count == 0 {
            0.0
        } else {
            self.success_count as f64 / self.total_count as f64
        }
    }

    pub(crate) fn get_aggregated(&self, window: Duration) -> Result<AggregatedMetrics> {
        let now = Utc::now();
        let window_start = now - chrono::Duration::from_std(window).unwrap();

        // Calculate percentiles
        let mut exec_times: Vec<_> = self.execution_times.iter().cloned().collect();
        exec_times.sort_unstable();

        let p50 = exec_times.get(exec_times.len() / 2).copied().unwrap_or(0);
        let p95 = exec_times.get(exec_times.len() * 95 / 100).copied().unwrap_or(0);
        let p99 = exec_times.get(exec_times.len() * 99 / 100).copied().unwrap_or(0);

        Ok(AggregatedMetrics {
            window_start,
            window_end: now,
            total_jobs: self.total_count,
            successful_jobs: self.success_count,
            failed_jobs: self.total_count - self.success_count,
            avg_queue_time_ms: self.queue_times.iter().sum::<u64>() as f64 / self.queue_times.len().max(1) as f64,
            avg_execution_time_ms: self.execution_times.iter().sum::<u64>() as f64
                / self.execution_times.len().max(1) as f64,
            p50_execution_time_ms: p50,
            p95_execution_time_ms: p95,
            p99_execution_time_ms: p99,
            jobs_per_second: self.total_count as f64 / window.as_secs_f64(),
            success_rate: self.success_rate(),
            top_job_types: vec![], // Would need to track separately
            top_errors: vec![],    // Would need to track separately
        })
    }
}
