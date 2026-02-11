//! Pure lock computation functions.
//!
//! This module contains pure functions for distributed lock operations.
//! All functions are deterministic and side-effect free.
//!
//! # Tiger Style
//!
//! - Uses saturating arithmetic for all calculations
//! - Time is passed explicitly (no calls to system time)
//! - Deterministic behavior for testing and verification

use crate::types::LockEntry;

/// Compute the next fencing token based on current lock state.
///
/// Fencing tokens are monotonically increasing to prevent split-brain scenarios.
/// When taking over an expired or released lock, the new token must be strictly
/// greater than the previous token.
///
/// # Arguments
///
/// * `current_entry` - The current lock entry, if any exists
///
/// # Returns
///
/// The next fencing token value (always >= 1).
///
/// # Example
///
/// ```ignore
/// // New lock (no previous entry)
/// assert_eq!(compute_next_fencing_token(None), 1);
///
/// // Taking over from previous holder
/// let entry = LockEntry { fencing_token: 5, .. };
/// assert_eq!(compute_next_fencing_token(Some(&entry)), 6);
/// ```
///
/// # Tiger Style
///
/// - Uses saturating_add to prevent overflow
/// - Always returns >= 1 (never 0)
#[inline]
pub fn compute_next_fencing_token(current_entry: Option<&LockEntry>) -> u64 {
    match current_entry {
        Some(entry) => entry.fencing_token.saturating_add(1),
        None => 1,
    }
}

/// Compute lock deadline from acquisition time and TTL.
///
/// # Arguments
///
/// * `acquired_at_ms` - Unix timestamp in milliseconds when lock was acquired
/// * `ttl_ms` - Time-to-live in milliseconds
///
/// # Returns
///
/// Deadline in Unix milliseconds.
///
/// # Tiger Style
///
/// - Uses saturating_add to prevent overflow
#[inline]
pub fn compute_lock_deadline(acquired_at_ms: u64, ttl_ms: u64) -> u64 {
    acquired_at_ms.saturating_add(ttl_ms)
}

/// Check if a lock entry has expired.
///
/// A lock is expired if:
/// - Its deadline_ms is 0 (released state), OR
/// - The current time has passed the deadline
///
/// # Arguments
///
/// * `deadline_ms` - Lock deadline in Unix milliseconds (0 = released)
/// * `now_ms` - Current time in Unix milliseconds
///
/// # Returns
///
/// `true` if the lock is expired or released.
///
/// # Example
///
/// ```ignore
/// // Released lock (deadline = 0)
/// assert!(is_lock_expired(0, 1000));
///
/// // Expired lock
/// assert!(is_lock_expired(1000, 2000));
///
/// // Active lock
/// assert!(!is_lock_expired(2000, 1000));
/// ```
#[inline]
pub fn is_lock_expired(deadline_ms: u64, now_ms: u64) -> bool {
    deadline_ms == 0 || now_ms > deadline_ms
}

/// Calculate remaining TTL for a lock.
///
/// # Arguments
///
/// * `deadline_ms` - Lock deadline in Unix milliseconds
/// * `now_ms` - Current time in Unix milliseconds
///
/// # Returns
///
/// Remaining time in milliseconds (0 if expired).
///
/// # Tiger Style
///
/// - Uses saturating_sub to prevent underflow
#[inline]
pub fn remaining_ttl_ms(deadline_ms: u64, now_ms: u64) -> u64 {
    deadline_ms.saturating_sub(now_ms)
}

/// Compute the new fencing token for lock acquisition.
///
/// This produces the new token based on the current maximum token issued.
///
/// # Arguments
///
/// * `max_token` - The current maximum fencing token issued
///
/// # Returns
///
/// The new fencing token (max_token + 1), or max_token if at u64::MAX.
#[inline]
pub fn compute_new_fencing_token(max_token: u64) -> u64 {
    max_token.saturating_add(1)
}

