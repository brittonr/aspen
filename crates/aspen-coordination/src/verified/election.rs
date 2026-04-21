//! Pure election computation functions.
//!
//! This module contains pure functions for leader election state transitions.
//! All functions are deterministic and side-effect free.
//! Formally verified - see `verus/election_state_spec.rs` and `verus/election_ops_spec.rs`.
//!
//! # Tiger Style
//!
//! - Explicit state machine transitions
//! - Deterministic behavior for testing and verification

use crate::election::LeadershipState;
use crate::types::FencingToken;

// ============================================================================
// State Predicates
// ============================================================================

/// Check if state indicates leadership.
#[inline]
pub fn is_leader_exec(state: &LeadershipState) -> bool {
    matches!(state, LeadershipState::Leader { .. })
}

/// Check if state indicates follower.
#[inline]
pub fn is_follower_exec(state: &LeadershipState) -> bool {
    matches!(state, LeadershipState::Follower)
}

/// Check if state indicates transitioning.
#[inline]
pub fn is_transitioning_exec(state: &LeadershipState) -> bool {
    matches!(state, LeadershipState::Transitioning)
}

// ============================================================================
// State Transition Validation
// ============================================================================

/// Check if a state transition is valid.
///
/// Valid transitions:
/// - Follower -> Transitioning (starting election)
/// - Transitioning -> Leader (won election)
/// - Transitioning -> Follower (lost election)
/// - Leader -> Follower (stepdown or lost)
/// - Same state (no change)
#[inline]
pub fn is_valid_state_transition(from: &LeadershipState, to: &LeadershipState) -> bool {
    match (from, to) {
        (LeadershipState::Follower, LeadershipState::Transitioning) => true,
        (LeadershipState::Transitioning, LeadershipState::Leader { .. }) => true,
        (LeadershipState::Transitioning, LeadershipState::Follower) => true,
        (LeadershipState::Leader { .. }, LeadershipState::Follower) => true,
        (LeadershipState::Follower, LeadershipState::Follower) => true,
        (LeadershipState::Leader { fencing_token: t1 }, LeadershipState::Leader { fencing_token: t2 }) => {
            t1.value() == t2.value() // Token can't change while leader
        }
        (LeadershipState::Transitioning, LeadershipState::Transitioning) => true,
        _ => false,
    }
}

/// Check if leader state is well-formed.
///
/// A leader state is well-formed if:
/// - It's not a leader, OR
/// - It is a leader with a valid fencing token (> 0 and <= max)
#[inline]
pub fn is_leader_state_wellformed(is_leader: bool, fencing_token: u64, max_fencing_token: u64) -> bool {
    !is_leader || (fencing_token > 0 && fencing_token <= max_fencing_token)
}

// ============================================================================
// Election Preconditions
// ============================================================================

/// Check if election can be started.
///
/// An election can only start when:
/// - Election loop is running
/// - Currently in Follower state
#[inline]
pub fn can_start_election(state: &LeadershipState, running: bool) -> bool {
    running && matches!(state, LeadershipState::Follower)
}

/// Check if election can be won with given token.
///
/// Election can be won when:
/// - Currently transitioning
/// - Max token has room for increment
/// - New token is greater than max token
#[inline]
pub fn can_win_election(state: &LeadershipState, max_fencing_token: u64, new_token: u64) -> bool {
    let is_transitioning = matches!(state, LeadershipState::Transitioning);
    let has_token_room = max_fencing_token < u64::MAX;
    let is_token_valid = new_token > max_fencing_token;

    is_transitioning && has_token_room && is_token_valid
}

/// Check if election can be lost.
#[inline]
pub fn can_lose_election(state: &LeadershipState) -> bool {
    matches!(state, LeadershipState::Transitioning)
}

/// Check if stepdown is possible.
#[inline]
pub fn can_stepdown(state: &LeadershipState) -> bool {
    matches!(state, LeadershipState::Leader { .. })
}

