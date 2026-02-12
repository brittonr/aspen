//! Write operation types for modifying key-value state.

use serde::Deserialize;
use serde::Serialize;

use crate::batch::BatchCondition;
use crate::batch::BatchOperation;
use crate::transaction::TxnCompare;
use crate::transaction::TxnOp;
use crate::transaction::TxnOpResult;

/// Commands for modifying key-value state through Raft consensus.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WriteCommand {
    /// Set a single key-value pair.
    Set { key: String, value: String },
    /// Set a key-value pair with a time-to-live.
    SetWithTTL {
        key: String,
        value: String,
        ttl_seconds: u32,
    },
    /// Set multiple key-value pairs atomically.
    SetMulti { pairs: Vec<(String, String)> },
    /// Set multiple keys with TTL.
    SetMultiWithTTL {
        pairs: Vec<(String, String)>,
        ttl_seconds: u32,
    },
    /// Delete a single key.
    Delete { key: String },
    /// Delete multiple keys atomically.
    DeleteMulti { keys: Vec<String> },
    /// Compare-and-swap: atomically update value if current value matches expected.
    CompareAndSwap {
        key: String,
        expected: Option<String>,
        new_value: String,
    },
    /// Compare-and-delete: atomically delete key if current value matches expected.
    CompareAndDelete { key: String, expected: String },
    /// Batch write: atomically apply multiple Set/Delete operations.
    Batch { operations: Vec<BatchOperation> },
    /// Conditional batch write: apply operations only if all conditions are met.
    ConditionalBatch {
        conditions: Vec<BatchCondition>,
        operations: Vec<BatchOperation>,
    },
    /// Set a key attached to a lease.
    SetWithLease { key: String, value: String, lease_id: u64 },
    /// Set multiple keys attached to a lease.
    SetMultiWithLease {
        pairs: Vec<(String, String)>,
        lease_id: u64,
    },
    /// Grant a new lease with specified TTL.
    LeaseGrant { lease_id: u64, ttl_seconds: u32 },
    /// Revoke a lease and delete all attached keys.
    LeaseRevoke { lease_id: u64 },
    /// Refresh a lease's TTL.
    LeaseKeepalive { lease_id: u64 },
    /// Transaction: atomic If/Then/Else with rich comparisons.
    Transaction {
        compare: Vec<TxnCompare>,
        success: Vec<TxnOp>,
        failure: Vec<TxnOp>,
    },
    /// Optimistic transaction with read set conflict detection.
    OptimisticTransaction {
        read_set: Vec<(String, i64)>,
        write_set: Vec<WriteOp>,
    },
    /// Split a shard into two shards at a given key.
    ShardSplit {
        source_shard: u32,
        split_key: String,
        new_shard_id: u32,
        topology_version: u64,
    },
    /// Merge two adjacent shards into one.
    ShardMerge {
        source_shard: u32,
        target_shard: u32,
        topology_version: u64,
    },
    /// Update the shard topology (internal command).
    TopologyUpdate { topology_data: Vec<u8> },
}

/// Operations for optimistic transactions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WriteOp {
    Set { key: String, value: String },
    Delete { key: String },
}

/// Request to perform a write operation through Raft consensus.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WriteRequest {
    pub command: WriteCommand,
}

impl WriteRequest {
    /// Create a Set command to store a key-value pair.
    pub fn set(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            command: WriteCommand::Set {
                key: key.into(),
                value: value.into(),
            },
        }
    }

    /// Create a Set command with TTL.
    pub fn set_with_ttl(key: impl Into<String>, value: impl Into<String>, ttl_seconds: u32) -> Self {
        Self {
            command: WriteCommand::SetWithTTL {
                key: key.into(),
                value: value.into(),
                ttl_seconds,
            },
        }
    }

    /// Create a Delete command.
    pub fn delete(key: impl Into<String>) -> Self {
        Self {
            command: WriteCommand::Delete { key: key.into() },
        }
    }

    /// Create a CompareAndSwap command.
    pub fn compare_and_swap(key: impl Into<String>, expected: Option<String>, new_value: impl Into<String>) -> Self {
        Self {
            command: WriteCommand::CompareAndSwap {
                key: key.into(),
                expected,
                new_value: new_value.into(),
            },
        }
    }

    /// Create from a raw WriteCommand.
    pub fn from_command(command: WriteCommand) -> Self {
        Self { command }
    }
}

