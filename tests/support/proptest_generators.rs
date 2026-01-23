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
        Just(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or(std::time::Duration::ZERO)
                .as_millis() as u64
        ),
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

// ============================================================================
// Forge COB Generators
// ============================================================================

use aspen::forge::CobChange;
use aspen::forge::CobOperation;
use aspen::forge::CobType;

/// Generator for COB types.
pub fn arbitrary_cob_type() -> impl Strategy<Value = CobType> {
    prop_oneof![
        Just(CobType::Issue),
        Just(CobType::Patch),
        Just(CobType::Review),
        Just(CobType::Discussion),
    ]
}

/// Generator for issue titles.
pub fn arbitrary_issue_title() -> impl Strategy<Value = String> {
    prop_oneof![
        // Short titles
        "[A-Z][a-z]{2,20}",
        // Multi-word titles
        "[A-Z][a-z]{2,10} [a-z]{2,10}",
        // With numbers
        "[A-Z][a-z]{2,10} #[0-9]{1,4}",
    ]
}

/// Generator for issue/patch bodies.
pub fn arbitrary_cob_body() -> impl Strategy<Value = String> {
    prop_oneof![
        // Short body
        "[A-Za-z0-9 ]{10,50}",
        // Multi-line body
        "[A-Za-z0-9 .!?]{50,200}",
        // Empty (valid for some operations)
        Just("".to_string()),
    ]
}

/// Generator for labels.
pub fn arbitrary_label() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("bug".to_string()),
        Just("feature".to_string()),
        Just("enhancement".to_string()),
        Just("documentation".to_string()),
        Just("urgent".to_string()),
        Just("help-wanted".to_string()),
        "[a-z]{3,15}",
    ]
}

/// Generator for label lists.
pub fn arbitrary_labels() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(arbitrary_label(), 0..5)
}

/// Generator for comment bodies.
pub fn arbitrary_comment_body() -> impl Strategy<Value = String> {
    "[A-Za-z0-9 .,!?]{5,200}"
}

/// Generator for issue operations (suitable for testing Issue COBs).
pub fn arbitrary_issue_operation() -> impl Strategy<Value = CobOperation> {
    prop_oneof![
        // CreateIssue (only valid as root)
        (arbitrary_issue_title(), arbitrary_cob_body(), arbitrary_labels())
            .prop_map(|(title, body, labels)| CobOperation::CreateIssue { title, body, labels }),
        // Comment
        arbitrary_comment_body().prop_map(|body| CobOperation::Comment { body }),
        // Label operations
        arbitrary_label().prop_map(|label| CobOperation::AddLabel { label }),
        arbitrary_label().prop_map(|label| CobOperation::RemoveLabel { label }),
        // State transitions
        prop::option::of("[A-Za-z ]{5,50}").prop_map(|reason| CobOperation::Close { reason }),
        Just(CobOperation::Reopen),
        // Title/Body edits
        arbitrary_issue_title().prop_map(|title| CobOperation::EditTitle { title }),
        arbitrary_cob_body().prop_map(|body| CobOperation::EditBody { body }),
        // Reactions
        prop_oneof![
            Just("👍".to_string()),
            Just("👎".to_string()),
            Just("❤️".to_string()),
            Just("🎉".to_string()),
        ]
        .prop_map(|emoji| CobOperation::React { emoji }),
    ]
}

/// Generator for non-create issue operations (for child changes).
pub fn arbitrary_issue_child_operation() -> impl Strategy<Value = CobOperation> {
    prop_oneof![
        // Comment
        arbitrary_comment_body().prop_map(|body| CobOperation::Comment { body }),
        // Label operations
        arbitrary_label().prop_map(|label| CobOperation::AddLabel { label }),
        arbitrary_label().prop_map(|label| CobOperation::RemoveLabel { label }),
        // State transitions
        prop::option::of("[A-Za-z ]{5,50}").prop_map(|reason| CobOperation::Close { reason }),
        Just(CobOperation::Reopen),
        // Title/Body edits
        arbitrary_issue_title().prop_map(|title| CobOperation::EditTitle { title }),
        arbitrary_cob_body().prop_map(|body| CobOperation::EditBody { body }),
        // Reactions
        prop_oneof![
            Just("👍".to_string()),
            Just("👎".to_string()),
            Just("❤️".to_string()),
            Just("🎉".to_string()),
        ]
        .prop_map(|emoji| CobOperation::React { emoji }),
    ]
}

