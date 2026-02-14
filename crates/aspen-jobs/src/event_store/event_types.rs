//! Workflow event type definitions.

use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

use super::types::WorkflowExecutionId;

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
