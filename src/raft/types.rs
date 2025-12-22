//! Type definitions for Raft consensus configuration.
//!
//! This module defines the core type configuration for openraft, specifying
//! the concrete types used for nodes, requests, responses, and storage.
//!
//! # Type Configuration
//!
//! - **NodeId**: Newtype wrapper around `u64` for type-safe node identification
//! - **Node**: `RaftMemberInfo` - Raft membership metadata with Iroh P2P addresses
//! - **AppRequest**: Application-level write commands (Set, SetMulti)
//! - **AppResponse**: Application-level read/write responses
//!
//! # Tiger Style
//!
//! - Explicitly sized types: `u64` for NodeId (not usize for portability)
//! - Newtype pattern: Prevents accidental mixing with log indices, terms, ports
//! - Bounded operations: SetMulti limited by MAX_SETMULTI_KEYS constant
//!
//! # Test Coverage
//!
//! Unit tests in `#[cfg(test)]` module below cover:
//!   - NodeId: FromStr parsing, Display formatting, ordering, conversions
//!   - AppRequest: All variants, Display formatting
//!   - AppResponse: Construction and field access
//!   - RaftMemberInfo: Construction, Default, Display

use std::fmt;
use std::num::ParseIntError;
use std::str::FromStr;

use iroh::EndpointAddr;
use openraft::declare_raft_types;
use serde::{Deserialize, Serialize};

/// Type-safe node identifier for Raft cluster nodes.
///
/// This newtype wrapper around `u64` prevents accidental mixing with other
/// numeric types like log indices, term numbers, or port numbers. The compiler
/// enforces correct usage at type-check time rather than runtime.
///
/// # Tiger Style
///
/// - Zero overhead: Compiles to bare `u64`, no runtime cost
/// - Ergonomic conversions: `From`/`Into` traits for seamless integration
/// - String parsing: `FromStr` with explicit error handling
/// - Ordering: Derived `PartialOrd`/`Ord` for deterministic sorting
///
/// # Example
///
/// ```ignore
/// use aspen::raft::types::NodeId;
///
/// let node_id = NodeId::new(1);
/// let node_id: NodeId = 42.into();
/// let node_id: NodeId = "123".parse()?;
/// let raw: u64 = node_id.into();
/// ```
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub struct NodeId(pub u64);

impl NodeId {
    /// Create a new `NodeId` from a raw `u64`.
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u64> for NodeId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<NodeId> for u64 {
    fn from(value: NodeId) -> Self {
        value.0
    }
}

impl FromStr for NodeId {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u64>().map(NodeId)
    }
}

/// Raft membership metadata containing Iroh P2P connection information.
///
/// This type is stored in Raft's membership set alongside each `NodeId`. Unlike
/// openraft's `BasicNode` which stores a simple address string, `RaftMemberInfo`
/// stores the full Iroh `EndpointAddr` containing:
/// - Endpoint ID (public key identifier)
/// - Relay URLs for NAT traversal
/// - Direct socket addresses
///
/// This enables peer addresses to be replicated via Raft consensus,
/// persisted in the state machine, and recovered on restart without
/// requiring gossip rediscovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaftMemberInfo {
    /// The Iroh endpoint address for connecting to this node.
    pub iroh_addr: EndpointAddr,
}

impl RaftMemberInfo {
    /// Creates a new `RaftMemberInfo` with the given Iroh endpoint address.
    pub fn new(iroh_addr: EndpointAddr) -> Self {
        Self { iroh_addr }
    }
}

impl Default for RaftMemberInfo {
    /// Creates a default `RaftMemberInfo` with a zero endpoint ID.
    ///
    /// This is primarily used by testing utilities (e.g., openraft's `membership_ent`)
    /// that require nodes to implement `Default`. In production, use `RaftMemberInfo::new()`
    /// or `create_test_raft_member_info()` to create nodes with proper endpoint addresses.
    fn default() -> Self {
        use iroh::{EndpointId, SecretKey};

        let seed = [0u8; 32];
        let secret_key = SecretKey::from(seed);
        let endpoint_id: EndpointId = secret_key.public();

        Self {
            iroh_addr: EndpointAddr::new(endpoint_id),
        }
    }
}

