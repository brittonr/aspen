//! Pure KV storage computation functions.
//!
//! This module contains pure functions for KV entry version computation,
//! TTL expiration, CAS validation, and lease entry construction.
//! All functions are deterministic and side-effect free.
//!
//! # Tiger Style
//!
//! - All calculations use saturating arithmetic to prevent overflow
//! - Deterministic behavior (no time, random, or I/O dependencies)
//! - Explicit error types for all failure modes
//! - Fixed size types prevent unbounded allocation

use super::calculate_expires_at_ms;

// ============================================================================
// KV Entry Version Computation
// ============================================================================

/// Version fields for a KV entry.
///
/// Returned by [`compute_kv_versions`] to provide all revision and version
/// information needed when creating or updating a KV entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KvVersions {
    /// Raft log index when key was first created.
    /// Preserved across updates, reset on delete+recreate.
    pub create_revision: i64,
    /// Raft log index of this modification.
    pub mod_revision: i64,
    /// Per-key version counter (1, 2, 3...).
    /// Reset to 1 on delete+recreate.
    pub version: i64,
}

/// Compute version fields for a KV entry.
///
/// This pure function encapsulates the versioning logic for KV operations:
/// - For new keys: create_revision = log_index, mod_revision = log_index, version = 1
/// - For existing keys: create_revision preserved, mod_revision = log_index, version incremented
///
/// # Arguments
///
/// * `existing_version` - Version info from existing entry, if any: (create_revision, version)
/// * `log_index` - Current Raft log index for this operation
///
/// # Returns
///
/// A [`KvVersions`] struct with the computed version fields.
///
/// # Example
///
/// ```
/// use aspen_raft::pure::kv::{compute_kv_versions, KvVersions};
///
/// // New key creation
/// let versions = compute_kv_versions(None, 100);
/// assert_eq!(versions, KvVersions {
///     create_revision: 100,
///     mod_revision: 100,
///     version: 1,
/// });
///
/// // Update existing key
/// let versions = compute_kv_versions(Some((50, 3)), 100);
/// assert_eq!(versions, KvVersions {
///     create_revision: 50,
///     mod_revision: 100,
///     version: 4,
/// });
/// ```
///
/// # Tiger Style
///
/// - Uses saturating arithmetic for version increment to prevent overflow
/// - Deterministic: same inputs always produce same output
/// - No panics for any input combination
#[inline]
pub fn compute_kv_versions(
    existing_version: Option<(i64, i64)>, // (create_revision, version)
    log_index: u64,
) -> KvVersions {
    match existing_version {
        Some((create_revision, version)) => KvVersions {
            create_revision,
            mod_revision: log_index as i64,
            version: version.saturating_add(1),
        },
        None => KvVersions {
            create_revision: log_index as i64,
            mod_revision: log_index as i64,
            version: 1,
        },
    }
}

// ============================================================================
// TTL Expiration Computation
// ============================================================================

/// Compute expiration timestamp for a key with optional TTL.
///
/// This is a convenience wrapper around [`calculate_expires_at_ms`] that handles
/// the optional TTL case. Returns `None` if no TTL is specified.
///
/// # Arguments
///
/// * `ttl_seconds` - Optional TTL in seconds. If `None` or `Some(0)`, no expiration.
/// * `now_ms` - Current timestamp in milliseconds since Unix epoch
///
/// # Returns
///
/// - `None` if no TTL specified or TTL is 0
/// - `Some(expires_at_ms)` with the absolute expiration timestamp
///
/// # Example
///
/// ```
/// use aspen_raft::pure::kv::compute_key_expiration;
///
/// let now_ms = 1704067200000; // Jan 1, 2024 00:00:00 UTC
///
/// // No TTL
/// assert_eq!(compute_key_expiration(None, now_ms), None);
/// assert_eq!(compute_key_expiration(Some(0), now_ms), None);
///
/// // 1 hour TTL
/// let expires = compute_key_expiration(Some(3600), now_ms);
/// assert_eq!(expires, Some(now_ms + 3600 * 1000));
/// ```
///
/// # Tiger Style
///
/// - Uses saturating arithmetic to prevent overflow
/// - Deterministic: same inputs always produce same output
#[inline]
pub fn compute_key_expiration(ttl_seconds: Option<u32>, now_ms: u64) -> Option<u64> {
    match ttl_seconds {
        None => None,
        Some(0) => None,
        Some(ttl) => Some(calculate_expires_at_ms(now_ms, ttl)),
    }
}

