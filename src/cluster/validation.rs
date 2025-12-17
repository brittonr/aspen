//! Pure validation functions for cluster configuration.
//!
//! This module extracts pure validation logic from `NodeConfig::validate()`
//! following the "Functional Core, Imperative Shell" pattern. All functions
//! are deterministic and side-effect free, enabling:
//! - Unit testing with explicit inputs/outputs
//! - Property-based testing with Bolero
//! - Reuse across different configuration sources
//!
//! # Tiger Style
//!
//! - Fail-fast semantics: Return first error encountered
//! - Explicit bounds: All limits documented and enforced
//! - Clear error messages: Context for operators to diagnose issues

use snafu::Snafu;

/// Validation errors for configuration fields.
#[derive(Debug, Snafu, PartialEq, Eq)]
pub enum ValidationError {
    #[snafu(display("node_id must be non-zero"))]
    NodeIdZero,

    #[snafu(display("cluster cookie cannot be empty"))]
    CookieEmpty,

    #[snafu(display("heartbeat_interval_ms must be greater than 0"))]
    HeartbeatIntervalZero,

    #[snafu(display("election_timeout_min_ms must be greater than 0"))]
    ElectionTimeoutMinZero,

    #[snafu(display("election_timeout_max_ms must be greater than 0"))]
    ElectionTimeoutMaxZero,

    #[snafu(display(
        "election_timeout_max_ms ({max}) must be greater than election_timeout_min_ms ({min})"
    ))]
    ElectionTimeoutOrder { min: u64, max: u64 },

    #[snafu(display("iroh secret key must be 64 hex characters (32 bytes), got {len}"))]
    SecretKeyLength { len: usize },

    #[snafu(display("iroh secret key must be valid hex: {reason}"))]
    SecretKeyInvalidHex { reason: String },
}

/// Required length for Iroh secret key in hex characters.
pub const SECRET_KEY_HEX_LENGTH: usize = 64;

// ============================================================================
// Core Validation Functions
// ============================================================================

/// Validate that node_id is non-zero.
///
/// # Tiger Style
///
/// Node ID 0 is reserved/invalid in Raft to enable sentinel values.
#[inline]
pub fn validate_node_id(node_id: u64) -> Result<(), ValidationError> {
    if node_id == 0 {
        return Err(ValidationError::NodeIdZero);
    }
    Ok(())
}

/// Validate that cluster cookie is non-empty.
///
/// The cookie is used for cluster authentication and gossip topic derivation.
#[inline]
pub fn validate_cookie(cookie: &str) -> Result<(), ValidationError> {
    if cookie.is_empty() {
        return Err(ValidationError::CookieEmpty);
    }
    Ok(())
}

/// Validate Raft timing configuration.
///
/// # Timing Constraints
///
/// - All timeouts must be > 0
/// - election_timeout_max_ms > election_timeout_min_ms (required for randomization)
///
/// # Arguments
///
/// * `heartbeat_interval_ms` - Raft heartbeat interval
/// * `election_timeout_min_ms` - Minimum election timeout
/// * `election_timeout_max_ms` - Maximum election timeout
#[inline]
pub fn validate_raft_timings(
    heartbeat_interval_ms: u64,
    election_timeout_min_ms: u64,
    election_timeout_max_ms: u64,
) -> Result<(), ValidationError> {
    if heartbeat_interval_ms == 0 {
        return Err(ValidationError::HeartbeatIntervalZero);
    }

    if election_timeout_min_ms == 0 {
        return Err(ValidationError::ElectionTimeoutMinZero);
    }

    if election_timeout_max_ms == 0 {
        return Err(ValidationError::ElectionTimeoutMaxZero);
    }

    if election_timeout_max_ms <= election_timeout_min_ms {
        return Err(ValidationError::ElectionTimeoutOrder {
            min: election_timeout_min_ms,
            max: election_timeout_max_ms,
        });
    }

    Ok(())
}

/// Check Raft timing sanity (returns warnings, not errors).
///
/// These are recommendations, not hard requirements:
/// - Heartbeat should be < election timeout (Raft correctness)
/// - Election timeout < 1000ms may be too aggressive
/// - Election timeout > 10000ms may be too conservative
///
/// # Returns
///
/// A vector of warning messages. Empty if configuration is optimal.
pub fn check_raft_timing_sanity(
    heartbeat_interval_ms: u64,
    election_timeout_min_ms: u64,
    election_timeout_max_ms: u64,
) -> Vec<String> {
    let mut warnings = Vec::new();

    if heartbeat_interval_ms >= election_timeout_min_ms {
        warnings.push(format!(
            "heartbeat interval ({heartbeat_interval_ms}ms) should be less than election timeout ({election_timeout_min_ms}ms) for Raft correctness"
        ));
    }

    if election_timeout_min_ms < 1000 {
        warnings.push(format!(
            "election timeout min ({election_timeout_min_ms}ms) < 1000ms may be too aggressive for production"
        ));
    }

    if election_timeout_max_ms > 10000 {
        warnings.push(format!(
            "election timeout max ({election_timeout_max_ms}ms) > 10000ms may be too conservative"
        ));
    }

    warnings
}