impl fmt::Display for RaftMemberInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RaftMemberInfo({})", self.iroh_addr.id)
    }
}

/// Application-level requests replicated through Raft.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AppRequest {
    Set {
        key: String,
        value: String,
    },
    /// Set with time-to-live. expires_at_ms is Unix timestamp when key expires.
    SetWithTTL {
        key: String,
        value: String,
        /// Expiration time as Unix timestamp in milliseconds.
        expires_at_ms: u64,
    },
    /// Set a key attached to a lease. Key is deleted when lease expires or is revoked.
    SetWithLease {
        key: String,
        value: String,
        /// Lease ID to attach this key to.
        lease_id: u64,
    },
    SetMulti {
        pairs: Vec<(String, String)>,
    },
    /// Set multiple keys with TTL.
    SetMultiWithTTL {
        pairs: Vec<(String, String)>,
        /// Expiration time as Unix timestamp in milliseconds.
        expires_at_ms: u64,
    },
    /// Set multiple keys attached to a lease.
    SetMultiWithLease {
        pairs: Vec<(String, String)>,
        /// Lease ID to attach these keys to.
        lease_id: u64,
    },
    Delete {
        key: String,
    },
    DeleteMulti {
        keys: Vec<String>,
    },
    /// Compare-and-swap: atomically update value if current value matches expected.
    CompareAndSwap {
        key: String,
        expected: Option<String>,
        new_value: String,
    },
    /// Compare-and-delete: atomically delete key if current value matches expected.
    CompareAndDelete {
        key: String,
        expected: String,
    },
    /// Batch write: atomically apply multiple Set/Delete operations.
    Batch {
        /// Operations as (is_set, key, value). is_set=true for Set, false for Delete.
        /// Value is empty string for Delete operations.
        operations: Vec<(bool, String, String)>,
    },
    /// Conditional batch write: apply operations only if all conditions are met.
    ConditionalBatch {
        /// Conditions as (condition_type, key, expected_value).
        /// condition_type: 0=ValueEquals, 1=KeyExists, 2=KeyNotExists.
        /// expected_value is only used for ValueEquals.
        conditions: Vec<(u8, String, String)>,
        /// Operations as (is_set, key, value). is_set=true for Set, false for Delete.
        operations: Vec<(bool, String, String)>,
    },
    // =========================================================================
    // Lease operations
    // =========================================================================
    /// Grant a new lease with specified TTL.
    /// Returns a unique lease_id that can be attached to keys.
    LeaseGrant {
        /// Lease ID (client-provided or 0 for auto-generated).
        /// If 0, server generates a unique ID.
        lease_id: u64,
        /// Time-to-live in seconds.
        ttl_seconds: u32,
    },
    /// Revoke a lease and delete all attached keys.
    LeaseRevoke {
        /// Lease ID to revoke.
        lease_id: u64,
    },
    /// Refresh a lease's TTL (keepalive).
    LeaseKeepalive {
        /// Lease ID to refresh.
        lease_id: u64,
    },
    // =========================================================================
    // Multi-key transactions
    // =========================================================================
    /// Transaction with If/Then/Else semantics (etcd-style).
    /// Compares conditions, then executes success branch if all pass, else failure branch.
    Transaction {
        /// Comparison conditions as (target, op, key, value).
        /// target: 0=Value, 1=Version, 2=CreateRevision, 3=ModRevision.
        /// op: 0=Equal, 1=NotEqual, 2=Greater, 3=Less.
        compare: Vec<(u8, u8, String, String)>,
        /// Success branch operations as (op_type, key, value).
        /// op_type: 0=Put, 1=Delete, 2=Get, 3=Range.
        /// For Range, value contains the limit as a string.
        success: Vec<(u8, String, String)>,
        /// Failure branch operations (same format as success).
        failure: Vec<(u8, String, String)>,
    },
}

