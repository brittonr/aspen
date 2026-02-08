//! Pure rwlock computation functions.
//!
//! This module contains pure functions for distributed read-write lock operations.
//! All functions are deterministic and side-effect free.
//!
//! # Tiger Style
//!
//! - Uses explicit types (u64 for timestamps, u32 for counts)
//! - Time is passed explicitly (no calls to system time)
//! - Deterministic behavior for testing and verification

use crate::rwlock::RWLockMode;

/// Check if a reader entry has expired.
///
/// # Arguments
///
/// * `deadline_ms` - Reader's deadline in Unix milliseconds (0 = released)
/// * `now_ms` - Current time in Unix milliseconds
///
/// # Returns
///
/// `true` if the reader has expired or been released.
#[inline]
pub fn is_reader_expired(deadline_ms: u64, now_ms: u64) -> bool {
    deadline_ms == 0 || now_ms > deadline_ms
}

/// Check if a writer entry has expired.
///
/// # Arguments
///
/// * `deadline_ms` - Writer's deadline in Unix milliseconds (0 = released)
/// * `now_ms` - Current time in Unix milliseconds
///
/// # Returns
///
/// `true` if the writer has expired or been released.
#[inline]
pub fn is_writer_expired(deadline_ms: u64, now_ms: u64) -> bool {
    deadline_ms == 0 || now_ms > deadline_ms
}

/// Check if a read lock can be acquired.
///
/// A read lock can be acquired if:
/// - No writer is holding the lock (mode != Write)
/// - No writers are pending (writer-preference fairness)
///
/// # Arguments
///
/// * `mode` - Current lock mode
/// * `pending_writers` - Number of writers waiting
/// * `writer_expired` - Whether the current writer (if any) has expired
///
/// # Returns
///
/// `true` if a read lock can be acquired.
#[inline]
pub fn can_acquire_read(mode: RWLockMode, pending_writers: u32, writer_expired: bool) -> bool {
    // Can't acquire if writer is holding (unless expired)
    if mode == RWLockMode::Write && !writer_expired {
        return false;
    }

    // Writer-preference: block new readers if writers are waiting
    if pending_writers > 0 {
        return false;
    }

    true
}

/// Check if a write lock can be acquired.
///
/// A write lock can be acquired if:
/// - No active readers (or all expired)
/// - No active writer (or expired)
///
/// # Arguments
///
/// * `mode` - Current lock mode
/// * `active_readers` - Number of non-expired readers
/// * `writer_expired` - Whether the current writer (if any) has expired
///
/// # Returns
///
/// `true` if a write lock can be acquired.
#[inline]
pub fn can_acquire_write(mode: RWLockMode, active_readers: u32, writer_expired: bool) -> bool {
    // Can't acquire if readers are holding
    if mode == RWLockMode::Read && active_readers > 0 {
        return false;
    }

    // Can't acquire if another writer is holding (unless expired)
    if mode == RWLockMode::Write && !writer_expired {
        return false;
    }

    true
}

/// Compute the next fencing token for a write lock acquisition.
///
/// # Arguments
///
/// * `current_token` - Current global fencing token
///
/// # Returns
///
/// The next fencing token.
///
/// # Tiger Style
///
/// - Uses saturating_add to prevent overflow
#[inline]
pub fn compute_next_write_token(current_token: u64) -> u64 {
    current_token.saturating_add(1)
}

