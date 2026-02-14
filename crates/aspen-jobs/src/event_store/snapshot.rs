//! Workflow snapshot types for state capture and optimization.

use std::collections::HashMap;

use aspen_hlc::SerializableTimestamp;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

use super::types::SNAPSHOT_KEY_PREFIX;
use super::types::WorkflowExecutionId;

/// Workflow state snapshot for optimization.
///
/// Snapshots capture the derived state at a point in time,
/// allowing replay to start from the snapshot instead of
/// replaying from the beginning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSnapshot {
    /// Schema version for forward compatibility.
    pub schema_version: u32,
    /// Snapshot ID.
    pub snapshot_id: u64,
    /// Workflow execution ID.
    pub workflow_id: WorkflowExecutionId,
    /// Event ID at snapshot time.
    pub at_event_id: u64,
    /// HLC timestamp of snapshot.
    pub hlc_timestamp: SerializableTimestamp,
    /// Wall-clock time.
    pub timestamp: DateTime<Utc>,
    /// Serialized workflow state.
    pub state: serde_json::Value,
    /// Pending activities (activity_id -> ActivityState).
    pub pending_activities: HashMap<String, ActivityState>,
    /// Pending timers (timer_id -> TimerState).
    pub pending_timers: HashMap<String, TimerState>,
    /// Registered compensations for saga rollback.
    pub compensation_stack: Vec<CompensationEntry>,
    /// Side effect sequence counter.
    pub side_effect_seq: u64,
}

impl WorkflowSnapshot {
    /// Get storage key for this snapshot.
    pub fn storage_key(&self) -> String {
        format!("{}{}::{:020}", SNAPSHOT_KEY_PREFIX, self.workflow_id, self.snapshot_id)
    }
}

/// State of a pending activity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityState {
    /// Activity ID.
    pub activity_id: String,
    /// Activity type.
    pub activity_type: String,
    /// Input arguments.
    pub input: serde_json::Value,
    /// Current attempt number.
    pub attempt: u32,
    /// When the activity was scheduled.
    pub scheduled_at: DateTime<Utc>,
    /// Whether the activity has started.
    pub started: bool,
}

/// State of a pending timer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerState {
    /// Timer ID.
    pub timer_id: String,
    /// When the timer should fire.
    pub fire_at: DateTime<Utc>,
}

/// Entry in the compensation stack for saga pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompensationEntry {
    /// Compensation ID.
    pub compensation_id: String,
    /// Activity to call for compensation.
    pub activity: String,
    /// Input for compensation activity.
    pub input: serde_json::Value,
}

/// Cached activity result for memoization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedActivityResult {
    /// Activity ID.
    pub activity_id: String,
    /// Cached result.
    pub result: serde_json::Value,
    /// When the result was cached.
    pub cached_at: DateTime<Utc>,
    /// Original execution duration.
    pub duration_ms: u64,
}
