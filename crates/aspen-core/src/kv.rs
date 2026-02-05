//! Key-value operation types.
//!
//! Types for distributed key-value operations through Raft consensus.

use serde::Deserialize;
use serde::Serialize;

use crate::constants::MAX_KEY_SIZE;
use crate::constants::MAX_SCAN_RESULTS;
use crate::constants::MAX_SETMULTI_KEYS;
use crate::constants::MAX_VALUE_SIZE;
use crate::error::KeyValueStoreError;

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

/// A single operation within a batch write.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BatchOperation {
    Set { key: String, value: String },
    Delete { key: String },
}

/// A condition for conditional batch writes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BatchCondition {
    ValueEquals { key: String, expected: String },
    KeyExists { key: String },
    KeyNotExists { key: String },
}

/// Comparison target for transaction conditions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompareTarget {
    Value,
    Version,
    CreateRevision,
    ModRevision,
}

/// Comparison operator for transaction conditions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompareOp {
    Equal,
    NotEqual,
    Greater,
    Less,
}

/// A comparison condition for transactions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TxnCompare {
    pub key: String,
    pub target: CompareTarget,
    pub op: CompareOp,
    pub value: String,
}

/// Operations that can be performed in a transaction branch.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TxnOp {
    Put { key: String, value: String },
    Delete { key: String },
    Get { key: String },
    Range { prefix: String, limit: u32 },
}

/// Result of a single transaction operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TxnOpResult {
    Put { revision: u64 },
    Delete { deleted: u32 },
    Get { kv: Option<KeyValueWithRevision> },
    Range { kvs: Vec<KeyValueWithRevision>, more: bool },
}

/// Key-value pair with revision metadata for optimistic concurrency control.
///
/// The revision fields enable clients to detect concurrent modifications and
/// implement watch/compare-and-swap patterns similar to etcd's MVCC model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeyValueWithRevision {
    /// The key identifying this entry.
    pub key: String,
    /// The stored value.
    pub value: String,
    /// Key-specific version number, incremented on each modification to this key.
    ///
    /// Starts at 1 when the key is first created. Use for optimistic locking:
    /// read the version, perform local computation, then use compare-and-swap
    /// to ensure the key wasn't modified concurrently.
    pub version: u64,
    /// The Raft log index when this key was first created.
    ///
    /// This value never changes after creation, even if the key is modified.
    /// Useful for determining the relative age of keys.
    pub create_revision: u64,
    /// The Raft log index of the most recent modification to this key.
    ///
    /// Updated on every write operation. For newly created keys,
    /// `mod_revision == create_revision`. Use with watch operations to
    /// resume from a known point in the change stream.
    pub mod_revision: u64,
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

/// Consistency level for read operations.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReadConsistency {
    #[default]
    Linearizable,
    Lease,
    Stale,
}

/// Request to read a single key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReadRequest {
    pub key: String,
    #[serde(default)]
    pub consistency: ReadConsistency,
}

impl ReadRequest {
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            consistency: ReadConsistency::default(),
        }
    }

    pub fn with_lease(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            consistency: ReadConsistency::Lease,
        }
    }

    pub fn stale(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            consistency: ReadConsistency::Stale,
        }
    }
}

/// Response from a read operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReadResult {
    pub kv: Option<KeyValueWithRevision>,
}

/// Request to delete a key from the distributed store.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeleteRequest {
    pub key: String,
}

impl DeleteRequest {
    /// Create a delete request for the specified key.
    pub fn new(key: impl Into<String>) -> Self {
        Self { key: key.into() }
    }
}

/// Result of a delete operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeleteResult {
    pub key: String,
    pub deleted: bool,
}

/// Request to scan keys with a given prefix.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanRequest {
    pub prefix: String,
    pub limit: Option<u32>,
    pub continuation_token: Option<String>,
}

/// Response from a scan operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanResult {
    pub entries: Vec<KeyValueWithRevision>,
    pub count: u32,
    pub is_truncated: bool,
    pub continuation_token: Option<String>,
}

