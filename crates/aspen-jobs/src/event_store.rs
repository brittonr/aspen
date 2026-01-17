//! Event sourcing layer for durable workflow execution.
//!
//! This module provides an append-only event log for workflow history,
//! enabling:
//! - Complete audit trail of all workflow operations
//! - Deterministic replay for crash recovery
//! - Activity result memoization (replay returns cached results)
//! - Snapshot-based optimization for long-running workflows
//!
//! ## Architecture
//!
//! Events are stored in Aspen's KV store with keys:
//! - `__wf_events::{workflow_id}::{event_id:020}` - Individual events
//! - `__wf_snapshots::{workflow_id}::{snapshot_id:020}` - State snapshots
//!
//! The 20-digit zero-padded IDs ensure lexicographic ordering for efficient
//! range scans via `scan()`.
//!
//! ## Tiger Style
//!
//! - `MAX_EVENTS_BEFORE_SNAPSHOT` = 1,000 events before mandatory snapshot
//! - `MAX_EVENTS_PER_WORKFLOW` = 50,000 events (Temporal limit)
//! - `MAX_EVENT_SIZE` = 1 MB (matches `MAX_VALUE_SIZE`)
//! - All operations have explicit bounds to prevent resource exhaustion

use std::collections::HashMap;
use std::sync::Arc;

use aspen_core::KeyValueStore;
use aspen_core::ScanRequest;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;
use aspen_core::hlc::HLC;
use aspen_core::hlc::SerializableTimestamp;
use aspen_core::hlc::create_hlc;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::error::JobError;
use crate::error::Result;

/// Key prefix for workflow events.
const EVENT_KEY_PREFIX: &str = "__wf_events::";

/// Key prefix for workflow snapshots.
const SNAPSHOT_KEY_PREFIX: &str = "__wf_snapshots::";

/// Maximum events before a snapshot is recommended.
/// Tiger Style: Fixed limit to prevent unbounded replay time.
const MAX_EVENTS_BEFORE_SNAPSHOT: u64 = 1_000;

/// Maximum events per workflow execution (Temporal-style limit).
/// Workflows exceeding this should use continue-as-new.
const MAX_EVENTS_PER_WORKFLOW: u64 = 50_000;

/// Maximum events to scan in a single replay operation.
const MAX_EVENTS_PER_SCAN: u32 = 10_000;

/// Schema version for event format evolution.
/// Increment this when making breaking changes to WorkflowEvent or WorkflowEventType.
const EVENT_SCHEMA_VERSION: u32 = 1;

/// Schema version for snapshot format evolution.
const _SNAPSHOT_SCHEMA_VERSION: u32 = 1;

/// Upcast an event from an older schema version to the current version.
///
/// This function implements the upcaster pattern from event sourcing:
/// when reading events from storage, older events may have different schemas.
/// Upcasting transforms them to the current format on read.
///
/// ## Versioning Strategy
///
/// - Version 1 (current): Initial schema, no migration needed
/// - Future versions: Add match arms to migrate from older versions
///
/// ## Tiger Style
///
/// - Each version migration is explicit and auditable
/// - Unknown versions are rejected rather than silently corrupted
/// - Migrations are deterministic and side-effect free
fn upcast_event(event: WorkflowEvent) -> std::result::Result<WorkflowEvent, String> {
    match event.schema_version {
        // Current version - no migration needed
        1 => Ok(event),

        // Version 0 was never released, but handle it for safety
        0 => {
            // If we ever had a v0 schema, migration logic would go here.
            // For now, reject as unknown since v1 is the first release.
            Err(format!(
                "unknown event schema version 0 for event_id {}: v1 is the first supported version",
                event.event_id
            ))
        }

        // Future versions would be handled here during downgrades (if supported)
        v if v > EVENT_SCHEMA_VERSION => Err(format!(
            "event schema version {} is newer than supported version {}: upgrade aspen-jobs",
            v, EVENT_SCHEMA_VERSION
        )),

        // Any other version is unknown
        v => Err(format!("unknown event schema version {} for event_id {}", v, event.event_id)),
    }
}

