//! Pure conversion functions for Raft request/response encoding.
//!
//! This module contains pure functions for converting between external API types
//! and compact wire formats used in Raft log entries. All functions are
//! deterministic and side-effect free.
//!
//! # Wire Format Encoding
//!
//! Operations are encoded as tuples for compact serialization:
//! - Batch operations: `(is_set: bool, key: String, value: String)`
//! - Conditions: `(type: u8, key: String, expected: String)`
//! - Compare: `(target: u8, op: u8, key: String, value: String)`
//! - Txn operations: `(op_type: u8, key: String, value: String)`
//!
//! # Tiger Style
//!
//! - Explicit type mappings documented with constants
//! - Deterministic encoding (same input always produces same output)
//! - No panics for any input

// ============================================================================
// Condition Type Constants
// ============================================================================

/// Condition type: value equals expected.
pub const CONDITION_VALUE_EQUALS: u8 = 0;

/// Condition type: key exists.
pub const CONDITION_KEY_EXISTS: u8 = 1;

/// Condition type: key does not exist.
pub const CONDITION_KEY_NOT_EXISTS: u8 = 2;

// ============================================================================
// Compare Target Constants
// ============================================================================

/// Compare target: compare value.
pub const COMPARE_TARGET_VALUE: u8 = 0;

/// Compare target: compare version.
pub const COMPARE_TARGET_VERSION: u8 = 1;

/// Compare target: compare create revision.
pub const COMPARE_TARGET_CREATE_REVISION: u8 = 2;

/// Compare target: compare mod revision.
pub const COMPARE_TARGET_MOD_REVISION: u8 = 3;

// ============================================================================
// Compare Operation Constants
// ============================================================================

/// Compare operation: equal.
pub const COMPARE_OP_EQUAL: u8 = 0;

/// Compare operation: not equal.
pub const COMPARE_OP_NOT_EQUAL: u8 = 1;

/// Compare operation: greater than.
pub const COMPARE_OP_GREATER: u8 = 2;

/// Compare operation: less than.
pub const COMPARE_OP_LESS: u8 = 3;

// ============================================================================
// Transaction Operation Constants
// ============================================================================

/// Transaction operation: put.
pub const TXN_OP_PUT: u8 = 0;

/// Transaction operation: delete.
pub const TXN_OP_DELETE: u8 = 1;

/// Transaction operation: get.
pub const TXN_OP_GET: u8 = 2;

/// Transaction operation: range scan.
pub const TXN_OP_RANGE: u8 = 3;

// ============================================================================
// Condition Validation
// ============================================================================

/// Check if a batch condition is met.
///
/// Evaluates a condition against the current value in the store.
///
/// # Arguments
///
/// * `condition_type` - Type of condition (0=ValueEquals, 1=KeyExists, 2=KeyNotExists)
/// * `current_value` - Current value in store (None if key doesn't exist)
/// * `expected` - Expected value for ValueEquals condition
///
/// # Returns
///
/// `true` if the condition is met, `false` otherwise.
///
/// # Example
///
/// ```
/// use aspen_raft::verified::conversion::{
///     check_condition_met, CONDITION_VALUE_EQUALS, CONDITION_KEY_EXISTS, CONDITION_KEY_NOT_EXISTS
/// };
///
/// // ValueEquals: value matches
/// assert!(check_condition_met(CONDITION_VALUE_EQUALS, Some("hello"), "hello"));
///
/// // ValueEquals: value doesn't match
/// assert!(!check_condition_met(CONDITION_VALUE_EQUALS, Some("world"), "hello"));
///
/// // KeyExists: key exists
/// assert!(check_condition_met(CONDITION_KEY_EXISTS, Some("any"), ""));
///
/// // KeyNotExists: key doesn't exist
/// assert!(check_condition_met(CONDITION_KEY_NOT_EXISTS, None, ""));
/// ```
#[inline]
pub fn check_condition_met(condition_type: u8, current_value: Option<&str>, expected: &str) -> bool {
    match condition_type {
        CONDITION_VALUE_EQUALS => current_value.map(|v| v == expected).unwrap_or(false),
        CONDITION_KEY_EXISTS => current_value.is_some(),
        CONDITION_KEY_NOT_EXISTS => current_value.is_none(),
        _ => false, // Unknown condition type
    }
}

/// Result of evaluating batch conditions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionResult {
    /// All conditions met, proceed with operations.
    AllMet,
    /// A condition failed at the given index.
    Failed {
        /// Index of the first failed condition.
        index: u32,
    },
}

