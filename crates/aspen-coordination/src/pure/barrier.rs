//! Pure barrier computation functions.
//!
//! This module contains pure functions for distributed barrier operations.
//! All functions are deterministic and side-effect free.
//!
//! # Tiger Style
//!
//! - Uses explicit types (u32 for counts)
//! - Deterministic behavior for testing and verification

use crate::barrier::BarrierPhase;

/// Compute the initial barrier phase based on required participant count.
///
/// If only one participant is required, the barrier is immediately ready.
/// Otherwise, it starts in the waiting phase.
///
/// # Arguments
///
/// * `required_count` - Number of participants required
///
/// # Returns
///
/// The initial `BarrierPhase`.
///
/// # Example
///
/// ```ignore
/// assert_eq!(compute_initial_barrier_phase(1), BarrierPhase::Ready);
/// assert_eq!(compute_initial_barrier_phase(3), BarrierPhase::Waiting);
/// ```
#[inline]
pub fn compute_initial_barrier_phase(required_count: u32) -> BarrierPhase {
    if required_count <= 1 {
        BarrierPhase::Ready
    } else {
        BarrierPhase::Waiting
    }
}

/// Check if a barrier is ready (all participants have arrived).
///
/// # Arguments
///
/// * `participant_count` - Current number of participants
/// * `required_count` - Number of participants required
///
/// # Returns
///
/// `true` if the barrier is ready.
///
/// # Example
///
/// ```ignore
/// assert!(is_barrier_ready(3, 3));
/// assert!(is_barrier_ready(5, 3)); // More than required is ok
/// assert!(!is_barrier_ready(2, 3));
/// ```
#[inline]
pub fn is_barrier_ready(participant_count: u32, required_count: u32) -> bool {
    participant_count >= required_count
}

/// Determine if a barrier should transition to the ready phase.
///
/// The barrier transitions to ready when the participant count
/// reaches or exceeds the required count.
///
/// # Arguments
///
/// * `participant_count` - Current number of participants (after adding new one)
/// * `required_count` - Number of participants required
///
/// # Returns
///
/// `true` if the barrier should transition to ready.
///
/// # Note
///
/// This is effectively the same as `is_barrier_ready` but named for
/// clarity in state transition contexts.
#[inline]
pub fn should_transition_to_ready(participant_count: u32, required_count: u32) -> bool {
    is_barrier_ready(participant_count, required_count)
}

// ============================================================================
// Leave Phase Logic
// ============================================================================

/// Determine if a barrier should start the leave phase.
///
/// The leave phase starts when all participants have signaled readiness
/// and are ready to leave the barrier.
///
/// # Arguments
///
/// * `phase` - Current barrier phase
/// * `leave_count` - Number of participants that have signaled to leave
/// * `required_count` - Total participants required
///
/// # Returns
///
/// `true` if the barrier should start the leave phase.
#[inline]
pub fn should_start_leave_phase(phase: BarrierPhase, leave_count: u32, required_count: u32) -> bool {
    phase == BarrierPhase::Ready && leave_count >= required_count
}

/// Validate a phase transition for a participant.
///
/// Ensures that phase transitions follow the valid state machine:
/// - Waiting -> Ready (when all arrive)
/// - Ready -> Leaving (when all signal leave)
///
/// # Arguments
///
/// * `old_phase` - The previous phase
/// * `new_phase` - The proposed new phase
///
/// # Returns
///
/// `true` if the transition is valid.
#[inline]
pub fn is_valid_phase_transition(old_phase: BarrierPhase, new_phase: BarrierPhase) -> bool {
    match (old_phase, new_phase) {
        // Valid transitions
        (BarrierPhase::Waiting, BarrierPhase::Waiting) => true,
        (BarrierPhase::Waiting, BarrierPhase::Ready) => true,
        (BarrierPhase::Ready, BarrierPhase::Ready) => true,
        (BarrierPhase::Ready, BarrierPhase::Leaving) => true,
        (BarrierPhase::Leaving, BarrierPhase::Leaving) => true,
        // Same phase is always valid
        (a, b) if a == b => true,
        // Invalid transitions
        _ => false,
    }
}

// ============================================================================
// Deadlock Detection
// ============================================================================

/// Information about a participant for deadlock detection.
#[derive(Debug, Clone)]
pub struct ParticipantActivity {
    /// Participant ID.
    pub participant_id: String,
    /// Last activity timestamp (Unix ms).
    pub last_activity_ms: u64,
}

/// Result of deadlock detection check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeadlockCheckResult {
    /// No deadlock detected, barrier is healthy.
    Healthy,
    /// Potential deadlock: some participants appear stalled.
    PotentialDeadlock {
        /// IDs of participants that appear stalled.
        stalled_participants: Vec<String>,
    },
    /// Definite deadlock: barrier cannot make progress.
    Deadlock {
        /// Reason for deadlock.
        reason: String,
    },
}

