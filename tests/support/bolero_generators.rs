//! Centralized Bolero generators for Aspen tests.
//!
//! This module provides reusable property-based testing generators using Bolero's
//! TypeGenerator trait. These generators:
//! - Enable unified testing across multiple backends (libFuzzer, AFL, Honggfuzz, Kani, Miri)
//! - Reduce code duplication across test files
//! - Ensure consistent test data generation
//! - Provide parameterized generators for edge cases
//! - Support distributed systems scenario generation
//!
//! Tiger Style: All generators respect resource bounds from `constants.rs`.

#![allow(dead_code)] // Many generators not yet used across all test files

use bolero_generator::{Driver, TypeGenerator};

use aspen::raft::types::{AppRequest, AppTypeConfig, NodeId};
use openraft::LogId;
use openraft::testing::log_id;

// Re-export constants for boundary testing
pub use aspen::raft::constants::{MAX_KEY_SIZE, MAX_SETMULTI_KEYS, MAX_VALUE_SIZE};

// ============================================================================
// Character Set Constants
// ============================================================================

const LOWERCASE: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const ALPHANUMERIC_UNDERSCORE: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789_";
const ALPHANUMERIC_SPACE: &[u8] =
    b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 ";
const SPECIAL_CHARS: &[u8] = b"!@#$%^&*()-_=+[]{};:'\",.<>?/";

// ============================================================================
// Core Key-Value Types
// ============================================================================

/// A valid key for property testing (1-20 alphanumeric chars).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidKey(pub String);

impl TypeGenerator for ValidKey {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        // First char: lowercase letter
        let first_idx = driver.produce::<usize>()? % LOWERCASE.len();
        let first = LOWERCASE[first_idx] as char;

        // Remaining chars: 0-19 alphanumeric + underscore
        let remaining_len = driver.produce::<usize>()? % 20;
        let mut key = String::with_capacity(1 + remaining_len);
        key.push(first);

        for _ in 0..remaining_len {
            let idx = driver.produce::<usize>()? % ALPHANUMERIC_UNDERSCORE.len();
            key.push(ALPHANUMERIC_UNDERSCORE[idx] as char);
        }

        Some(ValidKey(key))
    }
}

/// A valid value for property testing (1-100 alphanumeric chars with spaces).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidValue(pub String);

impl TypeGenerator for ValidValue {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let len = 1 + (driver.produce::<usize>()? % 100);
        let mut value = String::with_capacity(len);

        for _ in 0..len {
            let idx = driver.produce::<usize>()? % ALPHANUMERIC_SPACE.len();
            value.push(ALPHANUMERIC_SPACE[idx] as char);
        }

        Some(ValidValue(value))
    }
}

/// A key-value pair for property testing.
#[derive(Debug, Clone, TypeGenerator)]
pub struct KeyValuePair {
    pub key: ValidKey,
    pub value: ValidValue,
}

impl KeyValuePair {
    pub fn into_tuple(self) -> (String, String) {
        (self.key.0, self.value.0)
    }
}

// ============================================================================
// Edge Case Key-Value Types
// ============================================================================

/// Key-value with edge case coverage (special chars, whitespace, near-boundary sizes).
#[derive(Debug, Clone)]
pub struct EdgeCaseKeyValue {
    pub key: String,
    pub value: String,
}

impl TypeGenerator for EdgeCaseKeyValue {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let variant = driver.produce::<u8>()? % 4;

        let (key, value) = match variant {
            0 => {
                // Normal case (most common)
                let kv = KeyValuePair::generate(driver)?;
                (kv.key.0, kv.value.0)
            }
            1 => {
                // Special characters in values
                let key = ValidKey::generate(driver)?.0;
                let value_len = 1 + (driver.produce::<usize>()? % 50);
                let mut value = String::with_capacity(value_len);
                for _ in 0..value_len {
                    let idx = driver.produce::<usize>()? % SPECIAL_CHARS.len();
                    value.push(SPECIAL_CHARS[idx] as char);
                }
                (key, value)
            }
            2 => {
                // Whitespace variations
                let key = ValidKey::generate(driver)?.0;
                let value_len = 1 + (driver.produce::<usize>()? % 20);
                let mut value = String::with_capacity(value_len);
                for _ in 0..value_len {
                    let c = if driver.produce::<bool>()? { ' ' } else { '\t' };
                    value.push(c);
                }
                (key, value)
            }
            _ => {
                // Longer keys (near boundary)
                let key_len = 51 + (driver.produce::<usize>()? % 50);
                let mut key = String::with_capacity(key_len);
                for _ in 0..key_len {
                    let idx = driver.produce::<usize>()? % ALPHANUMERIC_UNDERSCORE.len();
                    key.push(ALPHANUMERIC_UNDERSCORE[idx] as char);
                }
                let value = ValidValue::generate(driver)?.0;
                (key, value)
            }
        };

        Some(EdgeCaseKeyValue { key, value })
    }
}

/// Key with skewed size distribution matching real workloads.
#[derive(Debug, Clone)]
pub struct SkewedSizeKey(pub String);

