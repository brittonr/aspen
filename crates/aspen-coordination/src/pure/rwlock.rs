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

/// RWLock key prefix.
pub const RWLOCK_PREFIX: &str = "__rwlock:";

// ============================================================================
// Key Generation
// ============================================================================

/// Generate the key for a RWLock.
///
/// # Example
///
/// ```ignore
/// assert_eq!(rwlock_key("my-lock"), "__rwlock:my-lock");
/// ```
#[inline]
pub fn rwlock_key(name: &str) -> String {
    format!("{}{}", RWLOCK_PREFIX, name)
}

// ============================================================================
// Deadline Computation
// ============================================================================

/// Compute the lock deadline from current time and TTL.
///
/// # Arguments
///
/// * `now_ms` - Current time in Unix milliseconds
/// * `ttl_ms` - Time-to-live in milliseconds
///
/// # Returns
///
/// Deadline in Unix milliseconds.
///
/// # Tiger Style
///
/// Uses saturating_add to prevent overflow.
#[inline]
pub fn compute_lock_deadline(now_ms: u64, ttl_ms: u64) -> u64 {
    now_ms.saturating_add(ttl_ms)
}

// ============================================================================
// Reader/Writer State Checks
// ============================================================================

/// Check if a holder has a read lock in the readers list.
///
/// # Arguments
///
/// * `readers` - List of (holder_id, deadline_ms) pairs
/// * `holder_id` - The holder to check
/// * `now_ms` - Current time for expiry check
///
/// # Returns
///
/// `true` if the holder has an active (non-expired) read lock.
#[inline]
pub fn has_read_lock(readers: &[(String, u64)], holder_id: &str, now_ms: u64) -> bool {
    readers.iter().any(|(h, deadline)| h == holder_id && !is_reader_expired(*deadline, now_ms))
}

/// Check if a holder has the write lock.
///
/// # Arguments
///
/// * `writer_holder_id` - Current writer's holder ID (if any)
/// * `writer_deadline` - Writer's deadline
/// * `holder_id` - The holder to check
/// * `now_ms` - Current time for expiry check
///
/// # Returns
///
/// `true` if the holder has an active write lock.
#[inline]
pub fn has_write_lock(
    writer_holder_id: Option<&str>,
    writer_deadline: u64,
    holder_id: &str,
    now_ms: u64,
) -> bool {
    match writer_holder_id {
        Some(w_id) => w_id == holder_id && !is_writer_expired(writer_deadline, now_ms),
        None => false,
    }
}

/// Count active (non-expired) readers.
///
/// # Arguments
///
/// * `readers` - List of (holder_id, deadline_ms) pairs
/// * `now_ms` - Current time for expiry check
///
/// # Returns
///
/// Number of active readers.
#[inline]
pub fn count_active_readers(readers: &[(String, u64)], now_ms: u64) -> u32 {
    readers.iter().filter(|(_, deadline)| !is_reader_expired(*deadline, now_ms)).count() as u32
}

/// Filter out expired readers, returning only active ones.
///
/// # Arguments
///
/// * `readers` - List of reader entries
/// * `now_ms` - Current time for expiry check
///
/// # Returns
///
/// New vec containing only active readers.
#[inline]
pub fn filter_expired_readers<T: Clone>(
    readers: &[(T, u64)],
    now_ms: u64,
) -> Vec<(T, u64)> {
    readers
        .iter()
        .filter(|(_, deadline)| !is_reader_expired(*deadline, now_ms))
        .cloned()
        .collect()
}

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

    // ========================================================================
    // Key Generation Tests
    // ========================================================================

    #[test]
    fn test_rwlock_key() {
        assert_eq!(rwlock_key("my-lock"), "__rwlock:my-lock");
        assert_eq!(rwlock_key(""), "__rwlock:");
    }

    // ========================================================================
    // Deadline Tests
    // ========================================================================

    #[test]
    fn test_compute_lock_deadline() {
        assert_eq!(compute_lock_deadline(1000, 5000), 6000);
        assert_eq!(compute_lock_deadline(u64::MAX, 1), u64::MAX); // Saturates
    }

    // ========================================================================
    // Reader/Writer State Tests
    // ========================================================================

    #[test]
    fn test_has_read_lock_present() {
        let readers = vec![
            ("reader1".to_string(), 2000u64),
            ("reader2".to_string(), 3000u64),
        ];
        assert!(has_read_lock(&readers, "reader1", 1000));
        assert!(has_read_lock(&readers, "reader2", 1000));
    }

    #[test]
    fn test_has_read_lock_absent() {
        let readers = vec![("reader1".to_string(), 2000u64)];
        assert!(!has_read_lock(&readers, "reader3", 1000));
    }

    #[test]
    fn test_has_read_lock_expired() {
        let readers = vec![("reader1".to_string(), 500u64)];
        assert!(!has_read_lock(&readers, "reader1", 1000)); // Expired
    }

    #[test]
    fn test_has_write_lock_present() {
        assert!(has_write_lock(Some("writer1"), 2000, "writer1", 1000));
    }

    #[test]
    fn test_has_write_lock_absent() {
        assert!(!has_write_lock(None, 0, "writer1", 1000));
    }

    #[test]
    fn test_has_write_lock_wrong_holder() {
        assert!(!has_write_lock(Some("writer1"), 2000, "writer2", 1000));
    }

    #[test]
    fn test_has_write_lock_expired() {
        assert!(!has_write_lock(Some("writer1"), 500, "writer1", 1000));
    }

    #[test]
    fn test_count_active_readers() {
        let readers = vec![
            ("reader1".to_string(), 2000u64), // Active
            ("reader2".to_string(), 500u64),  // Expired
            ("reader3".to_string(), 3000u64), // Active
        ];
        assert_eq!(count_active_readers(&readers, 1000), 2);
    }

    #[test]
    fn test_count_active_readers_all_expired() {
        let readers = vec![
            ("reader1".to_string(), 500u64),
            ("reader2".to_string(), 800u64),
        ];
        assert_eq!(count_active_readers(&readers, 1000), 0);
    }

    #[test]
    fn test_filter_expired_readers() {
        let readers = vec![
            ("reader1".to_string(), 2000u64), // Active
            ("reader2".to_string(), 500u64),  // Expired
            ("reader3".to_string(), 3000u64), // Active
        ];
        let active = filter_expired_readers(&readers, 1000);
        assert_eq!(active.len(), 2);
        assert_eq!(active[0].0, "reader1");
        assert_eq!(active[1].0, "reader3");
    }

    // ========================================================================
    // Original Tests
    // ========================================================================

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