/// Validate Iroh secret key format.
///
/// # Requirements
///
/// - Must be exactly 64 hex characters (32 bytes)
/// - Must be valid hexadecimal
///
/// # Arguments
///
/// * `key_hex` - Optional hex-encoded secret key
///
/// # Returns
///
/// `Ok(())` if key is None or valid, `Err` if key is invalid.
pub fn validate_secret_key(key_hex: Option<&str>) -> Result<(), ValidationError> {
    let Some(key) = key_hex else {
        return Ok(());
    };

    if key.len() != SECRET_KEY_HEX_LENGTH {
        return Err(ValidationError::SecretKeyLength { len: key.len() });
    }

    if let Err(e) = hex::decode(key) {
        return Err(ValidationError::SecretKeyInvalidHex {
            reason: e.to_string(),
        });
    }

    Ok(())
}

/// Check if HTTP port is a commonly conflicting default.
///
/// # Returns
///
/// `Some(warning)` if port may conflict, `None` otherwise.
pub fn check_http_port(port: u16) -> Option<String> {
    match port {
        8080 => Some("using default HTTP port 8080 (may conflict in production)".to_string()),
        80 => Some("using privileged HTTP port 80 (may require root)".to_string()),
        443 => Some("using privileged HTTPS port 443 (may require root)".to_string()),
        _ => None,
    }
}

