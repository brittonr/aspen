//! Application-level request and response types for Raft consensus.
//!
//! This module defines the core data types that are replicated through Raft:
//! - [`AppRequest`]: Write commands submitted to the cluster
//! - [`AppResponse`]: Responses returned after applying commands
//!
//! # Tiger Style
//!
//! - Bounded operations: SetMulti limited by MAX_SETMULTI_KEYS constant
//! - Explicit types: String keys/values for text data, u64 for timestamps/IDs
//! - Clear semantics: Each variant has well-defined success/failure behavior

use std::fmt;

use aspen_core::TxnOpResult;
use serde::Deserialize;
use serde::Serialize;

/// Application-level requests replicated through Raft.
///
/// All write operations go through Raft consensus to ensure linearizability.
/// Each variant represents a different KV operation or cluster management command.
///
/// # Operation Categories
///
/// - **Basic KV**: Set, SetMulti, Delete, DeleteMulti
/// - **TTL-based expiration**: SetWithTTL, SetMultiWithTTL
/// - **Lease-based expiration**: SetWithLease, SetMultiWithLease, LeaseGrant, LeaseRevoke,
///   LeaseKeepalive
/// - **Atomic operations**: CompareAndSwap, CompareAndDelete, Batch, ConditionalBatch
/// - **Transactions**: Transaction (etcd-style If/Then/Else semantics)
///
/// # Tiger Style
///
/// - Bounded operations: SetMulti limited by MAX_SETMULTI_KEYS constant
/// - Explicit types: String keys/values for text data, u64 for timestamps/IDs
/// - Clear semantics: Each variant has well-defined success/failure behavior
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AppRequest {
    /// Set a single key-value pair.
    Set {
        /// Key to set.
        key: String,
        /// Value to store.
        value: String,
    },
    /// Set with time-to-live. expires_at_ms is Unix timestamp when key expires.
    SetWithTTL {
        /// Key to set.
        key: String,
        /// Value to store.
        value: String,
        /// Expiration time as Unix timestamp in milliseconds.
        expires_at_ms: u64,
    },
    /// Set a key attached to a lease. Key is deleted when lease expires or is revoked.
    SetWithLease {
        /// Key to set.
        key: String,
        /// Value to store.
        value: String,
        /// Lease ID to attach this key to.
        lease_id: u64,
    },
    /// Set multiple key-value pairs atomically.
    SetMulti {
        /// Vector of (key, value) pairs to set.
        pairs: Vec<(String, String)>,
    },
    /// Set multiple keys with TTL.
    SetMultiWithTTL {
        /// Vector of (key, value) pairs to set.
        pairs: Vec<(String, String)>,
        /// Expiration time as Unix timestamp in milliseconds.
        expires_at_ms: u64,
    },
    /// Set multiple keys attached to a lease.
    SetMultiWithLease {
        /// Vector of (key, value) pairs to set.
        pairs: Vec<(String, String)>,
        /// Lease ID to attach these keys to.
        lease_id: u64,
    },
    /// Delete a single key.
    Delete {
        /// Key to delete.
        key: String,
    },
    /// Delete multiple keys atomically.
    DeleteMulti {
        /// Vector of keys to delete.
        keys: Vec<String>,
    },
    /// Compare-and-swap: atomically update value if current value matches expected.
    CompareAndSwap {
        /// Key to compare and swap.
        key: String,
        /// Expected current value (None if key should not exist).
        expected: Option<String>,
        /// New value to set if comparison succeeds.
        new_value: String,
    },
    /// Compare-and-delete: atomically delete key if current value matches expected.
    CompareAndDelete {
        /// Key to compare and delete.
        key: String,
        /// Expected current value that must match for deletion to succeed.
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
    // =========================================================================
    // Optimistic Concurrency Control (OCC) transaction
    // =========================================================================
    /// Optimistic transaction with read set conflict detection (FoundationDB-style).
    /// Validates that all keys in read_set still have expected versions, then applies write_set.
    /// If any key version has changed, returns ConflictError without applying any operations.
    OptimisticTransaction {
        /// Read set: keys with their expected versions at read time.
        /// Each tuple is (key, expected_version).
        /// If current version != expected_version, transaction fails with conflict.
        read_set: Vec<(String, i64)>,
        /// Write set: operations to apply if all version checks pass.
        /// Each tuple is (is_set, key, value). is_set=true for Set, false for Delete.
        write_set: Vec<(bool, String, String)>,
    },
    // =========================================================================
    // Shard topology operations
    // =========================================================================
    /// Split a shard into two shards at a given key.
    /// Applied atomically through Raft consensus on shard 0 (control plane).
    ShardSplit {
        /// Shard being split.
        source_shard: u32,
        /// Key at which to split (keys >= this go to new shard).
        split_key: String,
        /// ID for the new shard being created.
        new_shard_id: u32,
        /// Expected topology version (prevents stale splits).
        topology_version: u64,
    },
    /// Merge two adjacent shards into one.
    ShardMerge {
        /// Shard to be merged (will become tombstone).
        source_shard: u32,
        /// Shard to merge into (will expand to cover source's range).
        target_shard: u32,
        /// Expected topology version (prevents stale merges).
        topology_version: u64,
    },
    /// Direct topology update (internal command).
    TopologyUpdate {
        /// Serialized ShardTopology (bincode).
        topology_data: Vec<u8>,
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
                write!(f, "SetWithTTL {{ key: {key}, value: {value}, expires_at_ms: {expires_at_ms} }}")
            }
            AppRequest::SetWithLease { key, value, lease_id } => {
                write!(f, "SetWithLease {{ key: {key}, value: {value}, lease_id: {lease_id} }}")
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
            AppRequest::SetMultiWithTTL { pairs, expires_at_ms } => {
                write!(f, "SetMultiWithTTL {{ pairs: {}, expires_at_ms: {expires_at_ms} }}", pairs.len())
            }
            AppRequest::SetMultiWithLease { pairs, lease_id } => {
                write!(f, "SetMultiWithLease {{ pairs: {}, lease_id: {lease_id} }}", pairs.len())
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
                write!(f, "CompareAndSwap {{ key: {key}, expected: {expected:?}, new_value: {new_value} }}")
            }
            AppRequest::CompareAndDelete { key, expected } => {
                write!(f, "CompareAndDelete {{ key: {key}, expected: {expected} }}")
            }
            AppRequest::Batch { operations } => {
                write!(f, "Batch {{ operations: {} }}", operations.len())
            }
            AppRequest::ConditionalBatch { conditions, operations } => {
                write!(f, "ConditionalBatch {{ conditions: {}, operations: {} }}", conditions.len(), operations.len())
            }
            AppRequest::LeaseGrant { lease_id, ttl_seconds } => {
                write!(f, "LeaseGrant {{ lease_id: {lease_id}, ttl_seconds: {ttl_seconds} }}")
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
            AppRequest::OptimisticTransaction { read_set, write_set } => {
                write!(f, "OptimisticTransaction {{ read_set: {}, write_set: {} }}", read_set.len(), write_set.len())
            }
            AppRequest::ShardSplit {
                source_shard,
                split_key,
                new_shard_id,
                topology_version,
            } => {
                write!(
                    f,
                    "ShardSplit {{ source: {source_shard}, split_key: {split_key}, new: {new_shard_id}, version: {topology_version} }}"
                )
            }
            AppRequest::ShardMerge {
                source_shard,
                target_shard,
                topology_version,
            } => {
                write!(
                    f,
                    "ShardMerge {{ source: {source_shard}, target: {target_shard}, version: {topology_version} }}"
                )
            }
            AppRequest::TopologyUpdate { topology_data } => {
                write!(f, "TopologyUpdate {{ size: {} }}", topology_data.len())
            }
        }
    }
}

/// Response returned after applying a Raft request.
///
/// Contains the result of executing an `AppRequest` through the Raft state machine.
/// Different operations populate different fields based on their semantics.
///
/// # Field Usage by Operation
///
/// - **Read operations**: `value` contains the retrieved value
/// - **Delete operations**: `deleted` indicates whether a key was actually removed
/// - **CAS operations**: `cas_succeeded` indicates success/failure, `value` contains current value
///   on failure
/// - **Batch operations**: `batch_applied` contains count of applied operations
/// - **Lease operations**: `lease_id`, `ttl_seconds`, `keys_deleted` for lease management
/// - **Transactions**: `succeeded`, `txn_results`, `header_revision` for transaction results
///
/// # Tiger Style
///
/// - Optional fields: Only relevant fields are populated for each operation type
/// - Explicit types: u32 for counts, u64 for IDs/revisions, bool for flags
/// - Clear semantics: Field names indicate their purpose (e.g., `cas_succeeded` vs `succeeded`)
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AppResponse {
    /// Value retrieved by a read operation, or None if key doesn't exist.
    ///
    /// For CAS failures, contains the actual current value to enable retries.
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
    pub txn_results: Option<Vec<TxnOpResult>>,
    /// For Transaction: the cluster revision after this transaction.
    pub header_revision: Option<u64>,
    // =========================================================================
    // Optimistic Concurrency Control (OCC) responses
    // =========================================================================
    /// For OptimisticTransaction: key that caused a version conflict.
    /// Only set when occ_conflict is Some(true).
    pub conflict_key: Option<String>,
    /// For OptimisticTransaction: expected version of the conflicting key.
    /// Only set when occ_conflict is Some(true).
    pub conflict_expected_version: Option<i64>,
    /// For OptimisticTransaction: actual version of the conflicting key.
    /// Only set when occ_conflict is Some(true).
    pub conflict_actual_version: Option<i64>,
    /// For OptimisticTransaction: whether a version conflict was detected.
    /// - `Some(true)`: Conflict detected, transaction was NOT applied
    /// - `Some(false)` or `None`: No conflict, transaction was applied successfully
    pub occ_conflict: Option<bool>,
    // =========================================================================
    // Shard topology responses
    // =========================================================================
    /// For ShardSplit/ShardMerge/TopologyUpdate: the new topology version.
    /// Used for clients to track topology changes and invalidate caches.
    pub topology_version: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let req = AppRequest::Delete { key: "foo".to_string() };
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
        assert_eq!(format!("{}", req), "SetMulti { pairs: [(a, 1), (b, 2), (c, 3)] }");
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
        assert!(matches!(deserialized, AppRequest::Set { key, value } if key == "test" && value == "value"));
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
}