impl fmt::Display for AppRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppRequest::Set { key, value } => write!(f, "Set {{ key: {key}, value: {value} }}"),
            AppRequest::SetWithTTL {
                key,
                value,
                expires_at_ms,
            } => {
                write!(
                    f,
                    "SetWithTTL {{ key: {key}, value: {value}, expires_at_ms: {expires_at_ms} }}"
                )
            }
            AppRequest::SetWithLease {
                key,
                value,
                lease_id,
            } => {
                write!(
                    f,
                    "SetWithLease {{ key: {key}, value: {value}, lease_id: {lease_id} }}"
                )
            }
            AppRequest::SetMulti { pairs } => {
                write!(f, "SetMulti {{ pairs: [")?;
                for (i, (k, v)) in pairs.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "({k}, {v})")?;
                }
                write!(f, "] }}")
            }
            AppRequest::SetMultiWithTTL {
                pairs,
                expires_at_ms,
            } => {
                write!(
                    f,
                    "SetMultiWithTTL {{ pairs: {}, expires_at_ms: {expires_at_ms} }}",
                    pairs.len()
                )
            }
            AppRequest::SetMultiWithLease { pairs, lease_id } => {
                write!(
                    f,
                    "SetMultiWithLease {{ pairs: {}, lease_id: {lease_id} }}",
                    pairs.len()
                )
            }
            AppRequest::Delete { key } => write!(f, "Delete {{ key: {key} }}"),
            AppRequest::DeleteMulti { keys } => {
                write!(f, "DeleteMulti {{ keys: [")?;
                for (i, k) in keys.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}")?;
                }
                write!(f, "] }}")
            }
            AppRequest::CompareAndSwap {
                key,
                expected,
                new_value,
            } => {
                write!(
                    f,
                    "CompareAndSwap {{ key: {key}, expected: {expected:?}, new_value: {new_value} }}"
                )
            }
            AppRequest::CompareAndDelete { key, expected } => {
                write!(f, "CompareAndDelete {{ key: {key}, expected: {expected} }}")
            }
            AppRequest::Batch { operations } => {
                write!(f, "Batch {{ operations: {} }}", operations.len())
            }
            AppRequest::ConditionalBatch {
                conditions,
                operations,
            } => {
                write!(
                    f,
                    "ConditionalBatch {{ conditions: {}, operations: {} }}",
                    conditions.len(),
                    operations.len()
                )
            }
            AppRequest::LeaseGrant {
                lease_id,
                ttl_seconds,
            } => {
                write!(
                    f,
                    "LeaseGrant {{ lease_id: {lease_id}, ttl_seconds: {ttl_seconds} }}"
                )
            }
            AppRequest::LeaseRevoke { lease_id } => {
                write!(f, "LeaseRevoke {{ lease_id: {lease_id} }}")
            }
            AppRequest::LeaseKeepalive { lease_id } => {
                write!(f, "LeaseKeepalive {{ lease_id: {lease_id} }}")
            }
            AppRequest::Transaction {
                compare,
                success,
                failure,
            } => {
                write!(
                    f,
                    "Transaction {{ compare: {}, success: {}, failure: {} }}",
                    compare.len(),
                    success.len(),
                    failure.len()
                )
            }
        }
    }
}

