//! Execution state tracking
//!
//! Manages the state of local process executions with automatic cleanup

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::worker_trait::WorkResult;
use crate::common::timestamp::current_timestamp_or_zero;
use super::{ExecutionHandle, ExecutionStatus};
use super::cleanup::{CleanupConfig, CleanupMetrics};

/// State of a local process execution
#[derive(Debug, Clone)]
pub struct LocalProcessState {
    pub handle: ExecutionHandle,
    pub pid: u32,
    pub status: ExecutionStatus,
    pub started_at: u64,
    pub completed_at: Option<u64>,
    /// Last access time for LRU eviction
    pub last_accessed: u64,
}

/// Tracks execution state with automatic cleanup
pub struct ExecutionTracker {
    executions: Arc<RwLock<HashMap<String, LocalProcessState>>>,
    config: CleanupConfig,
    metrics: Arc<RwLock<CleanupMetrics>>,
    cleanup_task_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl ExecutionTracker {
    /// Create a new execution tracker with default config
    pub fn new() -> Self {
        Self::with_config(CleanupConfig::default())
    }

    /// Create a new execution tracker with custom config
    pub fn with_config(config: CleanupConfig) -> Self {
        let tracker = Self {
            executions: Arc::new(RwLock::new(HashMap::new())),
            config: config.clone(),
            metrics: Arc::new(RwLock::new(CleanupMetrics::new())),
            cleanup_task_handle: Arc::new(RwLock::new(None)),
        };

        // Start background cleanup task if enabled
        if config.enable_background_cleanup {
            tracker.start_background_cleanup();
        }

        tracker
    }

    /// Start the background cleanup task
    fn start_background_cleanup(&self) {
        let executions = self.executions.clone();
        let config = self.config.clone();
        let metrics = self.metrics.clone();
        let cleanup_task_handle = self.cleanup_task_handle.clone();

        let task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.cleanup_interval);
            loop {
                interval.tick().await;

                debug!("Running background cleanup for execution tracker");
                let start = std::time::Instant::now();

                let (ttl_count, lru_count) = Self::cleanup_internal(
                    &executions,
                    &config,
                ).await;

                let duration = start.elapsed();

                if ttl_count > 0 || lru_count > 0 {
                    info!(
                        "Execution tracker cleanup: {} TTL expired, {} LRU evicted in {:?}",
                        ttl_count, lru_count, duration
                    );
                }

                // Update metrics
                let mut m = metrics.write().await;
                m.record_cleanup(ttl_count, lru_count, duration);
            }
        });

        // Store the task handle so we can cancel it on shutdown
        let handle_clone = cleanup_task_handle.clone();
        tokio::spawn(async move {
            let mut handle = handle_clone.write().await;
            *handle = Some(task);
        });
    }

    /// Create a new execution
    pub async fn create_execution(&self, handle: ExecutionHandle) {
        let now = current_timestamp_or_zero() as u64;
        let state = LocalProcessState {
            handle: handle.clone(),
            pid: 0, // Will be updated when process starts
            status: ExecutionStatus::Running,
            started_at: now,
            completed_at: None,
            last_accessed: now,
        };

        let mut executions = self.executions.write().await;
        executions.insert(handle.id.clone(), state);

        // Check if we need immediate cleanup due to size limit
        if executions.len() > self.config.max_entries {
            drop(executions); // Release write lock
            self.cleanup_size_limit().await;
        }
    }

    /// Update the PID for an execution
    pub async fn update_pid(&self, handle_id: &str, pid: u32) {
        let mut executions = self.executions.write().await;
        if let Some(state) = executions.get_mut(handle_id) {
            state.pid = pid;
            state.last_accessed = current_timestamp_or_zero() as u64;
        }
    }

    /// Mark an execution as completed
    pub async fn mark_completed(&self, handle_id: &str, result: WorkResult) {
        let now = current_timestamp_or_zero() as u64;
        let mut executions = self.executions.write().await;
        if let Some(state) = executions.get_mut(handle_id) {
            state.status = ExecutionStatus::Completed(result);
            state.completed_at = Some(now);
            state.last_accessed = now;
        }
    }

    /// Mark an execution as failed
    pub async fn mark_failed(&self, handle_id: &str, error: String) {
        let now = current_timestamp_or_zero() as u64;
        let mut executions = self.executions.write().await;
        if let Some(state) = executions.get_mut(handle_id) {
            state.status = ExecutionStatus::Failed(error);
            state.completed_at = Some(now);
            state.last_accessed = now;
        }
    }

    /// Mark an execution as cancelled
    pub async fn mark_cancelled(&self, handle_id: &str) {
        let now = current_timestamp_or_zero() as u64;
        let mut executions = self.executions.write().await;
        if let Some(state) = executions.get_mut(handle_id) {
            state.status = ExecutionStatus::Cancelled;
            state.completed_at = Some(now);
            state.last_accessed = now;
        }
    }

    /// Get execution status
    pub async fn get_status(&self, handle_id: &str) -> Option<ExecutionStatus> {
        let mut executions = self.executions.write().await;
        if let Some(state) = executions.get_mut(handle_id) {
            state.last_accessed = current_timestamp_or_zero() as u64;
            Some(state.status.clone())
        } else {
            None
        }
    }

    /// Get execution state
    pub async fn get_state(&self, handle_id: &str) -> Option<LocalProcessState> {
        let mut executions = self.executions.write().await;
        if let Some(state) = executions.get_mut(handle_id) {
            state.last_accessed = current_timestamp_or_zero() as u64;
            Some(state.clone())
        } else {
            None
        }
    }

    /// Get PID for an execution
    pub async fn get_pid(&self, handle_id: &str) -> Option<u32> {
        let mut executions = self.executions.write().await;
        if let Some(state) = executions.get_mut(handle_id) {
            state.last_accessed = current_timestamp_or_zero() as u64;
            Some(state.pid)
        } else {
            None
        }
    }

    /// Count running executions
    pub async fn count_running(&self) -> usize {
        let executions = self.executions.read().await;
        executions
            .values()
            .filter(|state| matches!(state.status, ExecutionStatus::Running))
            .count()
    }

    /// List all execution handles
    pub async fn list_handles(&self) -> Vec<ExecutionHandle> {
        let executions = self.executions.read().await;
        executions
            .values()
            .map(|state| state.handle.clone())
            .collect()
    }

    /// Clean up old executions (legacy interface)
    pub async fn cleanup_old(&self, older_than: Duration) -> usize {
        let mut executions = self.executions.write().await;
        let cutoff = (current_timestamp_or_zero() as u64).saturating_sub(older_than.as_secs());
        let mut cleaned = 0;

        executions.retain(|_, state| {
            if let Some(completed_at) = state.completed_at {
                if completed_at < cutoff {
                    cleaned += 1;
                    return false;
                }
            }
            true
        });

        cleaned
    }

    /// Clean up based on TTL and size limits
    pub async fn cleanup(&self) -> (usize, usize) {
        Self::cleanup_internal(&self.executions, &self.config).await
    }

    /// Internal cleanup implementation (static to work with background task)
    async fn cleanup_internal(
        executions: &Arc<RwLock<HashMap<String, LocalProcessState>>>,
        config: &CleanupConfig,
    ) -> (usize, usize) {
        let mut exec_map = executions.write().await;
        let now = current_timestamp_or_zero() as u64;
        let mut ttl_cleaned = 0;

        // First pass: Remove entries that exceed TTL based on status
        exec_map.retain(|_, state| {
            if let Some(completed_at) = state.completed_at {
                let age = now.saturating_sub(completed_at);
                let should_remove = match &state.status {
                    ExecutionStatus::Completed(_) => age > config.completed_ttl.as_secs(),
                    ExecutionStatus::Failed(_) => age > config.failed_ttl.as_secs(),
                    ExecutionStatus::Cancelled => age > config.cancelled_ttl.as_secs(),
                    _ => false,
                };

                if should_remove {
                    ttl_cleaned += 1;
                    return false;
                }
            }
            true
        });

        // Second pass: If still over size limit, evict oldest by last_accessed (LRU)
        let mut lru_cleaned = 0;
        if exec_map.len() > config.max_entries {
            // Collect entries with their last accessed time
            let mut entries: Vec<_> = exec_map
                .iter()
                .map(|(k, v)| (k.clone(), v.last_accessed))
                .collect();

            // Sort by last_accessed ascending (oldest first)
            entries.sort_by_key(|(_, last_accessed)| *last_accessed);

            // Calculate how many to remove
            let to_remove = exec_map.len() - config.max_entries;

            // Remove the oldest entries
            for (key, _) in entries.iter().take(to_remove) {
                exec_map.remove(key);
                lru_cleaned += 1;
            }
        }

        (ttl_cleaned, lru_cleaned)
    }

    /// Cleanup when size limit is exceeded
    async fn cleanup_size_limit(&self) {
        let (ttl_count, lru_count) = self.cleanup().await;
        if lru_count > 0 {
            warn!(
                "Execution tracker size limit exceeded, evicted {} entries (LRU)",
                lru_count
            );
        }
        if ttl_count > 0 {
            debug!("Cleaned up {} TTL-expired entries", ttl_count);
        }
    }

    /// Get cleanup metrics
    pub async fn get_metrics(&self) -> CleanupMetrics {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }

    /// Stop the background cleanup task
    pub async fn shutdown(&self) {
        let mut handle = self.cleanup_task_handle.write().await;
        if let Some(task) = handle.take() {
            task.abort();
            info!("Stopped execution tracker background cleanup task");
        }
    }

}

impl Default for ExecutionTracker {
    fn default() -> Self {
        Self::new()
    }
}
