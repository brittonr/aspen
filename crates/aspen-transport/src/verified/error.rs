//! Pure error classification functions.
//!
//! This module contains pure functions for classifying error types.
//! All functions are deterministic and side-effect free.
//!
//! # Tiger Style
//!
//! - No I/O operations
//! - Explicit error classification
//! - Deterministic behavior

// ============================================================================
// Fatal Error Classification
// ============================================================================

/// Classification of fatal Raft errors.
///
/// This enum represents the possible fatal error categories that can occur
/// in a Raft system. It's used in wire protocols to communicate error types
/// across nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FatalErrorClassification {
    /// The Raft system panicked.
    Panicked,
    /// The Raft system was stopped gracefully.
    Stopped,
    /// A storage error occurred.
    StorageError,
}

impl FatalErrorClassification {
    /// Check if this is a recoverable error.
    ///
    /// Only `Stopped` is considered recoverable (graceful shutdown).
    /// `Panicked` and `StorageError` indicate serious problems.
    #[inline]
    pub fn is_recoverable(&self) -> bool {
        matches!(self, FatalErrorClassification::Stopped)
    }

    /// Check if this error requires immediate attention.
    ///
    /// `Panicked` and `StorageError` require immediate attention.
    #[inline]
    pub fn requires_attention(&self) -> bool {
        matches!(self, FatalErrorClassification::Panicked | FatalErrorClassification::StorageError)
    }
}

/// Classify a fatal error kind from a u8 discriminant.
///
/// This is useful for deserializing error types from wire protocols.
///
/// # Arguments
///
/// * `discriminant` - The u8 discriminant value (0 = Panicked, 1 = Stopped, other = StorageError)
///
/// # Returns
///
/// The corresponding `FatalErrorClassification`.
///
/// # Tiger Style
///
/// - Pure function with no side effects
/// - Uses explicit discriminants for wire compatibility
/// - Unknown values default to StorageError (safe fallback)
#[inline]
pub fn classify_fatal_error_kind(discriminant: u8) -> FatalErrorClassification {
    match discriminant {
        0 => FatalErrorClassification::Panicked,
        1 => FatalErrorClassification::Stopped,
        _ => FatalErrorClassification::StorageError,
    }
}

/// Convert a fatal error classification to a u8 discriminant.
///
/// # Arguments
///
/// * `classification` - The error classification to encode
///
/// # Returns
///
/// A u8 discriminant (0 = Panicked, 1 = Stopped, 2 = StorageError).
///
/// # Tiger Style
///
/// - Pure function with deterministic output
/// - Consistent with classify_fatal_error_kind
#[inline]
pub fn encode_fatal_error_kind(classification: FatalErrorClassification) -> u8 {
    match classification {
        FatalErrorClassification::Panicked => 0,
        FatalErrorClassification::Stopped => 1,
        FatalErrorClassification::StorageError => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Classification Tests
    // ========================================================================

    #[test]
    fn test_classify_panicked() {
        assert_eq!(classify_fatal_error_kind(0), FatalErrorClassification::Panicked);
    }

    #[test]
    fn test_classify_stopped() {
        assert_eq!(classify_fatal_error_kind(1), FatalErrorClassification::Stopped);
    }

    #[test]
    fn test_classify_storage_error() {
        assert_eq!(classify_fatal_error_kind(2), FatalErrorClassification::StorageError);
    }

    #[test]
    fn test_classify_unknown_defaults_to_storage() {
        // Any unknown value defaults to StorageError (safe fallback)
        for i in 3..=u8::MAX {
            assert_eq!(
                classify_fatal_error_kind(i),
                FatalErrorClassification::StorageError,
                "Unknown discriminant {} should map to StorageError",
                i
            );
        }
    }

    // ========================================================================
    // Encode Tests
    // ========================================================================

    #[test]
    fn test_encode_panicked() {
        assert_eq!(encode_fatal_error_kind(FatalErrorClassification::Panicked), 0);
    }

    #[test]
    fn test_encode_stopped() {
        assert_eq!(encode_fatal_error_kind(FatalErrorClassification::Stopped), 1);
    }

    #[test]
    fn test_encode_storage_error() {
        assert_eq!(encode_fatal_error_kind(FatalErrorClassification::StorageError), 2);
    }

    // ========================================================================
    // Roundtrip Tests
    // ========================================================================

    #[test]
    fn test_roundtrip_panicked() {
        let encoded = encode_fatal_error_kind(FatalErrorClassification::Panicked);
        assert_eq!(classify_fatal_error_kind(encoded), FatalErrorClassification::Panicked);
    }

    #[test]
    fn test_roundtrip_stopped() {
        let encoded = encode_fatal_error_kind(FatalErrorClassification::Stopped);
        assert_eq!(classify_fatal_error_kind(encoded), FatalErrorClassification::Stopped);
    }

    #[test]
    fn test_roundtrip_storage_error() {
        let encoded = encode_fatal_error_kind(FatalErrorClassification::StorageError);
        assert_eq!(classify_fatal_error_kind(encoded), FatalErrorClassification::StorageError);
    }

    // ========================================================================
    // Helper Method Tests
    // ========================================================================

    #[test]
    fn test_is_recoverable() {
        assert!(!FatalErrorClassification::Panicked.is_recoverable());
        assert!(FatalErrorClassification::Stopped.is_recoverable());
        assert!(!FatalErrorClassification::StorageError.is_recoverable());
    }

    #[test]
    fn test_requires_attention() {
        assert!(FatalErrorClassification::Panicked.requires_attention());
        assert!(!FatalErrorClassification::Stopped.requires_attention());
        assert!(FatalErrorClassification::StorageError.requires_attention());
    }
}