/// Check if disk usage is concerning.
///
/// # Arguments
///
/// * `usage_percent` - Current disk usage percentage (0-100)
///
/// # Returns
///
/// `Some(warning)` if usage > 80%, `None` otherwise.
pub fn check_disk_usage(usage_percent: u64) -> Option<String> {
    if usage_percent > 80 {
        Some(format!("disk space usage high ({}% > 80%)", usage_percent))
    } else {
        None
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Validate all core configuration fields.
///
/// This is a convenience function that calls individual validators.
/// For logging warnings, use the `check_*` functions separately.
///
/// # Arguments
///
/// * `node_id` - Logical node identifier
/// * `cookie` - Cluster authentication cookie
/// * `heartbeat_interval_ms` - Raft heartbeat interval
/// * `election_timeout_min_ms` - Minimum election timeout
/// * `election_timeout_max_ms` - Maximum election timeout
/// * `secret_key_hex` - Optional Iroh secret key
pub fn validate_core_config(
    node_id: u64,
    cookie: &str,
    heartbeat_interval_ms: u64,
    election_timeout_min_ms: u64,
    election_timeout_max_ms: u64,
    secret_key_hex: Option<&str>,
) -> Result<(), ValidationError> {
    validate_node_id(node_id)?;
    validate_cookie(cookie)?;
    validate_raft_timings(
        heartbeat_interval_ms,
        election_timeout_min_ms,
        election_timeout_max_ms,
    )?;
    validate_secret_key(secret_key_hex)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Node ID Tests
    // ========================================================================

    #[test]
    fn test_node_id_zero_invalid() {
        assert_eq!(validate_node_id(0), Err(ValidationError::NodeIdZero));
    }

    #[test]
    fn test_node_id_valid() {
        assert!(validate_node_id(1).is_ok());
        assert!(validate_node_id(u64::MAX).is_ok());
    }

    // ========================================================================
    // Cookie Tests
    // ========================================================================

    #[test]
    fn test_cookie_empty_invalid() {
        assert_eq!(validate_cookie(""), Err(ValidationError::CookieEmpty));
    }

    #[test]
    fn test_cookie_valid() {
        assert!(validate_cookie("aspen-cookie").is_ok());
        assert!(validate_cookie("x").is_ok());
    }

    // ========================================================================
    // Timing Tests
    // ========================================================================

    #[test]
    fn test_heartbeat_zero_invalid() {
        assert_eq!(
            validate_raft_timings(0, 1500, 3000),
            Err(ValidationError::HeartbeatIntervalZero)
        );
    }

    #[test]
    fn test_election_min_zero_invalid() {
        assert_eq!(
            validate_raft_timings(500, 0, 3000),
            Err(ValidationError::ElectionTimeoutMinZero)
        );
    }

    #[test]
    fn test_election_max_zero_invalid() {
        assert_eq!(
            validate_raft_timings(500, 1500, 0),
            Err(ValidationError::ElectionTimeoutMaxZero)
        );
    }

    #[test]
    fn test_election_order_invalid() {
        assert_eq!(
            validate_raft_timings(500, 3000, 1500),
            Err(ValidationError::ElectionTimeoutOrder {
                min: 3000,
                max: 1500
            })
        );
        assert_eq!(
            validate_raft_timings(500, 1500, 1500),
            Err(ValidationError::ElectionTimeoutOrder {
                min: 1500,
                max: 1500
            })
        );
    }

    #[test]
    fn test_timings_valid() {
        assert!(validate_raft_timings(500, 1500, 3000).is_ok());
        assert!(validate_raft_timings(100, 200, 300).is_ok());
    }

    #[test]
    fn test_timing_sanity_warnings() {
        // Heartbeat >= election timeout
        let warnings = check_raft_timing_sanity(2000, 1500, 3000);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("heartbeat"));

        // Election timeout too small
        let warnings = check_raft_timing_sanity(100, 500, 1000);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("aggressive"));

        // Election timeout too large
        let warnings = check_raft_timing_sanity(500, 5000, 15000);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("conservative"));

        // All good
        let warnings = check_raft_timing_sanity(500, 1500, 3000);
        assert!(warnings.is_empty());
    }

    // ========================================================================
    // Secret Key Tests
    // ========================================================================

    #[test]
    fn test_secret_key_none_valid() {
        assert!(validate_secret_key(None).is_ok());
    }

    #[test]
    fn test_secret_key_correct_length() {
        let valid_key = "a".repeat(64);
        assert!(validate_secret_key(Some(&valid_key)).is_ok());
    }

    #[test]
    fn test_secret_key_wrong_length() {
        assert_eq!(
            validate_secret_key(Some("abc")),
            Err(ValidationError::SecretKeyLength { len: 3 })
        );
        assert_eq!(
            validate_secret_key(Some(&"a".repeat(63))),
            Err(ValidationError::SecretKeyLength { len: 63 })
        );
        assert_eq!(
            validate_secret_key(Some(&"a".repeat(65))),
            Err(ValidationError::SecretKeyLength { len: 65 })
        );
    }

    #[test]
    fn test_secret_key_invalid_hex() {
        let invalid = "g".repeat(64); // 'g' is not valid hex
        let result = validate_secret_key(Some(&invalid));
        assert!(matches!(
            result,
            Err(ValidationError::SecretKeyInvalidHex { .. })
        ));
    }

    // ========================================================================
    // HTTP Port Tests
    // ========================================================================

    #[test]
    fn test_http_port_warnings() {
        assert!(check_http_port(8080).is_some());
        assert!(check_http_port(80).is_some());
        assert!(check_http_port(443).is_some());
        assert!(check_http_port(3000).is_none());
        assert!(check_http_port(9090).is_none());
    }

    // ========================================================================
    // Disk Usage Tests
    // ========================================================================

    #[test]
    fn test_disk_usage_warnings() {
        assert!(check_disk_usage(50).is_none());
        assert!(check_disk_usage(80).is_none());
        assert!(check_disk_usage(81).is_some());
        assert!(check_disk_usage(100).is_some());
    }

    // ========================================================================
    // Core Config Tests
    // ========================================================================

    #[test]
    fn test_validate_core_config_all_valid() {
        let result = validate_core_config(1, "aspen-cookie", 500, 1500, 3000, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_core_config_fails_fast() {
        // Should fail on node_id first
        let result = validate_core_config(0, "", 0, 0, 0, None);
        assert_eq!(result, Err(ValidationError::NodeIdZero));
    }
}

#[cfg(all(test, feature = "bolero"))]
mod property_tests {
    use super::*;
    use bolero::check;

    #[test]
    fn prop_node_id_zero_always_fails() {
        assert!(validate_node_id(0).is_err());
    }

    #[test]
    fn prop_node_id_nonzero_always_succeeds() {
        check!()
            .with_type::<u64>()
            .filter(|&id| id != 0)
            .for_each(|id| {
                assert!(validate_node_id(*id).is_ok());
            });
    }

    #[test]
    fn prop_valid_timings_succeed() {
        check!()
            .with_type::<(u64, u64, u64)>()
            .filter(|(hb, min, max)| *hb > 0 && *min > 0 && *max > *min)
            .for_each(|(heartbeat, min, max)| {
                assert!(validate_raft_timings(*heartbeat, *min, *max).is_ok());
            });
    }

    #[test]
    fn prop_secret_key_length_64_passes() {
        check!().with_type::<[u8; 32]>().for_each(|bytes| {
            let hex = hex::encode(bytes);
            assert!(validate_secret_key(Some(&hex)).is_ok());
        });
    }
}