/// Evaluate a list of conditions against current values.
///
/// Checks conditions in order, returning early on first failure.
///
/// # Arguments
///
/// * `conditions` - List of (type, key, expected) tuples
/// * `get_value` - Function to get current value for a key
///
/// # Returns
///
/// [`ConditionResult::AllMet`] if all conditions pass, or
/// [`ConditionResult::Failed`] with the index of the first failure.
///
/// # Example
///
/// ```
/// use aspen_raft::verified::conversion::{
///     evaluate_conditions, ConditionResult, CONDITION_KEY_EXISTS
/// };
/// use std::collections::HashMap;
///
/// let store: HashMap<&str, &str> = [("key1", "value1")].into_iter().collect();
///
/// let conditions = vec![
///     (CONDITION_KEY_EXISTS, "key1".to_string(), String::new()),
/// ];
///
/// let result = evaluate_conditions(&conditions, |k| store.get(k).map(|v| *v));
/// assert_eq!(result, ConditionResult::AllMet);
/// ```
pub fn evaluate_conditions<F>(conditions: &[(u8, String, String)], get_value: F) -> ConditionResult
where F: Fn(&str) -> Option<&str> {
    for (i, (cond_type, key, expected)) in conditions.iter().enumerate() {
        let current = get_value(key);
        if !check_condition_met(*cond_type, current, expected) {
            return ConditionResult::Failed { index: i as u32 };
        }
    }
    ConditionResult::AllMet
}

// ============================================================================
// Batch Operation Encoding/Decoding
// ============================================================================

/// Compact batch operation representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompactBatchOp {
    /// Set a key to a value.
    Set { key: String, value: String },
    /// Delete a key.
    Delete { key: String },
}

/// Decode a compact batch operation tuple.
///
/// # Arguments
///
/// * `is_set` - true for Set, false for Delete
/// * `key` - Key to operate on
/// * `value` - Value (only used for Set)
///
/// # Returns
///
/// A [`CompactBatchOp`] enum variant.
///
/// # Example
///
/// ```
/// use aspen_raft::verified::conversion::{decode_batch_op, CompactBatchOp};
///
/// let set_op = decode_batch_op(true, "key".to_string(), "value".to_string());
/// assert!(matches!(set_op, CompactBatchOp::Set { .. }));
///
/// let del_op = decode_batch_op(false, "key".to_string(), String::new());
/// assert!(matches!(del_op, CompactBatchOp::Delete { .. }));
/// ```
#[inline]
pub fn decode_batch_op(is_set: bool, key: String, value: String) -> CompactBatchOp {
    if is_set {
        CompactBatchOp::Set { key, value }
    } else {
        CompactBatchOp::Delete { key }
    }
}

/// Encode a batch operation to compact tuple format.
///
/// # Arguments
///
/// * `op` - The batch operation to encode
///
/// # Returns
///
/// A tuple `(is_set, key, value)` suitable for serialization.
///
/// # Example
///
/// ```
/// use aspen_raft::verified::conversion::{encode_batch_op, CompactBatchOp};
///
/// let (is_set, key, value) = encode_batch_op(&CompactBatchOp::Set {
///     key: "k".to_string(),
///     value: "v".to_string(),
/// });
/// assert!(is_set);
/// assert_eq!(key, "k");
/// assert_eq!(value, "v");
/// ```
#[inline]
pub fn encode_batch_op(op: &CompactBatchOp) -> (bool, String, String) {
    match op {
        CompactBatchOp::Set { key, value } => (true, key.clone(), value.clone()),
        CompactBatchOp::Delete { key } => (false, key.clone(), String::new()),
    }
}

// ============================================================================
// Batch Size Validation
// ============================================================================

/// Error when batch size exceeds maximum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchSizeError {
    /// Actual batch size in bytes.
    pub size_bytes: u32,
    /// Maximum allowed size in bytes.
    pub max_bytes: u32,
}

/// Validate that a batch size is within limits.
///
/// # Arguments
///
/// * `batch_size` - Number of operations in the batch
/// * `max_allowed` - Maximum allowed operations
///
/// # Returns
///
/// - `Ok(())` if within limits
/// - `Err(BatchSizeError)` if exceeds maximum
///
/// # Example
///
/// ```
/// use aspen_raft::verified::conversion::validate_batch_size;
///
/// assert!(validate_batch_size(10, 100).is_ok());
/// assert!(validate_batch_size(150, 100).is_err());
/// ```
#[inline]
pub fn validate_batch_size(batch_size: u32, max_allowed: u32) -> Result<(), BatchSizeError> {
    if batch_size > max_allowed {
        Err(BatchSizeError {
            size_bytes: batch_size,
            max_bytes: max_allowed,
        })
    } else {
        Ok(())
    }
}

