//! Pure functions for durable workflow recovery state classification.
//!
//! Formally verified — see `verus/recovery_spec.rs` for proofs.
//!
//! These functions determine whether a workflow needs recovery, can resume,
//! or is in a terminal state. They are deterministic and side-effect free,
//! accepting explicit state parameters rather than querying the event store.

/// Workflow status codes for recovery classification.
/// Mirrors `DurableWorkflowStatus` but as simple u8 constants for
/// verified function compatibility.
pub const STATUS_RUNNING: u8 = 0;
/// Workflow is replaying from event log.
pub const STATUS_REPLAYING: u8 = 1;
/// Workflow completed successfully.
pub const STATUS_COMPLETED: u8 = 2;
/// Workflow failed.
pub const STATUS_FAILED: u8 = 3;
/// Workflow was cancelled.
pub const STATUS_CANCELLED: u8 = 4;
/// Saga compensation is in progress.
pub const STATUS_COMPENSATING: u8 = 5;

/// Check if a workflow status represents a terminal state.
///
/// Terminal workflows do not need recovery and should not be resumed.
///
/// # Tiger Style
/// - Pure function, no I/O
/// - Exhaustive: every status code maps to a clear result
#[inline]
pub fn is_terminal_status(status: u8) -> bool {
    status == STATUS_COMPLETED || status == STATUS_FAILED || status == STATUS_CANCELLED
}

/// Check if a workflow needs recovery after a leadership change.
///
/// A workflow needs recovery if it is in a non-terminal state and
/// has at least one recorded event (the `WorkflowStarted` event).
///
/// # Arguments
/// - `status`: Current workflow status code
/// - `event_count`: Number of events in the workflow's event log
/// - `is_already_running`: Whether the workflow is already active on this executor
#[inline]
pub fn needs_recovery(status: u8, event_count: u64, is_already_running: bool) -> bool {
    if is_already_running {
        return false;
    }
    if is_terminal_status(status) {
        return false;
    }
    // Must have at least WorkflowStarted event.
    event_count > 0
}

/// Check if a recovered workflow can resume execution.
///
/// A workflow can resume if:
/// 1. It has a `WorkflowStarted` event
/// 2. It is not in a terminal state
/// 3. The event count doesn't exceed the maximum
///
/// # Arguments
/// - `status`: Current workflow status code
/// - `event_count`: Number of events in the workflow's event log
/// - `max_events`: Maximum allowed events per workflow (Temporal-style limit)
#[inline]
pub fn can_resume(status: u8, event_count: u64, max_events: u64) -> bool {
    if is_terminal_status(status) {
        return false;
    }
    if event_count == 0 {
        return false;
    }
    event_count < max_events
}

/// Compute the replay start point given a snapshot event ID.
///
/// If a snapshot exists, replay starts from the event after the snapshot.
/// If no snapshot exists (None), replay starts from event 0.
///
/// Uses saturating arithmetic for overflow safety.
#[inline]
pub fn compute_replay_start(last_snapshot_event_id: Option<u64>) -> u64 {
    match last_snapshot_event_id {
        Some(snap_id) => snap_id.saturating_add(1),
        None => 0,
    }
}

/// Determine whether an automatic snapshot should be taken.
///
/// Takes a snapshot when the number of events since the last snapshot
/// exceeds the threshold.
///
/// # Arguments
/// - `current_event_id`: ID of the most recent event
/// - `last_snapshot_event_id`: Event ID of the most recent snapshot (if any)
/// - `snapshot_threshold`: Number of events between snapshots
#[inline]
pub fn should_take_snapshot(
    current_event_id: u64,
    last_snapshot_event_id: Option<u64>,
    snapshot_threshold: u64,
) -> bool {
    if snapshot_threshold == 0 {
        return false;
    }
    let events_since = match last_snapshot_event_id {
        Some(snap_id) => current_event_id.saturating_sub(snap_id),
        None => current_event_id,
    };
    events_since >= snapshot_threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_terminal() {
        assert!(is_terminal_status(STATUS_COMPLETED));
        assert!(is_terminal_status(STATUS_FAILED));
        assert!(is_terminal_status(STATUS_CANCELLED));
        assert!(!is_terminal_status(STATUS_RUNNING));
        assert!(!is_terminal_status(STATUS_REPLAYING));
        assert!(!is_terminal_status(STATUS_COMPENSATING));
    }

    #[test]
    fn test_needs_recovery() {
        // Active workflow with events needs recovery.
        assert!(needs_recovery(STATUS_RUNNING, 5, false));
        // Terminal workflow does not.
        assert!(!needs_recovery(STATUS_COMPLETED, 5, false));
        assert!(!needs_recovery(STATUS_FAILED, 3, false));
        // Already running does not.
        assert!(!needs_recovery(STATUS_RUNNING, 5, true));
        // No events means nothing to recover.
        assert!(!needs_recovery(STATUS_RUNNING, 0, false));
        // Compensating workflow needs recovery.
        assert!(needs_recovery(STATUS_COMPENSATING, 10, false));
    }

    #[test]
    fn test_can_resume() {
        assert!(can_resume(STATUS_RUNNING, 5, 50_000));
        assert!(can_resume(STATUS_REPLAYING, 1, 50_000));
        assert!(can_resume(STATUS_COMPENSATING, 100, 50_000));
        // Terminal.
        assert!(!can_resume(STATUS_COMPLETED, 5, 50_000));
        // No events.
        assert!(!can_resume(STATUS_RUNNING, 0, 50_000));
        // At max events.
        assert!(!can_resume(STATUS_RUNNING, 50_000, 50_000));
    }

    #[test]
    fn test_compute_replay_start() {
        assert_eq!(compute_replay_start(None), 0);
        assert_eq!(compute_replay_start(Some(0)), 1);
        assert_eq!(compute_replay_start(Some(999)), 1000);
        // Saturating at u64::MAX.
        assert_eq!(compute_replay_start(Some(u64::MAX)), u64::MAX);
    }

    #[test]
    fn test_should_take_snapshot() {
        // Threshold of 1000, no previous snapshot.
        assert!(!should_take_snapshot(999, None, 1000));
        assert!(should_take_snapshot(1000, None, 1000));
        assert!(should_take_snapshot(1001, None, 1000));

        // With previous snapshot at event 1000.
        assert!(!should_take_snapshot(1999, Some(1000), 1000));
        assert!(should_take_snapshot(2000, Some(1000), 1000));

        // Zero threshold never snapshots.
        assert!(!should_take_snapshot(10_000, None, 0));
    }
}