/// Validate a write command against fixed size limits.
pub fn validate_write_command(command: &WriteCommand) -> Result<(), KeyValueStoreError> {
    let check_key = |key: &str| {
        if key.is_empty() {
            return Err(KeyValueStoreError::EmptyKey);
        }
        let len = key.len();
        if len > MAX_KEY_SIZE as usize {
            Err(KeyValueStoreError::KeyTooLarge {
                size: len as u32,
                max: MAX_KEY_SIZE,
            })
        } else {
            Ok(())
        }
    };

    let check_value = |value: &str| {
        let len = value.len();
        if len > MAX_VALUE_SIZE as usize {
            Err(KeyValueStoreError::ValueTooLarge {
                size: len as u32,
                max: MAX_VALUE_SIZE,
            })
        } else {
            Ok(())
        }
    };

    match command {
        WriteCommand::Set { key, value } => {
            check_key(key)?;
            check_value(value)?;
        }
        WriteCommand::SetWithTTL { key, value, .. } => {
            check_key(key)?;
            check_value(value)?;
        }
        WriteCommand::SetMulti { pairs } => {
            if pairs.len() > MAX_SETMULTI_KEYS as usize {
                return Err(KeyValueStoreError::BatchTooLarge {
                    size: pairs.len() as u32,
                    max: MAX_SETMULTI_KEYS,
                });
            }
            for (key, value) in pairs {
                check_key(key)?;
                check_value(value)?;
            }
        }
        WriteCommand::SetMultiWithTTL { pairs, .. } => {
            if pairs.len() > MAX_SETMULTI_KEYS as usize {
                return Err(KeyValueStoreError::BatchTooLarge {
                    size: pairs.len() as u32,
                    max: MAX_SETMULTI_KEYS,
                });
            }
            for (key, value) in pairs {
                check_key(key)?;
                check_value(value)?;
            }
        }
        WriteCommand::Delete { key } => {
            check_key(key)?;
        }
        WriteCommand::DeleteMulti { keys } => {
            if keys.len() > MAX_SETMULTI_KEYS as usize {
                return Err(KeyValueStoreError::BatchTooLarge {
                    size: keys.len() as u32,
                    max: MAX_SETMULTI_KEYS,
                });
            }
            for key in keys {
                check_key(key)?;
            }
        }
        WriteCommand::CompareAndSwap {
            key,
            expected,
            new_value,
        } => {
            check_key(key)?;
            if let Some(exp) = expected {
                check_value(exp)?;
            }
            check_value(new_value)?;
        }
        WriteCommand::CompareAndDelete { key, expected } => {
            check_key(key)?;
            check_value(expected)?;
        }
        WriteCommand::Batch { operations } => {
            if operations.len() > MAX_SETMULTI_KEYS as usize {
                return Err(KeyValueStoreError::BatchTooLarge {
                    size: operations.len() as u32,
                    max: MAX_SETMULTI_KEYS,
                });
            }
            for op in operations {
                match op {
                    BatchOperation::Set { key, value } => {
                        check_key(key)?;
                        check_value(value)?;
                    }
                    BatchOperation::Delete { key } => {
                        check_key(key)?;
                    }
                }
            }
        }
        WriteCommand::ConditionalBatch { conditions, operations } => {
            let total_size = conditions.len() + operations.len();
            if total_size > MAX_SETMULTI_KEYS as usize {
                return Err(KeyValueStoreError::BatchTooLarge {
                    size: total_size as u32,
                    max: MAX_SETMULTI_KEYS,
                });
            }
            for cond in conditions {
                match cond {
                    BatchCondition::ValueEquals { key, expected } => {
                        check_key(key)?;
                        check_value(expected)?;
                    }
                    BatchCondition::KeyExists { key } | BatchCondition::KeyNotExists { key } => {
                        check_key(key)?;
                    }
                }
            }
            for op in operations {
                match op {
                    BatchOperation::Set { key, value } => {
                        check_key(key)?;
                        check_value(value)?;
                    }
                    BatchOperation::Delete { key } => {
                        check_key(key)?;
                    }
                }
            }
        }
        WriteCommand::Transaction {
            compare,
            success,
            failure,
        } => {
            let total_size = compare.len() + success.len() + failure.len();
            if total_size > MAX_SETMULTI_KEYS as usize {
                return Err(KeyValueStoreError::BatchTooLarge {
                    size: total_size as u32,
                    max: MAX_SETMULTI_KEYS,
                });
            }
            for cmp in compare {
                check_key(&cmp.key)?;
                check_value(&cmp.value)?;
            }
            for op in success.iter().chain(failure.iter()) {
                match op {
                    TxnOp::Put { key, value } => {
                        check_key(key)?;
                        check_value(value)?;
                    }
                    TxnOp::Delete { key } | TxnOp::Get { key } => {
                        check_key(key)?;
                    }
                    TxnOp::Range { prefix, limit } => {
                        check_key(prefix)?;
                        if *limit > MAX_SCAN_RESULTS {
                            return Err(KeyValueStoreError::BatchTooLarge {
                                size: *limit,
                                max: MAX_SCAN_RESULTS,
                            });
                        }
                    }
                }
            }
        }
        WriteCommand::SetWithLease { key, value, .. } => {
            check_key(key)?;
            check_value(value)?;
        }
        WriteCommand::SetMultiWithLease { pairs, .. } => {
            if pairs.len() > MAX_SETMULTI_KEYS as usize {
                return Err(KeyValueStoreError::BatchTooLarge {
                    size: pairs.len() as u32,
                    max: MAX_SETMULTI_KEYS,
                });
            }
            for (key, value) in pairs {
                check_key(key)?;
                check_value(value)?;
            }
        }
        WriteCommand::LeaseGrant { .. } | WriteCommand::LeaseRevoke { .. } | WriteCommand::LeaseKeepalive { .. } => {}
        WriteCommand::OptimisticTransaction { read_set, write_set } => {
            if read_set.len() > MAX_SETMULTI_KEYS as usize {
                return Err(KeyValueStoreError::BatchTooLarge {
                    size: read_set.len() as u32,
                    max: MAX_SETMULTI_KEYS,
                });
            }
            if write_set.len() > MAX_SETMULTI_KEYS as usize {
                return Err(KeyValueStoreError::BatchTooLarge {
                    size: write_set.len() as u32,
                    max: MAX_SETMULTI_KEYS,
                });
            }
            for (key, _) in read_set {
                check_key(key)?;
            }
            for op in write_set {
                match op {
                    WriteOp::Set { key, value } => {
                        check_key(key)?;
                        check_value(value)?;
                    }
                    WriteOp::Delete { key } => {
                        check_key(key)?;
                    }
                }
            }
        }
        WriteCommand::ShardSplit { split_key, .. } => {
            check_key(split_key)?;
        }
        WriteCommand::ShardMerge { .. } => {}
        WriteCommand::TopologyUpdate { topology_data } => {
            if topology_data.len() > MAX_VALUE_SIZE as usize {
                return Err(KeyValueStoreError::ValueTooLarge {
                    size: topology_data.len() as u32,
                    max: MAX_VALUE_SIZE,
                });
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_key_rejected() {
        let cmd = WriteCommand::Set {
            key: "".into(),
            value: "v".into(),
        };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::EmptyKey)));
    }

    #[test]
    fn valid_key_accepted() {
        let cmd = WriteCommand::Set {
            key: "k".into(),
            value: "v".into(),
        };
        assert!(validate_write_command(&cmd).is_ok());
    }
}