/// Check if leadership can be lost.
#[inline]
pub fn can_lose_leadership(state: &LeadershipState) -> bool {
    matches!(state, LeadershipState::Leader { .. })
}

// ============================================================================
// State After Transitions
// ============================================================================

/// Get next state after starting election.
#[inline]
pub fn get_state_after_start_election() -> LeadershipState {
    LeadershipState::Transitioning
}

/// Get next state after winning election.
#[inline]
pub fn get_state_after_win_election(fencing_token: FencingToken) -> LeadershipState {
    LeadershipState::Leader { fencing_token }
}

/// Get next state after losing election.
#[inline]
pub fn get_state_after_lose_election() -> LeadershipState {
    LeadershipState::Follower
}

/// Get next state after stepdown.
#[inline]
pub fn get_state_after_stepdown() -> LeadershipState {
    LeadershipState::Follower
}

/// Get next state after losing leadership.
#[inline]
pub fn get_state_after_lose_leadership() -> LeadershipState {
    LeadershipState::Follower
}

/// Check if running should be set to false after stepdown.
///
/// Stepdown stops the election loop.
#[inline]
pub fn should_stop_running_after_stepdown() -> bool {
    false
}

// ============================================================================
// Token Computation
// ============================================================================

/// Compute next fencing token for new leadership term.
#[inline]
pub fn compute_next_election_token(current_max_token: u64) -> u64 {
    current_max_token.saturating_add(1)
}

/// Compute max token after winning election.
///
/// The max token becomes the new token.
#[inline]
pub fn compute_max_token_after_win(new_token: u64) -> u64 {
    new_token
}

// ============================================================================
// Election Timing
// ============================================================================

/// Calculate election timeout with jitter.
///
/// Adds randomized jitter to base timeout to prevent thundering herd.
#[inline]
pub fn calculate_election_timeout(base_timeout_ms: u64, jitter_seed: u64, jitter_range_ms: u64) -> u64 {
    let jitter = if jitter_range_ms == 0 {
        0
    } else {
        jitter_seed % jitter_range_ms
    };
    base_timeout_ms.saturating_add(jitter)
}

/// Check if we should start an election.
#[inline]
pub fn should_start_election(
    state: &LeadershipState,
    running: bool,
    last_heartbeat_ms: u64,
    current_time_ms: u64,
    election_timeout_ms: u64,
) -> bool {
    running
        && matches!(state, LeadershipState::Follower)
        && current_time_ms.saturating_sub(last_heartbeat_ms) >= election_timeout_ms
}

/// Check if we should step down as leader.
#[inline]
pub fn should_step_down_exec(is_leader: bool, lease_deadline_ms: u64, current_time_ms: u64) -> bool {
    is_leader && current_time_ms > lease_deadline_ms
}

/// Compute the next leadership state based on lock acquisition result.
///
/// This function determines the state transition after a lock acquisition
/// attempt in the election loop.
///
/// # Arguments
///
/// * `lock_acquired` - Whether the lock was successfully acquired
/// * `fencing_token` - The fencing token if lock was acquired
///
/// # Returns
///
/// The next `LeadershipState`.
///
/// # State Machine
///
/// ```text
/// Lock Acquired (true)  -> Leader { fencing_token }
/// Lock Not Acquired     -> Follower
/// ```
///
/// # Example
///
/// ```ignore
/// let token = FencingToken::new(5);
/// let state = compute_next_leadership_state(true, Some(token));
/// assert!(matches!(state, LeadershipState::Leader { .. }));
///
/// let state = compute_next_leadership_state(false, None);
/// assert!(matches!(state, LeadershipState::Follower));
/// ```
#[inline]
pub fn compute_next_leadership_state(lock_acquired: bool, fencing_token: Option<FencingToken>) -> LeadershipState {
    if lock_acquired {
        match fencing_token {
            Some(token) => LeadershipState::Leader { fencing_token: token },
            None => {
                // This shouldn't happen in practice - if lock is acquired,
                // we should always have a token. Default to Follower as safe fallback.
                LeadershipState::Follower
            }
        }
    } else {
        LeadershipState::Follower
    }
}