/// Unique identifier for a workflow execution.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowExecutionId(String);

impl WorkflowExecutionId {
    /// Create a new unique workflow execution ID.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Create from an existing string.
    pub fn from_string(id: String) -> Self {
        Self(id)
    }

    /// Get the string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for WorkflowExecutionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for WorkflowExecutionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Immutable workflow event for event sourcing.
///
/// Each event captures a discrete state change in the workflow execution.
/// Events are append-only and immutable once written.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEvent {
    /// Schema version for forward compatibility.
    pub schema_version: u32,
    /// Sequential event ID within this workflow execution.
    pub event_id: u64,
    /// Workflow execution ID.
    pub workflow_id: WorkflowExecutionId,
    /// HLC timestamp for causal ordering across nodes.
    pub hlc_timestamp: SerializableTimestamp,
    /// Wall-clock time for human readability.
    pub timestamp: DateTime<Utc>,
    /// Event type and payload.
    pub event_type: WorkflowEventType,
    /// Previous event ID for chain validation (None for first event).
    pub prev_event_id: Option<u64>,
}

impl WorkflowEvent {
    /// Create a new workflow event.
    pub fn new(
        event_id: u64,
        workflow_id: WorkflowExecutionId,
        event_type: WorkflowEventType,
        hlc: &HLC,
        prev_event_id: Option<u64>,
    ) -> Self {
        Self {
            schema_version: EVENT_SCHEMA_VERSION,
            event_id,
            workflow_id,
            hlc_timestamp: SerializableTimestamp::from(hlc.new_timestamp()),
            timestamp: Utc::now(),
            event_type,
            prev_event_id,
        }
    }

    /// Get storage key for this event.
    pub fn storage_key(&self) -> String {
        format!("{}{}::{:020}", EVENT_KEY_PREFIX, self.workflow_id, self.event_id)
    }
}