impl TypeGenerator for SkewedSizeKey {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        // Distribution: 70% small (1-5), 20% medium (6-50), 10% large (50-200)
        let size_class = driver.produce::<u8>()? % 10;
        let len = if size_class < 7 {
            1 + (driver.produce::<usize>()? % 5) // Small: 1-5
        } else if size_class < 9 {
            6 + (driver.produce::<usize>()? % 45) // Medium: 6-50
        } else {
            50 + (driver.produce::<usize>()? % 151) // Large: 50-200
        };

        let mut key = String::with_capacity(len);
        for _ in 0..len {
            let idx = driver.produce::<usize>()? % LOWERCASE.len();
            key.push(LOWERCASE[idx] as char);
        }

        Some(SkewedSizeKey(key))
    }
}

// ============================================================================
// Log Entry Types
// ============================================================================

/// Valid Raft term (1-100).
#[derive(Debug, Clone, Copy)]
pub struct ValidTerm(pub u64);

impl TypeGenerator for ValidTerm {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let term = 1 + (driver.produce::<u64>()? % 99);
        Some(ValidTerm(term))
    }
}

/// Valid node ID (0-9).
#[derive(Debug, Clone, Copy)]
pub struct ValidNodeId(pub NodeId);

impl TypeGenerator for ValidNodeId {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let id = driver.produce::<u64>()? % 10;
        Some(ValidNodeId(NodeId::from(id)))
    }
}

/// Valid log index (1-1000).
#[derive(Debug, Clone, Copy)]
pub struct ValidLogIndex(pub u64);

impl TypeGenerator for ValidLogIndex {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let index = 1 + (driver.produce::<u64>()? % 999);
        Some(ValidLogIndex(index))
    }
}

/// Helper function to create a log ID with given term, node, and index.
pub fn make_log_id(term: u64, node: u64, index: u64) -> LogId<AppTypeConfig> {
    log_id::<AppTypeConfig>(term, NodeId::from(node), index)
}

/// Valid vote with term and node ID.
#[derive(Debug, Clone, Copy)]
pub struct ValidVote {
    pub term: u64,
    pub node_id: NodeId,
}

impl TypeGenerator for ValidVote {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let term = 1 + (driver.produce::<u64>()? % 100);
        let node_id = NodeId::from(driver.produce::<u64>()? % 10);
        Some(ValidVote { term, node_id })
    }
}

// ============================================================================
// AppRequest Types
// ============================================================================

/// AppRequest covering all operation types with balanced distribution.
#[derive(Debug, Clone)]
pub struct BalancedAppRequest(pub AppRequest);

impl TypeGenerator for BalancedAppRequest {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let variant = driver.produce::<u8>()? % 4;

        let request = match variant {
            0 => {
                // Set operation (25%)
                let kv = KeyValuePair::generate(driver)?;
                AppRequest::Set {
                    key: kv.key.0,
                    value: kv.value.0,
                }
            }
            1 => {
                // SetMulti operation (25%)
                let count = 1 + (driver.produce::<usize>()? % 19);
                let mut pairs = Vec::with_capacity(count);
                for _ in 0..count {
                    let kv = KeyValuePair::generate(driver)?;
                    pairs.push((kv.key.0, kv.value.0));
                }
                AppRequest::SetMulti { pairs }
            }
            2 => {
                // Delete operation (25%)
                let key = ValidKey::generate(driver)?;
                AppRequest::Delete { key: key.0 }
            }
            _ => {
                // DeleteMulti operation (25%)
                let count = 1 + (driver.produce::<usize>()? % 9);
                let mut keys = Vec::with_capacity(count);
                for _ in 0..count {
                    let key = ValidKey::generate(driver)?;
                    keys.push(key.0);
                }
                AppRequest::DeleteMulti { keys }
            }
        };

        Some(BalancedAppRequest(request))
    }
}

/// Set request only.
#[derive(Debug, Clone)]
pub struct SetRequest(pub AppRequest);

impl TypeGenerator for SetRequest {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let kv = KeyValuePair::generate(driver)?;
        Some(SetRequest(AppRequest::Set {
            key: kv.key.0,
            value: kv.value.0,
        }))
    }
}

/// Delete request only.
#[derive(Debug, Clone)]
pub struct DeleteRequest(pub AppRequest);

impl TypeGenerator for DeleteRequest {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let key = ValidKey::generate(driver)?;
        Some(DeleteRequest(AppRequest::Delete { key: key.0 }))
    }
}

// ============================================================================
// API Validation Types
// ============================================================================

const API_KEY_CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_:/-";

/// A valid API key (alphanumeric + _:/-), 1-100 chars.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidApiKey(pub String);

impl TypeGenerator for ValidApiKey {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let len = 1 + (driver.produce::<usize>()? % 100);
        let mut key = String::with_capacity(len);
        for _ in 0..len {
            let idx = driver.produce::<usize>()? % API_KEY_CHARS.len();
            key.push(API_KEY_CHARS[idx] as char);
        }
        Some(ValidApiKey(key))
    }
}

/// A valid API value (alphanumeric + _:/-), 1-1000 chars.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidApiValue(pub String);

impl TypeGenerator for ValidApiValue {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let len = 1 + (driver.produce::<usize>()? % 1000);
        let mut value = String::with_capacity(len);
        for _ in 0..len {
            let idx = driver.produce::<usize>()? % API_KEY_CHARS.len();
            value.push(API_KEY_CHARS[idx] as char);
        }
        Some(ValidApiValue(value))
    }
}