// ============================================================================
// Timing Logic
// ============================================================================

/// Compute the next leadership renewal time.
///
/// # Arguments
///
/// * `now_ms` - Current time in Unix milliseconds
/// * `renew_interval_ms` - Interval between renewals in milliseconds
///
/// # Returns
///
/// Next renewal time in Unix milliseconds.
///
/// # Tiger Style
///
/// - Uses saturating_add to prevent overflow
#[inline]
pub fn compute_next_renew_time(now_ms: u64, renew_interval_ms: u64) -> u64 {
    now_ms.saturating_add(renew_interval_ms)
}

/// Check if it's time to renew leadership.
///
/// # Arguments
///
/// * `now_ms` - Current time in Unix milliseconds
/// * `last_renewed_ms` - Last renewal time in Unix milliseconds
/// * `renew_interval_ms` - Interval between renewals in milliseconds
///
/// # Returns
///
/// `true` if enough time has passed for renewal.
#[inline]
pub fn is_renewal_time(now_ms: u64, last_renewed_ms: u64, renew_interval_ms: u64) -> bool {
    now_ms.saturating_sub(last_renewed_ms) >= renew_interval_ms
}

/// Compute backoff delay for leadership renewal after failures.
///
/// Uses exponential backoff: base_delay * 2^failures, capped at max_delay.
///
/// # Arguments
///
/// * `consecutive_failures` - Number of consecutive renewal failures
/// * `base_delay_ms` - Base delay in milliseconds
/// * `max_delay_ms` - Maximum delay cap in milliseconds
///
/// # Returns
///
/// Backoff delay in milliseconds.
///
/// # Tiger Style
///
/// - Uses saturating arithmetic
/// - Caps at max_delay to prevent unbounded growth
#[inline]
pub fn compute_renewal_backoff(consecutive_failures: u32, base_delay_ms: u64, max_delay_ms: u64) -> u64 {
    if consecutive_failures == 0 {
        return 0;
    }

    // 2^failures, capped at 10 to prevent overflow
    let exponent = consecutive_failures.min(10);
    let multiplier = 1u64 << exponent;
    let delay = base_delay_ms.saturating_mul(multiplier);

    delay.min(max_delay_ms)
}

/// Determine if leadership should be maintained based on current state.
///
/// # Arguments
///
/// * `is_running` - Whether the election service is running
/// * `is_leader` - Whether we currently hold leadership
/// * `lease_valid` - Whether our leadership lease is still valid
///
/// # Returns
///
/// `true` if we should continue maintaining leadership.
#[inline]
pub fn should_maintain_leadership(is_running: bool, is_leader: bool, lease_valid: bool) -> bool {
    is_running && is_leader && lease_valid
}