/// Types of workflow events.
///
/// This enum covers the complete lifecycle of workflow execution,
/// enabling deterministic replay and full audit trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowEventType {
    // === Workflow Lifecycle Events ===
    /// Workflow execution started.
    WorkflowStarted {
        /// Workflow definition/type name.
        workflow_type: String,
        /// Input arguments (serialized).
        input: serde_json::Value,
        /// Parent workflow ID if this is a child workflow.
        parent_workflow_id: Option<WorkflowExecutionId>,
    },
    /// Workflow completed successfully.
    WorkflowCompleted {
        /// Output result (serialized).
        output: serde_json::Value,
    },
    /// Workflow failed with error.
    WorkflowFailed {
        /// Error message.
        error: String,
        /// Error type/code.
        error_type: Option<String>,
        /// Whether the error is retryable.
        is_retryable: bool,
    },
    /// Workflow was cancelled.
    WorkflowCancelled {
        /// Cancellation reason.
        reason: String,
    },
    /// Workflow timed out.
    WorkflowTimedOut {
        /// Timeout type (execution, run, etc.).
        timeout_type: String,
    },

    // === Activity Events ===
    /// Activity scheduled for execution.
    ActivityScheduled {
        /// Unique activity ID within this workflow.
        activity_id: String,
        /// Activity type/name.
        activity_type: String,
        /// Input arguments (serialized).
        input: serde_json::Value,
        /// Activity-specific timeout.
        timeout_ms: Option<u64>,
    },
    /// Activity started executing on a worker.
    ActivityStarted {
        /// Activity ID.
        activity_id: String,
        /// Worker ID that picked up the activity.
        worker_id: String,
        /// Attempt number (1-based).
        attempt: u32,
    },
    /// Activity completed successfully.
    ActivityCompleted {
        /// Activity ID.
        activity_id: String,
        /// Result value (serialized).
        result: serde_json::Value,
        /// Execution duration in milliseconds.
        duration_ms: u64,
    },
    /// Activity failed.
    ActivityFailed {
        /// Activity ID.
        activity_id: String,
        /// Error message.
        error: String,
        /// Whether retry is allowed.
        is_retryable: bool,
        /// Attempt number that failed.
        attempt: u32,
    },
    /// Activity being retried.
    ActivityRetrying {
        /// Activity ID.
        activity_id: String,
        /// Next attempt number.
        attempt: u32,
        /// Retry delay in milliseconds.
        retry_delay_ms: u64,
    },
    /// Activity was cancelled.
    ActivityCancelled {
        /// Activity ID.
        activity_id: String,
        /// Cancellation reason.
        reason: String,
    },

    // === Timer Events ===
    /// Timer scheduled.
    TimerScheduled {
        /// Unique timer ID within this workflow.
        timer_id: String,
        /// Absolute fire time.
        fire_at: DateTime<Utc>,
        /// Duration from schedule time.
        duration_ms: u64,
    },
    /// Timer fired.
    TimerFired {
        /// Timer ID.
        timer_id: String,
    },
    /// Timer was cancelled.
    TimerCancelled {
        /// Timer ID.
        timer_id: String,
    },

    // === Signal Events ===
    /// Signal received from external source.
    SignalReceived {
        /// Signal name.
        signal_name: String,
        /// Signal payload (serialized).
        payload: serde_json::Value,
    },

    // === State Transition Events ===
    /// Workflow state transitioned.
    StateTransitioned {
        /// Previous state name.
        from_state: String,
        /// New state name.
        to_state: String,
        /// Transition trigger.
        trigger: String,
    },

    // === Child Workflow Events ===
    /// Child workflow started.
    ChildWorkflowStarted {
        /// Child workflow execution ID.
        child_workflow_id: WorkflowExecutionId,
        /// Child workflow type.
        workflow_type: String,
        /// Input to child workflow.
        input: serde_json::Value,
    },
    /// Child workflow completed.
    ChildWorkflowCompleted {
        /// Child workflow execution ID.
        child_workflow_id: WorkflowExecutionId,
        /// Result from child workflow.
        result: serde_json::Value,
    },
    /// Child workflow failed.
    ChildWorkflowFailed {
        /// Child workflow execution ID.
        child_workflow_id: WorkflowExecutionId,
        /// Error message.
        error: String,
    },

    // === Compensation/Saga Events ===
    /// Compensation registered for saga pattern.
    CompensationRegistered {
        /// Unique compensation ID.
        compensation_id: String,
        /// Activity to call for compensation.
        compensation_activity: String,
        /// Compensation input.
        compensation_input: serde_json::Value,
    },
    /// Compensation started executing.
    CompensationStarted {
        /// Compensation ID.
        compensation_id: String,
    },
    /// Compensation completed.
    CompensationCompleted {
        /// Compensation ID.
        compensation_id: String,
    },
    /// Compensation failed.
    CompensationFailed {
        /// Compensation ID.
        compensation_id: String,
        /// Error message.
        error: String,
    },

    // === Snapshot Events ===
    /// Snapshot was taken.
    SnapshotTaken {
        /// Snapshot ID.
        snapshot_id: u64,
        /// Event ID at snapshot time.
        at_event_id: u64,
    },

    // === Continue-As-New ===
    /// Workflow continued as new execution.
    ContinuedAsNew {
        /// New workflow execution ID.
        new_workflow_id: WorkflowExecutionId,
        /// Input to new execution.
        input: serde_json::Value,
    },

    // === Side Effects ===
    /// Non-deterministic side effect recorded.
    /// Used for UUID generation, random numbers, current time, etc.
    SideEffectRecorded {
        /// Sequence number for this side effect.
        seq: u64,
        /// Recorded value (serialized).
        value: serde_json::Value,
    },

    // === Markers ===
    /// Version marker for code evolution (patching).
    VersionMarker {
        /// Patch/version ID.
        patch_id: String,
        /// Whether the new code path was taken.
        new_path: bool,
    },
}

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

/// Event store for durable workflow execution.
///
/// Provides append-only event storage with snapshot support,
/// enabling deterministic replay and crash recovery.
pub struct WorkflowEventStore<S: KeyValueStore + ?Sized> {
    /// KV store for persistence.
    store: Arc<S>,
    /// HLC for causal timestamps.
    hlc: Arc<HLC>,
    /// Node ID for logging (reserved for future tracing).
    #[allow(dead_code)]
    node_id: String,
}