/// Response returned to HTTP clients after applying a request.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AppResponse {
    pub value: Option<String>,
    /// Indicates whether a delete operation actually removed a key.
    /// None for operations where deletion is not applicable.
    pub deleted: Option<bool>,
    /// Indicates whether a compare-and-swap operation succeeded.
    /// - `Some(true)`: CAS condition matched and operation was applied
    /// - `Some(false)`: CAS condition did not match, operation was not applied
    /// - `None`: Not a CAS operation
    ///
    /// When `Some(false)`, the `value` field contains the actual current value
    /// of the key (or None if the key doesn't exist), allowing clients to retry.
    pub cas_succeeded: Option<bool>,
    /// Number of operations applied in a batch.
    /// Only set for Batch and ConditionalBatch operations.
    pub batch_applied: Option<u32>,
    /// For ConditionalBatch: index of first failed condition (0-indexed).
    /// Only set when conditions_met is Some(false).
    pub failed_condition_index: Option<u32>,
    /// For ConditionalBatch: whether all conditions were met.
    /// - `Some(true)`: All conditions passed, operations were applied
    /// - `Some(false)`: At least one condition failed, no operations applied
    /// - `None`: Not a conditional batch operation
    pub conditions_met: Option<bool>,
    // =========================================================================
    // Lease operation responses
    // =========================================================================
    /// Lease ID for LeaseGrant operation.
    pub lease_id: Option<u64>,
    /// TTL in seconds for lease operations.
    /// For LeaseGrant/LeaseKeepalive: the granted/remaining TTL.
    pub ttl_seconds: Option<u32>,
    /// Number of keys attached to a lease.
    /// For LeaseRevoke: number of keys deleted with the lease.
    pub keys_deleted: Option<u32>,
    // =========================================================================
    // Transaction operation responses
    // =========================================================================
    /// For Transaction: whether the success branch was executed.
    /// `Some(true)`: All comparisons passed, success branch executed.
    /// `Some(false)`: At least one comparison failed, failure branch executed.
    /// `None`: Not a transaction operation.
    pub succeeded: Option<bool>,
    /// For Transaction: results of the executed operations.
    pub txn_results: Option<Vec<crate::api::TxnOpResult>>,
    /// For Transaction: the cluster revision after this transaction.
    pub header_revision: Option<u64>,
}