/// A key that exceeds MAX_KEY_SIZE (for rejection testing).
#[derive(Debug, Clone)]
pub struct OversizedKey(pub String);

impl TypeGenerator for OversizedKey {
    fn generate<D: Driver>(_driver: &mut D) -> Option<Self> {
        // MAX_KEY_SIZE is 1024 bytes, generate 1025 byte key
        Some(OversizedKey("x".repeat(MAX_KEY_SIZE as usize + 1)))
    }
}

/// Collection of valid key-value pairs (1-10 pairs).
#[derive(Debug, Clone)]
pub struct ValidApiPairs(pub Vec<(String, String)>);

impl TypeGenerator for ValidApiPairs {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let count = 1 + (driver.produce::<usize>()? % 10);
        let mut pairs = Vec::with_capacity(count);
        for _ in 0..count {
            let key = ValidApiKey::generate(driver)?.0;
            let value = ValidApiValue::generate(driver)?.0;
            pairs.push((key, value));
        }
        Some(ValidApiPairs(pairs))
    }
}

/// Collection of valid keys (1-10 keys).
#[derive(Debug, Clone)]
pub struct ValidApiKeys(pub Vec<String>);

impl TypeGenerator for ValidApiKeys {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let count = 1 + (driver.produce::<usize>()? % 10);
        let mut keys = Vec::with_capacity(count);
        for _ in 0..count {
            keys.push(ValidApiKey::generate(driver)?.0);
        }
        Some(ValidApiKeys(keys))
    }
}

/// Scan prefix (0-20 lowercase chars).
#[derive(Debug, Clone)]
pub struct ScanPrefix(pub String);

impl TypeGenerator for ScanPrefix {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let len = driver.produce::<usize>()? % 21;
        let mut prefix = String::with_capacity(len);
        for _ in 0..len {
            let idx = driver.produce::<usize>()? % LOWERCASE.len();
            prefix.push(LOWERCASE[idx] as char);
        }
        Some(ScanPrefix(prefix))
    }
}

/// Optional scan limit (1-999).
#[derive(Debug, Clone)]
pub struct OptionalScanLimit(pub Option<u32>);

impl TypeGenerator for OptionalScanLimit {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let has_limit = driver.produce::<bool>()?;
        let limit = if has_limit {
            Some(1 + (driver.produce::<u32>()? % 999))
        } else {
            None
        };
        Some(OptionalScanLimit(limit))
    }
}

/// Optional continuation token (alphanumeric, 10-30 chars).
#[derive(Debug, Clone)]
pub struct OptionalContinuationToken(pub Option<String>);

impl TypeGenerator for OptionalContinuationToken {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let has_token = driver.produce::<bool>()?;
        let token = if has_token {
            let len = 10 + (driver.produce::<usize>()? % 21);
            let mut t = String::with_capacity(len);
            for _ in 0..len {
                let idx = driver.produce::<usize>()? % ALPHANUMERIC_SPACE.len();
                let c = ALPHANUMERIC_SPACE[idx];
                if c != b' ' {
                    t.push(c as char);
                } else {
                    t.push('a'); // Replace space with 'a' for tokens
                }
            }
            Some(t)
        } else {
            None
        };
        Some(OptionalContinuationToken(token))
    }
}

/// Simple address string (10-30 alphanumeric chars).
#[derive(Debug, Clone)]
pub struct SimpleAddr(pub String);

impl TypeGenerator for SimpleAddr {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let len = 10 + (driver.produce::<usize>()? % 21);
        let mut addr = String::with_capacity(len);
        for _ in 0..len {
            let idx = driver.produce::<usize>()? % ALPHANUMERIC_UNDERSCORE.len();
            addr.push(ALPHANUMERIC_UNDERSCORE[idx] as char);
        }
        Some(SimpleAddr(addr))
    }
}

/// Optional raft address (alphanumeric + .:, 10-30 chars).
#[derive(Debug, Clone)]
pub struct OptionalRaftAddr(pub Option<String>);

const RAFT_ADDR_CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789.:";

impl TypeGenerator for OptionalRaftAddr {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let has_addr = driver.produce::<bool>()?;
        let addr = if has_addr {
            let len = 10 + (driver.produce::<usize>()? % 21);
            let mut a = String::with_capacity(len);
            for _ in 0..len {
                let idx = driver.produce::<usize>()? % RAFT_ADDR_CHARS.len();
                a.push(RAFT_ADDR_CHARS[idx] as char);
            }
            Some(a)
        } else {
            None
        };
        Some(OptionalRaftAddr(addr))
    }
}

// ============================================================================
// Boundary and Edge Case Types
// ============================================================================

/// Key at MAX_KEY_SIZE boundary (under, at, or over limit).
#[derive(Debug, Clone)]
pub struct BoundaryKey(pub String);

impl TypeGenerator for BoundaryKey {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let variant = driver.produce::<u8>()? % 3;
        let len = match variant {
            0 => MAX_KEY_SIZE as usize - 1, // Just under
            1 => MAX_KEY_SIZE as usize,     // Exactly at
            _ => MAX_KEY_SIZE as usize + 1, // Just over
        };
        Some(BoundaryKey("x".repeat(len)))
    }
}

