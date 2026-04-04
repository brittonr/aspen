//! Verified pure functions for trust primitives.
//!
//! Deterministic functions with no I/O. Formally verified via Verus specs
//! in the `verus/` directory. See the FCIS pattern in `aspen-coordination`.
//!
//! # Verified Functions
//!
//! - `is_valid_threshold`: Check K >= 1, K <= N, N <= 255
//! - `default_threshold_for_size`: Compute (n/2) + 1 majority threshold

/// Check if a threshold/total pair is valid for Shamir secret sharing.
///
/// Valid when: K >= 1, K <= N, N <= 255 (GF(2^8) has 255 nonzero elements).
#[inline]
pub fn is_valid_threshold(threshold: u8, total: u8) -> bool {
    threshold >= 1 && threshold <= total && total >= 1
}

/// Compute the default majority threshold for a given cluster size.
///
/// Returns `(n / 2) + 1`. For n=0, returns 1 (minimum valid threshold).
/// Uses saturating arithmetic to prevent overflow.
#[inline]
pub fn default_threshold_for_size(n: u32) -> u8 {
    let majority = (n / 2).saturating_add(1);
    // Clamp to u8 range (max 255)
    if majority > 255 { 255 } else { majority as u8 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_threshold() {
        assert!(is_valid_threshold(1, 1));
        assert!(is_valid_threshold(2, 3));
        assert!(is_valid_threshold(3, 5));
        assert!(is_valid_threshold(255, 255));
    }

    #[test]
    fn test_invalid_threshold() {
        assert!(!is_valid_threshold(0, 5)); // K < 1
        assert!(!is_valid_threshold(6, 5)); // K > N
        assert!(!is_valid_threshold(0, 0)); // both zero
        assert!(!is_valid_threshold(1, 0)); // N = 0
    }

    #[test]
    fn test_default_threshold() {
        assert_eq!(default_threshold_for_size(0), 1);
        assert_eq!(default_threshold_for_size(1), 1);
        assert_eq!(default_threshold_for_size(2), 2);
        assert_eq!(default_threshold_for_size(3), 2);
        assert_eq!(default_threshold_for_size(4), 3);
        assert_eq!(default_threshold_for_size(5), 3);
        assert_eq!(default_threshold_for_size(7), 4);
        assert_eq!(default_threshold_for_size(100), 51);
    }
}
