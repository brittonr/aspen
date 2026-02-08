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