impl<S: KeyValueStore + ?Sized + 'static> WorkflowEventStore<S> {
    /// Create a new event store.
    pub fn new(store: Arc<S>, node_id: String) -> Self {
        let hlc = create_hlc(&node_id);
        Self {
            store,
            hlc: Arc::new(hlc),
            node_id,
        }
    }

    /// Get reference to the HLC.
    pub fn hlc(&self) -> &Arc<HLC> {
        &self.hlc
    }

    /// Append an event to the workflow history.
    ///
    /// Events are append-only and immutable once written.
    /// Returns the event ID of the appended event.
    pub async fn append_event(
        &self,
        workflow_id: &WorkflowExecutionId,
        event_type: WorkflowEventType,
        prev_event_id: Option<u64>,
    ) -> Result<u64> {
        let event_id = prev_event_id.map(|id| id + 1).unwrap_or(0);

        // Check workflow event limit
        if event_id >= MAX_EVENTS_PER_WORKFLOW {
            return Err(JobError::ExecutionFailed {
                reason: format!(
                    "Workflow {} exceeded max event limit ({} events). Use continue-as-new.",
                    workflow_id, MAX_EVENTS_PER_WORKFLOW
                ),
            });
        }

        let event = WorkflowEvent::new(event_id, workflow_id.clone(), event_type, &self.hlc, prev_event_id);

        let key = event.storage_key();
        let value = serde_json::to_string(&event).map_err(|e| JobError::SerializationError { source: e })?;

        self.store
            .write(WriteRequest {
                command: WriteCommand::Set { key, value },
            })
            .await
            .map_err(|e| JobError::StorageError { source: e })?;

        debug!(
            workflow_id = %workflow_id,
            event_id,
            event_type = ?std::mem::discriminant(&event.event_type),
            "event appended"
        );

        // Log warning if approaching snapshot threshold
        if event_id > 0 && event_id.is_multiple_of(MAX_EVENTS_BEFORE_SNAPSHOT) {
            warn!(
                workflow_id = %workflow_id,
                event_id,
                "workflow has {} events - consider taking a snapshot",
                event_id
            );
        }

        Ok(event_id)
    }

    /// Replay events from a starting point.
    ///
    /// Returns events in order, starting from `from_event_id`.
    /// If `from_event_id` is None, starts from the beginning.
    pub async fn replay_events(
        &self,
        workflow_id: &WorkflowExecutionId,
        from_event_id: Option<u64>,
    ) -> Result<Vec<WorkflowEvent>> {
        let prefix = format!("{}{}", EVENT_KEY_PREFIX, workflow_id);
        let start_suffix = from_event_id.map(|id| format!("::{:020}", id));

        // Build scan prefix
        let scan_prefix = if let Some(ref suffix) = start_suffix {
            format!("{}{}", prefix, suffix)
        } else {
            prefix.clone()
        };

        let scan_result = self
            .store
            .scan(ScanRequest {
                prefix: scan_prefix,
                limit: Some(MAX_EVENTS_PER_SCAN),
                continuation_token: None,
            })
            .await
            .map_err(|e| JobError::StorageError { source: e })?;

        let mut events = Vec::with_capacity(scan_result.entries.len());

        for entry in scan_result.entries {
            match serde_json::from_str::<WorkflowEvent>(&entry.value) {
                Ok(event) => {
                    // Handle schema version migration if needed
                    if event.schema_version != EVENT_SCHEMA_VERSION {
                        debug!(
                            workflow_id = %workflow_id,
                            event_id = event.event_id,
                            schema_version = event.schema_version,
                            current_version = EVENT_SCHEMA_VERSION,
                            "migrating event from different schema version"
                        );
                    }

                    // Upcast event to current schema version
                    match upcast_event(event) {
                        Ok(migrated_event) => {
                            events.push(migrated_event);
                        }
                        Err(e) => {
                            warn!(
                                key = %entry.key,
                                error = %e,
                                "failed to upcast event, skipping"
                            );
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        key = %entry.key,
                        error = %e,
                        "failed to deserialize event, skipping"
                    );
                }
            }
        }

        // Sort by event_id to ensure ordering
        events.sort_by_key(|e| e.event_id);

        info!(
            workflow_id = %workflow_id,
            from_event_id = ?from_event_id,
            event_count = events.len(),
            "replayed events"
        );

        Ok(events)
    }

    /// Get the latest event ID for a workflow.
    pub async fn get_latest_event_id(&self, workflow_id: &WorkflowExecutionId) -> Result<Option<u64>> {
        let events = self.replay_events(workflow_id, None).await?;
        Ok(events.last().map(|e| e.event_id))
    }

    /// Save a workflow snapshot.
    pub async fn save_snapshot(&self, snapshot: &WorkflowSnapshot) -> Result<()> {
        let key = snapshot.storage_key();
        let value = serde_json::to_string(snapshot).map_err(|e| JobError::SerializationError { source: e })?;

        self.store
            .write(WriteRequest {
                command: WriteCommand::Set { key, value },
            })
            .await
            .map_err(|e| JobError::StorageError { source: e })?;

        // Also append a SnapshotTaken event
        self.append_event(
            &snapshot.workflow_id,
            WorkflowEventType::SnapshotTaken {
                snapshot_id: snapshot.snapshot_id,
                at_event_id: snapshot.at_event_id,
            },
            Some(snapshot.at_event_id),
        )
        .await?;

        info!(
            workflow_id = %snapshot.workflow_id,
            snapshot_id = snapshot.snapshot_id,
            at_event_id = snapshot.at_event_id,
            "snapshot saved"
        );

        Ok(())
    }

    /// Load the latest snapshot for a workflow.
    pub async fn load_latest_snapshot(&self, workflow_id: &WorkflowExecutionId) -> Result<Option<WorkflowSnapshot>> {
        let prefix = format!("{}{}", SNAPSHOT_KEY_PREFIX, workflow_id);

        let scan_result = self
            .store
            .scan(ScanRequest {
                prefix,
                limit: Some(100), // Snapshots should be infrequent
                continuation_token: None,
            })
            .await
            .map_err(|e| JobError::StorageError { source: e })?;

        let mut snapshots: Vec<WorkflowSnapshot> =
            scan_result.entries.iter().filter_map(|entry| serde_json::from_str(&entry.value).ok()).collect();

        // Get the latest snapshot by snapshot_id
        snapshots.sort_by_key(|s| s.snapshot_id);

        Ok(snapshots.pop())
    }

    /// Check if a workflow should take a snapshot.
    pub fn should_snapshot(&self, current_event_id: u64, last_snapshot_event_id: Option<u64>) -> bool {
        let events_since_snapshot = match last_snapshot_event_id {
            Some(snap_id) => current_event_id.saturating_sub(snap_id),
            None => current_event_id,
        };
        events_since_snapshot >= MAX_EVENTS_BEFORE_SNAPSHOT
    }

    /// Get cached activity result from event history.
    ///
    /// This implements activity memoization - during replay,
    /// completed activities return their cached result instead
    /// of re-executing.
    pub async fn get_cached_activity_result(
        &self,
        workflow_id: &WorkflowExecutionId,
        activity_id: &str,
    ) -> Result<Option<CachedActivityResult>> {
        let events = self.replay_events(workflow_id, None).await?;

        // Find ActivityCompleted event for this activity
        for event in events.iter().rev() {
            if let WorkflowEventType::ActivityCompleted {
                activity_id: ref aid,
                ref result,
                duration_ms,
            } = event.event_type
            {
                if aid == activity_id {
                    return Ok(Some(CachedActivityResult {
                        activity_id: activity_id.to_string(),
                        result: result.clone(),
                        cached_at: event.timestamp,
                        duration_ms,
                    }));
                }
            }
        }

        Ok(None)
    }

    /// Get all workflow executions (for administrative purposes).
    ///
    /// Note: This scans all events, so use sparingly.
    pub async fn list_workflow_executions(&self, limit: u32) -> Result<Vec<WorkflowExecutionId>> {
        let scan_result = self
            .store
            .scan(ScanRequest {
                prefix: EVENT_KEY_PREFIX.to_string(),
                limit: Some(limit),
                continuation_token: None,
            })
            .await
            .map_err(|e| JobError::StorageError { source: e })?;

        let mut workflow_ids = std::collections::HashSet::new();

        for entry in scan_result.entries {
            // Extract workflow_id from key: __wf_events::{workflow_id}::{event_id}
            if let Some(rest) = entry.key.strip_prefix(EVENT_KEY_PREFIX) {
                if let Some(wf_id) = rest.split("::").next() {
                    workflow_ids.insert(WorkflowExecutionId::from_string(wf_id.to_string()));
                }
            }
        }

        Ok(workflow_ids.into_iter().collect())
    }
}

