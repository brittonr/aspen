//! Centralized proptest generators and strategies for Aspen tests.
//!
//! This module provides reusable property-based testing generators that:
//! - Reduce code duplication across test files
//! - Ensure consistent test data generation
//! - Provide parameterized generators for edge cases
//! - Support distributed systems scenario generation
//!
//! Tiger Style: All generators respect resource bounds from `constants.rs`.

#![allow(dead_code)] // Many generators not yet used across all test files

use proptest::prelude::*;

use aspen::raft::types::{AppRequest, AppTypeConfig, NodeId};
use openraft::LogId;
use openraft::testing::log_id;

// Re-export constants for boundary testing
pub use aspen::raft::constants::{MAX_KEY_SIZE, MAX_SETMULTI_KEYS, MAX_VALUE_SIZE};

// ============================================================================
// Core Key-Value Generators
// ============================================================================

/// Standard key-value generator for basic property tests.
///
/// Keys: 1-20 alphanumeric characters (lowercase + digits + underscore)
/// Values: 1-100 alphanumeric characters with spaces
///
/// Use `arbitrary_key_value_with_edge_cases()` for more comprehensive testing.
pub fn arbitrary_key_value() -> impl Strategy<Value = (String, String)> {
    (
        "[a-z][a-z0-9_]{0,19}",                                     // Key: 1-20 chars
        prop::string::string_regex("[a-zA-Z0-9 ]{1,100}").unwrap(), // Value: 1-100 chars
    )
}

/// Key-value generator with edge case coverage.
///
/// Includes:
/// - Normal alphanumeric keys/values
/// - Special characters in values
/// - Whitespace variations
/// - Near-boundary sizes
pub fn arbitrary_key_value_with_edge_cases() -> impl Strategy<Value = (String, String)> {
    prop_oneof![
        // Normal case (70% weight)
        (
            "[a-z][a-z0-9_]{0,19}",
            prop::string::string_regex("[a-zA-Z0-9 ]{1,100}").unwrap(),
        ),
        // Special characters in values
        (
            "[a-z]{1,10}",
            prop::string::string_regex("[!@#$%^&*()\\-_=+\\[\\]{};:'\",.<>?/]{1,50}").unwrap(),
        ),
        // Whitespace variations
        (
            "[a-z]{1,10}",
            prop::string::string_regex("[ \t]{1,20}").unwrap(),
        ),
        // Longer keys (near boundary)
        ("[a-z][a-z0-9_]{50,100}", "[a-zA-Z0-9]{1,50}",),
    ]
}

/// Generator for keys with size distribution matching real workloads.
///
/// - 70% small keys (1-5 chars)
/// - 20% medium keys (6-50 chars)
/// - 10% large keys (50-200 chars)
pub fn skewed_key_sizes() -> impl Strategy<Value = String> {
    prop_oneof![
        7 => "[a-z]{1,5}",      // Small (70%)
        2 => "[a-z]{6,50}",     // Medium (20%)
        1 => "[a-z]{50,200}",   // Large (10%)
    ]
}

// ============================================================================
// Log Entry Generators
// ============================================================================

/// Helper function to create a log ID with given term, node, and index.
pub fn make_log_id(term: u64, node: u64, index: u64) -> LogId<AppTypeConfig> {
    log_id::<AppTypeConfig>(term, NodeId::from(node), index)
}

/// Generator for valid Raft terms.
pub fn arbitrary_term() -> impl Strategy<Value = u64> {
    1u64..100u64
}

/// Generator for valid node IDs.
pub fn arbitrary_node_id() -> impl Strategy<Value = NodeId> {
    (0u64..10u64).prop_map(NodeId::from)
}

/// Generator for valid log indices.
pub fn arbitrary_log_index() -> impl Strategy<Value = u64> {
    1u64..1000u64
}

