//! Pure gossip computation functions.
//!
//! This module contains pure functions for gossip subsystem operations.
//! All functions are deterministic and side-effect free.
//!
//! # Tiger Style
//!
//! - Uses saturating arithmetic for all calculations
//! - Time is passed explicitly (no calls to system time)
//! - Deterministic behavior for testing and verification

// ============================================================================
// Topic ID Derivation
// ============================================================================

/// Derive a gossip topic ID from the cluster cookie.
///
/// Uses blake3 hash of the cookie bytes to create a deterministic
/// 32-byte topic ID. All nodes with the same cookie will join the
/// same gossip topic.
///
/// # Arguments
///
/// * `cookie` - The cluster cookie bytes
///
/// # Returns
///
/// A 32-byte array suitable for use as a TopicId.
///
/// # Tiger Style
///
/// - Fixed-size output (32 bytes)
/// - Deterministic (same input always produces same output)
/// - No I/O or system calls
#[inline]
pub fn derive_topic_id_bytes(cookie: &[u8]) -> [u8; 32] {
    let hash = blake3::hash(cookie);
    *hash.as_bytes()
}

// ============================================================================
// Backoff Calculation
// ============================================================================

/// Calculate backoff duration in milliseconds for retry scenarios.
///
/// Given a restart/retry count and a maximum backoff, computes the
/// appropriate backoff duration using exponential backoff with a cap.
///
/// # Arguments
///
/// * `restart_count` - Number of restarts/retries (0-indexed)
/// * `base_backoff_ms` - Base backoff duration in milliseconds
/// * `max_backoff_ms` - Maximum backoff duration in milliseconds
///
/// # Returns
///
/// Backoff duration in milliseconds, capped at max_backoff_ms.
///
/// # Formula
///
/// `min(base_backoff_ms * 2^restart_count, max_backoff_ms)`
///
/// # Tiger Style
///
/// - Uses saturating arithmetic to prevent overflow
/// - Explicit u64 types for durations
/// - Bounded output (capped at max_backoff_ms)
#[inline]
pub fn calculate_backoff_duration_ms(restart_count: u32, base_backoff_ms: u64, max_backoff_ms: u64) -> u64 {
    // Calculate 2^restart_count using checked shift
    // Cap restart_count to prevent excessive shifting (max 63 for u64)
    let capped_count = restart_count.min(63);
    // Use checked_shl and saturate to MAX on overflow
    let multiplier = 1u64.checked_shl(capped_count).unwrap_or(u64::MAX);

    // Calculate backoff with saturation
    let backoff = base_backoff_ms.saturating_mul(multiplier);

    // Cap at maximum
    backoff.min(max_backoff_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Topic ID Derivation Tests
    // ========================================================================

    #[test]
    fn test_derive_topic_id_deterministic() {
        let cookie = b"my-cluster-cookie";
        let id1 = derive_topic_id_bytes(cookie);
        let id2 = derive_topic_id_bytes(cookie);
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_derive_topic_id_different_cookies() {
        let id1 = derive_topic_id_bytes(b"cluster-1");
        let id2 = derive_topic_id_bytes(b"cluster-2");
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_derive_topic_id_empty_cookie() {
        let id = derive_topic_id_bytes(b"");
        // Should produce a valid 32-byte hash
        assert_eq!(id.len(), 32);
    }

    #[test]
    fn test_derive_topic_id_long_cookie() {
        let long_cookie = vec![0xAB; 10_000];
        let id = derive_topic_id_bytes(&long_cookie);
        assert_eq!(id.len(), 32);
    }

    // ========================================================================
    // Backoff Calculation Tests
    // ========================================================================

    #[test]
    fn test_backoff_first_retry() {
        let backoff = calculate_backoff_duration_ms(0, 1000, 60000);
        assert_eq!(backoff, 1000); // 1000 * 2^0 = 1000
    }

    #[test]
    fn test_backoff_second_retry() {
        let backoff = calculate_backoff_duration_ms(1, 1000, 60000);
        assert_eq!(backoff, 2000); // 1000 * 2^1 = 2000
    }

    #[test]
    fn test_backoff_third_retry() {
        let backoff = calculate_backoff_duration_ms(2, 1000, 60000);
        assert_eq!(backoff, 4000); // 1000 * 2^2 = 4000
    }

    #[test]
    fn test_backoff_capped_at_max() {
        let backoff = calculate_backoff_duration_ms(10, 1000, 60000);
        assert_eq!(backoff, 60000); // Would be 1024000, capped at 60000
    }

    #[test]
    fn test_backoff_large_restart_count() {
        // Even with huge restart count, should not overflow
        let backoff = calculate_backoff_duration_ms(100, 1000, 60000);
        assert_eq!(backoff, 60000); // Capped at max
    }

    #[test]
    fn test_backoff_u32_max_restart() {
        let backoff = calculate_backoff_duration_ms(u32::MAX, 1000, 60000);
        assert_eq!(backoff, 60000); // Should be capped, not overflow
    }

    #[test]
    fn test_backoff_zero_base() {
        let backoff = calculate_backoff_duration_ms(5, 0, 60000);
        assert_eq!(backoff, 0); // 0 * anything = 0
    }

    #[test]
    fn test_backoff_zero_max() {
        let backoff = calculate_backoff_duration_ms(5, 1000, 0);
        assert_eq!(backoff, 0); // Capped at 0
    }

    #[test]
    fn test_backoff_base_equals_max() {
        let backoff = calculate_backoff_duration_ms(0, 5000, 5000);
        assert_eq!(backoff, 5000);

        let backoff2 = calculate_backoff_duration_ms(1, 5000, 5000);
        assert_eq!(backoff2, 5000); // Would be 10000, capped at 5000
    }

    #[test]
    fn test_backoff_typical_sequence() {
        // Typical sequence: 1s, 2s, 4s, 8s, 16s, then capped at 16s
        let base = 1000;
        let max = 16000;

        assert_eq!(calculate_backoff_duration_ms(0, base, max), 1000);
        assert_eq!(calculate_backoff_duration_ms(1, base, max), 2000);
        assert_eq!(calculate_backoff_duration_ms(2, base, max), 4000);
        assert_eq!(calculate_backoff_duration_ms(3, base, max), 8000);
        assert_eq!(calculate_backoff_duration_ms(4, base, max), 16000);
        assert_eq!(calculate_backoff_duration_ms(5, base, max), 16000); // Capped
    }
}

#[cfg(all(test, feature = "bolero"))]
mod property_tests {
    use bolero::check;

    use super::*;

    #[test]
    fn prop_topic_id_deterministic() {
        check!().with_type::<Vec<u8>>().for_each(|cookie| {
            let id1 = derive_topic_id_bytes(cookie);
            let id2 = derive_topic_id_bytes(cookie);
            assert_eq!(id1, id2, "Topic ID derivation must be deterministic");
        });
    }

    #[test]
    fn prop_backoff_never_exceeds_max() {
        check!().with_type::<(u32, u64, u64)>().for_each(|(count, base, max)| {
            let backoff = calculate_backoff_duration_ms(*count, *base, *max);
            assert!(backoff <= *max, "Backoff must not exceed max_backoff_ms");
        });
    }

    #[test]
    fn prop_backoff_monotonic_up_to_cap() {
        check!().with_type::<(u32, u64, u64)>().for_each(|(count, base, max)| {
            if *count < 63 && *base > 0 && *max > 0 {
                let backoff1 = calculate_backoff_duration_ms(*count, *base, *max);
                let backoff2 = calculate_backoff_duration_ms(count.saturating_add(1), *base, *max);
                assert!(backoff2 >= backoff1, "Backoff should be monotonically non-decreasing");
            }
        });
    }
}