/// Generator for a blake3 hash (random 32 bytes).
pub fn arbitrary_blake3_hash() -> impl Strategy<Value = blake3::Hash> {
    prop::collection::vec(any::<u8>(), 32).prop_map(|bytes| {
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        blake3::Hash::from_bytes(arr)
    })
}

/// Generator for a linear COB change DAG (chain of changes).
///
/// Returns (cob_id, changes in order from root to head).
pub fn arbitrary_cob_linear_dag(max_changes: usize) -> impl Strategy<Value = (blake3::Hash, Vec<CobChange>)> {
    (1usize..=max_changes).prop_flat_map(move |count| {
        // Generate a cob_id
        arbitrary_blake3_hash().prop_flat_map(move |cob_id| {
            // Generate root operation
            (arbitrary_issue_title(), arbitrary_cob_body(), arbitrary_labels()).prop_flat_map(
                move |(title, body, labels)| {
                    // Generate child operations
                    prop::collection::vec(arbitrary_issue_child_operation(), count.saturating_sub(1)).prop_map(
                        move |child_ops| {
                            let mut changes = Vec::with_capacity(count);

                            // Root change
                            let root = CobChange::root(
                                CobType::Issue,
                                cob_id,
                                CobOperation::CreateIssue {
                                    title: title.clone(),
                                    body: body.clone(),
                                    labels: labels.clone(),
                                },
                            );
                            changes.push(root);

                            // Child changes (each references the previous)
                            let mut parent_hash = compute_change_hash(&changes[0]);
                            for op in child_ops {
                                let change = CobChange::new(CobType::Issue, cob_id, vec![parent_hash], op);
                                parent_hash = compute_change_hash(&change);
                                changes.push(change);
                            }

                            (cob_id, changes)
                        },
                    )
                },
            )
        })
    })
}

/// Compute a deterministic hash for a CobChange (for test purposes).
fn compute_change_hash(change: &CobChange) -> blake3::Hash {
    let bytes = postcard::to_allocvec(change).expect("serialization should not fail");
    blake3::hash(&bytes)
}

/// Topology types for COB DAG generation.
#[derive(Debug, Clone)]
pub enum CobDagTopology {
    /// Linear chain of N changes
    Linear(usize),
    /// Diamond: root -> two branches -> merge
    Diamond,
    /// N independent branches from root (multi-head scenario)
    MultiHead(usize),
}

/// Generator for COB DAG topologies.
pub fn arbitrary_cob_dag_topology() -> impl Strategy<Value = CobDagTopology> {
    prop_oneof![
        (2usize..20).prop_map(CobDagTopology::Linear),
        Just(CobDagTopology::Diamond),
        (2usize..5).prop_map(CobDagTopology::MultiHead),
    ]
}

// ============================================================================
// Coordination Primitive Generators
// ============================================================================

/// TTL (time-to-live) configuration for coordination primitives.
#[derive(Debug, Clone)]
pub struct TtlConfig {
    /// Minimum TTL in milliseconds.
    pub min_ms: u64,
    /// Maximum TTL in milliseconds.
    pub max_ms: u64,
}

impl Default for TtlConfig {
    fn default() -> Self {
        Self {
            min_ms: 100,
            max_ms: 30_000,
        }
    }
}

/// Generator for TTL values in milliseconds.
pub fn arbitrary_ttl_ms() -> impl Strategy<Value = u64> {
    prop_oneof![
        5 => 100u64..1_000u64,       // Short TTL (50%)
        3 => 1_000u64..10_000u64,    // Medium TTL (30%)
        2 => 10_000u64..60_000u64,   // Long TTL (20%)
    ]
}

/// Generator for lock holder IDs.
pub fn arbitrary_holder_id() -> impl Strategy<Value = String> {
    prop_oneof![
        // UUID-like format
        "[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}",
        // Simple alphanumeric
        "[a-z0-9]{8,16}",
        // Node-prefixed
        "node_[0-9]{1,3}_[a-z0-9]{6}",
    ]
}

/// Generator for lock key names.
pub fn arbitrary_lock_key() -> impl Strategy<Value = String> {
    prop_oneof![
        // Resource locks
        "lock:[a-z]{3,10}",
        // Distributed mutex
        "mutex:[a-z]{3,10}:[0-9]{1,4}",
        // Leader election
        "leader:[a-z]{3,10}",
    ]
}

