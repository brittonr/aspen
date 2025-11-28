//! Shared cleanup configuration and utilities for preventing memory leaks
//!
//! This module provides configurable TTL and size-based cleanup for execution tracking
//! HashMaps across all adapter implementations.

use std::time::Duration;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

/// Configuration for execution state cleanup to prevent unbounded memory growth
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupConfig {
    /// Time-to-live for completed executions (default: 24 hours)
    pub completed_ttl: Duration,

    /// Time-to-live for failed executions (default: 24 hours)
    pub failed_ttl: Duration,

    /// Time-to-live for cancelled executions (default: 1 hour)
    pub cancelled_ttl: Duration,

    /// Maximum number of execution records to keep (default: 10,000)
    pub max_entries: usize,

    /// Interval between background cleanup runs (default: 1 hour)
    pub cleanup_interval: Duration,

    /// Whether to enable background cleanup task (default: true)
    pub enable_background_cleanup: bool,
}

impl Default for CleanupConfig {
    fn default() -> Self {
        Self {
            completed_ttl: Duration::from_secs(24 * 60 * 60), // 24 hours
            failed_ttl: Duration::from_secs(24 * 60 * 60),    // 24 hours
            cancelled_ttl: Duration::from_secs(60 * 60),      // 1 hour
            max_entries: 10_000,
            cleanup_interval: Duration::from_secs(60 * 60),   // 1 hour
            enable_background_cleanup: true,
        }
    }
}

impl CleanupConfig {
    /// Create a config optimized for high-throughput scenarios
    pub fn high_throughput() -> Self {
        Self {
            completed_ttl: Duration::from_secs(6 * 60 * 60), // 6 hours
            failed_ttl: Duration::from_secs(12 * 60 * 60),   // 12 hours
            cancelled_ttl: Duration::from_secs(30 * 60),     // 30 minutes
            max_entries: 50_000,
            cleanup_interval: Duration::from_secs(30 * 60),  // 30 minutes
            enable_background_cleanup: true,
        }
    }

    /// Create a config optimized for low-memory scenarios
    pub fn low_memory() -> Self {
        Self {
            completed_ttl: Duration::from_secs(60 * 60),     // 1 hour
            failed_ttl: Duration::from_secs(60 * 60),        // 1 hour
            cancelled_ttl: Duration::from_secs(15 * 60),     // 15 minutes
            max_entries: 1_000,
            cleanup_interval: Duration::from_secs(15 * 60),  // 15 minutes
            enable_background_cleanup: true,
        }
    }

    /// Create a config for testing (aggressive cleanup)
    pub fn testing() -> Self {
        Self {
            completed_ttl: Duration::from_secs(60),          // 1 minute
            failed_ttl: Duration::from_secs(60),             // 1 minute
            cancelled_ttl: Duration::from_secs(30),          // 30 seconds
            max_entries: 100,
            cleanup_interval: Duration::from_secs(10),       // 10 seconds
            enable_background_cleanup: true,
        }
    }
}

/// Metrics for cleanup operations
#[derive(Debug, Clone, Default)]
pub struct CleanupMetrics {
    /// Total number of entries cleaned up
    pub total_cleaned: usize,

    /// Number of entries cleaned due to TTL expiration
    pub ttl_expired: usize,

    /// Number of entries cleaned due to size limit (LRU eviction)
    pub lru_evicted: usize,

    /// Timestamp of last cleanup operation
    pub last_cleanup: Option<u64>,

    /// Duration of last cleanup operation
    pub last_cleanup_duration_ms: Option<u64>,
}

impl CleanupMetrics {
    /// Create new metrics
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a cleanup operation
    pub fn record_cleanup(&mut self, ttl_count: usize, lru_count: usize, duration: Duration) {
        self.ttl_expired += ttl_count;
        self.lru_evicted += lru_count;
        self.total_cleaned += ttl_count + lru_count;
        self.last_cleanup = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        );
        self.last_cleanup_duration_ms = Some(duration.as_millis() as u64);
    }
}

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use crate::adapters::ExecutionStatus;

/// Trait for execution states that can be cleaned up
///
/// Implement this trait for your execution state types to enable
/// the shared cleanup logic.
pub trait CleanableExecution {
    /// Get the completion timestamp if the execution is complete
    fn completed_at(&self) -> Option<u64>;

    /// Get the last accessed timestamp
    fn last_accessed(&self) -> u64;

    /// Get the execution status
    fn status(&self) -> &ExecutionStatus;
}