/// Value at various size boundaries.
#[derive(Debug, Clone)]
pub struct BoundaryValue(pub String);

impl TypeGenerator for BoundaryValue {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let variant = driver.produce::<u8>()? % 4;
        let s = match variant {
            0 => "small".to_string(),
            1 => "x".repeat(100 * 1024),                  // 100KB
            2 => "x".repeat(MAX_VALUE_SIZE as usize - 1), // Just under
            _ => "x".repeat(MAX_VALUE_SIZE as usize),     // Exactly at
        };
        Some(BoundaryValue(s))
    }
}

/// Value exceeding MAX_VALUE_SIZE (for rejection testing).
#[derive(Debug, Clone)]
pub struct OversizedValue(pub String);

impl TypeGenerator for OversizedValue {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let extra = driver.produce::<usize>()? % 1000;
        let len = MAX_VALUE_SIZE as usize + 1 + extra;
        Some(OversizedValue("x".repeat(len)))
    }
}

/// SetMulti exceeding MAX_SETMULTI_KEYS (for rejection testing).
#[derive(Debug, Clone)]
pub struct OversizedSetMulti(pub Vec<(String, String)>);

impl TypeGenerator for OversizedSetMulti {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let extra = driver.produce::<usize>()? % 49;
        let count = MAX_SETMULTI_KEYS as usize + 1 + extra;
        let mut pairs = Vec::with_capacity(count);
        for _ in 0..count {
            let kv = KeyValuePair::generate(driver)?;
            pairs.push((kv.key.0, kv.value.0));
        }
        Some(OversizedSetMulti(pairs))
    }
}

/// SetMulti at exactly the limit.
#[derive(Debug, Clone)]
pub struct SetMultiAtLimit(pub Vec<(String, String)>);

impl TypeGenerator for SetMultiAtLimit {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let count = MAX_SETMULTI_KEYS as usize;
        let mut pairs = Vec::with_capacity(count);
        for _ in 0..count {
            let kv = KeyValuePair::generate(driver)?;
            pairs.push((kv.key.0, kv.value.0));
        }
        Some(SetMultiAtLimit(pairs))
    }
}

// ============================================================================
// NodeId String Parsing Types
// ============================================================================

/// Valid NodeId string representation.
#[derive(Debug, Clone)]
pub struct ValidNodeIdString(pub String);

impl TypeGenerator for ValidNodeIdString {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let variant = driver.produce::<u8>()? % 3;
        let s = match variant {
            0 => {
                // Small valid IDs
                let id = driver.produce::<u64>()? % 100;
                id.to_string()
            }
            1 => {
                // Large valid IDs
                let offset = driver.produce::<u64>()? % 1001;
                (u64::MAX - offset).to_string()
            }
            _ => "0".to_string(), // Zero edge case
        };
        Some(ValidNodeIdString(s))
    }
}

/// Invalid NodeId string (for rejection testing).
#[derive(Debug, Clone)]
pub struct InvalidNodeIdString(pub String);

impl TypeGenerator for InvalidNodeIdString {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let variant = driver.produce::<u8>()? % 8;
        let s = match variant {
            0 => String::new(),                                   // Empty
            1 => format!("-{}", driver.produce::<u16>()? % 1000), // Negative
            2 => {
                // Non-numeric
                let len = 1 + (driver.produce::<usize>()? % 10);
                let mut s = String::with_capacity(len);
                for _ in 0..len {
                    let idx = driver.produce::<usize>()? % 26;
                    s.push((b'a' + idx as u8) as char);
                }
                s
            }
            3 => "18446744073709551616".to_string(), // u64::MAX + 1
            4 => {
                // Mixed alphanumeric
                format!("{}{}", driver.produce::<u8>()? % 10, "abc")
            }
            5 => "123.456".to_string(), // Decimal
            6 => "0x123".to_string(),   // Hex notation
            _ => "123abc".to_string(),  // Non-digit chars
        };
        Some(InvalidNodeIdString(s))
    }
}

// ============================================================================
// Log Store Types
// ============================================================================

/// Sequence of log entries to append.
#[derive(Debug, Clone)]
pub struct LogAppendSequence {
    pub start_index: u64,
    pub entries: Vec<AppRequest>,
}

impl TypeGenerator for LogAppendSequence {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let start_index = 1 + (driver.produce::<u64>()? % 99);
        let count = 1 + (driver.produce::<usize>()? % 19);
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let req = BalancedAppRequest::generate(driver)?;
            entries.push(req.0);
        }
        Some(LogAppendSequence {
            start_index,
            entries,
        })
    }
}

/// Vote tuple (term, voted_for).
#[derive(Debug, Clone)]
pub struct Vote {
    pub term: u64,
    pub voted_for: NodeId,
}

impl TypeGenerator for Vote {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let term = ValidTerm::generate(driver)?.0;
        let voted_for = ValidNodeId::generate(driver)?.0;
        Some(Vote { term, voted_for })
    }
}

// ============================================================================
// Network Simulation Types
// ============================================================================

/// Network delay configuration for simulation testing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkDelayConfig {
    pub min_latency_ms: u64,
    pub max_latency_ms: u64,
    pub jitter_percent: u8,
}

