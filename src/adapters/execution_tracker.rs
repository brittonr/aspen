//! Execution state tracking
//!
//! Manages the state of local process executions

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

use crate::worker_trait::WorkResult;
use super::{ExecutionHandle, ExecutionStatus};

/// State of a local process execution
#[derive(Debug, Clone)]
pub struct LocalProcessState {
    pub handle: ExecutionHandle,
    pub pid: u32,
    pub status: ExecutionStatus,
    pub started_at: u64,
    pub completed_at: Option<u64>,
}

/// Tracks execution state
pub struct ExecutionTracker {
    executions: Arc<RwLock<HashMap<String, LocalProcessState>>>,
}

impl ExecutionTracker {
    /// Create a new execution tracker
    pub fn new() -> Self {
        Self {
            executions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new execution
    pub async fn create_execution(&self, handle: ExecutionHandle) {
        let state = LocalProcessState {
            handle: handle.clone(),
            pid: 0, // Will be updated when process starts
            status: ExecutionStatus::Running,
            started_at: Self::current_timestamp(),
            completed_at: None,
        };

        let mut executions = self.executions.write().await;
        executions.insert(handle.id.clone(), state);
    }

    /// Update the PID for an execution
    pub async fn update_pid(&self, handle_id: &str, pid: u32) {
        let mut executions = self.executions.write().await;
        if let Some(state) = executions.get_mut(handle_id) {
            state.pid = pid;
        }
    }

    /// Mark an execution as completed
    pub async fn mark_completed(&self, handle_id: &str, result: WorkResult) {
        let mut executions = self.executions.write().await;
        if let Some(state) = executions.get_mut(handle_id) {
            state.status = ExecutionStatus::Completed(result);
            state.completed_at = Some(Self::current_timestamp());
        }
    }

    /// Mark an execution as failed
    pub async fn mark_failed(&self, handle_id: &str, error: String) {
        let mut executions = self.executions.write().await;
        if let Some(state) = executions.get_mut(handle_id) {
            state.status = ExecutionStatus::Failed(error);
            state.completed_at = Some(Self::current_timestamp());
        }
    }

    /// Mark an execution as cancelled
    pub async fn mark_cancelled(&self, handle_id: &str) {
        let mut executions = self.executions.write().await;
        if let Some(state) = executions.get_mut(handle_id) {
            state.status = ExecutionStatus::Cancelled;
            state.completed_at = Some(Self::current_timestamp());
        }
    }

    /// Get execution status
    pub async fn get_status(&self, handle_id: &str) -> Option<ExecutionStatus> {
        let executions = self.executions.read().await;
        executions.get(handle_id).map(|state| state.status.clone())
    }

    /// Get execution state
    pub async fn get_state(&self, handle_id: &str) -> Option<LocalProcessState> {
        let executions = self.executions.read().await;
        executions.get(handle_id).cloned()
    }

    /// Get PID for an execution
    pub async fn get_pid(&self, handle_id: &str) -> Option<u32> {
        let executions = self.executions.read().await;
        executions.get(handle_id).map(|state| state.pid)
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

    /// Clean up old executions
    pub async fn cleanup_old(&self, older_than: Duration) -> usize {
        let mut executions = self.executions.write().await;
        let cutoff = Self::current_timestamp() - older_than.as_secs();
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

    /// Get current Unix timestamp
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time is before UNIX epoch")
            .as_secs()
    }
}

impl Default for ExecutionTracker {
    fn default() -> Self {
        Self::new()
    }
}