/// Compute the election attempt interval with jitter.
///
/// Jitter prevents thundering herd when multiple nodes attempt election.
///
/// # Arguments
///
/// * `base_interval_ms` - Base interval between election attempts
/// * `jitter_fraction` - Fraction of interval to use as jitter range (0.0 to 1.0)
/// * `random_value` - Random value in [0.0, 1.0] for jitter
///
/// # Returns
///
/// Interval with jitter applied.
#[inline]
pub fn compute_election_interval_with_jitter(base_interval_ms: u64, jitter_fraction: f32, random_value: f32) -> u64 {
    let jitter_range = (base_interval_ms as f32 * jitter_fraction.clamp(0.0, 1.0)) as u64;
    let jitter = (jitter_range as f32 * random_value.clamp(0.0, 1.0)) as u64;
    base_interval_ms.saturating_add(jitter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_acquired_becomes_leader() {
        let token = FencingToken::new(5);
        let state = compute_next_leadership_state(true, Some(token));

        match state {
            LeadershipState::Leader { fencing_token } => {
                assert_eq!(fencing_token.value(), 5);
            }
            _ => panic!("Expected Leader state"),
        }
    }

    #[test]
    fn test_lock_not_acquired_becomes_follower() {
        let state = compute_next_leadership_state(false, None);
        assert!(matches!(state, LeadershipState::Follower));
    }

    #[test]
    fn test_lock_acquired_without_token_becomes_follower() {
        // Edge case: lock acquired but no token (shouldn't happen in practice)
        let state = compute_next_leadership_state(true, None);
        assert!(matches!(state, LeadershipState::Follower));
    }

    #[test]
    fn test_lock_not_acquired_ignores_token() {
        // Edge case: token provided but lock not acquired
        let token = FencingToken::new(5);
        let state = compute_next_leadership_state(false, Some(token));
        assert!(matches!(state, LeadershipState::Follower));
    }

    // ========================================================================
    // Timing Logic Tests
    // ========================================================================

    #[test]
    fn test_compute_next_renew_time() {
        assert_eq!(compute_next_renew_time(1000, 5000), 6000);
        assert_eq!(compute_next_renew_time(0, 0), 0);
        assert_eq!(compute_next_renew_time(u64::MAX, 1), u64::MAX); // Saturates
    }

    #[test]
    fn test_is_renewal_time() {
        // Time to renew
        assert!(is_renewal_time(6000, 1000, 5000));
        assert!(is_renewal_time(10000, 1000, 5000));

        // Not yet time
        assert!(!is_renewal_time(4000, 1000, 5000));
        assert!(!is_renewal_time(5999, 1000, 5000));

        // Edge case: exactly at interval
        assert!(is_renewal_time(6000, 1000, 5000));
    }

    #[test]
    fn test_compute_renewal_backoff_no_failures() {
        assert_eq!(compute_renewal_backoff(0, 100, 10000), 0);
    }

    #[test]
    fn test_compute_renewal_backoff_exponential() {
        // 100 * 2^1 = 200
        assert_eq!(compute_renewal_backoff(1, 100, 10000), 200);
        // 100 * 2^2 = 400
        assert_eq!(compute_renewal_backoff(2, 100, 10000), 400);
        // 100 * 2^3 = 800
        assert_eq!(compute_renewal_backoff(3, 100, 10000), 800);
    }

    #[test]
    fn test_compute_renewal_backoff_capped() {
        // Should cap at max_delay
        assert_eq!(compute_renewal_backoff(10, 100, 5000), 5000);
        assert_eq!(compute_renewal_backoff(20, 100, 5000), 5000);
    }

    #[test]
    fn test_should_maintain_leadership() {
        assert!(should_maintain_leadership(true, true, true));
        assert!(!should_maintain_leadership(false, true, true)); // Not running
        assert!(!should_maintain_leadership(true, false, true)); // Not leader
        assert!(!should_maintain_leadership(true, true, false)); // Lease invalid
    }

    #[test]
    fn test_compute_election_interval_with_jitter() {
        // No jitter
        assert_eq!(compute_election_interval_with_jitter(1000, 0.0, 0.5), 1000);

        // Min jitter
        assert_eq!(compute_election_interval_with_jitter(1000, 0.2, 0.0), 1000);

        // Max jitter
        assert_eq!(compute_election_interval_with_jitter(1000, 0.2, 1.0), 1200);

        // Mid jitter
        let result = compute_election_interval_with_jitter(1000, 0.2, 0.5);
        assert!(result >= 1000);
        assert!(result <= 1200);
    }

    // ========================================================================
    // State Predicate Tests (targeted: mutation testing survivors)
    // ========================================================================

    #[test]
    fn test_is_leader_exec() {
        assert!(is_leader_exec(&LeadershipState::Leader {
            fencing_token: FencingToken::new(1),
        }));
        assert!(!is_leader_exec(&LeadershipState::Follower));
        assert!(!is_leader_exec(&LeadershipState::Transitioning));
    }

    #[test]
    fn test_is_follower_exec() {
        assert!(is_follower_exec(&LeadershipState::Follower));
        assert!(!is_follower_exec(&LeadershipState::Transitioning));
        assert!(!is_follower_exec(&LeadershipState::Leader {
            fencing_token: FencingToken::new(1),
        }));
    }

    #[test]
    fn test_is_transitioning_exec() {
        assert!(is_transitioning_exec(&LeadershipState::Transitioning));
        assert!(!is_transitioning_exec(&LeadershipState::Follower));
        assert!(!is_transitioning_exec(&LeadershipState::Leader {
            fencing_token: FencingToken::new(1),
        }));
    }

    // ========================================================================
    // State Transition Validation Tests (targeted: mutation testing survivors)
    // ========================================================================

    #[test]
    fn test_valid_state_transitions() {
        let t = LeadershipState::Transitioning;
        let f = LeadershipState::Follower;
        let l1 = LeadershipState::Leader {
            fencing_token: FencingToken::new(5),
        };
        let l2 = LeadershipState::Leader {
            fencing_token: FencingToken::new(5),
        };
        let l3 = LeadershipState::Leader {
            fencing_token: FencingToken::new(6),
        };

        // Valid transitions
        assert!(is_valid_state_transition(&f, &t)); // Follower -> Transitioning
        assert!(is_valid_state_transition(&t, &l1)); // Transitioning -> Leader
        assert!(is_valid_state_transition(&t, &f)); // Transitioning -> Follower
        assert!(is_valid_state_transition(&l1, &f)); // Leader -> Follower
        assert!(is_valid_state_transition(&f, &f)); // Follower -> Follower
        assert!(is_valid_state_transition(&t, &t)); // Transitioning -> Transitioning
        assert!(is_valid_state_transition(&l1, &l2)); // Leader -> Leader (same token)

        // Invalid transitions
        assert!(!is_valid_state_transition(&f, &l1)); // Follower -> Leader (skip transitioning)
        assert!(!is_valid_state_transition(&l1, &t)); // Leader -> Transitioning
        assert!(!is_valid_state_transition(&l1, &l3)); // Leader -> Leader (different token)
    }

    // ========================================================================
    // Election Precondition Tests (targeted: mutation testing survivors)
    // ========================================================================

    #[test]
    fn test_can_start_election() {
        assert!(can_start_election(&LeadershipState::Follower, true));
        assert!(!can_start_election(&LeadershipState::Follower, false));
        assert!(!can_start_election(&LeadershipState::Transitioning, true));
        assert!(!can_start_election(
            &LeadershipState::Leader {
                fencing_token: FencingToken::new(1)
            },
            true
        ));
    }

    #[test]
    fn test_can_win_election() {
        // Can win: transitioning + token room + valid new token
        assert!(can_win_election(&LeadershipState::Transitioning, 5, 6));
        // Can't win: not transitioning
        assert!(!can_win_election(&LeadershipState::Follower, 5, 6));
        // Can't win: new token not greater
        assert!(!can_win_election(&LeadershipState::Transitioning, 5, 5));
        assert!(!can_win_election(&LeadershipState::Transitioning, 5, 4));
        // Can't win: no token room
        assert!(!can_win_election(&LeadershipState::Transitioning, u64::MAX, u64::MAX));
    }

    #[test]
    fn test_can_lose_election() {
        assert!(can_lose_election(&LeadershipState::Transitioning));
        assert!(!can_lose_election(&LeadershipState::Follower));
    }

    #[test]
    fn test_can_stepdown() {
        assert!(can_stepdown(&LeadershipState::Leader {
            fencing_token: FencingToken::new(1)
        }));
        assert!(!can_stepdown(&LeadershipState::Follower));
        assert!(!can_stepdown(&LeadershipState::Transitioning));
    }

    #[test]
    fn test_can_lose_leadership() {
        assert!(can_lose_leadership(&LeadershipState::Leader {
            fencing_token: FencingToken::new(1)
        }));
        assert!(!can_lose_leadership(&LeadershipState::Follower));
    }

    // ========================================================================
    // Token Computation Tests (targeted: mutation testing survivors)
    // ========================================================================

    #[test]
    fn test_compute_next_election_token() {
        assert_eq!(compute_next_election_token(0), 1);
        assert_eq!(compute_next_election_token(5), 6);
        assert_eq!(compute_next_election_token(u64::MAX), u64::MAX); // saturating
    }

    #[test]
    fn test_compute_max_token_after_win() {
        assert_eq!(compute_max_token_after_win(7), 7);
        assert_eq!(compute_max_token_after_win(0), 0);
    }

    // ========================================================================
    // Election Timing Tests (targeted: mutation testing survivors)
    // ========================================================================

    #[test]
    fn test_calculate_election_timeout() {
        // Zero jitter range
        assert_eq!(calculate_election_timeout(1000, 42, 0), 1000);
        // Normal jitter
        assert_eq!(calculate_election_timeout(1000, 7, 10), 1007);
        // Jitter modulo
        assert_eq!(calculate_election_timeout(1000, 15, 10), 1005);
    }

    #[test]
    fn test_should_start_election() {
        // Time elapsed >= timeout, follower, running
        assert!(should_start_election(&LeadershipState::Follower, true, 0, 5000, 5000));
        // Not running
        assert!(!should_start_election(&LeadershipState::Follower, false, 0, 5000, 5000));
        // Not follower
        assert!(!should_start_election(&LeadershipState::Transitioning, true, 0, 5000, 5000));
        // Time not elapsed
        assert!(!should_start_election(&LeadershipState::Follower, true, 0, 4999, 5000));
    }

    #[test]
    fn test_should_step_down_exec() {
        // Is leader, past deadline
        assert!(should_step_down_exec(true, 1000, 1001));
        // Is leader, at deadline
        assert!(!should_step_down_exec(true, 1000, 1000));
        // Not leader
        assert!(!should_step_down_exec(false, 1000, 2000));
    }

    #[test]
    fn test_is_leader_state_wellformed() {
        // Not a leader: always well-formed
        assert!(is_leader_state_wellformed(false, 0, 0));
        // Leader with valid token
        assert!(is_leader_state_wellformed(true, 5, 10));
        assert!(is_leader_state_wellformed(true, 10, 10));
        // Leader with token = 0: not well-formed
        assert!(!is_leader_state_wellformed(true, 0, 10));
        // Leader with token > max: not well-formed
        assert!(!is_leader_state_wellformed(true, 11, 10));
    }
}

#[cfg(all(test, feature = "bolero"))]
mod property_tests {
    use bolero::check;

    use super::*;

    #[test]
    fn prop_state_deterministic() {
        check!().with_type::<(bool, Option<u64>)>().for_each(|(acquired, token_val)| {
            let token = token_val.map(FencingToken::new);
            let state1 = compute_next_leadership_state(*acquired, token);
            let state2 = compute_next_leadership_state(*acquired, token);
            assert_eq!(state1, state2, "State transition must be deterministic");
        });
    }

    #[test]
    fn prop_not_acquired_always_follower() {
        check!().with_type::<Option<u64>>().for_each(|token_val| {
            let token = token_val.map(FencingToken::new);
            let state = compute_next_leadership_state(false, token);
            assert!(matches!(state, LeadershipState::Follower), "Not acquiring lock must result in Follower");
        });
    }

    #[test]
    fn prop_acquired_with_token_is_leader() {
        check!().with_type::<u64>().for_each(|token_val| {
            let token = FencingToken::new(*token_val);
            let state = compute_next_leadership_state(true, Some(token));
            match state {
                LeadershipState::Leader { fencing_token } => {
                    assert_eq!(fencing_token.value(), *token_val);
                }
                _ => panic!("Acquiring lock with token must result in Leader"),
            }
        });
    }
}
