//! Pure functions for authentication logic.
//!
//! These functions encapsulate authentication-related computations without
//! any async/I/O dependencies, enabling:
//!
//! - Unit testing with explicit inputs/outputs
//! - Property-based testing with Bolero
//! - Deterministic behavior verification
//!
//! # Tiger Style
//!
//! - Fixed iteration count for constant-time comparison
//! - No early exit on mismatch (timing attack prevention)
//! - Explicit types for all parameters

/// Size of authentication nonces (32 bytes = 256 bits).
pub const AUTH_NONCE_SIZE: usize = 32;

/// Maximum age of a valid challenge in seconds.
pub const AUTH_CHALLENGE_MAX_AGE_SECS: u64 = 60;

/// Constant-time comparison of two 32-byte arrays.
///
/// Prevents timing attacks by ensuring comparison takes the same time
/// regardless of where (if anywhere) the values differ.
///
/// # Tiger Style
///
/// - Fixed iteration count (always 32 iterations)
/// - No early exit on mismatch
/// - XOR accumulation prevents branch prediction leakage
#[inline]
pub fn constant_time_compare(a: &[u8; 32], b: &[u8; 32]) -> bool {
    a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Verify that a challenge timestamp is still valid.
///
/// Challenges have a maximum age to prevent replay attacks. This function
/// checks if the challenge is still within the validity window.
///
/// # Arguments
///
/// * `challenge_timestamp_ms` - When the challenge was created (ms since epoch)
/// * `current_time_ms` - Current time (ms since epoch)
/// * `max_age_secs` - Maximum allowed age in seconds
///
/// # Returns
///
/// `true` if the challenge is still valid, `false` if expired.
///
/// # Tiger Style
///
/// - Uses saturating arithmetic to prevent underflow
/// - Explicit timeout in seconds for clarity
///
/// # Example
///
/// ```rust
/// use aspen_auth::verified_auth::is_challenge_valid;
///
/// let challenge_time = 1000000;
/// let now = 1030000; // 30 seconds later
///
/// assert!(is_challenge_valid(challenge_time, now, 60));  // 30s < 60s
/// assert!(!is_challenge_valid(challenge_time, now, 20)); // 30s > 20s
/// ```
#[inline]
pub const fn is_challenge_valid(challenge_timestamp_ms: u64, current_time_ms: u64, max_age_secs: u64) -> bool {
    let age_ms = current_time_ms.saturating_sub(challenge_timestamp_ms);
    let max_age_ms = max_age_secs.saturating_mul(1000);
    age_ms <= max_age_ms
}

/// Calculate the age of a challenge in milliseconds.
///
/// # Arguments
///
/// * `challenge_timestamp_ms` - When the challenge was created (ms since epoch)
/// * `current_time_ms` - Current time (ms since epoch)
///
/// # Returns
///
/// Age in milliseconds (0 if current time is before challenge time).
///
/// # Example
///
/// ```rust
/// use aspen_auth::verified_auth::calculate_challenge_age_ms;
///
/// assert_eq!(calculate_challenge_age_ms(1000, 1500), 500);
/// assert_eq!(calculate_challenge_age_ms(1000, 1000), 0);
/// assert_eq!(calculate_challenge_age_ms(1500, 1000), 0); // Edge case
/// ```
#[inline]
pub const fn calculate_challenge_age_ms(challenge_timestamp_ms: u64, current_time_ms: u64) -> u64 {
    current_time_ms.saturating_sub(challenge_timestamp_ms)
}

/// Derive an HMAC key from a cluster cookie using Blake3.
///
/// Blake3 ensures the derived key has full entropy regardless of
/// the cookie's quality. This is a pure function that produces
/// deterministic output.
///
/// # Arguments
///
/// * `cookie` - The cluster cookie string
///
/// # Returns
///
/// 32-byte key derived from the cookie.
///
/// # Example
///
/// ```rust
/// use aspen_auth::verified_auth::derive_hmac_key;
///
/// let key1 = derive_hmac_key("my-cookie");
/// let key2 = derive_hmac_key("my-cookie");
/// let key3 = derive_hmac_key("different-cookie");
///
/// assert_eq!(key1, key2);
/// assert_ne!(key1, key3);
/// ```
#[inline]
pub fn derive_hmac_key(cookie: &str) -> [u8; 32] {
    let hash = blake3::hash(cookie.as_bytes());
    *hash.as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // constant_time_compare tests
    // ========================================================================

    #[test]
    fn test_constant_time_compare_equal() {
        let a = [1u8; 32];
        let b = [1u8; 32];
        assert!(constant_time_compare(&a, &b));
    }

    #[test]
    fn test_constant_time_compare_not_equal() {
        let a = [1u8; 32];
        let c = [2u8; 32];
        assert!(!constant_time_compare(&a, &c));
    }

    #[test]
    fn test_constant_time_compare_single_byte_diff() {
        let a = [1u8; 32];
        let mut d = a;
        d[15] = 99;
        assert!(!constant_time_compare(&a, &d));
    }

    // ========================================================================
    // is_challenge_valid tests
    // ========================================================================

    #[test]
    fn test_fresh_challenge() {
        assert!(is_challenge_valid(1000, 1000, 60)); // 0s age
        assert!(is_challenge_valid(1000, 1001, 60)); // 1ms age
    }

    #[test]
    fn test_challenge_within_window() {
        let challenge_time = 1_000_000;
        let now = challenge_time + 30_000; // 30 seconds later
        assert!(is_challenge_valid(challenge_time, now, 60));
    }

    #[test]
    fn test_challenge_at_boundary() {
        let challenge_time = 1_000_000;
        let now = challenge_time + 60_000; // Exactly 60 seconds
        assert!(is_challenge_valid(challenge_time, now, 60));
    }

    #[test]
    fn test_challenge_expired() {
        let challenge_time = 1_000_000;
        let now = challenge_time + 61_000; // 61 seconds later
        assert!(!is_challenge_valid(challenge_time, now, 60));
    }

    #[test]
    fn test_challenge_future_timestamp() {
        // Edge case: challenge timestamp in the future (clock skew)
        assert!(is_challenge_valid(2000, 1000, 60)); // age = 0 (saturating)
    }

    #[test]
    fn test_challenge_zero_max_age() {
        // Zero max age means only same-millisecond is valid
        assert!(is_challenge_valid(1000, 1000, 0));
        assert!(!is_challenge_valid(1000, 1001, 0));
    }

    // ========================================================================
    // calculate_challenge_age_ms tests
    // ========================================================================

    #[test]
    fn test_age_normal() {
        assert_eq!(calculate_challenge_age_ms(1000, 1500), 500);
        assert_eq!(calculate_challenge_age_ms(1000, 61000), 60000);
    }

    #[test]
    fn test_age_same_time() {
        assert_eq!(calculate_challenge_age_ms(1000, 1000), 0);
    }

    #[test]
    fn test_age_future_challenge() {
        // Challenge in the future returns 0 (saturating)
        assert_eq!(calculate_challenge_age_ms(2000, 1000), 0);
    }

    // ========================================================================
    // derive_hmac_key tests
    // ========================================================================

    #[test]
    fn test_key_deterministic() {
        let key1 = derive_hmac_key("test-cookie");
        let key2 = derive_hmac_key("test-cookie");
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_key_different_cookies() {
        let key1 = derive_hmac_key("cookie-1");
        let key2 = derive_hmac_key("cookie-2");
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_key_empty_cookie() {
        // Empty cookie should produce a valid key
        let key = derive_hmac_key("");
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_key_long_cookie() {
        // Long cookie should work fine
        let long_cookie = "a".repeat(10000);
        let key = derive_hmac_key(&long_cookie);
        assert_eq!(key.len(), 32);
    }
}