/// Result of a write operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct WriteResult {
    pub command: Option<WriteCommand>,
    pub batch_applied: Option<u32>,
    pub conditions_met: Option<bool>,
    pub failed_condition_index: Option<u32>,
    pub lease_id: Option<u64>,
    pub ttl_seconds: Option<u32>,
    pub keys_deleted: Option<u32>,
    pub succeeded: Option<bool>,
    pub txn_results: Option<Vec<TxnOpResult>>,
    pub header_revision: Option<u64>,
    pub occ_conflict: Option<bool>,
    pub conflict_key: Option<String>,
    pub conflict_expected_version: Option<i64>,
    pub conflict_actual_version: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_command_clone_and_eq() {
        let cmd = WriteCommand::Set {
            key: "k".into(),
            value: "v".into(),
        };
        let cloned = cmd.clone();
        assert_eq!(cmd, cloned);
    }

    #[test]
    fn write_op_clone_and_eq() {
        let set = WriteOp::Set {
            key: "k".into(),
            value: "v".into(),
        };
        let delete = WriteOp::Delete { key: "k".into() };

        assert_eq!(set, set.clone());
        assert_eq!(delete, delete.clone());
        assert_ne!(set, delete);
    }

    #[test]
    fn write_request_set() {
        let req = WriteRequest::set("key", "value");
        match req.command {
            WriteCommand::Set { key, value } => {
                assert_eq!(key, "key");
                assert_eq!(value, "value");
            }
            _ => panic!("wrong command type"),
        }
    }

    #[test]
    fn write_request_set_with_ttl() {
        let req = WriteRequest::set_with_ttl("key", "value", 60);
        match req.command {
            WriteCommand::SetWithTTL {
                key,
                value,
                ttl_seconds,
            } => {
                assert_eq!(key, "key");
                assert_eq!(value, "value");
                assert_eq!(ttl_seconds, 60);
            }
            _ => panic!("wrong command type"),
        }
    }

    #[test]
    fn write_request_delete() {
        let req = WriteRequest::delete("key");
        match req.command {
            WriteCommand::Delete { key } => {
                assert_eq!(key, "key");
            }
            _ => panic!("wrong command type"),
        }
    }

    #[test]
    fn write_request_compare_and_swap() {
        let req = WriteRequest::compare_and_swap("key", Some("old".to_string()), "new");
        match req.command {
            WriteCommand::CompareAndSwap {
                key,
                expected,
                new_value,
            } => {
                assert_eq!(key, "key");
                assert_eq!(expected, Some("old".to_string()));
                assert_eq!(new_value, "new");
            }
            _ => panic!("wrong command type"),
        }
    }

    #[test]
    fn write_request_from_command() {
        let cmd = WriteCommand::LeaseGrant {
            lease_id: 123,
            ttl_seconds: 60,
        };
        let req = WriteRequest::from_command(cmd.clone());
        assert_eq!(req.command, cmd);
    }

    #[test]
    fn write_result_default() {
        let result = WriteResult::default();
        assert!(result.command.is_none());
        assert!(result.batch_applied.is_none());
        assert!(result.conditions_met.is_none());
        assert!(result.succeeded.is_none());
    }

    #[test]
    fn write_command_debug() {
        let cmd = WriteCommand::Set {
            key: "k".into(),
            value: "v".into(),
        };
        let debug = format!("{:?}", cmd);
        assert!(debug.contains("Set"));
        assert!(debug.contains("\"k\""));
    }

    #[test]
    fn write_command_serialization_roundtrip() {
        let cmd = WriteCommand::Set {
            key: "k".into(),
            value: "v".into(),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let deserialized: WriteCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, deserialized);
    }

    #[test]
    fn write_request_serialization_roundtrip() {
        let req = WriteRequest::set("key", "value");
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: WriteRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, deserialized);
    }
}