/// Generator for counter names.
pub fn arbitrary_counter_name() -> impl Strategy<Value = String> {
    prop_oneof![
        // Metrics counters
        "counter:[a-z]{3,10}",
        // Rate counters
        "rate:[a-z]{3,10}",
        // Sequence counters
        "seq:[a-z]{3,10}",
    ]
}

/// Generator for queue names.
pub fn arbitrary_queue_name() -> impl Strategy<Value = String> {
    prop_oneof![
        // Task queues
        "queue:[a-z]{3,10}",
        // Job queues
        "jobs:[a-z]{3,10}",
        // Message queues
        "mq:[a-z]{3,10}",
    ]
}

/// Generator for barrier names.
pub fn arbitrary_barrier_name() -> impl Strategy<Value = String> {
    prop_oneof![
        "barrier:[a-z]{3,10}",
        "sync:[a-z]{3,10}:[0-9]{1,3}",
        "rendezvous:[a-z]{3,10}",
    ]
}

/// Generator for semaphore names.
pub fn arbitrary_semaphore_name() -> impl Strategy<Value = String> {
    prop_oneof!["sem:[a-z]{3,10}", "semaphore:[a-z]{3,10}", "pool:[a-z]{3,10}",]
}

/// Generator for semaphore permit counts.
pub fn arbitrary_permit_count() -> impl Strategy<Value = u32> {
    prop_oneof![
        5 => 1u32..10u32,       // Small (50%)
        3 => 10u32..100u32,     // Medium (30%)
        2 => 100u32..1000u32,   // Large (20%)
    ]
}

/// Generator for fencing tokens (monotonically increasing).
pub fn arbitrary_fencing_token() -> impl Strategy<Value = u64> {
    1u64..u64::MAX
}

/// Counter operation types for property testing.
#[derive(Debug, Clone, PartialEq)]
pub enum CounterOp {
    Increment(i64),
    Decrement(i64),
    Set(i64),
    Get,
    CompareAndSwap { expected: i64, new: i64 },
}

/// Generator for counter operations.
pub fn arbitrary_counter_op() -> impl Strategy<Value = CounterOp> {
    prop_oneof![
        3 => (1i64..1000i64).prop_map(CounterOp::Increment),
        2 => (1i64..1000i64).prop_map(CounterOp::Decrement),
        1 => (-10_000i64..10_000i64).prop_map(CounterOp::Set),
        2 => Just(CounterOp::Get),
        2 => (-1000i64..1000i64, -1000i64..1000i64).prop_map(|(expected, new)| CounterOp::CompareAndSwap { expected, new }),
    ]
}

/// Lock operation types for property testing.
#[derive(Debug, Clone, PartialEq)]
pub enum LockOp {
    Acquire { holder_id: String, ttl_ms: u64 },
    Release { holder_id: String },
    Renew { holder_id: String, ttl_ms: u64 },
    TryAcquire { holder_id: String, ttl_ms: u64 },
}

/// Generator for lock operations.
pub fn arbitrary_lock_op() -> impl Strategy<Value = LockOp> {
    prop_oneof![
        3 => (arbitrary_holder_id(), arbitrary_ttl_ms()).prop_map(|(holder_id, ttl_ms)| LockOp::Acquire { holder_id, ttl_ms }),
        2 => arbitrary_holder_id().prop_map(|holder_id| LockOp::Release { holder_id }),
        2 => (arbitrary_holder_id(), arbitrary_ttl_ms()).prop_map(|(holder_id, ttl_ms)| LockOp::Renew { holder_id, ttl_ms }),
        3 => (arbitrary_holder_id(), arbitrary_ttl_ms()).prop_map(|(holder_id, ttl_ms)| LockOp::TryAcquire { holder_id, ttl_ms }),
    ]
}

/// Queue operation types for property testing.
#[derive(Debug, Clone, PartialEq)]
pub enum QueueOp {
    Enqueue { data: Vec<u8>, priority: Option<i32> },
    Dequeue { visibility_timeout_ms: u64 },
    Ack { receipt_handle: String },
    Nack { receipt_handle: String },
    Peek,
    Len,
}

