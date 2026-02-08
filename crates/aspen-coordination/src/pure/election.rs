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
}

#[cfg(all(test, feature = "bolero"))]
mod property_tests {
    use super::*;
    use bolero::check;

    #[test]
    fn prop_state_deterministic() {
        check!()
            .with_type::<(bool, Option<u64>)>()
            .for_each(|(acquired, token_val)| {
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
            assert!(
                matches!(state, LeadershipState::Follower),
                "Not acquiring lock must result in Follower"
            );
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
