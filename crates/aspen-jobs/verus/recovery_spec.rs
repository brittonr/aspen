use vstd::prelude::*;

verus! {
    // ========================================================================
    // Status Constants
    // ========================================================================

    pub const STATUS_RUNNING: u8 = 0;
    pub const STATUS_REPLAYING: u8 = 1;
    pub const STATUS_COMPLETED: u8 = 2;
    pub const STATUS_FAILED: u8 = 3;
    pub const STATUS_CANCELLED: u8 = 4;
    pub const STATUS_COMPENSATING: u8 = 5;

    // ========================================================================
    // Spec Functions (mathematical definitions)
    // ========================================================================

    /// A status is terminal if the workflow has reached a final state.
    pub open spec fn spec_is_terminal(status: u8) -> bool {
        status == STATUS_COMPLETED
        || status == STATUS_FAILED
        || status == STATUS_CANCELLED
    }

    /// A workflow needs recovery if it is non-terminal, has events,
    /// and is not already running.
    pub open spec fn spec_needs_recovery(
        status: u8,
        event_count: u64,
        is_already_running: bool,
    ) -> bool {
        !is_already_running
        && !spec_is_terminal(status)
        && event_count > 0
    }

    /// A workflow can resume if it is non-terminal, has events,
    /// and the event count is below the max.
    pub open spec fn spec_can_resume(
        status: u8,
        event_count: u64,
        max_events: u64,
    ) -> bool {
        !spec_is_terminal(status)
        && event_count > 0
        && event_count < max_events
    }

    // ========================================================================
    // Exec Functions (verified implementations)
    // ========================================================================

    /// Check if a workflow status is terminal.
    pub fn is_terminal_status(status: u8) -> (result: bool)
        ensures result == spec_is_terminal(status)
    {
        status == STATUS_COMPLETED
        || status == STATUS_FAILED
        || status == STATUS_CANCELLED
    }

    /// Check if a workflow needs recovery after leadership change.
    pub fn needs_recovery(
        status: u8,
        event_count: u64,
        is_already_running: bool,
    ) -> (result: bool)
        ensures result == spec_needs_recovery(status, event_count, is_already_running)
    {
        if is_already_running {
            return false;
        }
        if is_terminal_status(status) {
            return false;
        }
        event_count > 0
    }

    /// Check if a recovered workflow can resume execution.
    pub fn can_resume(
        status: u8,
        event_count: u64,
        max_events: u64,
    ) -> (result: bool)
        ensures result == spec_can_resume(status, event_count, max_events)
    {
        if is_terminal_status(status) {
            return false;
        }
        if event_count == 0 {
            return false;
        }
        event_count < max_events
    }

    /// Compute replay start point from snapshot.
    /// Returns snapshot_event_id + 1, or 0 if no snapshot.
    /// Saturates at u64::MAX.
    pub fn compute_replay_start(last_snapshot_event_id: Option<u64>) -> (result: u64)
        ensures
            last_snapshot_event_id.is_none() ==> result == 0,
            last_snapshot_event_id.is_some() ==>
                if last_snapshot_event_id.unwrap() < u64::MAX {
                    result == last_snapshot_event_id.unwrap() + 1
                } else {
                    result == u64::MAX
                }
    {
        match last_snapshot_event_id {
            Some(snap_id) => {
                if snap_id < u64::MAX {
                    snap_id + 1
                } else {
                    u64::MAX
                }
            }
            None => 0,
        }
    }

    /// Determine if automatic snapshot should be taken.
    pub fn should_take_snapshot(
        current_event_id: u64,
        last_snapshot_event_id: Option<u64>,
        snapshot_threshold: u64,
    ) -> (result: bool)
        ensures
            snapshot_threshold == 0 ==> result == false,
    {
        if snapshot_threshold == 0 {
            return false;
        }
        let events_since: u64 = match last_snapshot_event_id {
            Some(snap_id) => {
                if current_event_id >= snap_id {
                    current_event_id - snap_id
                } else {
                    0
                }
            }
            None => current_event_id,
        };
        events_since >= snapshot_threshold
    }

    // ========================================================================
    // Invariants
    // ========================================================================

    /// REC-1: Terminal status is idempotent — checking twice gives the same result.
    pub open spec fn terminal_idempotent(status: u8) -> bool {
        spec_is_terminal(status) == spec_is_terminal(status)
    }

    /// REC-2: A terminal workflow never needs recovery.
    pub open spec fn terminal_no_recovery(
        status: u8,
        event_count: u64,
        is_already_running: bool,
    ) -> bool {
        spec_is_terminal(status) ==> !spec_needs_recovery(status, event_count, is_already_running)
    }

    /// REC-3: A terminal workflow cannot resume.
    pub open spec fn terminal_no_resume(
        status: u8,
        event_count: u64,
        max_events: u64,
    ) -> bool {
        spec_is_terminal(status) ==> !spec_can_resume(status, event_count, max_events)
    }

    /// REC-4: An already-running workflow never needs recovery.
    pub open spec fn running_no_recovery(
        status: u8,
        event_count: u64,
    ) -> bool {
        !spec_needs_recovery(status, event_count, true)
    }

    // ========================================================================
    // Proofs
    // ========================================================================

    /// Proof: terminal workflows never need recovery.
    proof fn terminal_never_recovers(status: u8, event_count: u64, is_already_running: bool)
        ensures terminal_no_recovery(status, event_count, is_already_running)
    {
    }

    /// Proof: terminal workflows cannot resume.
    proof fn terminal_never_resumes(status: u8, event_count: u64, max_events: u64)
        ensures terminal_no_resume(status, event_count, max_events)
    {
    }

    /// Proof: already-running workflows don't need recovery.
    proof fn already_running_no_recovery(status: u8, event_count: u64)
        ensures running_no_recovery(status, event_count)
    {
    }
}