/// Replay engine for deterministic workflow execution.
///
/// This engine replays events and returns memoized results
/// for activities, timers, and side effects, ensuring
/// deterministic execution during recovery.
pub struct WorkflowReplayEngine {
    /// Events being replayed.
    events: Vec<WorkflowEvent>,
    /// Current replay position.
    replay_index: usize,
    /// Whether we're in replay mode.
    is_replaying: bool,
    /// Cached activity results (activity_id -> result).
    activity_cache: HashMap<String, serde_json::Value>,
    /// Cached side effects (seq -> value).
    side_effect_cache: HashMap<u64, serde_json::Value>,
    /// Side effect sequence counter.
    side_effect_seq: u64,
}

impl WorkflowReplayEngine {
    /// Create a new replay engine from event history.
    pub fn new(events: Vec<WorkflowEvent>) -> Self {
        // Pre-populate caches from events
        let mut activity_cache = HashMap::new();
        let mut side_effect_cache = HashMap::new();

        for event in &events {
            match &event.event_type {
                WorkflowEventType::ActivityCompleted {
                    activity_id, result, ..
                } => {
                    activity_cache.insert(activity_id.clone(), result.clone());
                }
                WorkflowEventType::SideEffectRecorded { seq, value } => {
                    side_effect_cache.insert(*seq, value.clone());
                }
                _ => {}
            }
        }

        Self {
            events,
            replay_index: 0,
            is_replaying: true,
            activity_cache,
            side_effect_cache,
            side_effect_seq: 0,
        }
    }

