//! Reusable OpenRaft KV app types.
//!
//! This crate is the reusable type boundary for the Redb Raft KV stack. It owns
//! OpenRaft app data, membership metadata, and storage-facing errors without
//! pulling Aspen node bootstrap, handler registries, trust, secrets, SQL,
//! coordination runtime, dogfood defaults, binaries, or concrete iroh endpoint
//! construction into the default feature set.

#![forbid(unsafe_code)]

use core::fmt;

use aspen_kv_types::TxnOpResult;
use openraft::declare_raft_types;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

/// Node identifier used by the reusable Redb Raft KV stack.
pub type NodeId = u64;

const DEFAULT_MEMBER_ENDPOINT_ID: &str = "placeholder-node";

/// Transport-neutral Raft membership metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaftKvMemberInfo {
    /// Stable endpoint identifier for this member.
    pub endpoint_id: String,
    /// Transport addresses encoded as strings by the selected adapter.
    pub transport_addrs: Vec<String>,
    /// Optional cluster-internal relay server URL for this member.
    pub relay_url: Option<String>,
}

impl RaftKvMemberInfo {
    /// Create metadata with no relay URL.
    #[must_use]
    pub fn new(endpoint_id: String, transport_addrs: Vec<String>) -> Self {
        assert!(!endpoint_id.is_empty());
        Self {
            endpoint_id,
            transport_addrs,
            relay_url: None,
        }
    }

    /// Create metadata with an explicit relay URL.
    #[must_use]
    pub fn with_relay(endpoint_id: String, transport_addrs: Vec<String>, relay_url: String) -> Self {
        assert!(!endpoint_id.is_empty());
        assert!(!relay_url.is_empty());
        Self {
            endpoint_id,
            transport_addrs,
            relay_url: Some(relay_url),
        }
    }

    /// Count configured transport addresses.
    #[must_use]
    pub fn transport_addr_count(&self) -> u32 {
        u32::try_from(self.transport_addrs.len()).unwrap_or(u32::MAX)
    }
}

impl Default for RaftKvMemberInfo {
    fn default() -> Self {
        Self {
            endpoint_id: DEFAULT_MEMBER_ENDPOINT_ID.to_string(),
            transport_addrs: Vec::new(),
            relay_url: None,
        }
    }
}

impl fmt::Display for RaftKvMemberInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RaftKvMemberInfo({})", self.endpoint_id)
    }
}

/// Application-level requests replicated by the reusable KV state machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RaftKvRequest {
    /// Set a single key-value pair.
    Set { key: String, value: String },
    /// Set a single key-value pair until an absolute Unix timestamp in milliseconds.
    SetWithTtl {
        key: String,
        value: String,
        expires_at_ms: u64,
    },
    /// Set a single key-value pair attached to a lease.
    SetWithLease { key: String, value: String, lease_id: u64 },
    /// Set multiple key-value pairs atomically.
    SetMulti { pairs: Vec<(String, String)> },
    /// Set multiple key-value pairs with a shared absolute expiration timestamp.
    SetMultiWithTtl {
        pairs: Vec<(String, String)>,
        expires_at_ms: u64,
    },
    /// Set multiple key-value pairs attached to a lease.
    SetMultiWithLease {
        pairs: Vec<(String, String)>,
        lease_id: u64,
    },
    /// Delete one key.
    Delete { key: String },
    /// Delete multiple keys atomically.
    DeleteMulti { keys: Vec<String> },
    /// Compare-and-swap one key.
    CompareAndSwap {
        key: String,
        expected: Option<String>,
        new_value: String,
    },
    /// Compare-and-delete one key.
    CompareAndDelete { key: String, expected: String },
    /// Batch set/delete operations.
    Batch { operations: Vec<BatchWriteOp> },
    /// Conditional batch set/delete operations.
    ConditionalBatch {
        conditions: Vec<BatchCondition>,
        operations: Vec<BatchWriteOp>,
    },
    /// Grant or refresh lease state.
    LeaseGrant { lease_id: u64, ttl_seconds: u32 },
    /// Revoke a lease and delete attached keys.
    LeaseRevoke { lease_id: u64 },
    /// Keep a lease alive.
    LeaseKeepalive { lease_id: u64 },
    /// Etcd-style transaction.
    Transaction {
        compare: Vec<TxnCompareSpec>,
        success: Vec<TxnOpSpec>,
        failure: Vec<TxnOpSpec>,
    },
    /// Optimistic transaction with read-set validation.
    OptimisticTransaction {
        read_set: Vec<(String, i64)>,
        write_set: Vec<BatchWriteOp>,
    },
}

