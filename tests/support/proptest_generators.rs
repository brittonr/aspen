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

// Re-export constants for boundary testing
pub use aspen::raft::constants::MAX_KEY_SIZE;
pub use aspen::raft::constants::MAX_SETMULTI_KEYS;
pub use aspen::raft::constants::MAX_VALUE_SIZE;
use aspen::raft::types::AppRequest;
use aspen::raft::types::AppTypeConfig;
use aspen::raft::types::NodeId;
use openraft::LogId;
use openraft::testing::log_id;
use proptest::prelude::*;

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
        ("[a-z][a-z0-9_]{0,19}", prop::string::string_regex("[a-zA-Z0-9 ]{1,100}").unwrap(),),
        // Special characters in values
        ("[a-z]{1,10}", prop::string::string_regex("[!@#$%^&*()\\-_=+\\[\\]{};:'\",.<>?/]{1,50}").unwrap(),),
        // Whitespace variations
        ("[a-z]{1,10}", prop::string::string_regex("[ \t]{1,20}").unwrap(),),
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
        prop::collection::vec(arbitrary_key_value(), 1..20).prop_map(|pairs| AppRequest::SetMulti { pairs }),
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
    prop::collection::vec(arbitrary_key_value(), 1..max_pairs).prop_map(|pairs| AppRequest::SetMulti { pairs })
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
    prop::collection::vec(arbitrary_key_value(), (MAX_SETMULTI_KEYS as usize + 1)..(MAX_SETMULTI_KEYS as usize + 50))
}

/// Generator for SetMulti at exactly the limit.
pub fn setmulti_at_limit() -> impl Strategy<Value = Vec<(String, String)>> {
    prop::collection::vec(arbitrary_key_value(), MAX_SETMULTI_KEYS as usize)
}

// ============================================================================
// NodeId String Parsing Generators
// ============================================================================

/// Generator for valid NodeId string representations.
///
/// NodeId uses u64 internally, so valid strings are numeric.
pub fn arbitrary_node_id_string() -> impl Strategy<Value = String> {
    prop_oneof![
        // Small valid IDs
        (0u64..100u64).prop_map(|n| n.to_string()),
        // Large valid IDs
        (u64::MAX - 1000..=u64::MAX).prop_map(|n| n.to_string()),
        // Zero (edge case)
        Just("0".to_string()),
    ]
}

/// Generator for invalid NodeId strings (for rejection testing).
///
/// These should fail parsing when used with NodeId::from_str even when trimmed.
pub fn invalid_node_id_string() -> impl Strategy<Value = String> {
    prop_oneof![
        // Empty string
        Just("".to_string()),
        // Negative numbers
        (-1000i64..-1i64).prop_map(|n| n.to_string()),
        // Non-numeric strings
        "[a-zA-Z]{1,10}".prop_map(String::from),
        // Overflow (larger than u64::MAX)
        Just("18446744073709551616".to_string()), // u64::MAX + 1
        // Mixed alphanumeric
        "[0-9][a-z]{1,5}".prop_map(String::from),
        // Decimal numbers
        Just("123.456".to_string()),
        // Hexadecimal notation (not accepted by u64 parse)
        Just("0x123".to_string()),
        // With non-digit characters
        Just("123abc".to_string()),
    ]
}

// ============================================================================
// Log Store Generators
// ============================================================================

/// Generator for a sequence of log entries to append.
///
/// Returns (start_index, entries) where entries are contiguous from start_index.
pub fn arbitrary_log_append_sequence(max_entries: usize) -> impl Strategy<Value = (u64, Vec<AppRequest>)> {
    (1u64..100u64, prop::collection::vec(arbitrary_app_request(), 1..max_entries))
}

/// Generator for Raft vote tuples (term, voted_for).
pub fn arbitrary_vote() -> impl Strategy<Value = (u64, NodeId)> {
    (arbitrary_term(), arbitrary_node_id())
}

/// Generator for log truncation points within a given range.
pub fn arbitrary_truncation_point(max_index: u64) -> impl Strategy<Value = u64> {
    1u64..=max_index
}

/// Generator for purge boundaries (committed index up to which logs can be deleted).
pub fn arbitrary_purge_point(max_committed: u64) -> impl Strategy<Value = u64> {
    0u64..=max_committed
}

// ============================================================================
// Network Simulation Generators
// ============================================================================

/// Network delay configuration for simulation testing.
#[derive(Debug, Clone)]
pub struct NetworkDelayConfig {
    pub min_latency_ms: u64,
    pub max_latency_ms: u64,
    pub jitter_percent: u8,
}