impl TypeGenerator for NetworkDelayConfig {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let variant = driver.produce::<u8>()? % 4;
        let config = match variant {
            0 => NetworkDelayConfig {
                min_latency_ms: 0,
                max_latency_ms: 5,
                jitter_percent: 10,
            },
            1 => NetworkDelayConfig {
                min_latency_ms: 10,
                max_latency_ms: 100,
                jitter_percent: 20,
            },
            2 => NetworkDelayConfig {
                min_latency_ms: 100,
                max_latency_ms: 300,
                jitter_percent: 30,
            },
            _ => {
                let min = driver.produce::<u64>()? % 50;
                let max_extra = 50 + (driver.produce::<u64>()? % 450);
                let jitter = driver.produce::<u8>()? % 50;
                NetworkDelayConfig {
                    min_latency_ms: min,
                    max_latency_ms: min + max_extra,
                    jitter_percent: jitter,
                }
            }
        };
        Some(config)
    }
}

/// Test operation for concurrent operation testing.
#[derive(Debug, Clone)]
pub enum TestOperation {
    Write(String, String),
    Read(String),
    Delete(String),
    Scan(String, u32),
}

impl TypeGenerator for TestOperation {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let variant = driver.produce::<u8>()? % 4;
        let op = match variant {
            0 => {
                let kv = KeyValuePair::generate(driver)?;
                TestOperation::Write(kv.key.0, kv.value.0)
            }
            1 => {
                let key = ValidKey::generate(driver)?;
                TestOperation::Read(key.0)
            }
            2 => {
                let key = ValidKey::generate(driver)?;
                TestOperation::Delete(key.0)
            }
            _ => {
                // Scan with prefix and limit
                let prefix_len = 1 + (driver.produce::<usize>()? % 5);
                let mut prefix = String::with_capacity(prefix_len);
                for _ in 0..prefix_len {
                    let idx = driver.produce::<usize>()? % LOWERCASE.len();
                    prefix.push(LOWERCASE[idx] as char);
                }
                let limit = 1 + (driver.produce::<u32>()? % 99);
                TestOperation::Scan(prefix, limit)
            }
        };
        Some(op)
    }
}

/// Sequence of test operations.
#[derive(Debug, Clone)]
pub struct OperationSequence(pub Vec<TestOperation>);

impl TypeGenerator for OperationSequence {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let count = 1 + (driver.produce::<usize>()? % 49);
        let mut ops = Vec::with_capacity(count);
        for _ in 0..count {
            let op = TestOperation::generate(driver)?;
            ops.push(op);
        }
        Some(OperationSequence(ops))
    }
}

// ============================================================================
// Distributed Systems Types
// ============================================================================

/// Network latency with realistic distribution.
#[derive(Debug, Clone, Copy)]
pub struct LatencyMs(pub u64);

impl TypeGenerator for LatencyMs {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        // Distribution: 50% fast (0-10), 30% normal (10-100), 20% slow (100-1000)
        let class = driver.produce::<u8>()? % 10;
        let ms = if class < 5 {
            driver.produce::<u64>()? % 11 // 0-10ms
        } else if class < 8 {
            10 + (driver.produce::<u64>()? % 91) // 10-100ms
        } else {
            100 + (driver.produce::<u64>()? % 901) // 100-1000ms
        };
        Some(LatencyMs(ms))
    }
}

/// Packet loss rate with realistic distribution.
#[derive(Debug, Clone, Copy)]
pub struct PacketLoss(pub f32);

impl TypeGenerator for PacketLoss {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        // Distribution: 70% no loss, 20% low (1-10%), 10% high (10-50%)
        let class = driver.produce::<u8>()? % 10;
        let loss = if class < 7 {
            0.0
        } else if class < 9 {
            0.01 + (driver.produce::<u8>()? as f32 / 255.0) * 0.09
        } else {
            0.10 + (driver.produce::<u8>()? as f32 / 255.0) * 0.40
        };
        Some(PacketLoss(loss))
    }
}

/// Membership configuration (voters, learners).
#[derive(Debug, Clone)]
pub struct Membership {
    pub voters: Vec<NodeId>,
    pub learners: Vec<NodeId>,
}

impl TypeGenerator for Membership {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let voter_count = 1 + (driver.produce::<usize>()? % 4);
        let learner_count = driver.produce::<usize>()? % 3;

        let mut voters = Vec::with_capacity(voter_count);
        for _ in 0..voter_count {
            voters.push(ValidNodeId::generate(driver)?.0);
        }

        let mut learners = Vec::with_capacity(learner_count);
        for _ in 0..learner_count {
            learners.push(ValidNodeId::generate(driver)?.0);
        }

        Some(Membership { voters, learners })
    }
}

// ============================================================================
// Gossip Discovery Types
// ============================================================================

/// Timestamp in milliseconds since epoch.
#[derive(Debug, Clone, Copy)]
pub struct TimestampMs(pub u64);

impl TypeGenerator for TimestampMs {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let variant = driver.produce::<u8>()? % 3;
        let ts = match variant {
            0 => {
                // Recent (approximate current time)
                1_700_000_000_000 + (driver.produce::<u64>()? % 100_000_000_000)
            }
            1 => {
                // Older timestamps
                1_700_000_000_000 + (driver.produce::<u64>()? % 100_000_000_000)
            }
            _ => 0, // Zero edge case
        };
        Some(TimestampMs(ts))
    }
}

