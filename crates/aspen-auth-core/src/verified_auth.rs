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
//! - Explicit parameter structs for time-related helpers

/// Size of authentication nonces (32 bytes = 256 bits).
pub const AUTH_NONCE_SIZE: usize = 32;

/// Maximum age of a valid challenge in seconds.
pub const AUTH_CHALLENGE_MAX_AGE_SECS: u64 = 60;

/// Number of milliseconds in one second.
pub const MILLISECONDS_PER_SECOND: u64 = 1_000;

/// Inputs for challenge validity checks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChallengeValidityInput {
    /// When the challenge was created (ms since epoch).
    pub challenge_timestamp_ms: u64,
    /// Current time (ms since epoch).
    pub current_time_ms: u64,
    /// Maximum allowed age in seconds.
    pub max_age_secs: u64,
}

/// Inputs for challenge age calculations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChallengeAgeInput {
    /// When the challenge was created (ms since epoch).
    pub challenge_timestamp_ms: u64,
    /// Current time (ms since epoch).
    pub current_time_ms: u64,
}

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
/// use aspen_auth_core::verified_auth::ChallengeValidityInput;
/// use aspen_auth_core::verified_auth::is_challenge_valid;
///
/// let challenge_time_ms = 1_000_000;
/// let current_time_ms = 1_030_000;
///
/// assert!(is_challenge_valid(ChallengeValidityInput {
///     challenge_timestamp_ms: challenge_time_ms,
///     current_time_ms,
///     max_age_secs: 60,
/// }));
/// assert!(!is_challenge_valid(ChallengeValidityInput {
///     challenge_timestamp_ms: challenge_time_ms,
///     current_time_ms,
///     max_age_secs: 20,
/// }));
/// ```
#[inline]
pub const fn is_challenge_valid(input: ChallengeValidityInput) -> bool {
    let age_ms = calculate_challenge_age_ms(ChallengeAgeInput {
        challenge_timestamp_ms: input.challenge_timestamp_ms,
        current_time_ms: input.current_time_ms,
    });
    let max_age_ms = input.max_age_secs.saturating_mul(MILLISECONDS_PER_SECOND);
    age_ms <= max_age_ms
}