/// Generator for queue operations.
pub fn arbitrary_queue_op() -> impl Strategy<Value = QueueOp> {
    prop_oneof![
        3 => (prop::collection::vec(any::<u8>(), 1..1000), prop::option::of(-100i32..100i32))
            .prop_map(|(data, priority)| QueueOp::Enqueue { data, priority }),
        3 => (100u64..60_000u64).prop_map(|visibility_timeout_ms| QueueOp::Dequeue { visibility_timeout_ms }),
        2 => "[a-z0-9]{16,32}".prop_map(|receipt_handle| QueueOp::Ack { receipt_handle }),
        1 => "[a-z0-9]{16,32}".prop_map(|receipt_handle| QueueOp::Nack { receipt_handle }),
        1 => Just(QueueOp::Peek),
        1 => Just(QueueOp::Len),
    ]
}

/// Generator for sequences of coordination operations targeting the same resource.
pub fn arbitrary_lock_op_sequence(max_ops: usize) -> impl Strategy<Value = (String, Vec<LockOp>)> {
    (arbitrary_lock_key(), prop::collection::vec(arbitrary_lock_op(), 1..max_ops))
}

/// Generator for sequences of counter operations targeting the same counter.
pub fn arbitrary_counter_op_sequence(max_ops: usize) -> impl Strategy<Value = (String, Vec<CounterOp>)> {
    (arbitrary_counter_name(), prop::collection::vec(arbitrary_counter_op(), 1..max_ops))
}

/// Generator for sequences of queue operations targeting the same queue.
pub fn arbitrary_queue_op_sequence(max_ops: usize) -> impl Strategy<Value = (String, Vec<QueueOp>)> {
    (arbitrary_queue_name(), prop::collection::vec(arbitrary_queue_op(), 1..max_ops))
}

// ============================================================================
// Chaos/Fault Injection Generators
// ============================================================================

/// Network fault types for chaos testing.
#[derive(Debug, Clone, PartialEq)]
pub enum NetworkFault {
    /// Partition a node from the cluster.
    Partition { node_id: u64 },
    /// Heal a previously partitioned node.
    Heal { node_id: u64 },
    /// Add latency to all network calls.
    AddLatency { latency_ms: u64 },
    /// Drop a percentage of packets.
    DropPackets { percent: u8 },
    /// Reset network to normal.
    Reset,
}

/// Generator for network faults.
pub fn arbitrary_network_fault(max_nodes: u64) -> impl Strategy<Value = NetworkFault> {
    prop_oneof![
        3 => (0u64..max_nodes).prop_map(|node_id| NetworkFault::Partition { node_id }),
        2 => (0u64..max_nodes).prop_map(|node_id| NetworkFault::Heal { node_id }),
        2 => (10u64..500u64).prop_map(|latency_ms| NetworkFault::AddLatency { latency_ms }),
        2 => (1u8..50u8).prop_map(|percent| NetworkFault::DropPackets { percent }),
        1 => Just(NetworkFault::Reset),
    ]
}

/// Node fault types for chaos testing.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeFault {
    /// Kill a node (crash).
    Kill { node_id: u64 },
    /// Restart a previously killed node.
    Restart { node_id: u64 },
    /// Pause a node (freeze).
    Pause { node_id: u64 },
    /// Resume a paused node.
    Resume { node_id: u64 },
}

/// Generator for node faults.
pub fn arbitrary_node_fault(max_nodes: u64) -> impl Strategy<Value = NodeFault> {
    prop_oneof![
        3 => (0u64..max_nodes).prop_map(|node_id| NodeFault::Kill { node_id }),
        2 => (0u64..max_nodes).prop_map(|node_id| NodeFault::Restart { node_id }),
        2 => (0u64..max_nodes).prop_map(|node_id| NodeFault::Pause { node_id }),
        2 => (0u64..max_nodes).prop_map(|node_id| NodeFault::Resume { node_id }),
    ]
}

/// Combined fault type for chaos testing.
#[derive(Debug, Clone, PartialEq)]
pub enum ChaosFault {
    Network(NetworkFault),
    Node(NodeFault),
    /// No fault (normal operation).
    None,
}

/// Generator for chaos faults.
pub fn arbitrary_chaos_fault(max_nodes: u64) -> impl Strategy<Value = ChaosFault> {
    prop_oneof![
        4 => Just(ChaosFault::None),
        3 => arbitrary_network_fault(max_nodes).prop_map(ChaosFault::Network),
        3 => arbitrary_node_fault(max_nodes).prop_map(ChaosFault::Node),
    ]
}

/// Generator for a sequence of chaos faults.
pub fn arbitrary_chaos_sequence(max_nodes: u64, max_faults: usize) -> impl Strategy<Value = Vec<ChaosFault>> {
    prop::collection::vec(arbitrary_chaos_fault(max_nodes), 0..max_faults)
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