/// Detect stalled participants in a barrier.
///
/// A participant is considered stalled if they haven't shown activity
/// within the timeout period.
///
/// # Arguments
///
/// * `participants` - List of participant activity records
/// * `now_ms` - Current time in Unix milliseconds
/// * `stall_timeout_ms` - Time after which a participant is considered stalled
///
/// # Returns
///
/// List of stalled participant IDs.
#[inline]
pub fn detect_stalled_participants(
    participants: &[ParticipantActivity],
    now_ms: u64,
    stall_timeout_ms: u64,
) -> Vec<String> {
    participants
        .iter()
        .filter(|p| now_ms.saturating_sub(p.last_activity_ms) > stall_timeout_ms)
        .map(|p| p.participant_id.clone())
        .collect()
}

/// Check for deadlock conditions in a barrier.
///
/// # Arguments
///
/// * `phase` - Current barrier phase
/// * `participant_count` - Number of participants that have joined
/// * `required_count` - Required number of participants
/// * `participants` - Activity records for participants
/// * `now_ms` - Current time
/// * `stall_timeout_ms` - Timeout for stalled participant detection
///
/// # Returns
///
/// Deadlock check result.
#[inline]
pub fn check_barrier_deadlock(
    phase: BarrierPhase,
    participant_count: u32,
    required_count: u32,
    participants: &[ParticipantActivity],
    now_ms: u64,
    stall_timeout_ms: u64,
) -> DeadlockCheckResult {
    // If we're in the leaving phase, check for stalled leavers
    if phase == BarrierPhase::Leaving {
        let stalled = detect_stalled_participants(participants, now_ms, stall_timeout_ms);
        if !stalled.is_empty() {
            return DeadlockCheckResult::PotentialDeadlock {
                stalled_participants: stalled,
            };
        }
        return DeadlockCheckResult::Healthy;
    }

    // If waiting and not enough participants have joined
    if phase == BarrierPhase::Waiting {
        if participant_count < required_count {
            let stalled = detect_stalled_participants(participants, now_ms, stall_timeout_ms);
            if !stalled.is_empty() {
                return DeadlockCheckResult::PotentialDeadlock {
                    stalled_participants: stalled,
                };
            }
        }
        return DeadlockCheckResult::Healthy;
    }

    DeadlockCheckResult::Healthy
}

/// Compute the expected completion time for a barrier.
///
/// # Arguments
///
/// * `creation_time_ms` - When the barrier was created
/// * `expected_participant_interval_ms` - Expected time between participant arrivals
/// * `required_count` - Number of participants required
///
/// # Returns
///
/// Expected completion time in Unix milliseconds.
#[inline]
pub fn compute_expected_completion_time(
    creation_time_ms: u64,
    expected_participant_interval_ms: u64,
    required_count: u32,
) -> u64 {
    let total_wait = expected_participant_interval_ms.saturating_mul(required_count as u64);
    creation_time_ms.saturating_add(total_wait)
}