// ============================================================================
// CAS (Compare-and-Swap) Validation
// ============================================================================

/// CAS validation error types.
///
/// These represent the different ways a CAS precondition check can fail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CasValidationError {
    /// Expected key to exist but it was not found.
    KeyNotFound,
    /// Key exists but value does not match expected.
    ValueMismatch {
        /// Length of the expected value in bytes.
        expected_len: u32,
        /// Length of the actual value in bytes.
        actual_len: u32,
    },
}

/// Validate CAS preconditions for a compare-and-swap operation.
///
/// Compares the expected value against the current value to determine
/// if the CAS operation should proceed.
///
/// # Arguments
///
/// * `existing_value` - Current value in the store, if any
/// * `expected_value` - Value the caller expects to be present
///
/// # Returns
///
/// - `Ok(())` if CAS should proceed (values match)
/// - `Err(CasValidationError::KeyNotFound)` if key doesn't exist
/// - `Err(CasValidationError::ValueMismatch { .. })` if values don't match
///
/// # Example
///
/// ```
/// use aspen_raft::pure::kv::{validate_cas_precondition, CasValidationError};
///
/// // Matching values - CAS proceeds
/// assert!(validate_cas_precondition(Some(b"hello"), b"hello").is_ok());
///
/// // Key not found
/// assert_eq!(
///     validate_cas_precondition(None, b"hello"),
///     Err(CasValidationError::KeyNotFound)
/// );
///
/// // Value mismatch
/// assert_eq!(
///     validate_cas_precondition(Some(b"world"), b"hello"),
///     Err(CasValidationError::ValueMismatch {
///         expected_len: 5,
///         actual_len: 5,
///     })
/// );
/// ```
///
/// # Tiger Style
///
/// - Explicit error types for each failure mode
/// - No panics for any input
/// - Bounded length reporting (u32)
#[inline]
pub fn validate_cas_precondition(
    existing_value: Option<&[u8]>,
    expected_value: &[u8],
) -> Result<(), CasValidationError> {
    match existing_value {
        None => Err(CasValidationError::KeyNotFound),
        Some(actual) => {
            if actual == expected_value {
                Ok(())
            } else {
                Err(CasValidationError::ValueMismatch {
                    expected_len: expected_value.len().min(u32::MAX as usize) as u32,
                    actual_len: actual.len().min(u32::MAX as usize) as u32,
                })
            }
        }
    }
}

/// Validate CAS preconditions using string comparison.
///
/// Variant of [`validate_cas_precondition`] for string values, matching
/// the storage model where values are stored as strings.
///
/// # Arguments
///
/// * `existing_value` - Current value in the store, if any
/// * `expected_value` - Value the caller expects to be present
///
/// # Returns
///
/// - `Ok(())` if CAS should proceed
/// - `Err(CasValidationError::KeyNotFound)` if key doesn't exist
/// - `Err(CasValidationError::ValueMismatch { .. })` if values don't match
///
/// # Example
///
/// ```
/// use aspen_raft::pure::kv::{validate_cas_precondition_str, CasValidationError};
///
/// assert!(validate_cas_precondition_str(Some("hello"), "hello").is_ok());
/// assert_eq!(
///     validate_cas_precondition_str(None, "hello"),
///     Err(CasValidationError::KeyNotFound)
/// );
/// ```
#[inline]
pub fn validate_cas_precondition_str(
    existing_value: Option<&str>,
    expected_value: &str,
) -> Result<(), CasValidationError> {
    validate_cas_precondition(existing_value.map(str::as_bytes), expected_value.as_bytes())
}

/// Check CAS condition for optional expected value.
///
/// This handles the full CAS semantics including:
/// - Expected None + Actual None = condition met (key shouldn't exist and doesn't)
/// - Expected Some + Actual Some with matching values = condition met
/// - Any other combination = condition not met
///
/// # Arguments
///
/// * `expected` - Expected value (None means key should not exist)
/// * `current` - Current value in store (None means key doesn't exist)
///
/// # Returns
///
/// `true` if condition is met and CAS should proceed, `false` otherwise.
///
/// # Example
///
/// ```
/// use aspen_raft::pure::kv::check_cas_condition;
///
/// // Key shouldn't exist and doesn't
/// assert!(check_cas_condition(None, None));
///
/// // Values match
/// assert!(check_cas_condition(Some("hello"), Some("hello")));
///
/// // Key exists when expected not to
/// assert!(!check_cas_condition(None, Some("value")));
///
/// // Key doesn't exist when expected to
/// assert!(!check_cas_condition(Some("expected"), None));
///
/// // Values don't match
/// assert!(!check_cas_condition(Some("expected"), Some("actual")));
/// ```
#[inline]
pub fn check_cas_condition(expected: Option<&str>, current: Option<&str>) -> bool {
    match (expected, current) {
        (None, None) => true,                 // Expected no key, found no key
        (Some(exp), Some(cur)) => exp == cur, // Values match
        _ => false,                           // Mismatch
    }
}