/// Determine the new lock mode after cleanup.
///
/// # Arguments
///
/// * `current_mode` - Current lock mode
/// * `active_readers` - Number of non-expired readers
/// * `writer_active` - Whether the writer is active (non-expired)
///
/// # Returns
///
/// The new lock mode after accounting for expirations.
#[inline]
pub fn compute_mode_after_cleanup(current_mode: RWLockMode, active_readers: u32, writer_active: bool) -> RWLockMode {
    match current_mode {
        RWLockMode::Write => {
            if writer_active {
                RWLockMode::Write
            } else {
                RWLockMode::Free
            }
        }
        RWLockMode::Read => {
            if active_readers > 0 {
                RWLockMode::Read
            } else {
                RWLockMode::Free
            }
        }
        RWLockMode::Free => RWLockMode::Free,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reader_expired() {
        assert!(is_reader_expired(0, 1000)); // Released
        assert!(is_reader_expired(1000, 2000)); // Expired
        assert!(!is_reader_expired(2000, 1000)); // Active
        assert!(!is_reader_expired(1000, 1000)); // At deadline
    }

    #[test]
    fn test_writer_expired() {
        assert!(is_writer_expired(0, 1000)); // Released
        assert!(is_writer_expired(1000, 2000)); // Expired
        assert!(!is_writer_expired(2000, 1000)); // Active
    }

    #[test]
    fn test_can_acquire_read_free() {
        assert!(can_acquire_read(RWLockMode::Free, 0, false));
    }

    #[test]
    fn test_can_acquire_read_blocked_by_writer() {
        assert!(!can_acquire_read(RWLockMode::Write, 0, false));
    }

    #[test]
    fn test_can_acquire_read_writer_expired() {
        assert!(can_acquire_read(RWLockMode::Write, 0, true));
    }

    #[test]
    fn test_can_acquire_read_blocked_by_pending_writer() {
        assert!(!can_acquire_read(RWLockMode::Free, 1, false));
        assert!(!can_acquire_read(RWLockMode::Read, 1, false));
    }

    #[test]
    fn test_can_acquire_read_with_other_readers() {
        assert!(can_acquire_read(RWLockMode::Read, 0, false));
    }

    #[test]
    fn test_can_acquire_write_free() {
        assert!(can_acquire_write(RWLockMode::Free, 0, false));
    }

    #[test]
    fn test_can_acquire_write_blocked_by_readers() {
        assert!(!can_acquire_write(RWLockMode::Read, 1, false));
        assert!(!can_acquire_write(RWLockMode::Read, 5, false));
    }

    #[test]
    fn test_can_acquire_write_readers_expired() {
        assert!(can_acquire_write(RWLockMode::Read, 0, false));
    }

    #[test]
    fn test_can_acquire_write_blocked_by_writer() {
        assert!(!can_acquire_write(RWLockMode::Write, 0, false));
    }

    #[test]
    fn test_can_acquire_write_writer_expired() {
        assert!(can_acquire_write(RWLockMode::Write, 0, true));
    }

    #[test]
    fn test_next_write_token() {
        assert_eq!(compute_next_write_token(0), 1);
        assert_eq!(compute_next_write_token(5), 6);
        assert_eq!(compute_next_write_token(u64::MAX), u64::MAX);
    }

    #[test]
    fn test_mode_after_cleanup_write_active() {
        assert_eq!(compute_mode_after_cleanup(RWLockMode::Write, 0, true), RWLockMode::Write);
    }

    #[test]
    fn test_mode_after_cleanup_write_expired() {
        assert_eq!(compute_mode_after_cleanup(RWLockMode::Write, 0, false), RWLockMode::Free);
    }

    #[test]
    fn test_mode_after_cleanup_read_active() {
        assert_eq!(compute_mode_after_cleanup(RWLockMode::Read, 3, false), RWLockMode::Read);
    }

    #[test]
    fn test_mode_after_cleanup_read_all_expired() {
        assert_eq!(compute_mode_after_cleanup(RWLockMode::Read, 0, false), RWLockMode::Free);
    }

    #[test]
    fn test_mode_after_cleanup_free() {
        assert_eq!(compute_mode_after_cleanup(RWLockMode::Free, 0, false), RWLockMode::Free);
    }
}

#[cfg(all(test, feature = "bolero"))]
mod property_tests {
    use super::*;
    use bolero::check;

    #[test]
    fn prop_expired_is_consistent() {
        check!().with_type::<(u64, u64)>().for_each(|(deadline, now)| {
            // Reader and writer expiry should behave identically
            assert_eq!(is_reader_expired(*deadline, *now), is_writer_expired(*deadline, *now));
        });
    }

    #[test]
    fn prop_write_token_monotonic() {
        check!().with_type::<u64>().for_each(|current| {
            let next = compute_next_write_token(*current);
            assert!(next >= *current);
        });
    }

    #[test]
    fn prop_free_mode_always_allows_acquisition() {
        check!().with_type::<(u32, bool)>().for_each(|(pending, expired)| {
            // Free mode with no pending writers allows read
            if *pending == 0 {
                assert!(can_acquire_read(RWLockMode::Free, *pending, *expired));
            }
            // Free mode always allows write (no readers, no writer)
            assert!(can_acquire_write(RWLockMode::Free, 0, *expired));
        });
    }
}