// ============================================================================
// TTL Expiration Computation
// ============================================================================

/// Compute absolute expiration timestamp from TTL seconds.
///
/// Converts a relative TTL in seconds to an absolute millisecond timestamp.
///
/// # Arguments
///
/// * `now_ms` - Current timestamp in milliseconds since Unix epoch
/// * `ttl_seconds` - Time-to-live in seconds
///
/// # Returns
///
/// Absolute expiration timestamp in milliseconds.
///
/// # Tiger Style
///
/// Uses saturating arithmetic to prevent overflow.
///
/// # Example
///
/// ```
/// use aspen_raft::verified::conversion::compute_ttl_expiration_ms;
///
/// let now_ms = 1704067200000; // Jan 1, 2024
/// let expires = compute_ttl_expiration_ms(now_ms, 3600); // 1 hour
/// assert_eq!(expires, now_ms + 3600 * 1000);
/// ```
#[inline]
pub fn compute_ttl_expiration_ms(now_ms: u64, ttl_seconds: u32) -> u64 {
    now_ms.saturating_add((ttl_seconds as u64).saturating_mul(1000))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Condition Validation Tests
    // ========================================================================

    #[test]
    fn test_check_condition_value_equals_match() {
        assert!(check_condition_met(CONDITION_VALUE_EQUALS, Some("hello"), "hello"));
    }

    #[test]
    fn test_check_condition_value_equals_mismatch() {
        assert!(!check_condition_met(CONDITION_VALUE_EQUALS, Some("world"), "hello"));
    }

    #[test]
    fn test_check_condition_value_equals_key_missing() {
        assert!(!check_condition_met(CONDITION_VALUE_EQUALS, None, "hello"));
    }

    #[test]
    fn test_check_condition_key_exists_present() {
        assert!(check_condition_met(CONDITION_KEY_EXISTS, Some("any"), ""));
    }

    #[test]
    fn test_check_condition_key_exists_missing() {
        assert!(!check_condition_met(CONDITION_KEY_EXISTS, None, ""));
    }

    #[test]
    fn test_check_condition_key_not_exists_missing() {
        assert!(check_condition_met(CONDITION_KEY_NOT_EXISTS, None, ""));
    }

    #[test]
    fn test_check_condition_key_not_exists_present() {
        assert!(!check_condition_met(CONDITION_KEY_NOT_EXISTS, Some("value"), ""));
    }

    #[test]
    fn test_check_condition_unknown_type() {
        assert!(!check_condition_met(255, Some("value"), "value"));
    }

    #[test]
    fn test_evaluate_conditions_all_met() {
        use std::collections::HashMap;

        let store: HashMap<&str, &str> = [("key1", "value1"), ("key2", "value2")].into_iter().collect();

        let conditions = vec![
            (CONDITION_VALUE_EQUALS, "key1".to_string(), "value1".to_string()),
            (CONDITION_KEY_EXISTS, "key2".to_string(), String::new()),
        ];

        let result = evaluate_conditions(&conditions, |k| store.get(k).copied());
        assert_eq!(result, ConditionResult::AllMet);
    }

    #[test]
    fn test_evaluate_conditions_first_fails() {
        use std::collections::HashMap;

        let store: HashMap<&str, &str> = HashMap::new();

        let conditions = vec![
            (CONDITION_KEY_EXISTS, "key1".to_string(), String::new()),
            (CONDITION_KEY_EXISTS, "key2".to_string(), String::new()),
        ];

        let result = evaluate_conditions(&conditions, |k| store.get(k).copied());
        assert_eq!(result, ConditionResult::Failed { index: 0 });
    }

    #[test]
    fn test_evaluate_conditions_second_fails() {
        use std::collections::HashMap;

        let store: HashMap<&str, &str> = [("key1", "value1")].into_iter().collect();

        let conditions = vec![
            (CONDITION_KEY_EXISTS, "key1".to_string(), String::new()),
            (CONDITION_KEY_EXISTS, "key2".to_string(), String::new()),
        ];

        let result = evaluate_conditions(&conditions, |k| store.get(k).copied());
        assert_eq!(result, ConditionResult::Failed { index: 1 });
    }

    #[test]
    fn test_evaluate_conditions_empty() {
        let result = evaluate_conditions(&[], |_| None);
        assert_eq!(result, ConditionResult::AllMet);
    }

    // ========================================================================
    // Batch Operation Tests
    // ========================================================================

    #[test]
    fn test_decode_batch_op_set() {
        let op = decode_batch_op(true, "key".to_string(), "value".to_string());
        assert_eq!(op, CompactBatchOp::Set {
            key: "key".to_string(),
            value: "value".to_string(),
        });
    }

    #[test]
    fn test_decode_batch_op_delete() {
        let op = decode_batch_op(false, "key".to_string(), String::new());
        assert_eq!(op, CompactBatchOp::Delete { key: "key".to_string() });
    }

    #[test]
    fn test_encode_batch_op_set() {
        let op = CompactBatchOp::Set {
            key: "k".to_string(),
            value: "v".to_string(),
        };
        let (is_set, key, value) = encode_batch_op(&op);
        assert!(is_set);
        assert_eq!(key, "k");
        assert_eq!(value, "v");
    }

    #[test]
    fn test_encode_batch_op_delete() {
        let op = CompactBatchOp::Delete { key: "k".to_string() };
        let (is_set, key, value) = encode_batch_op(&op);
        assert!(!is_set);
        assert_eq!(key, "k");
        assert!(value.is_empty());
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let original = CompactBatchOp::Set {
            key: "test".to_string(),
            value: "data".to_string(),
        };
        let (is_set, key, value) = encode_batch_op(&original);
        let decoded = decode_batch_op(is_set, key, value);
        assert_eq!(original, decoded);
    }

    // ========================================================================
    // Batch Size Tests
    // ========================================================================

    #[test]
    fn test_validate_batch_size_ok() {
        assert!(validate_batch_size(10, 100).is_ok());
    }

    #[test]
    fn test_validate_batch_size_exact() {
        assert!(validate_batch_size(100, 100).is_ok());
    }

    #[test]
    fn test_validate_batch_size_exceeded() {
        let err = validate_batch_size(150, 100).unwrap_err();
        assert_eq!(err.size_bytes, 150);
        assert_eq!(err.max_bytes, 100);
    }

    #[test]
    fn test_validate_batch_size_zero() {
        assert!(validate_batch_size(0, 100).is_ok());
    }

    // ========================================================================
    // TTL Expiration Tests
    // ========================================================================

    #[test]
    fn test_compute_ttl_expiration_basic() {
        let now_ms = 1000;
        let expires = compute_ttl_expiration_ms(now_ms, 60);
        assert_eq!(expires, 61000);
    }

    #[test]
    fn test_compute_ttl_expiration_zero_ttl() {
        let now_ms = 1000;
        let expires = compute_ttl_expiration_ms(now_ms, 0);
        assert_eq!(expires, 1000);
    }

    #[test]
    fn test_compute_ttl_expiration_overflow_saturates() {
        let now_ms = u64::MAX - 1000;
        let expires = compute_ttl_expiration_ms(now_ms, u32::MAX);
        assert_eq!(expires, u64::MAX);
    }
}

#[cfg(all(test, feature = "bolero"))]
mod property_tests {
    use bolero::check;

    use super::*;

    #[test]
    fn prop_decode_encode_roundtrip() {
        check!().with_type::<(bool, String, String)>().for_each(|(is_set, key, value)| {
            let op = decode_batch_op(*is_set, key.clone(), value.clone());
            let (is_set2, key2, value2) = encode_batch_op(&op);
            let op2 = decode_batch_op(is_set2, key2, value2);
            assert_eq!(op, op2);
        });
    }

    #[test]
    fn prop_validate_batch_size_consistent() {
        check!().with_type::<(usize, u32)>().for_each(|(size, max)| {
            let result = validate_batch_size(*size, *max);
            if *size <= *max as usize {
                assert!(result.is_ok());
            } else {
                assert!(result.is_err());
            }
        });
    }

    #[test]
    fn prop_ttl_expiration_monotonic() {
        check!().with_type::<(u64, u32, u32)>().for_each(|(now, ttl1, ttl2)| {
            let exp1 = compute_ttl_expiration_ms(*now, *ttl1);
            let exp2 = compute_ttl_expiration_ms(*now, *ttl2);
            if ttl1 <= ttl2 {
                assert!(exp1 <= exp2);
            }
        });
    }
}