/// Compute the deadline for a lock acquisition.
///
/// # Arguments
///
/// * `acquired_at_ms` - Unix timestamp when lock was acquired
/// * `ttl_ms` - Time-to-live in milliseconds
///
/// # Returns
///
/// The deadline (acquired_at_ms + ttl_ms), saturating at u64::MAX.
#[inline]
pub fn compute_acquire_deadline(acquired_at_ms: u64, ttl_ms: u64) -> u64 {
    acquired_at_ms.saturating_add(ttl_ms)
}

/// Result of backoff calculation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackoffResult {
    /// Sleep duration in milliseconds (includes jitter).
    pub sleep_ms: u64,
    /// Next backoff value (for exponential increase).
    pub next_backoff_ms: u64,
}

/// Compute exponential backoff with jitter.
///
/// Implements exponential backoff with additive jitter to prevent
/// thundering herd problems when multiple clients retry simultaneously.
///
/// # Arguments
///
/// * `current_backoff_ms` - Current backoff duration in milliseconds
/// * `max_backoff_ms` - Maximum allowed backoff in milliseconds
/// * `jitter_seed` - Random value for jitter calculation (0 to current_backoff_ms/2)
///
/// # Returns
///
/// A `BackoffResult` containing:
/// - `sleep_ms`: The actual sleep duration (backoff + jitter)
/// - `next_backoff_ms`: The backoff value for the next iteration
///
/// # Example
///
/// ```ignore
/// let result = compute_backoff_with_jitter(100, 1000, 25);
/// assert_eq!(result.sleep_ms, 125); // 100 + 25 jitter
/// assert_eq!(result.next_backoff_ms, 200); // doubled
/// ```
///
/// # Tiger Style
///
/// - Uses saturating arithmetic throughout
/// - Bounded by max_backoff_ms
#[inline]
pub fn compute_backoff_with_jitter(current_backoff_ms: u64, max_backoff_ms: u64, jitter_seed: u64) -> BackoffResult {
    // Jitter is bounded to half the current backoff + 1
    let max_jitter = current_backoff_ms.saturating_div(2).saturating_add(1);
    let jitter = jitter_seed % max_jitter;

    let sleep_ms = current_backoff_ms.saturating_add(jitter);

    // Double for next iteration, capped at max
    let doubled = current_backoff_ms.saturating_mul(2);
    let next_backoff_ms = if doubled < max_backoff_ms {
        doubled
    } else {
        max_backoff_ms
    };

    BackoffResult {
        sleep_ms,
        next_backoff_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_next_fencing_token_new_lock() {
        assert_eq!(compute_next_fencing_token(None), 1);
    }

    #[test]
    fn test_compute_next_fencing_token_existing_lock() {
        let entry = LockEntry {
            holder_id: "test".to_string(),
            fencing_token: 5,
            acquired_at_ms: 1000,
            ttl_ms: 30000,
            deadline_ms: 31000,
        };
        assert_eq!(compute_next_fencing_token(Some(&entry)), 6);
    }

    #[test]
    fn test_compute_next_fencing_token_overflow_safety() {
        let entry = LockEntry {
            holder_id: "test".to_string(),
            fencing_token: u64::MAX,
            acquired_at_ms: 1000,
            ttl_ms: 30000,
            deadline_ms: 31000,
        };
        // Should saturate, not panic
        assert_eq!(compute_next_fencing_token(Some(&entry)), u64::MAX);
    }

    #[test]
    fn test_compute_lock_deadline() {
        assert_eq!(compute_lock_deadline(1000, 30000), 31000);
    }

    #[test]
    fn test_compute_lock_deadline_overflow_safety() {
        assert_eq!(compute_lock_deadline(u64::MAX, 1), u64::MAX);
    }

    #[test]
    fn test_is_lock_expired_released() {
        // deadline_ms = 0 means released
        assert!(is_lock_expired(0, 1000));
    }

    #[test]
    fn test_is_lock_expired_past_deadline() {
        assert!(is_lock_expired(1000, 2000));
    }

    #[test]
    fn test_is_lock_expired_active() {
        assert!(!is_lock_expired(2000, 1000));
    }

    #[test]
    fn test_is_lock_expired_at_deadline() {
        // At exactly deadline, not yet expired
        assert!(!is_lock_expired(1000, 1000));
    }

    #[test]
    fn test_remaining_ttl_ms_active() {
        assert_eq!(remaining_ttl_ms(2000, 1000), 1000);
    }

    #[test]
    fn test_remaining_ttl_ms_expired() {
        assert_eq!(remaining_ttl_ms(1000, 2000), 0);
    }

    #[test]
    fn test_backoff_with_jitter() {
        let result = compute_backoff_with_jitter(100, 1000, 25);
        // jitter = 25 % (100/2 + 1) = 25 % 51 = 25
        assert_eq!(result.sleep_ms, 125);
        assert_eq!(result.next_backoff_ms, 200);
    }

    #[test]
    fn test_backoff_capped_at_max() {
        let result = compute_backoff_with_jitter(800, 1000, 0);
        // Would double to 1600, but capped at 1000
        assert_eq!(result.next_backoff_ms, 1000);
    }

    #[test]
    fn test_backoff_overflow_safety() {
        let result = compute_backoff_with_jitter(u64::MAX, u64::MAX, 0);
        // Should not panic
        assert_eq!(result.next_backoff_ms, u64::MAX);
    }
}

#[cfg(all(test, feature = "bolero"))]
mod property_tests {
    use bolero::check;

    use super::*;

    #[test]
    fn prop_fencing_token_always_positive() {
        check!().with_type::<Option<u64>>().for_each(|token| {
            let entry = token.map(|t| LockEntry {
                holder_id: "test".to_string(),
                fencing_token: t,
                acquired_at_ms: 0,
                ttl_ms: 0,
                deadline_ms: 0,
            });
            let result = compute_next_fencing_token(entry.as_ref());
            assert!(result >= 1, "Fencing token must be >= 1");
        });
    }

    #[test]
    fn prop_fencing_token_monotonic() {
        check!().with_type::<u64>().for_each(|token| {
            let entry = LockEntry {
                holder_id: "test".to_string(),
                fencing_token: *token,
                acquired_at_ms: 0,
                ttl_ms: 0,
                deadline_ms: 0,
            };
            let next = compute_next_fencing_token(Some(&entry));
            assert!(next >= *token, "Next token must be >= current token");
        });
    }

    #[test]
    fn prop_deadline_never_less_than_acquired() {
        check!().with_type::<(u64, u64)>().for_each(|(acquired, ttl)| {
            let deadline = compute_lock_deadline(*acquired, *ttl);
            assert!(deadline >= *acquired, "Deadline must be >= acquired time");
        });
    }

    #[test]
    fn prop_remaining_ttl_never_negative() {
        check!().with_type::<(u64, u64)>().for_each(|(deadline, now)| {
            let remaining = remaining_ttl_ms(*deadline, *now);
            // remaining is u64, so it can't be negative, but we verify the logic
            assert!(remaining <= *deadline, "Remaining TTL must be <= deadline");
        });
    }

    #[test]
    fn prop_backoff_sleep_at_least_base() {
        check!().with_type::<(u64, u64, u64)>().for_each(|(backoff, max, jitter)| {
            if *backoff > 0 {
                let result = compute_backoff_with_jitter(*backoff, *max, *jitter);
                assert!(result.sleep_ms >= *backoff, "Sleep must be >= base backoff");
            }
        });
    }

    #[test]
    fn prop_next_backoff_capped() {
        check!().with_type::<(u64, u64, u64)>().for_each(|(backoff, max, jitter)| {
            let result = compute_backoff_with_jitter(*backoff, *max, *jitter);
            assert!(result.next_backoff_ms <= *max || result.next_backoff_ms == u64::MAX);
        });
    }
}
