//! Pure message validation functions.
//!
//! This module contains pure functions for validating gossip messages.
//! All functions are deterministic and side-effect free.
//!
//! # Tiger Style
//!
//! - No I/O operations
//! - Uses explicitly sized types (u32, u8)
//! - Deterministic behavior for testing and verification

// ============================================================================
// Size Validation
// ============================================================================

/// Check if a message size is within the allowed limit.
///
/// Used to reject oversized messages before deserialization to prevent
/// memory exhaustion attacks.
///
/// # Arguments
///
/// * `size` - The message size in bytes
/// * `max_size` - Maximum allowed size in bytes
///
/// # Returns
///
/// `true` if the message size is valid (size <= max_size).
///
/// # Tiger Style
///
/// - Pure function with no side effects
/// - Uses u32 for bounded size values
/// - Check happens BEFORE deserialization (fail-fast)
#[inline]
pub fn is_message_size_valid(size: u32, max_size: u32) -> bool {
    size <= max_size
}

// ============================================================================
// Version Compatibility
// ============================================================================

/// Check if a message version is compatible with the current version.
///
/// For forward compatibility, we only accept messages with versions
/// less than or equal to the current version. Messages with higher
/// versions may have fields or semantics we don't understand.
///
/// # Arguments
///
/// * `message_version` - The version in the received message
/// * `current_version` - Our current supported version
///
/// # Returns
///
/// `true` if the message version is compatible (message_version <= current_version).
///
/// # Tiger Style
///
/// - Pure function for version checking
/// - Uses u8 for version numbers (256 versions is plenty)
/// - Rejects unknown future versions
#[inline]
pub fn is_version_compatible(message_version: u8, current_version: u8) -> bool {
    message_version <= current_version
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Size Validation Tests
    // ========================================================================

    #[test]
    fn test_size_valid_under_limit() {
        assert!(is_message_size_valid(100, 1000));
    }

    #[test]
    fn test_size_valid_at_limit() {
        assert!(is_message_size_valid(1000, 1000));
    }

    #[test]
    fn test_size_invalid_over_limit() {
        assert!(!is_message_size_valid(1001, 1000));
    }

    #[test]
    fn test_size_valid_zero() {
        assert!(is_message_size_valid(0, 1000));
        assert!(is_message_size_valid(0, 0));
    }

    #[test]
    fn test_size_invalid_any_with_zero_max() {
        assert!(!is_message_size_valid(1, 0));
    }

    #[test]
    fn test_size_max_values() {
        assert!(is_message_size_valid(u32::MAX, u32::MAX));
        assert!(!is_message_size_valid(u32::MAX, u32::MAX - 1));
    }

    // ========================================================================
    // Version Compatibility Tests
    // ========================================================================

    #[test]
    fn test_version_same() {
        assert!(is_version_compatible(2, 2));
    }

    #[test]
    fn test_version_older() {
        assert!(is_version_compatible(1, 2));
    }

    #[test]
    fn test_version_newer_incompatible() {
        assert!(!is_version_compatible(3, 2));
    }

    #[test]
    fn test_version_zero() {
        assert!(is_version_compatible(0, 0));
        assert!(is_version_compatible(0, 1));
        assert!(!is_version_compatible(1, 0));
    }

    #[test]
    fn test_version_max() {
        assert!(is_version_compatible(u8::MAX, u8::MAX));
        assert!(is_version_compatible(u8::MAX - 1, u8::MAX));
        assert!(!is_version_compatible(u8::MAX, u8::MAX - 1));
    }

    #[test]
    fn test_version_typical_usage() {
        // Current version is 2, accept v1 and v2, reject v3+
        let current = 2u8;
        assert!(is_version_compatible(1, current));
        assert!(is_version_compatible(2, current));
        assert!(!is_version_compatible(3, current));
        assert!(!is_version_compatible(10, current));
    }
}

#[cfg(all(test, feature = "bolero"))]
mod property_tests {
    use bolero::check;

    use super::*;

    #[test]
    fn prop_size_reflexive() {
        check!().with_type::<u32>().for_each(|size| {
            assert!(is_message_size_valid(*size, *size), "Size should be valid when equal to max");
        });
    }

    #[test]
    fn prop_size_transitive() {
        check!().with_type::<(u32, u32, u32)>().for_each(|(a, b, c)| {
            // If a <= b and b <= c, then a <= c
            if is_message_size_valid(*a, *b) && is_message_size_valid(*b, *c) {
                assert!(is_message_size_valid(*a, *c));
            }
        });
    }

    #[test]
    fn prop_version_reflexive() {
        check!().with_type::<u8>().for_each(|version| {
            assert!(is_version_compatible(*version, *version), "Version should be compatible with itself");
        });
    }

    #[test]
    fn prop_version_transitive() {
        check!().with_type::<(u8, u8, u8)>().for_each(|(a, b, c)| {
            // If a <= b and b <= c, then a <= c
            if is_version_compatible(*a, *b) && is_version_compatible(*b, *c) {
                assert!(is_version_compatible(*a, *c));
            }
        });
    }

    #[test]
    fn prop_size_valid_implies_smaller_valid() {
        check!().with_type::<(u32, u32)>().for_each(|(size, max)| {
            if is_message_size_valid(*size, *max) && *size > 0 {
                assert!(is_message_size_valid(size - 1, *max), "Smaller sizes should also be valid");
            }
        });
    }
}