declare_raft_types!(
    /// Declare type config used by Aspen's embedded Raft node.
    pub AppTypeConfig:
        D = AppRequest,
        R = AppResponse,
        NodeId = NodeId,
        Node = RaftMemberInfo,
);

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // NodeId Tests
    // =========================================================================

    #[test]
    fn test_node_id_new() {
        let id = NodeId::new(42);
        assert_eq!(id.0, 42);
    }

    #[test]
    fn test_node_id_from_u64() {
        let id: NodeId = 123.into();
        assert_eq!(id.0, 123);
    }

    #[test]
    fn test_node_id_into_u64() {
        let id = NodeId::new(456);
        let raw: u64 = id.into();
        assert_eq!(raw, 456);
    }

    #[test]
    fn test_node_id_display() {
        let id = NodeId::new(789);
        assert_eq!(format!("{}", id), "789");
    }

    #[test]
    fn test_node_id_from_str_valid() {
        let id: NodeId = "12345".parse().expect("should parse valid u64");
        assert_eq!(id.0, 12345);
    }

    #[test]
    fn test_node_id_from_str_zero() {
        let id: NodeId = "0".parse().expect("should parse zero");
        assert_eq!(id.0, 0);
    }

    #[test]
    fn test_node_id_from_str_max() {
        let id: NodeId = u64::MAX.to_string().parse().expect("should parse max");
        assert_eq!(id.0, u64::MAX);
    }

    #[test]
    fn test_node_id_from_str_invalid() {
        let result: Result<NodeId, _> = "not_a_number".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_node_id_from_str_negative() {
        let result: Result<NodeId, _> = "-1".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_node_id_from_str_overflow() {
        let result: Result<NodeId, _> = "99999999999999999999999999".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_node_id_ordering() {
        let a = NodeId::new(1);
        let b = NodeId::new(2);
        let c = NodeId::new(2);

        assert!(a < b);
        assert!(b > a);
        assert!(b == c);
        assert!(a <= b);
        assert!(b >= a);
    }

    #[test]
    fn test_node_id_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(NodeId::new(1));
        set.insert(NodeId::new(2));
        set.insert(NodeId::new(1)); // Duplicate

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_node_id_default() {
        let id = NodeId::default();
        assert_eq!(id.0, 0);
    }

    #[test]
    fn test_node_id_serde_roundtrip() {
        let original = NodeId::new(42);
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: NodeId = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_node_id_display_roundtrip() {
        let original = NodeId::new(12345);
        let displayed = original.to_string();
        let parsed: NodeId = displayed.parse().expect("should parse");
        assert_eq!(original, parsed);
    }

    // =========================================================================
    // AppRequest Tests
    // =========================================================================

    #[test]
    fn test_app_request_set_display() {
        let req = AppRequest::Set {
            key: "foo".to_string(),
            value: "bar".to_string(),
        };
        assert_eq!(format!("{}", req), "Set { key: foo, value: bar }");
    }

    #[test]
    fn test_app_request_delete_display() {
        let req = AppRequest::Delete {
            key: "foo".to_string(),
        };
        assert_eq!(format!("{}", req), "Delete { key: foo }");
    }

    #[test]
    fn test_app_request_set_multi_empty() {
        let req = AppRequest::SetMulti { pairs: vec![] };
        assert_eq!(format!("{}", req), "SetMulti { pairs: [] }");
    }

    #[test]
    fn test_app_request_set_multi_one() {
        let req = AppRequest::SetMulti {
            pairs: vec![("a".to_string(), "1".to_string())],
        };
        assert_eq!(format!("{}", req), "SetMulti { pairs: [(a, 1)] }");
    }

    #[test]
    fn test_app_request_set_multi_many() {
        let req = AppRequest::SetMulti {
            pairs: vec![
                ("a".to_string(), "1".to_string()),
                ("b".to_string(), "2".to_string()),
                ("c".to_string(), "3".to_string()),
            ],
        };
        assert_eq!(
            format!("{}", req),
            "SetMulti { pairs: [(a, 1), (b, 2), (c, 3)] }"
        );
    }

    #[test]
    fn test_app_request_delete_multi_empty() {
        let req = AppRequest::DeleteMulti { keys: vec![] };
        assert_eq!(format!("{}", req), "DeleteMulti { keys: [] }");
    }

    #[test]
    fn test_app_request_delete_multi_many() {
        let req = AppRequest::DeleteMulti {
            keys: vec!["x".to_string(), "y".to_string(), "z".to_string()],
        };
        assert_eq!(format!("{}", req), "DeleteMulti { keys: [x, y, z] }");
    }

    #[test]
    fn test_app_request_serde_set() {
        let original = AppRequest::Set {
            key: "test".to_string(),
            value: "value".to_string(),
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: AppRequest = serde_json::from_str(&json).expect("deserialize");
        assert!(
            matches!(deserialized, AppRequest::Set { key, value } if key == "test" && value == "value")
        );
    }

    #[test]
    fn test_app_request_serde_delete() {
        let original = AppRequest::Delete {
            key: "to_delete".to_string(),
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: AppRequest = serde_json::from_str(&json).expect("deserialize");
        assert!(matches!(deserialized, AppRequest::Delete { key } if key == "to_delete"));
    }

    #[test]
    fn test_app_request_serde_set_multi() {
        let original = AppRequest::SetMulti {
            pairs: vec![
                ("k1".to_string(), "v1".to_string()),
                ("k2".to_string(), "v2".to_string()),
            ],
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: AppRequest = serde_json::from_str(&json).expect("deserialize");
        assert!(matches!(deserialized, AppRequest::SetMulti { pairs } if pairs.len() == 2));
    }

    #[test]
    fn test_app_request_serde_delete_multi() {
        let original = AppRequest::DeleteMulti {
            keys: vec!["a".to_string(), "b".to_string()],
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: AppRequest = serde_json::from_str(&json).expect("deserialize");
        assert!(matches!(deserialized, AppRequest::DeleteMulti { keys } if keys.len() == 2));
    }

    // =========================================================================
    // AppResponse Tests
    // =========================================================================

    #[test]
    fn test_app_response_with_value() {
        let resp = AppResponse {
            value: Some("result".to_string()),
            ..Default::default()
        };
        assert_eq!(resp.value, Some("result".to_string()));
        assert!(resp.deleted.is_none());
        assert!(resp.cas_succeeded.is_none());
    }

    #[test]
    fn test_app_response_deleted_true() {
        let resp = AppResponse {
            deleted: Some(true),
            ..Default::default()
        };
        assert!(resp.deleted == Some(true));
    }

    #[test]
    fn test_app_response_deleted_false() {
        let resp = AppResponse {
            deleted: Some(false),
            ..Default::default()
        };
        assert!(resp.deleted == Some(false));
    }

    #[test]
    fn test_app_response_cas_succeeded() {
        let resp = AppResponse {
            value: Some("new_value".to_string()),
            cas_succeeded: Some(true),
            ..Default::default()
        };
        assert_eq!(resp.cas_succeeded, Some(true));
    }

    #[test]
    fn test_app_response_cas_failed() {
        let resp = AppResponse {
            value: Some("actual_value".to_string()),
            cas_succeeded: Some(false),
            ..Default::default()
        };
        assert_eq!(resp.cas_succeeded, Some(false));
        assert_eq!(resp.value, Some("actual_value".to_string()));
    }

    #[test]
    fn test_app_response_serde_roundtrip() {
        let original = AppResponse {
            value: Some("test_value".to_string()),
            deleted: Some(true),
            ..Default::default()
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: AppResponse = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original.value, deserialized.value);
        assert_eq!(original.deleted, deserialized.deleted);
        assert_eq!(original.cas_succeeded, deserialized.cas_succeeded);
    }

    #[test]
    fn test_app_response_cas_serde_roundtrip() {
        let original = AppResponse {
            value: Some("cas_value".to_string()),
            cas_succeeded: Some(true),
            ..Default::default()
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: AppResponse = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original.cas_succeeded, deserialized.cas_succeeded);
    }

    // =========================================================================
    // RaftMemberInfo Tests
    // =========================================================================

    #[test]
    fn test_raft_member_info_default() {
        let info = RaftMemberInfo::default();
        // Default should create a valid RaftMemberInfo with a zero-seed endpoint
        assert!(format!("{}", info).starts_with("RaftMemberInfo("));
    }

    #[test]
    fn test_raft_member_info_display() {
        let info = RaftMemberInfo::default();
        let display = format!("{}", info);
        // Should contain "RaftMemberInfo(" prefix
        assert!(display.contains("RaftMemberInfo("));
    }

    #[test]
    fn test_raft_member_info_equality() {
        let info1 = RaftMemberInfo::default();
        let info2 = RaftMemberInfo::default();
        // Two defaults with same seed should be equal
        assert_eq!(info1, info2);
    }

    #[test]
    fn test_raft_member_info_new() {
        use iroh::{EndpointAddr, EndpointId, SecretKey};

        let seed = [1u8; 32];
        let secret_key = SecretKey::from(seed);
        let endpoint_id: EndpointId = secret_key.public();
        let addr = EndpointAddr::new(endpoint_id);

        let info = RaftMemberInfo::new(addr.clone());
        assert_eq!(info.iroh_addr.id, endpoint_id);
    }

    #[test]
    fn test_raft_member_info_serde_roundtrip() {
        let original = RaftMemberInfo::default();
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: RaftMemberInfo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_raft_member_info_clone() {
        let original = RaftMemberInfo::default();
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }
}