/// Generator for AppRequest covering all operation types.
///
/// Generates Set, SetMulti, Delete, and DeleteMulti with balanced distribution.
pub fn arbitrary_app_request() -> impl Strategy<Value = AppRequest> {
    prop_oneof![
        // Set operation (25%)
        arbitrary_key_value().prop_map(|(key, value)| AppRequest::Set { key, value }),
        // SetMulti operation (25%)
        prop::collection::vec(arbitrary_key_value(), 1..20)
            .prop_map(|pairs| AppRequest::SetMulti { pairs }),
        // Delete operation (25%)
        "[a-z][a-z0-9_]{0,19}".prop_map(|key| AppRequest::Delete { key }),
        // DeleteMulti operation (25%)
        prop::collection::vec("[a-z][a-z0-9_]{0,19}".prop_map(String::from), 1..10)
            .prop_map(|keys| AppRequest::DeleteMulti { keys }),
    ]
}

/// Generator for Set operations only.
pub fn arbitrary_set_request() -> impl Strategy<Value = AppRequest> {
    arbitrary_key_value().prop_map(|(key, value)| AppRequest::Set { key, value })
}

/// Generator for Delete operations only.
pub fn arbitrary_delete_request() -> impl Strategy<Value = AppRequest> {
    "[a-z][a-z0-9_]{0,19}".prop_map(|key| AppRequest::Delete { key })
}

/// Generator for SetMulti operations with configurable size.
pub fn arbitrary_setmulti_request(max_pairs: usize) -> impl Strategy<Value = AppRequest> {
    prop::collection::vec(arbitrary_key_value(), 1..max_pairs)
        .prop_map(|pairs| AppRequest::SetMulti { pairs })
}

/// Generator for DeleteMulti operations with configurable size.
pub fn arbitrary_deletemulti_request(max_keys: usize) -> impl Strategy<Value = AppRequest> {
    prop::collection::vec("[a-z][a-z0-9_]{0,19}".prop_map(String::from), 1..max_keys)
        .prop_map(|keys| AppRequest::DeleteMulti { keys })
}

// ============================================================================
// Boundary and Edge Case Generators
// ============================================================================

/// Generator for keys at the MAX_KEY_SIZE boundary.
///
/// Tiger Style: Tests that size limits are enforced correctly.
pub fn boundary_key() -> impl Strategy<Value = String> {
    prop_oneof![
        // Just under limit
        Just("x".repeat(MAX_KEY_SIZE as usize - 1)),
        // Exactly at limit
        Just("x".repeat(MAX_KEY_SIZE as usize)),
        // Just over limit (for rejection testing)
        Just("x".repeat(MAX_KEY_SIZE as usize + 1)),
    ]
}

/// Generator for values at the MAX_VALUE_SIZE boundary.
///
/// Note: Generates up to 2MB values for boundary testing.
/// Use sparingly due to memory cost.
pub fn boundary_value() -> impl Strategy<Value = String> {
    prop_oneof![
        // Small value
        Just("small".to_string()),
        // Medium value (100KB)
        Just("x".repeat(100 * 1024)),
        // Just under limit
        Just("x".repeat(MAX_VALUE_SIZE as usize - 1)),
        // Exactly at limit
        Just("x".repeat(MAX_VALUE_SIZE as usize)),
    ]
}

/// Generator for oversized values (exceeds MAX_VALUE_SIZE).
///
/// Use for testing rejection of oversized payloads.
pub fn oversized_value() -> impl Strategy<Value = String> {
    (MAX_VALUE_SIZE as usize + 1..MAX_VALUE_SIZE as usize + 1000).prop_map(|size| "x".repeat(size))
}

/// Generator for SetMulti that exceeds MAX_SETMULTI_KEYS.
///
/// Use for testing rejection of oversized batches.
pub fn oversized_setmulti() -> impl Strategy<Value = Vec<(String, String)>> {
    prop::collection::vec(
        arbitrary_key_value(),
        (MAX_SETMULTI_KEYS as usize + 1)..(MAX_SETMULTI_KEYS as usize + 50),
    )
}