/// Generator for network delay configurations.
pub fn arbitrary_network_delay_config() -> impl Strategy<Value = NetworkDelayConfig> {
    prop_oneof![
        // Fast local network
        Just(NetworkDelayConfig {
            min_latency_ms: 0,
            max_latency_ms: 5,
            jitter_percent: 10,
        }),
        // Normal WAN
        Just(NetworkDelayConfig {
            min_latency_ms: 10,
            max_latency_ms: 100,
            jitter_percent: 20,
        }),
        // High latency (cross-continent)
        Just(NetworkDelayConfig {
            min_latency_ms: 100,
            max_latency_ms: 300,
            jitter_percent: 30,
        }),
        // Variable (random)
        (0u64..50u64, 50u64..500u64, 0u8..50u8).prop_map(|(min, max, jitter)| {
            NetworkDelayConfig {
                min_latency_ms: min,
                max_latency_ms: max.max(min + 1),
                jitter_percent: jitter,
            }
        }),
    ]
}

/// Operation sequence for testing concurrent operations.
#[derive(Debug, Clone)]
pub enum TestOperation {
    Write(String, String),
    Read(String),
    Delete(String),
    Scan(String, u32),
}

/// Generator for sequences of test operations.
pub fn arbitrary_operation_sequence(max_ops: usize) -> impl Strategy<Value = Vec<TestOperation>> {
    prop::collection::vec(
        prop_oneof![
            arbitrary_key_value().prop_map(|(k, v)| TestOperation::Write(k, v)),
            "[a-z][a-z0-9_]{0,19}".prop_map(TestOperation::Read),
            "[a-z][a-z0-9_]{0,19}".prop_map(TestOperation::Delete),
            ("[a-z]{1,5}", 1u32..100u32).prop_map(|(prefix, limit)| TestOperation::Scan(prefix, limit)),
        ],
        1..max_ops,
    )
}

// ============================================================================
// Gossip Discovery Generators
// ============================================================================

/// Generator for peer announcement timestamps.
///
/// Returns milliseconds since epoch, biased towards recent times.
pub fn arbitrary_timestamp_ms() -> impl Strategy<Value = u64> {
    prop_oneof![
        // Recent (within last hour)
        Just(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64),
        // Older timestamps
        (1_700_000_000_000u64..1_800_000_000_000u64),
        // Zero (edge case)
        Just(0u64),
    ]
}

/// Generator for endpoint addresses as strings.
///
/// These are NOT real Iroh endpoint addresses, just for serialization testing.
pub fn arbitrary_endpoint_addr_bytes() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 32..64)
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
    (0..count).map(|i| (format!("key_{}", i), format!("value_{}", i), start_index + i as u64)).collect()
}

/// Create a sequence of Delete entries for testing.
pub fn create_delete_entries(count: usize, start_index: u64) -> Vec<(String, u64)> {
    (0..count).map(|i| (format!("key_{}", i), start_index + i as u64)).collect()
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

        #[test]
        fn test_arbitrary_node_id_string_produces_valid_numeric(
            s in arbitrary_node_id_string()
        ) {
            let parsed: Result<u64, _> = s.parse();
            prop_assert!(parsed.is_ok(), "Valid NodeId string should parse as u64: {}", s);
        }

        #[test]
        fn test_invalid_node_id_string_fails_u64_parse(
            s in invalid_node_id_string()
        ) {
            let parsed: Result<u64, _> = s.parse();
            prop_assert!(parsed.is_err(), "Invalid NodeId string should fail u64 parse: {}", s);
        }

        #[test]
        fn test_arbitrary_vote_produces_valid_term_and_node(
            (term, node_id) in arbitrary_vote()
        ) {
            prop_assert!((1..100).contains(&term), "Term should be in valid range");
            prop_assert!(u64::from(node_id) < 10, "NodeId should be in valid range");
        }

        #[test]
        fn test_arbitrary_log_append_sequence_nonempty(
            (start, entries) in arbitrary_log_append_sequence(20)
        ) {
            prop_assert!(start >= 1, "Start index should be >= 1");
            prop_assert!(!entries.is_empty(), "Entries should not be empty");
            prop_assert!(entries.len() < 20, "Entries should respect max");
        }

        #[test]
        fn test_network_delay_config_valid_ranges(
            config in arbitrary_network_delay_config()
        ) {
            prop_assert!(
                config.max_latency_ms >= config.min_latency_ms,
                "Max latency should be >= min latency"
            );
            prop_assert!(config.jitter_percent <= 100, "Jitter should be <= 100%");
        }

        #[test]
        fn test_operation_sequence_nonempty(
            ops in arbitrary_operation_sequence(50)
        ) {
            prop_assert!(!ops.is_empty(), "Operation sequence should not be empty");
            prop_assert!(ops.len() < 50, "Operation sequence should respect max");
        }
    }
}