/// Endpoint address bytes (32-64 bytes).
#[derive(Debug, Clone)]
pub struct EndpointAddrBytes(pub Vec<u8>);

impl TypeGenerator for EndpointAddrBytes {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let len = 32 + (driver.produce::<usize>()? % 33);
        let mut bytes = Vec::with_capacity(len);
        for _ in 0..len {
            bytes.push(driver.produce::<u8>()?);
        }
        Some(EndpointAddrBytes(bytes))
    }
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

// ============================================================================
// Convenience Type Aliases for Check Macro
// ============================================================================

/// Number of entries for tests (1-100).
#[derive(Debug, Clone, Copy)]
pub struct NumEntries(pub usize);

impl TypeGenerator for NumEntries {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let n = 1 + (driver.produce::<usize>()? % 99);
        Some(NumEntries(n))
    }
}

/// Number of writes for tests (1-30).
#[derive(Debug, Clone, Copy)]
pub struct NumWrites(pub usize);

impl TypeGenerator for NumWrites {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let n = 1 + (driver.produce::<usize>()? % 29);
        Some(NumWrites(n))
    }
}

/// Batch size for tests (1-50).
#[derive(Debug, Clone, Copy)]
pub struct BatchSize(pub usize);

impl TypeGenerator for BatchSize {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let n = 1 + (driver.produce::<usize>()? % 49);
        Some(BatchSize(n))
    }
}

// ============================================================================
// Cluster Configuration Types
// ============================================================================

const CLUSTER_ID_CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789-";

/// Valid cluster ID (lowercase letters, digits, hyphens, starts with letter).
#[derive(Debug, Clone)]
pub struct ValidClusterId(pub String);

impl TypeGenerator for ValidClusterId {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        // First char must be lowercase letter
        let first_idx = driver.produce::<usize>()? % 26;
        let first = LOWERCASE[first_idx] as char;

        // Remaining chars can be lowercase, digits, or hyphens
        let remaining_len = driver.produce::<usize>()? % 31;
        let mut id = String::with_capacity(1 + remaining_len);
        id.push(first);

        for _ in 0..remaining_len {
            let idx = driver.produce::<usize>()? % CLUSTER_ID_CHARS.len();
            id.push(CLUSTER_ID_CHARS[idx] as char);
        }
        Some(ValidClusterId(id))
    }
}

/// Valid timeout in milliseconds (50-10000).
#[derive(Debug, Clone, Copy)]
pub struct ValidTimeoutMs(pub u64);

impl TypeGenerator for ValidTimeoutMs {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let ms = 50 + (driver.produce::<u64>()? % 9950);
        Some(ValidTimeoutMs(ms))
    }
}

/// Valid IP address string.
#[derive(Debug, Clone)]
pub struct ValidIpAddr(pub String);

impl TypeGenerator for ValidIpAddr {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let a = driver.produce::<u8>()?;
        let b = driver.produce::<u8>()?;
        let c = driver.produce::<u8>()?;
        let d = driver.produce::<u8>()?;
        let port = 1024 + (driver.produce::<u16>()? % 64511);
        Some(ValidIpAddr(format!("{}.{}.{}.{}:{}", a, b, c, d, port)))
    }
}

/// Membership list (1-10 node IDs).
#[derive(Debug, Clone)]
pub struct MembershipList(pub Vec<u64>);

impl TypeGenerator for MembershipList {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let count = 1 + (driver.produce::<usize>()? % 9);
        let mut members = Vec::with_capacity(count);
        for _ in 0..count {
            let id = 1 + (driver.produce::<u64>()? % 999);
            members.push(id);
        }
        Some(MembershipList(members))
    }
}

// ============================================================================
// TUI RPC Generators
// ============================================================================

const TUI_KEY_CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_:/-";
const UPPERCASE_UNDERSCORE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ_";

/// TUI RPC key (alphanumeric + _:/-), 1-100 chars.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TuiRpcKey(pub String);

impl TypeGenerator for TuiRpcKey {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let len = 1 + (driver.produce::<usize>()? % 100);
        let mut key = String::with_capacity(len);
        for _ in 0..len {
            let idx = driver.produce::<usize>()? % TUI_KEY_CHARS.len();
            key.push(TUI_KEY_CHARS[idx] as char);
        }
        Some(TuiRpcKey(key))
    }
}

/// Health status string: "healthy", "degraded", or "unhealthy".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusString(pub String);

impl TypeGenerator for StatusString {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let statuses = ["healthy", "degraded", "unhealthy"];
        let idx = driver.produce::<usize>()? % statuses.len();
        Some(StatusString(statuses[idx].to_string()))
    }
}

/// Raft state string: "Leader", "Follower", "Candidate", or "Learner".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftStateString(pub String);

impl TypeGenerator for RaftStateString {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let states = ["Leader", "Follower", "Candidate", "Learner"];
        let idx = driver.produce::<usize>()? % states.len();
        Some(RaftStateString(states[idx].to_string()))
    }
}

/// Error code string: uppercase letters and underscores, 3-20 chars.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorCode(pub String);

