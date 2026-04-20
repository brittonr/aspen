//! Verified pure functions for trust primitives.
//!
//! Deterministic functions with no I/O. Formally verified via Verus specs
//! in the `verus/` directory. See the FCIS pattern in `aspen-coordination`.
//!
//! # Verified Functions
//!
//! - `is_valid_threshold`: Check K >= 1, K <= N, N <= 255
//! - `default_threshold_for_size`: Compute (n/2) + 1 majority threshold

/// Threshold/total pair for Shamir secret sharing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThresholdConfig {
    /// Reconstruction threshold K.
    pub threshold: u8,
    /// Total share count N.
    pub total: u8,
}

/// Check if a threshold/total pair is valid for Shamir secret sharing.
///
/// Valid when: K >= 1, K <= N, N <= 255 (GF(2^8) has 255 nonzero elements).
#[inline]
pub fn is_valid_threshold(config: ThresholdConfig) -> bool {
    config.threshold >= 1 && config.threshold <= config.total && config.total >= 1
}

/// Compute the default majority threshold for a given cluster size.
///
/// Returns `(n / 2) + 1`. For n=0, returns 1 (minimum valid threshold).
/// Uses saturating arithmetic to prevent overflow.
#[inline]
pub fn default_threshold_for_size(n: u32) -> u8 {
    let majority = (n / 2).saturating_add(1);
    let clamped_majority = majority.min(u32::from(u8::MAX));
    clamped_majority as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_threshold() {
        assert!(is_valid_threshold(ThresholdConfig { threshold: 1, total: 1 }));
        assert!(is_valid_threshold(ThresholdConfig { threshold: 2, total: 3 }));
        assert!(is_valid_threshold(ThresholdConfig { threshold: 3, total: 5 }));
        assert!(is_valid_threshold(ThresholdConfig {
            threshold: 255,
            total: 255,
        }));
    }

    #[test]
    fn test_invalid_threshold() {
        assert!(!is_valid_threshold(ThresholdConfig { threshold: 0, total: 5 })); // K < 1
        assert!(!is_valid_threshold(ThresholdConfig { threshold: 6, total: 5 })); // K > N
        assert!(!is_valid_threshold(ThresholdConfig { threshold: 0, total: 0 })); // both zero
        assert!(!is_valid_threshold(ThresholdConfig { threshold: 1, total: 0 })); // N = 0
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