impl fmt::Display for RaftKvRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Set { key, .. } => write!(f, "Set {{ key: {key} }}"),
            Self::SetWithTtl { key, expires_at_ms, .. } => {
                write!(f, "SetWithTtl {{ key: {key}, expires_at_ms: {expires_at_ms} }}")
            }
            Self::SetWithLease { key, lease_id, .. } => {
                write!(f, "SetWithLease {{ key: {key}, lease_id: {lease_id} }}")
            }
            Self::SetMulti { pairs } => write!(f, "SetMulti {{ pairs: {} }}", pairs.len()),
            Self::SetMultiWithTtl { pairs, expires_at_ms } => {
                write!(f, "SetMultiWithTtl {{ pairs: {}, expires_at_ms: {expires_at_ms} }}", pairs.len())
            }
            Self::SetMultiWithLease { pairs, lease_id } => {
                write!(f, "SetMultiWithLease {{ pairs: {}, lease_id: {lease_id} }}", pairs.len())
            }
            Self::Delete { key } => write!(f, "Delete {{ key: {key} }}"),
            Self::DeleteMulti { keys } => write!(f, "DeleteMulti {{ keys: {} }}", keys.len()),
            Self::CompareAndSwap { key, .. } => write!(f, "CompareAndSwap {{ key: {key} }}"),
            Self::CompareAndDelete { key, .. } => write!(f, "CompareAndDelete {{ key: {key} }}"),
            Self::Batch { operations } => write!(f, "Batch {{ operations: {} }}", operations.len()),
            Self::ConditionalBatch { conditions, operations } => {
                write!(f, "ConditionalBatch {{ conditions: {}, operations: {} }}", conditions.len(), operations.len())
            }
            Self::LeaseGrant { lease_id, ttl_seconds } => {
                write!(f, "LeaseGrant {{ lease_id: {lease_id}, ttl_seconds: {ttl_seconds} }}")
            }
            Self::LeaseRevoke { lease_id } => write!(f, "LeaseRevoke {{ lease_id: {lease_id} }}"),
            Self::LeaseKeepalive { lease_id } => write!(f, "LeaseKeepalive {{ lease_id: {lease_id} }}"),
            Self::Transaction {
                compare,
                success,
                failure,
            } => write!(
                f,
                "Transaction {{ compare: {}, success: {}, failure: {} }}",
                compare.len(),
                success.len(),
                failure.len()
            ),
            Self::OptimisticTransaction { read_set, write_set } => {
                write!(f, "OptimisticTransaction {{ read_set: {}, write_set: {} }}", read_set.len(), write_set.len())
            }
        }
    }
}

/// Batch write operation with named fields instead of tuple booleans.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BatchWriteOp {
    /// Put key/value.
    Put { key: String, value: String },
    /// Delete key.
    Delete { key: String },
}

/// Conditional batch precondition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BatchCondition {
    /// Stored value equals expected value.
    ValueEquals { key: String, expected: String },
    /// Key exists.
    KeyExists { key: String },
    /// Key does not exist.
    KeyNotExists { key: String },
}

/// Transaction comparison target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxnCompareTarget {
    /// Compare stored value.
    Value,
    /// Compare key version.
    Version,
    /// Compare creation revision.
    CreateRevision,
    /// Compare modification revision.
    ModRevision,
}

/// Transaction comparison operator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxnCompareOp {
    /// Equal.
    Equal,
    /// Not equal.
    NotEqual,
    /// Greater than.
    Greater,
    /// Less than.
    Less,
}

/// Named transaction comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxnCompareSpec {
    /// Field to compare.
    pub target: TxnCompareTarget,
    /// Comparison operator.
    pub op: TxnCompareOp,
    /// Key being compared.
    pub key: String,
    /// Expected value encoded as string for value/revision comparisons.
    pub value: String,
}

/// Named transaction operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxnOpSpec {
    /// Put key/value.
    Put { key: String, value: String },
    /// Delete key.
    Delete { key: String },
    /// Get key.
    Get { key: String },
    /// Range from key with bounded result count.
    Range { key: String, limit: u32 },
}

/// Response returned after applying a reusable KV request.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaftKvResponse {
    /// Retrieved value or actual value for failed CAS.
    pub value: Option<String>,
    /// Whether a delete removed a key.
    pub deleted: Option<bool>,
    /// Whether compare-and-swap succeeded.
    pub cas_succeeded: Option<bool>,
    /// Number of batch operations applied.
    pub batch_applied: Option<u32>,
    /// First failed conditional-batch condition index.
    pub failed_condition_index: Option<u32>,
    /// Whether conditional-batch conditions passed.
    pub conditions_met: Option<bool>,
    /// Lease identifier.
    pub lease_id: Option<u64>,
    /// Lease TTL in seconds.
    pub ttl_seconds: Option<u32>,
    /// Number of keys deleted through a lease action.
    pub keys_deleted: Option<u32>,
    /// Whether a transaction executed the success branch.
    pub succeeded: Option<bool>,
    /// Results of executed transaction operations.
    pub txn_results: Option<Vec<TxnOpResult>>,
    /// Cluster revision after transaction execution.
    pub header_revision: Option<u64>,
    /// Key that caused optimistic-transaction conflict.
    pub conflict_key: Option<String>,
    /// Expected conflict version.
    pub conflict_expected_version: Option<i64>,
    /// Actual conflict version.
    pub conflict_actual_version: Option<i64>,
    /// Whether optimistic conflict was detected.
    pub occ_conflict: Option<bool>,
}