impl TypeGenerator for ErrorCode {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let len = 3 + (driver.produce::<usize>()? % 18);
        let mut code = String::with_capacity(len);
        for _ in 0..len {
            let idx = driver.produce::<usize>()? % UPPERCASE_UNDERSCORE.len();
            code.push(UPPERCASE_UNDERSCORE[idx] as char);
        }
        Some(ErrorCode(code))
    }
}

/// Error message string: alphanumeric with spaces, 10-100 chars.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorMessage(pub String);

impl TypeGenerator for ErrorMessage {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let len = 10 + (driver.produce::<usize>()? % 91);
        let mut msg = String::with_capacity(len);
        for _ in 0..len {
            let idx = driver.produce::<usize>()? % ALPHANUMERIC_SPACE.len();
            msg.push(ALPHANUMERIC_SPACE[idx] as char);
        }
        Some(ErrorMessage(msg))
    }
}

/// Cluster ticket string: alphanumeric, 50-200 chars.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterTicketString(pub String);

impl TypeGenerator for ClusterTicketString {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let len = 50 + (driver.produce::<usize>()? % 151);
        let mut ticket = String::with_capacity(len);
        for _ in 0..len {
            let idx = driver.produce::<usize>()? % ALPHANUMERIC_UNDERSCORE.len();
            ticket.push(ALPHANUMERIC_UNDERSCORE[idx] as char);
        }
        Some(ClusterTicketString(ticket))
    }
}

/// Topic ID string: alphanumeric, exactly 32 chars.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicIdString(pub String);

impl TypeGenerator for TopicIdString {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let mut id = String::with_capacity(32);
        for _ in 0..32 {
            let idx = driver.produce::<usize>()? % ALPHANUMERIC_UNDERSCORE.len();
            id.push(ALPHANUMERIC_UNDERSCORE[idx] as char);
        }
        Some(TopicIdString(id))
    }
}

/// Endpoint ID string: alphanumeric, 32-64 chars.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointIdString(pub String);

impl TypeGenerator for EndpointIdString {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let len = 32 + (driver.produce::<usize>()? % 33);
        let mut id = String::with_capacity(len);
        for _ in 0..len {
            let idx = driver.produce::<usize>()? % ALPHANUMERIC_UNDERSCORE.len();
            id.push(ALPHANUMERIC_UNDERSCORE[idx] as char);
        }
        Some(EndpointIdString(id))
    }
}

/// Prefix string for scan operations: 1-5 lowercase letters followed by ":".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanPrefixWithColon(pub String);

impl TypeGenerator for ScanPrefixWithColon {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let len = 1 + (driver.produce::<usize>()? % 5);
        let mut prefix = String::with_capacity(len + 1);
        for _ in 0..len {
            let idx = driver.produce::<usize>()? % LOWERCASE.len();
            prefix.push(LOWERCASE[idx] as char);
        }
        prefix.push(':');
        Some(ScanPrefixWithColon(prefix))
    }
}

/// Number of keys for scan tests (1-20).
#[derive(Debug, Clone, Copy)]
pub struct ScanKeyCount(pub usize);

impl TypeGenerator for ScanKeyCount {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let n = 1 + (driver.produce::<usize>()? % 19);
        Some(ScanKeyCount(n))
    }
}

/// Binary data for TUI RPC (0-1000 bytes).
#[derive(Debug, Clone)]
pub struct TuiBinaryData(pub Vec<u8>);

impl TypeGenerator for TuiBinaryData {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let len = driver.produce::<usize>()? % 1001;
        let mut data = Vec::with_capacity(len);
        for _ in 0..len {
            data.push(driver.produce::<u8>()?);
        }
        Some(TuiBinaryData(data))
    }
}

/// Hex address string for AddLearner (32 alphanumeric chars).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HexAddrString(pub String);

impl TypeGenerator for HexAddrString {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let mut addr = String::with_capacity(32);
        for _ in 0..32 {
            let idx = driver.produce::<usize>()? % ALPHANUMERIC_UNDERSCORE.len();
            addr.push(ALPHANUMERIC_UNDERSCORE[idx] as char);
        }
        Some(HexAddrString(addr))
    }
}

/// Optional error string for response types.
#[derive(Debug, Clone)]
pub struct OptionalErrorString(pub Option<String>);

impl TypeGenerator for OptionalErrorString {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let has_error = driver.produce::<bool>()?;
        let error = if has_error {
            let len = 10 + (driver.produce::<usize>()? % 41);
            let mut msg = String::with_capacity(len);
            for _ in 0..len {
                let idx = driver.produce::<usize>()? % ALPHANUMERIC_SPACE.len();
                msg.push(ALPHANUMERIC_SPACE[idx] as char);
            }
            Some(msg)
        } else {
            None
        };
        Some(OptionalErrorString(error))
    }
}

/// Optional binary value for read results.
#[derive(Debug, Clone)]
pub struct OptionalBinaryValue(pub Option<Vec<u8>>);