// ============================================================================
// Lease Entry Construction
// ============================================================================

/// Data for a newly created lease entry.
///
/// Contains all the computed fields needed to store a lease.
/// The `keys` field starts empty and is populated as keys are attached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseEntryData {
    /// Time-to-live in seconds.
    pub ttl_seconds: u32,
    /// Absolute expiration timestamp in milliseconds since Unix epoch.
    pub expires_at_ms: u64,
    /// Keys attached to this lease (starts empty).
    pub keys: Vec<String>,
}

/// Create a new lease entry with computed expiration.
///
/// # Arguments
///
/// * `ttl_seconds` - Lease time-to-live in seconds
/// * `now_ms` - Current timestamp in milliseconds since Unix epoch
///
/// # Returns
///
/// A [`LeaseEntryData`] with the computed expiration and empty key list.
///
/// # Example
///
/// ```
/// use aspen_raft::pure::kv::create_lease_entry;
///
/// let now_ms = 1704067200000; // Jan 1, 2024 00:00:00 UTC
/// let lease = create_lease_entry(3600, now_ms); // 1 hour TTL
///
/// assert_eq!(lease.ttl_seconds, 3600);
/// assert_eq!(lease.expires_at_ms, now_ms + 3600 * 1000);
/// assert!(lease.keys.is_empty());
/// ```
///
/// # Tiger Style
///
/// - Uses saturating arithmetic via calculate_expires_at_ms
/// - Deterministic: same inputs always produce same output
/// - Bounded key vector (starts empty, filled by caller)
#[inline]
pub fn create_lease_entry(ttl_seconds: u32, now_ms: u64) -> LeaseEntryData {
    LeaseEntryData {
        ttl_seconds,
        expires_at_ms: calculate_expires_at_ms(now_ms, ttl_seconds),
        keys: Vec::new(),
    }
}

/// Calculate refreshed expiration time for a lease keepalive.
///
/// # Arguments
///
/// * `ttl_seconds` - Original lease TTL in seconds
/// * `now_ms` - Current timestamp in milliseconds since Unix epoch
///
/// # Returns
///
/// New expiration timestamp in milliseconds.
///
/// # Example
///
/// ```
/// use aspen_raft::pure::kv::compute_lease_refresh;
///
/// let now_ms = 1704067200000;
/// let ttl_seconds = 3600;
/// let new_expires = compute_lease_refresh(ttl_seconds, now_ms);
/// assert_eq!(new_expires, now_ms + 3600 * 1000);
/// ```
#[inline]
pub fn compute_lease_refresh(ttl_seconds: u32, now_ms: u64) -> u64 {
    calculate_expires_at_ms(now_ms, ttl_seconds)
}

