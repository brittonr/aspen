//! Pure validation functions for RPC handler logic.
//!
//! These functions encapsulate validation logic that can be tested
//! independently of the async handler code.
//!
//! # Tiger Style
//!
//! - Pure functions with no side effects
//! - Deterministic: same inputs always produce same outputs
//! - Explicit error types

/// Check if a lock is held by the specified holder.
///
/// This validation is used when releasing or renewing a lock to ensure
/// the requester actually holds the lock they're trying to modify.
///
/// # Arguments
///
/// * `current_holder_id` - The holder ID currently holding the lock
/// * `current_fencing_token` - The fencing token of the current lock
/// * `claimed_holder_id` - The holder ID the requester claims to have
/// * `claimed_fencing_token` - The fencing token the requester claims to have
///
/// # Returns
///
/// `true` if the claimed credentials match the current lock holder.
///
/// # Example
///
/// ```
/// use aspen_rpc_handlers::verified::is_lock_owner;
///
/// // Matching credentials
/// assert!(is_lock_owner("client-1", 42, "client-1", 42));
///
/// // Different holder
/// assert!(!is_lock_owner("client-1", 42, "client-2", 42));
///
/// // Different token
/// assert!(!is_lock_owner("client-1", 42, "client-1", 43));
///
/// // Both different
/// assert!(!is_lock_owner("client-1", 42, "client-2", 43));
/// ```
#[inline]
pub fn is_lock_owner(
    current_holder_id: &str,
    current_fencing_token: u64,
    claimed_holder_id: &str,
    claimed_fencing_token: u64,
) -> bool {
    current_holder_id == claimed_holder_id && current_fencing_token == claimed_fencing_token
}

/// Lock ownership check result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockOwnershipError {
    /// The holder ID doesn't match.
    HolderIdMismatch {
        /// The actual holder ID.
        actual: String,
        /// The claimed holder ID.
        claimed: String,
    },
    /// The fencing token doesn't match.
    FencingTokenMismatch {
        /// The actual fencing token.
        actual: u64,
        /// The claimed fencing token.
        claimed: u64,
    },
    /// Both holder ID and fencing token don't match.
    BothMismatch {
        /// The actual holder ID.
        actual_holder: String,
        /// The claimed holder ID.
        claimed_holder: String,
        /// The actual fencing token.
        actual_token: u64,
        /// The claimed fencing token.
        claimed_token: u64,
    },
}

/// Validate lock ownership with detailed error information.
///
/// This function provides detailed error information about which
/// aspect of the ownership check failed.
///
/// # Arguments
///
/// * `current_holder_id` - The holder ID currently holding the lock
/// * `current_fencing_token` - The fencing token of the current lock
/// * `claimed_holder_id` - The holder ID the requester claims to have
/// * `claimed_fencing_token` - The fencing token the requester claims to have
///
/// # Returns
///
/// - `Ok(())` if ownership is valid
/// - `Err(LockOwnershipError)` with details about the mismatch
///
/// # Example
///
/// ```
/// use aspen_rpc_handlers::verified::{validate_lock_ownership, LockOwnershipError};
///
/// // Valid ownership
/// assert!(validate_lock_ownership("client-1", 42, "client-1", 42).is_ok());
///
/// // Holder mismatch
/// let err = validate_lock_ownership("client-1", 42, "client-2", 42);
/// assert!(matches!(err, Err(LockOwnershipError::HolderIdMismatch { .. })));
/// ```
#[inline]
pub fn validate_lock_ownership(
    current_holder_id: &str,
    current_fencing_token: u64,
    claimed_holder_id: &str,
    claimed_fencing_token: u64,
) -> Result<(), LockOwnershipError> {
    let holder_matches = current_holder_id == claimed_holder_id;
    let token_matches = current_fencing_token == claimed_fencing_token;

    match (holder_matches, token_matches) {
        (true, true) => Ok(()),
        (false, false) => Err(LockOwnershipError::BothMismatch {
            actual_holder: current_holder_id.to_string(),
            claimed_holder: claimed_holder_id.to_string(),
            actual_token: current_fencing_token,
            claimed_token: claimed_fencing_token,
        }),
        (false, true) => Err(LockOwnershipError::HolderIdMismatch {
            actual: current_holder_id.to_string(),
            claimed: claimed_holder_id.to_string(),
        }),
        (true, false) => Err(LockOwnershipError::FencingTokenMismatch {
            actual: current_fencing_token,
            claimed: claimed_fencing_token,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // is_lock_owner tests
    // ========================================================================

    #[test]
    fn test_is_lock_owner_matching() {
        assert!(is_lock_owner("client-1", 42, "client-1", 42));
    }

    #[test]
    fn test_is_lock_owner_holder_mismatch() {
        assert!(!is_lock_owner("client-1", 42, "client-2", 42));
    }

    #[test]
    fn test_is_lock_owner_token_mismatch() {
        assert!(!is_lock_owner("client-1", 42, "client-1", 43));
    }

    #[test]
    fn test_is_lock_owner_both_mismatch() {
        assert!(!is_lock_owner("client-1", 42, "client-2", 43));
    }

    #[test]
    fn test_is_lock_owner_empty_holder() {
        assert!(is_lock_owner("", 0, "", 0));
        assert!(!is_lock_owner("", 0, "client", 0));
    }

    #[test]
    fn test_is_lock_owner_large_token() {
        assert!(is_lock_owner("client", u64::MAX, "client", u64::MAX));
        assert!(!is_lock_owner("client", u64::MAX, "client", u64::MAX - 1));
    }

    // ========================================================================
    // validate_lock_ownership tests
    // ========================================================================

    #[test]
    fn test_validate_lock_ownership_valid() {
        assert!(validate_lock_ownership("client-1", 42, "client-1", 42).is_ok());
    }

    #[test]
    fn test_validate_lock_ownership_holder_mismatch() {
        let result = validate_lock_ownership("client-1", 42, "client-2", 42);
        match result {
            Err(LockOwnershipError::HolderIdMismatch { actual, claimed }) => {
                assert_eq!(actual, "client-1");
                assert_eq!(claimed, "client-2");
            }
            _ => panic!("Expected HolderIdMismatch"),
        }
    }

    #[test]
    fn test_validate_lock_ownership_token_mismatch() {
        let result = validate_lock_ownership("client-1", 42, "client-1", 43);
        match result {
            Err(LockOwnershipError::FencingTokenMismatch { actual, claimed }) => {
                assert_eq!(actual, 42);
                assert_eq!(claimed, 43);
            }
            _ => panic!("Expected FencingTokenMismatch"),
        }
    }

    #[test]
    fn test_validate_lock_ownership_both_mismatch() {
        let result = validate_lock_ownership("client-1", 42, "client-2", 43);
        match result {
            Err(LockOwnershipError::BothMismatch {
                actual_holder,
                claimed_holder,
                actual_token,
                claimed_token,
            }) => {
                assert_eq!(actual_holder, "client-1");
                assert_eq!(claimed_holder, "client-2");
                assert_eq!(actual_token, 42);
                assert_eq!(claimed_token, 43);
            }
            _ => panic!("Expected BothMismatch"),
        }
    }
}
