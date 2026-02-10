//! Pure election computation functions.
//!
//! This module contains pure functions for leader election state transitions.
//! All functions are deterministic and side-effect free.
//!
//! # Tiger Style
//!
//! - Explicit state machine transitions
//! - Deterministic behavior for testing and verification

use crate::election::LeadershipState;
use crate::types::FencingToken;

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
        assert!(result >= 1000 && result <= 1200);
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