/// Check if a lease has expired.
///
/// # Arguments
///
/// * `expires_at_ms` - Lease expiration timestamp
/// * `now_ms` - Current timestamp
///
/// # Returns
///
/// `true` if the lease has expired, `false` otherwise.
///
/// # Example
///
/// ```
/// use aspen_raft::pure::kv::is_lease_expired;
///
/// assert!(is_lease_expired(1000, 2000));   // Past expiration
/// assert!(!is_lease_expired(2000, 1000));  // Before expiration
/// assert!(!is_lease_expired(1000, 1000));  // At exact expiration time (not expired yet)
/// ```
#[inline]
pub fn is_lease_expired(expires_at_ms: u64, now_ms: u64) -> bool {
    now_ms > expires_at_ms
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // KV Version Computation Tests
    // ========================================================================

    #[test]
    fn test_compute_kv_versions_new_key() {
        let versions = compute_kv_versions(None, 100);
        assert_eq!(versions.create_revision, 100);
        assert_eq!(versions.mod_revision, 100);
        assert_eq!(versions.version, 1);
    }

    #[test]
    fn test_compute_kv_versions_update_existing() {
        let versions = compute_kv_versions(Some((50, 3)), 100);
        assert_eq!(versions.create_revision, 50); // Preserved
        assert_eq!(versions.mod_revision, 100); // Updated
        assert_eq!(versions.version, 4); // Incremented
    }

    #[test]
    fn test_compute_kv_versions_first_update() {
        let versions = compute_kv_versions(Some((10, 1)), 20);
        assert_eq!(versions.create_revision, 10);
        assert_eq!(versions.mod_revision, 20);
        assert_eq!(versions.version, 2);
    }

    #[test]
    fn test_compute_kv_versions_version_saturates() {
        let versions = compute_kv_versions(Some((1, i64::MAX)), 100);
        assert_eq!(versions.version, i64::MAX); // Saturated, no overflow
    }

    #[test]
    fn test_compute_kv_versions_log_index_zero() {
        let versions = compute_kv_versions(None, 0);
        assert_eq!(versions.create_revision, 0);
        assert_eq!(versions.mod_revision, 0);
        assert_eq!(versions.version, 1);
    }

    #[test]
    fn test_compute_kv_versions_large_log_index() {
        let versions = compute_kv_versions(None, u64::MAX);
        // i64::MAX is the maximum value after casting from u64
        assert_eq!(versions.create_revision, u64::MAX as i64);
        assert_eq!(versions.mod_revision, u64::MAX as i64);
        assert_eq!(versions.version, 1);
    }

    #[test]
    fn test_compute_kv_versions_deterministic() {
        let v1 = compute_kv_versions(Some((10, 5)), 100);
        let v2 = compute_kv_versions(Some((10, 5)), 100);
        assert_eq!(v1, v2);
    }

    // ========================================================================
    // TTL Expiration Computation Tests
    // ========================================================================

    #[test]
    fn test_compute_key_expiration_none() {
        assert_eq!(compute_key_expiration(None, 1000), None);
    }

    #[test]
    fn test_compute_key_expiration_zero() {
        assert_eq!(compute_key_expiration(Some(0), 1000), None);
    }

    #[test]
    fn test_compute_key_expiration_one_second() {
        let now_ms = 1000;
        let result = compute_key_expiration(Some(1), now_ms);
        assert_eq!(result, Some(2000));
    }

    #[test]
    fn test_compute_key_expiration_one_hour() {
        let now_ms = 1704067200000;
        let result = compute_key_expiration(Some(3600), now_ms);
        assert_eq!(result, Some(now_ms + 3600 * 1000));
    }

    #[test]
    fn test_compute_key_expiration_max_ttl() {
        let now_ms = 0;
        let result = compute_key_expiration(Some(u32::MAX), now_ms);
        assert_eq!(result, Some(4294967295000));
    }

    #[test]
    fn test_compute_key_expiration_overflow_protection() {
        let now_ms = u64::MAX - 1000;
        let result = compute_key_expiration(Some(u32::MAX), now_ms);
        assert_eq!(result, Some(u64::MAX)); // Saturates
    }

    // ========================================================================
    // CAS Validation Tests
    // ========================================================================

    #[test]
    fn test_validate_cas_matching_values() {
        let result = validate_cas_precondition(Some(b"hello"), b"hello");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_cas_key_not_found() {
        let result = validate_cas_precondition(None, b"hello");
        assert_eq!(result, Err(CasValidationError::KeyNotFound));
    }

    #[test]
    fn test_validate_cas_value_mismatch() {
        let result = validate_cas_precondition(Some(b"world"), b"hello");
        assert_eq!(
            result,
            Err(CasValidationError::ValueMismatch {
                expected_len: 5,
                actual_len: 5,
            })
        );
    }

    #[test]
    fn test_validate_cas_different_lengths() {
        let result = validate_cas_precondition(Some(b"hi"), b"hello");
        assert_eq!(
            result,
            Err(CasValidationError::ValueMismatch {
                expected_len: 5,
                actual_len: 2,
            })
        );
    }

    #[test]
    fn test_validate_cas_empty_values() {
        let result = validate_cas_precondition(Some(b""), b"");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_cas_empty_expected_nonempty_actual() {
        let result = validate_cas_precondition(Some(b"value"), b"");
        assert_eq!(
            result,
            Err(CasValidationError::ValueMismatch {
                expected_len: 0,
                actual_len: 5,
            })
        );
    }

    #[test]
    fn test_validate_cas_str_matching() {
        let result = validate_cas_precondition_str(Some("hello"), "hello");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_cas_str_not_found() {
        let result = validate_cas_precondition_str(None, "hello");
        assert_eq!(result, Err(CasValidationError::KeyNotFound));
    }

    #[test]
    fn test_validate_cas_str_mismatch() {
        let result = validate_cas_precondition_str(Some("world"), "hello");
        assert!(matches!(result, Err(CasValidationError::ValueMismatch { .. })));
    }

    #[test]
    fn test_check_cas_condition_both_none() {
        assert!(check_cas_condition(None, None));
    }

    #[test]
    fn test_check_cas_condition_matching_values() {
        assert!(check_cas_condition(Some("hello"), Some("hello")));
    }

    #[test]
    fn test_check_cas_condition_expected_none_got_some() {
        assert!(!check_cas_condition(None, Some("value")));
    }

    #[test]
    fn test_check_cas_condition_expected_some_got_none() {
        assert!(!check_cas_condition(Some("expected"), None));
    }

    #[test]
    fn test_check_cas_condition_values_mismatch() {
        assert!(!check_cas_condition(Some("expected"), Some("actual")));
    }

    #[test]
    fn test_check_cas_condition_empty_strings() {
        assert!(check_cas_condition(Some(""), Some("")));
    }

    // ========================================================================
    // Lease Entry Construction Tests
    // ========================================================================

    #[test]
    fn test_create_lease_entry_basic() {
        let now_ms = 1704067200000;
        let lease = create_lease_entry(3600, now_ms);

        assert_eq!(lease.ttl_seconds, 3600);
        assert_eq!(lease.expires_at_ms, now_ms + 3600 * 1000);
        assert!(lease.keys.is_empty());
    }

    #[test]
    fn test_create_lease_entry_zero_ttl() {
        let now_ms = 1000;
        let lease = create_lease_entry(0, now_ms);

        assert_eq!(lease.ttl_seconds, 0);
        assert_eq!(lease.expires_at_ms, 1000); // Expires immediately
        assert!(lease.keys.is_empty());
    }

    #[test]
    fn test_create_lease_entry_one_second() {
        let now_ms = 1000;
        let lease = create_lease_entry(1, now_ms);

        assert_eq!(lease.ttl_seconds, 1);
        assert_eq!(lease.expires_at_ms, 2000);
    }

    #[test]
    fn test_create_lease_entry_max_ttl() {
        let now_ms = 0;
        let lease = create_lease_entry(u32::MAX, now_ms);

        assert_eq!(lease.ttl_seconds, u32::MAX);
        assert_eq!(lease.expires_at_ms, 4294967295000);
    }

    #[test]
    fn test_create_lease_entry_overflow_protection() {
        let now_ms = u64::MAX - 1000;
        let lease = create_lease_entry(u32::MAX, now_ms);

        assert_eq!(lease.expires_at_ms, u64::MAX); // Saturates
    }

    #[test]
    fn test_create_lease_entry_deterministic() {
        let lease1 = create_lease_entry(3600, 1704067200000);
        let lease2 = create_lease_entry(3600, 1704067200000);
        assert_eq!(lease1, lease2);
    }

    #[test]
    fn test_compute_lease_refresh() {
        let now_ms = 1704067200000;
        let new_expires = compute_lease_refresh(3600, now_ms);
        assert_eq!(new_expires, now_ms + 3600 * 1000);
    }

    #[test]
    fn test_compute_lease_refresh_zero_ttl() {
        let now_ms = 1000;
        let new_expires = compute_lease_refresh(0, now_ms);
        assert_eq!(new_expires, 1000);
    }

    #[test]
    fn test_is_lease_expired_true() {
        assert!(is_lease_expired(1000, 2000));
    }

    #[test]
    fn test_is_lease_expired_false() {
        assert!(!is_lease_expired(2000, 1000));
    }

    #[test]
    fn test_is_lease_expired_at_boundary() {
        // At exact expiration time, lease is NOT yet expired
        assert!(!is_lease_expired(1000, 1000));
    }

    #[test]
    fn test_is_lease_expired_just_after() {
        assert!(is_lease_expired(1000, 1001));
    }
}

#[cfg(all(test, feature = "bolero"))]
mod property_tests {
    use bolero::check;

    use super::*;

    #[test]
    fn prop_compute_kv_versions_version_always_positive() {
        check!().with_type::<(Option<(i64, i64)>, u64)>().for_each(|(existing, log_index)| {
            let versions = compute_kv_versions(*existing, *log_index);
            assert!(versions.version >= 1, "Version should always be >= 1");
        });
    }

    #[test]
    fn prop_compute_kv_versions_mod_revision_equals_log_index() {
        check!().with_type::<(Option<(i64, i64)>, u64)>().for_each(|(existing, log_index)| {
            let versions = compute_kv_versions(*existing, *log_index);
            assert_eq!(versions.mod_revision, *log_index as i64, "mod_revision should equal log_index");
        });
    }

    #[test]
    fn prop_compute_kv_versions_create_preserved_on_update() {
        check!().with_type::<(i64, i64, u64)>().for_each(|(create_rev, version, log_index)| {
            let versions = compute_kv_versions(Some((*create_rev, *version)), *log_index);
            assert_eq!(versions.create_revision, *create_rev, "create_revision should be preserved on update");
        });
    }

    #[test]
    fn prop_compute_kv_versions_version_increments() {
        check!().with_type::<(i64, i64, u64)>().filter(|(_, version, _)| *version < i64::MAX).for_each(
            |(create_rev, version, log_index)| {
                let versions = compute_kv_versions(Some((*create_rev, *version)), *log_index);
                assert_eq!(versions.version, version + 1, "version should increment by 1");
            },
        );
    }

    #[test]
    fn prop_compute_key_expiration_none_or_some() {
        check!().with_type::<(Option<u32>, u64)>().for_each(|(ttl, now_ms)| {
            let result = compute_key_expiration(*ttl, *now_ms);
            match ttl {
                None | Some(0) => assert!(result.is_none()),
                Some(_) => assert!(result.is_some()),
            }
        });
    }

    #[test]
    fn prop_compute_key_expiration_monotonic() {
        check!().with_type::<(u32, u32, u64)>().filter(|(ttl1, ttl2, _)| *ttl1 > 0 && *ttl2 > 0).for_each(
            |(ttl1, ttl2, now_ms)| {
                let exp1 = compute_key_expiration(Some(*ttl1), *now_ms).unwrap();
                let exp2 = compute_key_expiration(Some(*ttl2), *now_ms).unwrap();
                if ttl1 <= ttl2 {
                    assert!(exp1 <= exp2, "Larger TTL should give larger expiration");
                }
            },
        );
    }

    #[test]
    fn prop_validate_cas_matching_always_succeeds() {
        check!().with_type::<Vec<u8>>().for_each(|data| {
            let result = validate_cas_precondition(Some(data.as_slice()), data.as_slice());
            assert!(result.is_ok(), "Matching values should always succeed");
        });
    }

    #[test]
    fn prop_validate_cas_none_always_fails() {
        check!().with_type::<Vec<u8>>().for_each(|expected| {
            let result = validate_cas_precondition(None, expected.as_slice());
            assert_eq!(result, Err(CasValidationError::KeyNotFound));
        });
    }

    #[test]
    fn prop_check_cas_condition_symmetric_none() {
        // Both None should always match
        assert!(check_cas_condition(None, None));
    }

    #[test]
    fn prop_check_cas_condition_same_string_matches() {
        check!().with_type::<String>().for_each(|s| {
            assert!(check_cas_condition(Some(s.as_str()), Some(s.as_str())));
        });
    }

    #[test]
    fn prop_create_lease_entry_keys_empty() {
        check!().with_type::<(u32, u64)>().for_each(|(ttl, now_ms)| {
            let lease = create_lease_entry(*ttl, *now_ms);
            assert!(lease.keys.is_empty(), "New lease should have no keys");
        });
    }

    #[test]
    fn prop_create_lease_entry_ttl_preserved() {
        check!().with_type::<(u32, u64)>().for_each(|(ttl, now_ms)| {
            let lease = create_lease_entry(*ttl, *now_ms);
            assert_eq!(lease.ttl_seconds, *ttl, "TTL should be preserved");
        });
    }

    #[test]
    fn prop_is_lease_expired_consistent() {
        check!().with_type::<(u64, u64)>().for_each(|(expires, now)| {
            let expired = is_lease_expired(*expires, *now);
            if *now > *expires {
                assert!(expired, "Should be expired when now > expires");
            } else {
                assert!(!expired, "Should not be expired when now <= expires");
            }
        });
    }
}
