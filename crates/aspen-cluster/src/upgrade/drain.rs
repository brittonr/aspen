//! Graceful drain logic for node upgrades.
//!
//! Before replacing a binary, the node drains in-flight operations:
//! 1. Set drain flag → reject new client RPCs (return NOT_LEADER)
//! 2. Continue serving Raft replication traffic
//! 3. Wait up to DRAIN_TIMEOUT_SECS for in-flight ops to complete
//! 4. Force-proceed if timeout, logging cancelled operation count

use std::sync::Arc;
use std::time::Duration;

use tracing::info;
use tracing::warn;

use super::types::DrainState;

/// Result of a drain operation.
#[derive(Debug)]
pub struct DrainResult {
    /// Whether drain completed within the timeout.
    pub completed: bool,
    /// Number of operations that were still in-flight when drain finished.
    /// Zero if drain completed cleanly.
    pub cancelled_ops: u64,
    /// How long the drain took.
    pub elapsed: Duration,
}

/// Execute graceful drain on the given drain state.
///
/// Sets the drain flag, then waits up to `timeout` for in-flight operations
/// to complete. If timeout is reached, returns the count of cancelled ops.
///
/// Raft replication traffic is NOT affected — only client RPCs are drained.
pub async fn execute_drain(drain_state: &Arc<DrainState>, timeout: Duration) -> DrainResult {
    let start = std::time::Instant::now();

    // Set drain flag — new RPCs will be rejected after this point.
    drain_state.is_draining.store(true, std::sync::atomic::Ordering::Release);

    let in_flight = drain_state.in_flight_count();
    if in_flight == 0 {
        info!("drain complete: no in-flight operations");
        return DrainResult {
            completed: true,
            cancelled_ops: 0,
            elapsed: start.elapsed(),
        };
    }

    info!(in_flight, "draining in-flight operations");

    // Wait for in-flight ops to complete, with timeout.
    let drained = tokio::time::timeout(timeout, async {
        loop {
            if drain_state.in_flight_count() == 0 {
                break;
            }
            drain_state.drained.notified().await;
        }
    })
    .await;

    let elapsed = start.elapsed();
    let remaining = drain_state.in_flight_count();

    match drained {
        Ok(()) => {
            info!(elapsed_ms = elapsed.as_millis() as u64, "drain complete");
            DrainResult {
                completed: true,
                cancelled_ops: 0,
                elapsed,
            }
        }
        Err(_) => {
            warn!(remaining, elapsed_ms = elapsed.as_millis() as u64, "drain timeout: proceeding with upgrade");
            DrainResult {
                completed: false,
                cancelled_ops: remaining,
                elapsed,
            }
        }
    }
}

/// Reset drain state after upgrade/rollback (for testing and re-use).
pub fn reset_drain(drain_state: &DrainState) {
    drain_state.is_draining.store(false, std::sync::atomic::Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_drain_no_inflight() {
        let state = DrainState::new();
        let result = execute_drain(&state, Duration::from_secs(5)).await;
        assert!(result.completed);
        assert_eq!(result.cancelled_ops, 0);
        assert!(state.is_draining());
    }

    #[tokio::test]
    async fn test_drain_with_inflight_completes() {
        let state = DrainState::new();
        // Simulate an in-flight op.
        assert!(state.try_start_op());
        assert_eq!(state.in_flight_count(), 1);

        let state_clone = state.clone();
        // Finish the op after 50ms.
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            state_clone.finish_op();
        });

        let result = execute_drain(&state, Duration::from_secs(5)).await;
        assert!(result.completed);
        assert_eq!(result.cancelled_ops, 0);
    }

    #[tokio::test]
    async fn test_drain_timeout() {
        let state = DrainState::new();
        // Simulate an in-flight op that never finishes.
        assert!(state.try_start_op());

        let result = execute_drain(&state, Duration::from_millis(50)).await;
        assert!(!result.completed);
        assert_eq!(result.cancelled_ops, 1);
    }

    #[tokio::test]
    async fn test_drain_rejects_new_ops() {
        let state = DrainState::new();
        // Start drain.
        state.is_draining.store(true, std::sync::atomic::Ordering::Release);
        // New ops should be rejected.
        assert!(!state.try_start_op());
        assert_eq!(state.in_flight_count(), 0);
    }

    #[tokio::test]
    async fn test_drain_state_try_start_op_race() {
        // Verify the double-check pattern: start op, then drain starts.
        let state = DrainState::new();
        assert!(state.try_start_op());
        assert_eq!(state.in_flight_count(), 1);

        // Now start draining — the op is already tracked.
        state.is_draining.store(true, std::sync::atomic::Ordering::Release);

        // New ops are rejected.
        assert!(!state.try_start_op());
        // But the existing op is still tracked.
        assert_eq!(state.in_flight_count(), 1);

        // Finish the existing op.
        state.finish_op();
        assert_eq!(state.in_flight_count(), 0);
    }

    #[tokio::test]
    async fn test_drain_multiple_inflight() {
        let state = DrainState::new();
        assert!(state.try_start_op());
        assert!(state.try_start_op());
        assert!(state.try_start_op());
        assert_eq!(state.in_flight_count(), 3);

        let state_clone = state.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            state_clone.finish_op();
            tokio::time::sleep(Duration::from_millis(10)).await;
            state_clone.finish_op();
            tokio::time::sleep(Duration::from_millis(10)).await;
            state_clone.finish_op();
        });

        let result = execute_drain(&state, Duration::from_secs(5)).await;
        assert!(result.completed);
        assert_eq!(result.cancelled_ops, 0);
    }

    #[test]
    fn test_reset_drain() {
        let state = DrainState::new();
        state.is_draining.store(true, std::sync::atomic::Ordering::Release);
        assert!(state.is_draining());
        reset_drain(&state);
        assert!(!state.is_draining());
    }
}