    /// Check if we're still replaying.
    pub fn is_replaying(&self) -> bool {
        self.is_replaying && self.replay_index < self.events.len()
    }

    /// Get cached activity result during replay.
    pub fn get_activity_result(&self, activity_id: &str) -> Option<&serde_json::Value> {
        if self.is_replaying {
            self.activity_cache.get(activity_id)
        } else {
            None
        }
    }

    /// Get cached side effect during replay.
    pub fn get_side_effect(&mut self) -> Option<serde_json::Value> {
        if self.is_replaying {
            let seq = self.side_effect_seq;
            self.side_effect_seq += 1;
            self.side_effect_cache.get(&seq).cloned()
        } else {
            None
        }
    }

    /// Record a new side effect (during non-replay execution).
    pub fn record_side_effect(&mut self, value: serde_json::Value) -> u64 {
        let seq = self.side_effect_seq;
        self.side_effect_seq += 1;
        self.side_effect_cache.insert(seq, value);
        seq
    }

    /// Advance replay position.
    pub fn advance(&mut self) {
        if self.replay_index < self.events.len() {
            self.replay_index += 1;
        }
        if self.replay_index >= self.events.len() {
            self.is_replaying = false;
        }
    }

    /// Get current event during replay.
    pub fn current_event(&self) -> Option<&WorkflowEvent> {
        self.events.get(self.replay_index)
    }

