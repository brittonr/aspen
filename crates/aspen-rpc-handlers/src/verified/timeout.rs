//! Pure timeout normalization functions.
//!
//! These functions encapsulate the common pattern of converting timeout values
//! where 0 means "no timeout" to Option types.
//!
//! # Tiger Style
//!
//! - Pure functions with no side effects
//! - Deterministic: same inputs always produce same outputs

/// Normalize a timeout value where 0 means no timeout.
///
/// This is a common pattern in RPC handlers where clients send 0 to indicate
/// "wait forever" or "no timeout". This function converts that convention
/// to an Option type.
///
/// # Arguments
///
/// * `timeout_ms` - Timeout in milliseconds, where 0 means no timeout
///
/// # Returns
///
/// - `None` if timeout_ms is 0 (no timeout / wait forever)
/// - `Some(timeout_ms)` if timeout_ms is non-zero
///
/// # Example
///
/// ```
/// use aspen_rpc_handlers::verified::normalize_timeout_ms;
///
/// assert_eq!(normalize_timeout_ms(0), None);
/// assert_eq!(normalize_timeout_ms(5000), Some(5000));
/// assert_eq!(normalize_timeout_ms(1), Some(1));
/// ```
#[inline]
pub const fn normalize_timeout_ms(timeout_ms: u64) -> Option<u64> {
    if timeout_ms == 0 { None } else { Some(timeout_ms) }
}

/// Normalize a timeout value where 0 means no timeout (u32 version).
///
/// Same as [`normalize_timeout_ms`] but for u32 values commonly used in
/// some API types.
///
/// # Example
///
/// ```
/// use aspen_rpc_handlers::verified::normalize_timeout_ms_u32;
///
/// assert_eq!(normalize_timeout_ms_u32(0), None);
/// assert_eq!(normalize_timeout_ms_u32(5000), Some(5000));
/// ```
#[inline]
pub const fn normalize_timeout_ms_u32(timeout_ms: u32) -> Option<u32> {
    if timeout_ms == 0 { None } else { Some(timeout_ms) }
}

/// Check if a timeout value indicates "wait forever".
///
/// # Arguments
///
/// * `timeout_ms` - Timeout in milliseconds
///
/// # Returns
///
/// `true` if timeout_ms is 0 (no timeout / wait forever)
///
/// # Example
///
/// ```
/// use aspen_rpc_handlers::verified::is_indefinite_timeout;
///
/// assert!(is_indefinite_timeout(0));
/// assert!(!is_indefinite_timeout(5000));
/// ```
#[inline]
pub const fn is_indefinite_timeout(timeout_ms: u64) -> bool {
    timeout_ms == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_timeout_zero() {
        assert_eq!(normalize_timeout_ms(0), None);
    }

    #[test]
    fn test_normalize_timeout_nonzero() {
        assert_eq!(normalize_timeout_ms(1), Some(1));
        assert_eq!(normalize_timeout_ms(5000), Some(5000));
        assert_eq!(normalize_timeout_ms(u64::MAX), Some(u64::MAX));
    }

    #[test]
    fn test_normalize_timeout_u32_zero() {
        assert_eq!(normalize_timeout_ms_u32(0), None);
    }

    #[test]
    fn test_normalize_timeout_u32_nonzero() {
        assert_eq!(normalize_timeout_ms_u32(1), Some(1));
        assert_eq!(normalize_timeout_ms_u32(5000), Some(5000));
        assert_eq!(normalize_timeout_ms_u32(u32::MAX), Some(u32::MAX));
    }

    #[test]
    fn test_is_indefinite_timeout_true() {
        assert!(is_indefinite_timeout(0));
    }

    #[test]
    fn test_is_indefinite_timeout_false() {
        assert!(!is_indefinite_timeout(1));
        assert!(!is_indefinite_timeout(5000));
        assert!(!is_indefinite_timeout(u64::MAX));
    }
}