impl TypeGenerator for OptionalBinaryValue {
    fn generate<D: Driver>(driver: &mut D) -> Option<Self> {
        let has_value = driver.produce::<bool>()?;
        let value = if has_value {
            let len = driver.produce::<usize>()? % 101;
            let mut data = Vec::with_capacity(len);
            for _ in 0..len {
                data.push(driver.produce::<u8>()?);
            }
            Some(data)
        } else {
            None
        };
        Some(OptionalBinaryValue(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bolero::check;

    #[test]
    fn test_valid_key_produces_valid_keys() {
        check!()
            .with_iterations(100)
            .with_type::<ValidKey>()
            .for_each(|key| {
                assert!(!key.0.is_empty(), "Keys should not be empty");
                assert!(key.0.len() <= 20, "Keys should be <= 20 chars");
                // First char should be lowercase
                assert!(
                    key.0.chars().next().unwrap().is_ascii_lowercase(),
                    "First char should be lowercase"
                );
            });
    }

    #[test]
    fn test_valid_value_produces_valid_values() {
        check!()
            .with_iterations(100)
            .with_type::<ValidValue>()
            .for_each(|value| {
                assert!(!value.0.is_empty(), "Values should not be empty");
                assert!(value.0.len() <= 100, "Values should be <= 100 chars");
            });
    }

    #[test]
    fn test_balanced_app_request_covers_variants() {
        let mut has_set = false;
        let mut has_set_multi = false;
        let mut has_delete = false;
        let mut has_delete_multi = false;

        check!()
            .with_iterations(200)
            .with_type::<BalancedAppRequest>()
            .for_each(|req| match &req.0 {
                AppRequest::Set { .. } => has_set = true,
                AppRequest::SetMulti { .. } => has_set_multi = true,
                AppRequest::Delete { .. } => has_delete = true,
                AppRequest::DeleteMulti { .. } => has_delete_multi = true,
                // TTL, CAS, Batch, and Lease operations are not generated by BalancedAppRequest yet
                AppRequest::SetWithTTL { .. }
                | AppRequest::SetMultiWithTTL { .. }
                | AppRequest::CompareAndSwap { .. }
                | AppRequest::CompareAndDelete { .. }
                | AppRequest::Batch { .. }
                | AppRequest::ConditionalBatch { .. }
                | AppRequest::SetWithLease { .. }
                | AppRequest::SetMultiWithLease { .. }
                | AppRequest::LeaseGrant { .. }
                | AppRequest::LeaseRevoke { .. }
                | AppRequest::LeaseKeepalive { .. }
                | AppRequest::Transaction { .. }
                | AppRequest::OptimisticTransaction { .. }
                | AppRequest::ShardSplit { .. }
                | AppRequest::ShardMerge { .. }
                | AppRequest::TopologyUpdate { .. } => {}
            });

        // With 200 iterations and 25% probability each, should see all variants
        assert!(has_set, "Should generate Set operations");
        assert!(has_set_multi, "Should generate SetMulti operations");
        assert!(has_delete, "Should generate Delete operations");
        assert!(has_delete_multi, "Should generate DeleteMulti operations");
    }

    #[test]
    fn test_boundary_key_at_limits() {
        check!()
            .with_iterations(50)
            .with_type::<BoundaryKey>()
            .for_each(|key| {
                assert!(
                    key.0.len() >= MAX_KEY_SIZE as usize - 1,
                    "Boundary keys should be near the limit"
                );
                assert!(
                    key.0.len() <= MAX_KEY_SIZE as usize + 1,
                    "Boundary keys should not exceed limit + 1"
                );
            });
    }

    #[test]
    fn test_oversized_setmulti_exceeds_limit() {
        check!()
            .with_iterations(20)
            .with_type::<OversizedSetMulti>()
            .for_each(|pairs| {
                assert!(
                    pairs.0.len() > MAX_SETMULTI_KEYS as usize,
                    "Oversized SetMulti should exceed limit"
                );
            });
    }

    #[test]
    fn test_valid_node_id_string_parses() {
        check!()
            .with_iterations(100)
            .with_type::<ValidNodeIdString>()
            .for_each(|s| {
                let parsed: Result<u64, _> = s.0.parse();
                assert!(
                    parsed.is_ok(),
                    "Valid NodeId string should parse as u64: {}",
                    s.0
                );
            });
    }

    #[test]
    fn test_invalid_node_id_string_fails_parse() {
        check!()
            .with_iterations(100)
            .with_type::<InvalidNodeIdString>()
            .for_each(|s| {
                let parsed: Result<u64, _> = s.0.parse();
                assert!(
                    parsed.is_err(),
                    "Invalid NodeId string should fail u64 parse: {}",
                    s.0
                );
            });
    }

    #[test]
    fn test_network_delay_config_valid_ranges() {
        check!()
            .with_iterations(100)
            .with_type::<NetworkDelayConfig>()
            .for_each(|config| {
                assert!(
                    config.max_latency_ms >= config.min_latency_ms,
                    "Max latency should be >= min latency"
                );
                assert!(config.jitter_percent <= 100, "Jitter should be <= 100%");
            });
    }

    #[test]
    fn test_operation_sequence_nonempty() {
        check!()
            .with_iterations(50)
            .with_type::<OperationSequence>()
            .for_each(|ops| {
                assert!(!ops.0.is_empty(), "Operation sequence should not be empty");
                assert!(ops.0.len() < 50, "Operation sequence should respect max");
            });
    }
}