    /// Mark replay as complete.
    pub fn finish_replay(&mut self) {
        self.is_replaying = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_execution_id() {
        let id = WorkflowExecutionId::new();
        assert!(!id.as_str().is_empty());

        let id2 = WorkflowExecutionId::from_string("test-123".to_string());
        assert_eq!(id2.as_str(), "test-123");
    }

    #[test]
    fn test_event_storage_key() {
        let workflow_id = WorkflowExecutionId::from_string("wf-123".to_string());
        let hlc = create_hlc("test-node");
        let event = WorkflowEvent::new(
            42,
            workflow_id.clone(),
            WorkflowEventType::WorkflowStarted {
                workflow_type: "test".to_string(),
                input: serde_json::json!({}),
                parent_workflow_id: None,
            },
            &hlc,
            Some(41),
        );

        let key = event.storage_key();
        assert!(key.starts_with("__wf_events::wf-123::"));
        assert!(key.contains("00000000000000000042"));
    }

    #[test]
    fn test_snapshot_storage_key() {
        let workflow_id = WorkflowExecutionId::from_string("wf-456".to_string());
        let hlc = create_hlc("test-node");
        let snapshot = WorkflowSnapshot {
            schema_version: 1,
            snapshot_id: 5,
            workflow_id: workflow_id.clone(),
            at_event_id: 1000,
            hlc_timestamp: SerializableTimestamp::from(hlc.new_timestamp()),
            timestamp: Utc::now(),
            state: serde_json::json!({"counter": 42}),
            pending_activities: HashMap::new(),
            pending_timers: HashMap::new(),
            compensation_stack: Vec::new(),
            side_effect_seq: 10,
        };

        let key = snapshot.storage_key();
        assert!(key.starts_with("__wf_snapshots::wf-456::"));
        assert!(key.contains("00000000000000000005"));
    }

    #[test]
    fn test_replay_engine_memoization() {
        let workflow_id = WorkflowExecutionId::new();
        let hlc = create_hlc("test-node");

        let events = vec![
            WorkflowEvent::new(
                0,
                workflow_id.clone(),
                WorkflowEventType::WorkflowStarted {
                    workflow_type: "test".to_string(),
                    input: serde_json::json!({}),
                    parent_workflow_id: None,
                },
                &hlc,
                None,
            ),
            WorkflowEvent::new(
                1,
                workflow_id.clone(),
                WorkflowEventType::ActivityScheduled {
                    activity_id: "act-1".to_string(),
                    activity_type: "fetch_data".to_string(),
                    input: serde_json::json!({"url": "http://example.com"}),
                    timeout_ms: Some(30000),
                },
                &hlc,
                Some(0),
            ),
            WorkflowEvent::new(
                2,
                workflow_id.clone(),
                WorkflowEventType::ActivityCompleted {
                    activity_id: "act-1".to_string(),
                    result: serde_json::json!({"data": "cached_result"}),
                    duration_ms: 150,
                },
                &hlc,
                Some(1),
            ),
            WorkflowEvent::new(
                3,
                workflow_id.clone(),
                WorkflowEventType::SideEffectRecorded {
                    seq: 0,
                    value: serde_json::json!("uuid-12345"),
                },
                &hlc,
                Some(2),
            ),
        ];

        let mut engine = WorkflowReplayEngine::new(events);

        // Should have cached activity result
        let cached = engine.get_activity_result("act-1");
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), &serde_json::json!({"data": "cached_result"}));

        // Should have cached side effect
        let side_effect = engine.get_side_effect();
        assert!(side_effect.is_some());
        assert_eq!(side_effect.unwrap(), serde_json::json!("uuid-12345"));

        // Is replaying should be true initially
        assert!(engine.is_replaying());
    }

    #[test]
    fn test_should_snapshot() {
        let store = aspen_core::inmemory::DeterministicKeyValueStore::new();
        let event_store = WorkflowEventStore::new(Arc::new(store), "test-node".to_string());

        // No previous snapshot, just started
        assert!(!event_store.should_snapshot(100, None));
        assert!(event_store.should_snapshot(1000, None));
        assert!(event_store.should_snapshot(1500, None));

        // With previous snapshot
        assert!(!event_store.should_snapshot(1100, Some(1000)));
        assert!(event_store.should_snapshot(2000, Some(1000)));
    }
}