/// Check if a barrier has exceeded its expected completion time.
///
/// # Arguments
///
/// * `expected_completion_ms` - Expected completion time
/// * `now_ms` - Current time
/// * `grace_period_ms` - Grace period before considering overdue
///
/// # Returns
///
/// `true` if the barrier is overdue.
#[inline]
pub fn is_barrier_overdue(expected_completion_ms: u64, now_ms: u64, grace_period_ms: u64) -> bool {
    now_ms > expected_completion_ms.saturating_add(grace_period_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_phase_single_participant() {
        assert_eq!(compute_initial_barrier_phase(1), BarrierPhase::Ready);
    }

    #[test]
    fn test_initial_phase_zero_participants() {
        // Edge case: 0 required count means immediately ready
        assert_eq!(compute_initial_barrier_phase(0), BarrierPhase::Ready);
    }

    #[test]
    fn test_initial_phase_multiple_participants() {
        assert_eq!(compute_initial_barrier_phase(2), BarrierPhase::Waiting);
        assert_eq!(compute_initial_barrier_phase(10), BarrierPhase::Waiting);
    }

    #[test]
    fn test_barrier_ready_exact() {
        assert!(is_barrier_ready(3, 3));
    }

    #[test]
    fn test_barrier_ready_excess() {
        assert!(is_barrier_ready(5, 3));
    }

    #[test]
    fn test_barrier_not_ready() {
        assert!(!is_barrier_ready(2, 3));
        assert!(!is_barrier_ready(0, 3));
    }

    #[test]
    fn test_barrier_ready_zero_required() {
        // 0 required means always ready
        assert!(is_barrier_ready(0, 0));
        assert!(is_barrier_ready(1, 0));
    }

    #[test]
    fn test_should_transition() {
        assert!(should_transition_to_ready(3, 3));
        assert!(!should_transition_to_ready(2, 3));
    }

    // ========================================================================
    // Leave Phase Tests
    // ========================================================================

    #[test]
    fn test_should_start_leave_phase() {
        assert!(should_start_leave_phase(BarrierPhase::Ready, 3, 3));
        assert!(should_start_leave_phase(BarrierPhase::Ready, 5, 3));
        assert!(!should_start_leave_phase(BarrierPhase::Ready, 2, 3));
        assert!(!should_start_leave_phase(BarrierPhase::Waiting, 3, 3));
    }

    #[test]
    fn test_is_valid_phase_transition() {
        // Valid transitions
        assert!(is_valid_phase_transition(BarrierPhase::Waiting, BarrierPhase::Ready));
        assert!(is_valid_phase_transition(BarrierPhase::Ready, BarrierPhase::Leaving));
        assert!(is_valid_phase_transition(BarrierPhase::Waiting, BarrierPhase::Waiting));
        assert!(is_valid_phase_transition(BarrierPhase::Ready, BarrierPhase::Ready));

        // Invalid transitions
        assert!(!is_valid_phase_transition(BarrierPhase::Leaving, BarrierPhase::Waiting));
        assert!(!is_valid_phase_transition(BarrierPhase::Leaving, BarrierPhase::Ready));
        assert!(!is_valid_phase_transition(BarrierPhase::Waiting, BarrierPhase::Leaving));
    }

    // ========================================================================
    // Deadlock Detection Tests
    // ========================================================================

    #[test]
    fn test_detect_stalled_participants_none() {
        let participants = vec![
            ParticipantActivity { participant_id: "p1".to_string(), last_activity_ms: 900 },
            ParticipantActivity { participant_id: "p2".to_string(), last_activity_ms: 950 },
        ];
        let stalled = detect_stalled_participants(&participants, 1000, 200);
        assert!(stalled.is_empty());
    }

    #[test]
    fn test_detect_stalled_participants_some() {
        let participants = vec![
            ParticipantActivity { participant_id: "p1".to_string(), last_activity_ms: 500 },
            ParticipantActivity { participant_id: "p2".to_string(), last_activity_ms: 950 },
        ];
        let stalled = detect_stalled_participants(&participants, 1000, 200);
        assert_eq!(stalled, vec!["p1".to_string()]);
    }

    #[test]
    fn test_check_barrier_deadlock_healthy() {
        let participants = vec![
            ParticipantActivity { participant_id: "p1".to_string(), last_activity_ms: 950 },
        ];
        let result = check_barrier_deadlock(
            BarrierPhase::Waiting,
            1,
            3,
            &participants,
            1000,
            200,
        );
        assert_eq!(result, DeadlockCheckResult::Healthy);
    }

    #[test]
    fn test_check_barrier_deadlock_potential() {
        let participants = vec![
            ParticipantActivity { participant_id: "p1".to_string(), last_activity_ms: 500 },
        ];
        let result = check_barrier_deadlock(
            BarrierPhase::Waiting,
            1,
            3,
            &participants,
            1000,
            200,
        );
        assert!(matches!(result, DeadlockCheckResult::PotentialDeadlock { .. }));
    }

    #[test]
    fn test_compute_expected_completion_time() {
        let expected = compute_expected_completion_time(1000, 100, 5);
        assert_eq!(expected, 1500); // 1000 + (100 * 5)
    }

    #[test]
    fn test_is_barrier_overdue() {
        assert!(is_barrier_overdue(1000, 1500, 100)); // 1500 > 1000 + 100
        assert!(!is_barrier_overdue(1000, 1050, 100)); // 1050 <= 1100
        assert!(!is_barrier_overdue(1000, 1100, 100)); // Exactly at deadline
    }
}

#[cfg(all(test, feature = "bolero"))]
mod property_tests {
    use super::*;
    use bolero::check;

    #[test]
    fn prop_initial_phase_deterministic() {
        check!().with_type::<u32>().for_each(|required| {
            let phase1 = compute_initial_barrier_phase(*required);
            let phase2 = compute_initial_barrier_phase(*required);
            assert_eq!(phase1, phase2, "Initial phase must be deterministic");
        });
    }

    #[test]
    fn prop_ready_monotonic() {
        check!()
            .with_type::<(u32, u32, u32)>()
            .for_each(|(count, extra, required)| {
                // If ready with count participants, should still be ready with more
                if is_barrier_ready(*count, *required) {
                    let more = count.saturating_add(*extra);
                    assert!(
                        is_barrier_ready(more, *required),
                        "Adding participants should not make barrier unready"
                    );
                }
            });
    }

    #[test]
    fn prop_initial_phase_matches_ready() {
        check!().with_type::<u32>().for_each(|required| {
            let phase = compute_initial_barrier_phase(*required);
            // With 1 participant (the creator), should match ready check
            let is_ready = is_barrier_ready(1, *required);
            match phase {
                BarrierPhase::Ready => assert!(is_ready || *required <= 1),
                BarrierPhase::Waiting => assert!(!is_ready || *required <= 1),
                BarrierPhase::Leaving => unreachable!("Initial phase cannot be Leaving"),
            }
        });
    }
}