/// Calculate the age of a challenge in milliseconds.
///
/// # Returns
///
/// Age in milliseconds (0 if current time is before challenge time).
///
/// # Example
///
/// ```rust
/// use aspen_auth_core::verified_auth::ChallengeAgeInput;
/// use aspen_auth_core::verified_auth::calculate_challenge_age_ms;
///
/// assert_eq!(
///     calculate_challenge_age_ms(ChallengeAgeInput {
///         challenge_timestamp_ms: 1_000,
///         current_time_ms: 1_500,
///     }),
///     500,
/// );
/// assert_eq!(
///     calculate_challenge_age_ms(ChallengeAgeInput {
///         challenge_timestamp_ms: 1_000,
///         current_time_ms: 1_000,
///     }),
///     0,
/// );
/// assert_eq!(
///     calculate_challenge_age_ms(ChallengeAgeInput {
///         challenge_timestamp_ms: 1_500,
///         current_time_ms: 1_000,
///     }),
///     0,
/// );
/// ```
#[inline]
pub const fn calculate_challenge_age_ms(input: ChallengeAgeInput) -> u64 {
    input.current_time_ms.saturating_sub(input.challenge_timestamp_ms)
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
/// use aspen_auth_core::verified_auth::derive_hmac_key;
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

    const BASE_CHALLENGE_TIME_MS: u64 = 1_000_000;
    const THIRTY_SECONDS_MS: u64 = 30_000;
    const SIXTY_SECONDS_MS: u64 = 60_000;
    const SIXTY_ONE_SECONDS_MS: u64 = 61_000;
    const SHORT_MAX_AGE_SECS: u64 = 20;
    const ZERO_MAX_AGE_SECS: u64 = 0;
    const ONE_MILLISECOND_MS: u64 = 1;
    const FUTURE_CHALLENGE_TIME_MS: u64 = 2_000;
    const EARLIER_CURRENT_TIME_MS: u64 = 1_000;
    const SMALL_CHALLENGE_TIME_MS: u64 = 1_000;
    const SMALL_LATER_TIME_MS: u64 = 1_500;
    const ONE_MINUTE_AND_ONE_SECOND_MS: u64 = 61_000;
    const DERIVED_HMAC_KEY_SIZE_BYTES: usize = blake3::OUT_LEN;
    const LONG_COOKIE_LENGTH_BYTES: usize = 10_000;

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
        assert!(is_challenge_valid(ChallengeValidityInput {
            challenge_timestamp_ms: SMALL_CHALLENGE_TIME_MS,
            current_time_ms: SMALL_CHALLENGE_TIME_MS,
            max_age_secs: AUTH_CHALLENGE_MAX_AGE_SECS,
        }));
        assert!(is_challenge_valid(ChallengeValidityInput {
            challenge_timestamp_ms: SMALL_CHALLENGE_TIME_MS,
            current_time_ms: SMALL_CHALLENGE_TIME_MS.saturating_add(ONE_MILLISECOND_MS),
            max_age_secs: AUTH_CHALLENGE_MAX_AGE_SECS,
        }));
    }

    #[test]
    fn test_challenge_within_window() {
        let now = BASE_CHALLENGE_TIME_MS.saturating_add(THIRTY_SECONDS_MS);
        assert!(is_challenge_valid(ChallengeValidityInput {
            challenge_timestamp_ms: BASE_CHALLENGE_TIME_MS,
            current_time_ms: now,
            max_age_secs: AUTH_CHALLENGE_MAX_AGE_SECS,
        }));
    }

    #[test]
    fn test_challenge_at_boundary() {
        let now = BASE_CHALLENGE_TIME_MS.saturating_add(SIXTY_SECONDS_MS);
        assert!(is_challenge_valid(ChallengeValidityInput {
            challenge_timestamp_ms: BASE_CHALLENGE_TIME_MS,
            current_time_ms: now,
            max_age_secs: AUTH_CHALLENGE_MAX_AGE_SECS,
        }));
    }

    #[test]
    fn test_challenge_expired() {
        let now = BASE_CHALLENGE_TIME_MS.saturating_add(SIXTY_ONE_SECONDS_MS);
        assert!(!is_challenge_valid(ChallengeValidityInput {
            challenge_timestamp_ms: BASE_CHALLENGE_TIME_MS,
            current_time_ms: now,
            max_age_secs: AUTH_CHALLENGE_MAX_AGE_SECS,
        }));
    }

    #[test]
    fn test_challenge_future_timestamp() {
        assert!(is_challenge_valid(ChallengeValidityInput {
            challenge_timestamp_ms: FUTURE_CHALLENGE_TIME_MS,
            current_time_ms: EARLIER_CURRENT_TIME_MS,
            max_age_secs: AUTH_CHALLENGE_MAX_AGE_SECS,
        }));
    }

    #[test]
    fn test_challenge_zero_max_age() {
        assert!(is_challenge_valid(ChallengeValidityInput {
            challenge_timestamp_ms: SMALL_CHALLENGE_TIME_MS,
            current_time_ms: SMALL_CHALLENGE_TIME_MS,
            max_age_secs: ZERO_MAX_AGE_SECS,
        }));
        assert!(!is_challenge_valid(ChallengeValidityInput {
            challenge_timestamp_ms: SMALL_CHALLENGE_TIME_MS,
            current_time_ms: SMALL_CHALLENGE_TIME_MS.saturating_add(ONE_MILLISECOND_MS),
            max_age_secs: ZERO_MAX_AGE_SECS,
        }));
    }

    #[test]
    fn test_short_max_age_expires_challenge() {
        let now = BASE_CHALLENGE_TIME_MS.saturating_add(THIRTY_SECONDS_MS);
        assert!(!is_challenge_valid(ChallengeValidityInput {
            challenge_timestamp_ms: BASE_CHALLENGE_TIME_MS,
            current_time_ms: now,
            max_age_secs: SHORT_MAX_AGE_SECS,
        }));
    }

    // ========================================================================
    // calculate_challenge_age_ms tests
    // ========================================================================

    #[test]
    fn test_age_normal() {
        assert_eq!(
            calculate_challenge_age_ms(ChallengeAgeInput {
                challenge_timestamp_ms: SMALL_CHALLENGE_TIME_MS,
                current_time_ms: SMALL_LATER_TIME_MS,
            }),
            500,
        );
        assert_eq!(
            calculate_challenge_age_ms(ChallengeAgeInput {
                challenge_timestamp_ms: SMALL_CHALLENGE_TIME_MS,
                current_time_ms: ONE_MINUTE_AND_ONE_SECOND_MS,
            }),
            60_000,
        );
    }

    #[test]
    fn test_age_same_time() {
        assert_eq!(
            calculate_challenge_age_ms(ChallengeAgeInput {
                challenge_timestamp_ms: SMALL_CHALLENGE_TIME_MS,
                current_time_ms: SMALL_CHALLENGE_TIME_MS,
            }),
            0,
        );
    }

    #[test]
    fn test_age_future_challenge() {
        assert_eq!(
            calculate_challenge_age_ms(ChallengeAgeInput {
                challenge_timestamp_ms: FUTURE_CHALLENGE_TIME_MS,
                current_time_ms: EARLIER_CURRENT_TIME_MS,
            }),
            0,
        );
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
        let key = derive_hmac_key("");
        assert_eq!(key.len(), DERIVED_HMAC_KEY_SIZE_BYTES);
    }

    #[test]
    fn test_key_long_cookie() {
        let long_cookie = "a".repeat(LONG_COOKIE_LENGTH_BYTES);
        let key = derive_hmac_key(&long_cookie);
        assert_eq!(key.len(), DERIVED_HMAC_KEY_SIZE_BYTES);
    }
}