/// Storage-level errors shared by reusable storage/facade layers.
#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum RaftKvStorageError {
    /// Storage backend failed.
    #[error("storage failure: {reason}")]
    Storage { reason: String },
    /// Snapshot data failed validation.
    #[error("snapshot integrity failure: {reason}")]
    SnapshotIntegrity { reason: String },
    /// Chain hash verification failed.
    #[error("chain integrity failure at index {index}: {reason}")]
    ChainIntegrity { index: u64, reason: String },
    /// Request could not be applied.
    #[error("apply failure: {reason}")]
    Apply { reason: String },
}

declare_raft_types!(
    /// OpenRaft type configuration for reusable Redb Raft KV.
    pub RaftKvTypeConfig:
        D = RaftKvRequest,
        R = RaftKvResponse,
        NodeId = NodeId,
        Node = RaftKvMemberInfo,
);

#[cfg(test)]
mod tests {
    use super::*;

    const NODE_ID: &str = "node-a";
    const RELAY_URL: &str = "https://relay.example";
    const TEST_KEY: &str = "alpha";
    const TEST_VALUE: &str = "one";
    const TRANSPORT_ADDR: &str = "quic://127.0.0.1:1234";
    const BAD_HASH_REASON: &str = "bad hash";
    const EXPIRES_AT_MS: u64 = 1_000;
    const LEASE_ID: u64 = 42;
    const TTL_SECONDS: u32 = 60;
    const CHAIN_INDEX: u64 = 7;

    #[test]
    fn member_info_records_transport_without_runtime_types() {
        let info =
            RaftKvMemberInfo::with_relay(NODE_ID.to_string(), vec![TRANSPORT_ADDR.to_string()], RELAY_URL.to_string());
        assert_eq!(info.endpoint_id, NODE_ID);
        assert_eq!(info.transport_addr_count(), 1);
        assert_eq!(info.relay_url.as_deref(), Some(RELAY_URL));
        assert!(format!("{info}").contains(NODE_ID));
    }

    #[test]
    fn member_info_rejects_empty_endpoint() {
        let result = std::panic::catch_unwind(|| RaftKvMemberInfo::new(String::new(), Vec::new()));
        assert!(result.is_err());
    }

    #[test]
    fn kv_request_serializes_without_trust_or_sharding_payloads() {
        let request = RaftKvRequest::SetWithTtl {
            key: TEST_KEY.to_string(),
            value: TEST_VALUE.to_string(),
            expires_at_ms: EXPIRES_AT_MS,
        };
        let bytes = postcard::to_allocvec(&request).expect("serialize request");
        let decoded: RaftKvRequest = postcard::from_bytes(&bytes).expect("deserialize request");
        assert_eq!(decoded, request);
        assert!(format!("{decoded}").contains(TEST_KEY));
    }

    #[test]
    fn response_carries_lease_and_transaction_results() {
        let response = RaftKvResponse {
            lease_id: Some(LEASE_ID),
            ttl_seconds: Some(TTL_SECONDS),
            txn_results: Some(Vec::new()),
            ..RaftKvResponse::default()
        };
        assert_eq!(response.lease_id, Some(LEASE_ID));
        assert_eq!(response.ttl_seconds, Some(TTL_SECONDS));
        assert_eq!(response.txn_results.as_ref().map(Vec::len), Some(0));
    }

    #[test]
    fn storage_error_display_includes_context() {
        let error = RaftKvStorageError::ChainIntegrity {
            index: CHAIN_INDEX,
            reason: BAD_HASH_REASON.to_string(),
        };
        let rendered = error.to_string();
        assert!(rendered.contains(&CHAIN_INDEX.to_string()));
        assert!(rendered.contains(BAD_HASH_REASON));
    }

    #[test]
    fn raft_type_config_is_constructible_at_type_level() {
        static_assertions::assert_impl_all!(RaftKvRequest: Clone, core::fmt::Debug);
        static_assertions::assert_impl_all!(RaftKvResponse: Clone, core::fmt::Debug, Default);
        static_assertions::assert_impl_all!(RaftKvMemberInfo: Clone, core::fmt::Debug, Default);
    }
}