/// Generator for SetMulti at exactly the limit.
pub fn setmulti_at_limit() -> impl Strategy<Value = Vec<(String, String)>> {
    prop::collection::vec(arbitrary_key_value(), MAX_SETMULTI_KEYS as usize)
}

// ============================================================================
// Distributed Systems Generators
// ============================================================================

/// Generator for network latency durations.
pub fn arbitrary_latency_ms() -> impl Strategy<Value = u64> {
    prop_oneof![
        5 => 0u64..10u64,      // Fast (50%)
        3 => 10u64..100u64,    // Normal (30%)
        2 => 100u64..1000u64,  // Slow (20%)
    ]
}

/// Generator for packet loss rates (0.0 to 0.5).
pub fn arbitrary_packet_loss() -> impl Strategy<Value = f32> {
    prop_oneof![
        7 => Just(0.0f32),           // No loss (70%)
        2 => 0.01f32..0.1f32,        // Low loss (20%)
        1 => 0.1f32..0.5f32,         // High loss (10%)
    ]
}

/// Generator for membership configurations (voters, learners).
pub fn arbitrary_membership() -> impl Strategy<Value = (Vec<NodeId>, Vec<NodeId>)> {
    (
        prop::collection::vec(arbitrary_node_id(), 1..5), // 1-4 voters
        prop::collection::vec(arbitrary_node_id(), 0..3), // 0-2 learners
    )
}

// ============================================================================
// Test Helper Functions
// ============================================================================

/// Create a sequence of Set entries for testing.
pub fn create_set_entries(count: usize, start_index: u64) -> Vec<(String, String, u64)> {
    (0..count)
        .map(|i| {
            (
                format!("key_{}", i),
                format!("value_{}", i),
                start_index + i as u64,
            )
        })
        .collect()
}

/// Create a sequence of Delete entries for testing.
pub fn create_delete_entries(count: usize, start_index: u64) -> Vec<(String, u64)> {
    (0..count)
        .map(|i| (format!("key_{}", i), start_index + i as u64))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn test_arbitrary_key_value_produces_valid_keys(
            (key, value) in arbitrary_key_value()
        ) {
            prop_assert!(!key.is_empty(), "Keys should not be empty");
            prop_assert!(key.len() <= 20, "Keys should be <= 20 chars");
            prop_assert!(!value.is_empty(), "Values should not be empty");
            prop_assert!(value.len() <= 100, "Values should be <= 100 chars");
        }

        #[test]
        fn test_arbitrary_app_request_covers_all_variants(
            requests in prop::collection::vec(arbitrary_app_request(), 100..200)
        ) {
            let has_set = requests.iter().any(|r| matches!(r, AppRequest::Set { .. }));
            let has_set_multi = requests
                .iter()
                .any(|r| matches!(r, AppRequest::SetMulti { .. }));
            let has_delete = requests
                .iter()
                .any(|r| matches!(r, AppRequest::Delete { .. }));
            let has_delete_multi = requests
                .iter()
                .any(|r| matches!(r, AppRequest::DeleteMulti { .. }));

            // With 100+ requests, we should see all variants
            // (probability of missing one is negligible with 25% each)
            prop_assert!(has_set, "Set operations should be generated");
            prop_assert!(has_set_multi, "SetMulti operations should be generated");
            prop_assert!(has_delete, "Delete operations should be generated");
            prop_assert!(has_delete_multi, "DeleteMulti operations should be generated");
        }

        #[test]
        fn test_boundary_key_at_limits(
            key in boundary_key()
        ) {
            prop_assert!(
                key.len() >= MAX_KEY_SIZE as usize - 1,
                "Boundary keys should be near the limit"
            );
            prop_assert!(
                key.len() <= MAX_KEY_SIZE as usize + 1,
                "Boundary keys should not exceed limit + 1"
            );
        }

        #[test]
        fn test_oversized_setmulti_exceeds_limit(
            pairs in oversized_setmulti()
        ) {
            prop_assert!(
                pairs.len() > MAX_SETMULTI_KEYS as usize,
                "Oversized SetMulti should exceed limit"
            );
        }
    }
}
