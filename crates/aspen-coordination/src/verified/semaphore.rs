//! Pure semaphore computation functions.
//!
//! This module contains pure functions for distributed semaphore operations.
//! All functions are deterministic and side-effect free.
//!
//! # Tiger Style
//!
//! - Uses saturating arithmetic for all calculations
//! - Time is passed explicitly (no calls to system time)
//! - Deterministic behavior for testing and verification

/// Semaphore key prefix.
pub const SEMAPHORE_PREFIX: &str = "__semaphore:";

// ============================================================================
// Key Generation
// ============================================================================

/// Generate the key for a semaphore.
///
/// # Example
///
/// ```ignore
/// assert_eq!(semaphore_key("my-sem"), "__semaphore:my-sem");
/// ```
#[inline]
pub fn semaphore_key(name: &str) -> String {
    format!("{}{}", SEMAPHORE_PREFIX, name)
}

// ============================================================================
// Expiry
// ============================================================================

/// Check if a semaphore holder has expired.
///
/// # Arguments
///
/// * `deadline_ms` - Holder's deadline in Unix milliseconds
/// * `now_ms` - Current time in Unix milliseconds
///
/// # Returns
///
/// `true` if the holder has expired.
#[inline]
pub fn is_holder_expired(deadline_ms: u64, now_ms: u64) -> bool {
    now_ms > deadline_ms
}

/// Calculate available permits for a semaphore.
///
/// Sums permits held by non-expired holders and subtracts from capacity.
///
/// # Arguments
///
/// * `capacity` - Total semaphore capacity
/// * `holders` - Iterator of (permits, deadline_ms) for each holder
/// * `now_ms` - Current time in Unix milliseconds
///
/// # Returns
///
/// Number of available permits.
///
/// # Tiger Style
///
/// - Uses saturating_sub to prevent underflow
#[inline]
pub fn calculate_available_permits<I>(capacity: u32, holders: I, now_ms: u64) -> u32
where I: IntoIterator<Item = (u32, u64)> {
    let used: u32 = holders
        .into_iter()
        .filter(|(_, deadline_ms)| *deadline_ms > now_ms)
        .map(|(permits, _)| permits)
        .sum();
    capacity.saturating_sub(used)
}

/// Check if permits can be acquired from a semaphore.
///
/// # Arguments
///
/// * `requested` - Number of permits requested
/// * `available` - Currently available permits
///
/// # Returns
///
/// `true` if the requested permits can be acquired.
#[inline]
pub fn can_acquire_permits(requested: u32, available: u32) -> bool {
    requested <= available
}

/// Compute the new deadline for a holder.
///
/// # Arguments
///
/// * `now_ms` - Current time in Unix milliseconds
/// * `ttl_ms` - Time-to-live in milliseconds
///
/// # Returns
///
/// The new deadline in Unix milliseconds.
///
/// # Tiger Style
///
/// - Uses saturating_add to prevent overflow
#[inline]
pub fn compute_holder_deadline(now_ms: u64, ttl_ms: u64) -> u64 {
    now_ms.saturating_add(ttl_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semaphore_key() {
        assert_eq!(semaphore_key("my-sem"), "__semaphore:my-sem");
        assert_eq!(semaphore_key(""), "__semaphore:");
    }

    #[test]
    fn test_holder_expired() {
        assert!(is_holder_expired(1000, 2000));
        assert!(!is_holder_expired(2000, 1000));
        assert!(!is_holder_expired(1000, 1000)); // At deadline, not expired
    }

    #[test]
    fn test_available_permits_no_holders() {
        let holders: Vec<(u32, u64)> = vec![];
        assert_eq!(calculate_available_permits(10, holders, 1000), 10);
    }

    #[test]
    fn test_available_permits_with_active_holders() {
        let holders = vec![
            (2, 2000), // Active, holding 2
            (3, 2000), // Active, holding 3
        ];
        assert_eq!(calculate_available_permits(10, holders, 1000), 5);
    }

    #[test]
    fn test_available_permits_with_expired_holders() {
        let holders = vec![
            (2, 500),  // Expired
            (3, 2000), // Active
        ];
        assert_eq!(calculate_available_permits(10, holders, 1000), 7);
    }

    #[test]
    fn test_available_permits_all_expired() {
        let holders = vec![
            (2, 500), // Expired
            (3, 500), // Expired
        ];
        assert_eq!(calculate_available_permits(10, holders, 1000), 10);
    }

    #[test]
    fn test_available_permits_overflow_protection() {
        let holders = vec![
            (u32::MAX, 2000), // More than capacity
        ];
        assert_eq!(calculate_available_permits(10, holders, 1000), 0);
    }

    #[test]
    fn test_can_acquire_permits() {
        assert!(can_acquire_permits(5, 10));
        assert!(can_acquire_permits(10, 10));
        assert!(!can_acquire_permits(11, 10));
        assert!(can_acquire_permits(0, 10));
    }

    #[test]
    fn test_compute_holder_deadline() {
        assert_eq!(compute_holder_deadline(1000, 5000), 6000);
    }

    #[test]
    fn test_compute_holder_deadline_overflow() {
        assert_eq!(compute_holder_deadline(u64::MAX, 1), u64::MAX);
    }
}

#[cfg(all(test, feature = "bolero"))]
mod property_tests {
    use bolero::check;

    use super::*;

    #[test]
    fn prop_available_never_exceeds_capacity() {
        check!().with_type::<(u32, Vec<(u32, u64)>, u64)>().for_each(|(capacity, holders, now)| {
            let available = calculate_available_permits(*capacity, holders.clone(), *now);
            assert!(available <= *capacity);
        });
    }

    #[test]
    fn prop_available_monotonic_with_expiry() {
        check!()
            .with_type::<(u32, u32, u64, u64, u64)>()
            .for_each(|(capacity, permits, deadline, now1, now2)| {
                let holders1 = vec![(*permits, *deadline)];
                let holders2 = vec![(*permits, *deadline)];
                let later = now1.saturating_add(*now2);

                let avail1 = calculate_available_permits(*capacity, holders1, *now1);
                let avail2 = calculate_available_permits(*capacity, holders2, later);

                // More time = more expired = more available
                assert!(avail2 >= avail1);
            });
    }
}
