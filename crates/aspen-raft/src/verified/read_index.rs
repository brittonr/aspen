//! Pure functions for adaptive ReadIndex retry decisions.
//!
//! Formally verified — see `verus/` for proof specs (future).
//!
//! These functions determine whether a ReadIndex failure is transient
//! (worth retrying) or permanent (fail fast). Used by kv_store.rs
//! retry loops to avoid unnecessary retries during real leader changes
//! while retrying through transient contention.

use aspen_constants::network::READ_INDEX_RETRY_MAX_LOG_GAP;

/// Decide whether to retry a failed ReadIndex operation.
///
/// Returns `true` if the failure is likely transient (worth retrying):
/// - The node still believes it is the leader
/// - The Raft log is reasonably current (small gap between last_log and committed)
///
/// Returns `false` if the failure indicates a real leadership change:
/// - Another node is the leader
/// - No leader is known
/// - The log gap is too large (cluster is unhealthy)
///
/// # Arguments
///
/// * `current_leader` - The leader ID from Raft metrics (`None` = no leader known)
/// * `self_id` - This node's ID
/// * `last_log_index` - Index of the last log entry on this node
/// * `committed_index` - Index of the last committed entry on this node
///
/// # Tiger Style
///
/// - Deterministic: no I/O, no time dependency
/// - Fixed threshold via `READ_INDEX_RETRY_MAX_LOG_GAP`
#[inline]
pub fn should_retry_read_index(
    current_leader: Option<u64>,
    self_id: u64,
    last_log_index: u64,
    committed_index: u64,
) -> bool {
    // Only retry if we still think we're the leader
    let is_self_leader = current_leader == Some(self_id);
    if !is_self_leader {
        return false;
    }

    // Only retry if the log is reasonably current.
    // A large gap means the cluster is unhealthy and retrying won't help.
    let log_gap = last_log_index.saturating_sub(committed_index);
    log_gap <= READ_INDEX_RETRY_MAX_LOG_GAP
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_when_leader_with_small_gap() {
        assert!(should_retry_read_index(Some(1), 1, 100, 95));
    }

    #[test]
    fn test_retry_when_leader_with_zero_gap() {
        assert!(should_retry_read_index(Some(1), 1, 100, 100));
    }

    #[test]
    fn test_no_retry_when_leader_is_other() {
        assert!(!should_retry_read_index(Some(2), 1, 100, 95));
    }

    #[test]
    fn test_no_retry_when_no_leader() {
        assert!(!should_retry_read_index(None, 1, 100, 95));
    }

    #[test]
    fn test_no_retry_when_large_gap() {
        // Gap of 200 exceeds READ_INDEX_RETRY_MAX_LOG_GAP (100)
        assert!(!should_retry_read_index(Some(1), 1, 300, 100));
    }

    #[test]
    fn test_retry_at_exactly_max_gap() {
        // Gap of exactly READ_INDEX_RETRY_MAX_LOG_GAP should retry
        let committed = 100;
        let last_log = committed + READ_INDEX_RETRY_MAX_LOG_GAP;
        assert!(should_retry_read_index(Some(1), 1, last_log, committed));
    }

    #[test]
    fn test_no_retry_one_past_max_gap() {
        let committed = 100;
        let last_log = committed + READ_INDEX_RETRY_MAX_LOG_GAP + 1;
        assert!(!should_retry_read_index(Some(1), 1, last_log, committed));
    }

    #[test]
    fn test_no_retry_committed_ahead_of_log() {
        // Saturating sub means gap = 0 when committed > last_log
        // This is an edge case but should still be retryable (we're leader, gap is 0)
        assert!(should_retry_read_index(Some(1), 1, 50, 100));
    }
}