/// Performs cleanup on a collection of execution states
///
/// This function implements two cleanup strategies:
/// 1. TTL-based cleanup: Remove entries that exceed their TTL based on status
/// 2. LRU eviction: If still over size limit, evict oldest by last_accessed
///
/// Entries are removed when:
/// - Completed executions older than `completed_ttl` (default: 24 hours)
/// - Failed executions older than `failed_ttl` (default: 24 hours)
/// - Cancelled executions older than `cancelled_ttl` (default: 1 hour)
/// - If still over size limit, oldest accessed entries are evicted (LRU)
///
/// # Arguments
/// * `executions` - Mutable HashMap of execution states to clean
/// * `config` - Cleanup configuration with TTL and size limits
/// * `current_timestamp` - Current Unix timestamp in seconds
///
/// # Returns
/// A tuple of (ttl_cleaned_count, lru_cleaned_count)
pub fn cleanup_executions<T>(
    executions: &mut HashMap<String, T>,
    config: &CleanupConfig,
    current_timestamp: u64,
) -> (usize, usize)
where
    T: CleanableExecution,
{
    let mut ttl_cleaned = 0;
    let initial_count = executions.len();

    // First pass: Remove entries that exceed TTL based on status
    executions.retain(|key, state| {
        if let Some(completed_at) = state.completed_at() {
            let age = current_timestamp.saturating_sub(completed_at);
            let ttl_secs = match state.status() {
                ExecutionStatus::Completed(_) => Some(config.completed_ttl.as_secs()),
                ExecutionStatus::Failed(_) => Some(config.failed_ttl.as_secs()),
                ExecutionStatus::Cancelled => Some(config.cancelled_ttl.as_secs()),
                _ => None,
            };

            if let Some(ttl) = ttl_secs {
                if age > ttl {
                    debug!(
                        "Removing execution {} (age: {}s, ttl: {}s, status: {:?})",
                        key, age, ttl, state.status()
                    );
                    ttl_cleaned += 1;
                    return false;
                }
            }
        }
        true
    });

    // Second pass: If still over size limit, evict oldest by last_accessed (LRU)
    let mut lru_cleaned = 0;
    if executions.len() > config.max_entries {
        let overflow = executions.len() - config.max_entries;
        let mut entries: Vec<_> = executions
            .iter()
            .map(|(k, v)| (k.clone(), v.last_accessed()))
            .collect();

        entries.sort_by_key(|(_, last_accessed)| *last_accessed);

        for (key, last_accessed) in entries.iter().take(overflow) {
            if let Some(state) = executions.get(key) {
                debug!(
                    "Evicting execution {} via LRU (last_accessed: {}, status: {:?})",
                    key, last_accessed, state.status()
                );
            }
            executions.remove(key);
            lru_cleaned += 1;
        }

        if lru_cleaned > 0 {
            warn!(
                "Execution HashMap exceeded max_entries ({}), evicted {} oldest entries via LRU",
                config.max_entries, lru_cleaned
            );
        }
    }

    if ttl_cleaned > 0 || lru_cleaned > 0 {
        debug!(
            "Cleanup completed: {}/{} entries removed (TTL: {}, LRU: {})",
            ttl_cleaned + lru_cleaned, initial_count, ttl_cleaned, lru_cleaned
        );
    }

    (ttl_cleaned, lru_cleaned)
}

/// Reusable background cleanup task that can be used by any adapter
///
/// This eliminates code duplication by providing a shared cleanup task
/// that works with any HashMap of cleanable execution states.
pub struct CleanupTask<T>
where
    T: CleanableExecution + Send + Sync + 'static,
{
    /// Handle to the background cleanup task
    task_handle: Option<JoinHandle<()>>,
    /// Cleanup metrics
    metrics: Arc<RwLock<CleanupMetrics>>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> CleanupTask<T>
where
    T: CleanableExecution + Send + Sync + 'static,
{
    /// Start a new background cleanup task
    ///
    /// # Arguments
    /// * `executions` - Shared HashMap of execution states to clean
    /// * `config` - Cleanup configuration
    /// * `adapter_name` - Name of the adapter (for logging)
    pub fn start(
        executions: Arc<RwLock<HashMap<String, T>>>,
        config: CleanupConfig,
        adapter_name: String,
    ) -> Self {
        let metrics = Arc::new(RwLock::new(CleanupMetrics::new()));
        let metrics_clone = metrics.clone();

        let task_handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.cleanup_interval);
            loop {
                interval.tick().await;

                debug!("Running background cleanup for {}", adapter_name);
                let start = std::time::Instant::now();

                // Perform cleanup
                let (ttl_count, lru_count) = {
                    let mut exec_map = executions.write().await;
                    let now = crate::common::get_unix_timestamp_or_zero();
                    cleanup_executions(&mut exec_map, &config, now)
                };

                let duration = start.elapsed();

                if ttl_count > 0 || lru_count > 0 {
                    tracing::info!(
                        "{} cleanup: {} TTL expired, {} LRU evicted in {:?}",
                        adapter_name, ttl_count, lru_count, duration
                    );
                }

                // Update metrics
                let mut m = metrics_clone.write().await;
                m.record_cleanup(ttl_count, lru_count, duration);
            }
        });

        Self {
            task_handle: Some(task_handle),
            metrics,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Get cleanup metrics
    pub async fn get_metrics(&self) -> CleanupMetrics {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }

    /// Stop the background cleanup task
    pub fn stop(&mut self) {
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
    }
}

impl<T> Drop for CleanupTask<T>
where
    T: CleanableExecution + Send + Sync + 'static,
{
    fn drop(&mut self) {
        self.stop();
    }
}
