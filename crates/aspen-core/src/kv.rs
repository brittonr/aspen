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

    // ============================================================================
    // WriteCommand validation - basic Set
    // ============================================================================

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

    #[test]
    fn key_at_max_size_accepted() {
        let key = "k".repeat(MAX_KEY_SIZE as usize);
        let cmd = WriteCommand::Set { key, value: "v".into() };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn key_over_max_size_rejected() {
        let key = "k".repeat(MAX_KEY_SIZE as usize + 1);
        let cmd = WriteCommand::Set { key, value: "v".into() };
        match validate_write_command(&cmd) {
            Err(KeyValueStoreError::KeyTooLarge { size, max }) => {
                assert_eq!(size, MAX_KEY_SIZE + 1);
                assert_eq!(max, MAX_KEY_SIZE);
            }
            other => panic!("unexpected result: {:?}", other),
        }
    }

    #[test]
    fn value_at_max_size_accepted() {
        let value = "v".repeat(MAX_VALUE_SIZE as usize);
        let cmd = WriteCommand::Set { key: "k".into(), value };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn value_over_max_size_rejected() {
        let value = "v".repeat(MAX_VALUE_SIZE as usize + 1);
        let cmd = WriteCommand::Set { key: "k".into(), value };
        match validate_write_command(&cmd) {
            Err(KeyValueStoreError::ValueTooLarge { size, max }) => {
                assert_eq!(size, MAX_VALUE_SIZE + 1);
                assert_eq!(max, MAX_VALUE_SIZE);
            }
            other => panic!("unexpected result: {:?}", other),
        }
    }

    // ============================================================================
    // WriteCommand validation - SetWithTTL
    // ============================================================================

    #[test]
    fn set_with_ttl_valid() {
        let cmd = WriteCommand::SetWithTTL {
            key: "k".into(),
            value: "v".into(),
            ttl_seconds: 60,
        };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn set_with_ttl_empty_key_rejected() {
        let cmd = WriteCommand::SetWithTTL {
            key: "".into(),
            value: "v".into(),
            ttl_seconds: 60,
        };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::EmptyKey)));
    }

    // ============================================================================
    // WriteCommand validation - SetMulti
    // ============================================================================

    #[test]
    fn set_multi_valid() {
        let cmd = WriteCommand::SetMulti {
            pairs: vec![("k1".into(), "v1".into()), ("k2".into(), "v2".into())],
        };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn set_multi_at_max_size_accepted() {
        let pairs: Vec<_> = (0..MAX_SETMULTI_KEYS).map(|i| (format!("k{}", i), format!("v{}", i))).collect();
        let cmd = WriteCommand::SetMulti { pairs };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn set_multi_over_max_size_rejected() {
        let pairs: Vec<_> = (0..=MAX_SETMULTI_KEYS).map(|i| (format!("k{}", i), format!("v{}", i))).collect();
        let cmd = WriteCommand::SetMulti { pairs };
        match validate_write_command(&cmd) {
            Err(KeyValueStoreError::BatchTooLarge { size, max }) => {
                assert_eq!(size, MAX_SETMULTI_KEYS + 1);
                assert_eq!(max, MAX_SETMULTI_KEYS);
            }
            other => panic!("unexpected result: {:?}", other),
        }
    }

    #[test]
    fn set_multi_with_empty_key_rejected() {
        let cmd = WriteCommand::SetMulti {
            pairs: vec![("k1".into(), "v1".into()), ("".into(), "v2".into())],
        };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::EmptyKey)));
    }

    // ============================================================================
    // WriteCommand validation - SetMultiWithTTL
    // ============================================================================

    #[test]
    fn set_multi_with_ttl_valid() {
        let cmd = WriteCommand::SetMultiWithTTL {
            pairs: vec![("k1".into(), "v1".into())],
            ttl_seconds: 300,
        };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn set_multi_with_ttl_over_max_rejected() {
        let pairs: Vec<_> = (0..=MAX_SETMULTI_KEYS).map(|i| (format!("k{}", i), format!("v{}", i))).collect();
        let cmd = WriteCommand::SetMultiWithTTL { pairs, ttl_seconds: 60 };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::BatchTooLarge { .. })));
    }

    // ============================================================================
    // WriteCommand validation - Delete
    // ============================================================================

    #[test]
    fn delete_valid() {
        let cmd = WriteCommand::Delete { key: "k".into() };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn delete_empty_key_rejected() {
        let cmd = WriteCommand::Delete { key: "".into() };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::EmptyKey)));
    }

    // ============================================================================
    // WriteCommand validation - DeleteMulti
    // ============================================================================

    #[test]
    fn delete_multi_valid() {
        let cmd = WriteCommand::DeleteMulti {
            keys: vec!["k1".into(), "k2".into()],
        };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn delete_multi_over_max_rejected() {
        let keys: Vec<_> = (0..=MAX_SETMULTI_KEYS).map(|i| format!("k{}", i)).collect();
        let cmd = WriteCommand::DeleteMulti { keys };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::BatchTooLarge { .. })));
    }

    #[test]
    fn delete_multi_with_empty_key_rejected() {
        let cmd = WriteCommand::DeleteMulti {
            keys: vec!["k1".into(), "".into()],
        };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::EmptyKey)));
    }

    // ============================================================================
    // WriteCommand validation - CompareAndSwap
    // ============================================================================

    #[test]
    fn compare_and_swap_valid() {
        let cmd = WriteCommand::CompareAndSwap {
            key: "k".into(),
            expected: Some("old".into()),
            new_value: "new".into(),
        };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn compare_and_swap_with_none_expected_valid() {
        let cmd = WriteCommand::CompareAndSwap {
            key: "k".into(),
            expected: None,
            new_value: "new".into(),
        };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn compare_and_swap_empty_key_rejected() {
        let cmd = WriteCommand::CompareAndSwap {
            key: "".into(),
            expected: None,
            new_value: "new".into(),
        };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::EmptyKey)));
    }

    #[test]
    fn compare_and_swap_oversized_expected_rejected() {
        let cmd = WriteCommand::CompareAndSwap {
            key: "k".into(),
            expected: Some("v".repeat(MAX_VALUE_SIZE as usize + 1)),
            new_value: "new".into(),
        };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::ValueTooLarge { .. })));
    }

    // ============================================================================
    // WriteCommand validation - CompareAndDelete
    // ============================================================================

    #[test]
    fn compare_and_delete_valid() {
        let cmd = WriteCommand::CompareAndDelete {
            key: "k".into(),
            expected: "value".into(),
        };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn compare_and_delete_empty_key_rejected() {
        let cmd = WriteCommand::CompareAndDelete {
            key: "".into(),
            expected: "value".into(),
        };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::EmptyKey)));
    }

    // ============================================================================
    // WriteCommand validation - Batch
    // ============================================================================

    #[test]
    fn batch_valid() {
        let cmd = WriteCommand::Batch {
            operations: vec![
                BatchOperation::Set {
                    key: "k1".into(),
                    value: "v1".into(),
                },
                BatchOperation::Delete { key: "k2".into() },
            ],
        };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn batch_over_max_rejected() {
        let operations: Vec<_> = (0..=MAX_SETMULTI_KEYS)
            .map(|i| BatchOperation::Set {
                key: format!("k{}", i),
                value: format!("v{}", i),
            })
            .collect();
        let cmd = WriteCommand::Batch { operations };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::BatchTooLarge { .. })));
    }

    #[test]
    fn batch_with_empty_key_rejected() {
        let cmd = WriteCommand::Batch {
            operations: vec![BatchOperation::Set {
                key: "".into(),
                value: "v".into(),
            }],
        };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::EmptyKey)));
    }

    // ============================================================================
    // WriteCommand validation - ConditionalBatch
    // ============================================================================

    #[test]
    fn conditional_batch_valid() {
        let cmd = WriteCommand::ConditionalBatch {
            conditions: vec![BatchCondition::KeyExists { key: "k1".into() }],
            operations: vec![BatchOperation::Set {
                key: "k2".into(),
                value: "v2".into(),
            }],
        };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn conditional_batch_over_max_rejected() {
        let conditions: Vec<_> = (0..MAX_SETMULTI_KEYS / 2 + 1)
            .map(|i| BatchCondition::KeyExists { key: format!("c{}", i) })
            .collect();
        let operations: Vec<_> = (0..MAX_SETMULTI_KEYS / 2 + 1)
            .map(|i| BatchOperation::Set {
                key: format!("k{}", i),
                value: format!("v{}", i),
            })
            .collect();
        let cmd = WriteCommand::ConditionalBatch { conditions, operations };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::BatchTooLarge { .. })));
    }

    #[test]
    fn conditional_batch_value_equals_valid() {
        let cmd = WriteCommand::ConditionalBatch {
            conditions: vec![BatchCondition::ValueEquals {
                key: "k".into(),
                expected: "v".into(),
            }],
            operations: vec![],
        };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn conditional_batch_key_not_exists_valid() {
        let cmd = WriteCommand::ConditionalBatch {
            conditions: vec![BatchCondition::KeyNotExists { key: "k".into() }],
            operations: vec![],
        };
        assert!(validate_write_command(&cmd).is_ok());
    }

    // ============================================================================
    // WriteCommand validation - Transaction
    // ============================================================================

    #[test]
    fn transaction_valid() {
        let cmd = WriteCommand::Transaction {
            compare: vec![TxnCompare {
                key: "k".into(),
                target: CompareTarget::Value,
                op: CompareOp::Equal,
                value: "v".into(),
            }],
            success: vec![TxnOp::Put {
                key: "k".into(),
                value: "new".into(),
            }],
            failure: vec![],
        };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn transaction_over_max_rejected() {
        let compare: Vec<_> = (0..MAX_SETMULTI_KEYS / 2 + 1)
            .map(|i| TxnCompare {
                key: format!("k{}", i),
                target: CompareTarget::Value,
                op: CompareOp::Equal,
                value: format!("v{}", i),
            })
            .collect();
        let success: Vec<_> = (0..MAX_SETMULTI_KEYS / 2 + 1)
            .map(|i| TxnOp::Put {
                key: format!("s{}", i),
                value: format!("sv{}", i),
            })
            .collect();
        let cmd = WriteCommand::Transaction {
            compare,
            success,
            failure: vec![],
        };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::BatchTooLarge { .. })));
    }

    #[test]
    fn transaction_with_get_valid() {
        let cmd = WriteCommand::Transaction {
            compare: vec![],
            success: vec![TxnOp::Get { key: "k".into() }],
            failure: vec![],
        };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn transaction_with_delete_valid() {
        let cmd = WriteCommand::Transaction {
            compare: vec![],
            success: vec![TxnOp::Delete { key: "k".into() }],
            failure: vec![],
        };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn transaction_with_range_valid() {
        let cmd = WriteCommand::Transaction {
            compare: vec![],
            success: vec![TxnOp::Range {
                prefix: "prefix".into(),
                limit: 100,
            }],
            failure: vec![],
        };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn transaction_with_range_over_max_rejected() {
        let cmd = WriteCommand::Transaction {
            compare: vec![],
            success: vec![TxnOp::Range {
                prefix: "prefix".into(),
                limit: MAX_SCAN_RESULTS + 1,
            }],
            failure: vec![],
        };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::BatchTooLarge { .. })));
    }

    // ============================================================================
    // WriteCommand validation - SetWithLease
    // ============================================================================

    #[test]
    fn set_with_lease_valid() {
        let cmd = WriteCommand::SetWithLease {
            key: "k".into(),
            value: "v".into(),
            lease_id: 12345,
        };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn set_with_lease_empty_key_rejected() {
        let cmd = WriteCommand::SetWithLease {
            key: "".into(),
            value: "v".into(),
            lease_id: 12345,
        };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::EmptyKey)));
    }

    // ============================================================================
    // WriteCommand validation - SetMultiWithLease
    // ============================================================================

    #[test]
    fn set_multi_with_lease_valid() {
        let cmd = WriteCommand::SetMultiWithLease {
            pairs: vec![("k".into(), "v".into())],
            lease_id: 12345,
        };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn set_multi_with_lease_over_max_rejected() {
        let pairs: Vec<_> = (0..=MAX_SETMULTI_KEYS).map(|i| (format!("k{}", i), format!("v{}", i))).collect();
        let cmd = WriteCommand::SetMultiWithLease { pairs, lease_id: 12345 };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::BatchTooLarge { .. })));
    }

    // ============================================================================
    // WriteCommand validation - Lease operations (no validation)
    // ============================================================================

    #[test]
    fn lease_grant_valid() {
        let cmd = WriteCommand::LeaseGrant {
            lease_id: 12345,
            ttl_seconds: 60,
        };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn lease_revoke_valid() {
        let cmd = WriteCommand::LeaseRevoke { lease_id: 12345 };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn lease_keepalive_valid() {
        let cmd = WriteCommand::LeaseKeepalive { lease_id: 12345 };
        assert!(validate_write_command(&cmd).is_ok());
    }

    // ============================================================================
    // WriteCommand validation - OptimisticTransaction
    // ============================================================================

    #[test]
    fn optimistic_transaction_valid() {
        let cmd = WriteCommand::OptimisticTransaction {
            read_set: vec![("k1".into(), 1)],
            write_set: vec![WriteOp::Set {
                key: "k1".into(),
                value: "new".into(),
            }],
        };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn optimistic_transaction_read_set_over_max_rejected() {
        let read_set: Vec<_> = (0..=MAX_SETMULTI_KEYS).map(|i| (format!("k{}", i), i as i64)).collect();
        let cmd = WriteCommand::OptimisticTransaction {
            read_set,
            write_set: vec![],
        };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::BatchTooLarge { .. })));
    }

    #[test]
    fn optimistic_transaction_write_set_over_max_rejected() {
        let write_set: Vec<_> = (0..=MAX_SETMULTI_KEYS)
            .map(|i| WriteOp::Set {
                key: format!("k{}", i),
                value: format!("v{}", i),
            })
            .collect();
        let cmd = WriteCommand::OptimisticTransaction {
            read_set: vec![],
            write_set,
        };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::BatchTooLarge { .. })));
    }

    #[test]
    fn optimistic_transaction_with_delete_valid() {
        let cmd = WriteCommand::OptimisticTransaction {
            read_set: vec![],
            write_set: vec![WriteOp::Delete { key: "k".into() }],
        };
        assert!(validate_write_command(&cmd).is_ok());
    }

    // ============================================================================
    // WriteCommand validation - Shard operations
    // ============================================================================

    #[test]
    fn shard_split_valid() {
        let cmd = WriteCommand::ShardSplit {
            source_shard: 0,
            split_key: "m".into(),
            new_shard_id: 1,
            topology_version: 1,
        };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn shard_split_empty_key_rejected() {
        let cmd = WriteCommand::ShardSplit {
            source_shard: 0,
            split_key: "".into(),
            new_shard_id: 1,
            topology_version: 1,
        };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::EmptyKey)));
    }

    #[test]
    fn shard_merge_valid() {
        let cmd = WriteCommand::ShardMerge {
            source_shard: 1,
            target_shard: 0,
            topology_version: 2,
        };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn topology_update_valid() {
        let cmd = WriteCommand::TopologyUpdate {
            topology_data: vec![1, 2, 3, 4],
        };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn topology_update_over_max_rejected() {
        let cmd = WriteCommand::TopologyUpdate {
            topology_data: vec![0; MAX_VALUE_SIZE as usize + 1],
        };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::ValueTooLarge { .. })));
    }

    // ============================================================================
    // WriteRequest builder tests
    // ============================================================================

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

    // ============================================================================
    // ReadRequest tests
    // ============================================================================

    #[test]
    fn read_request_new_default_consistency() {
        let req = ReadRequest::new("key");
        assert_eq!(req.key, "key");
        assert_eq!(req.consistency, ReadConsistency::Linearizable);
    }

    #[test]
    fn read_request_with_lease() {
        let req = ReadRequest::with_lease("key");
        assert_eq!(req.key, "key");
        assert_eq!(req.consistency, ReadConsistency::Lease);
    }

    #[test]
    fn read_request_stale() {
        let req = ReadRequest::stale("key");
        assert_eq!(req.key, "key");
        assert_eq!(req.consistency, ReadConsistency::Stale);
    }

    // ============================================================================
    // DeleteRequest tests
    // ============================================================================

    #[test]
    fn delete_request_new() {
        let req = DeleteRequest::new("key");
        assert_eq!(req.key, "key");
    }

    // ============================================================================
    // Type equality and clone tests
    // ============================================================================

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
    fn batch_operation_clone_and_eq() {
        let op = BatchOperation::Set {
            key: "k".into(),
            value: "v".into(),
        };
        assert_eq!(op, op.clone());

        let op2 = BatchOperation::Delete { key: "k".into() };
        assert_eq!(op2, op2.clone());
        assert_ne!(op, op2);
    }

    #[test]
    fn batch_condition_clone_and_eq() {
        let c1 = BatchCondition::ValueEquals {
            key: "k".into(),
            expected: "v".into(),
        };
        let c2 = BatchCondition::KeyExists { key: "k".into() };
        let c3 = BatchCondition::KeyNotExists { key: "k".into() };

        assert_eq!(c1, c1.clone());
        assert_eq!(c2, c2.clone());
        assert_eq!(c3, c3.clone());
        assert_ne!(c1, c2);
        assert_ne!(c2, c3);
    }

    #[test]
    fn compare_target_clone_and_eq() {
        assert_eq!(CompareTarget::Value, CompareTarget::Value.clone());
        assert_eq!(CompareTarget::Version, CompareTarget::Version.clone());
        assert_eq!(CompareTarget::CreateRevision, CompareTarget::CreateRevision.clone());
        assert_eq!(CompareTarget::ModRevision, CompareTarget::ModRevision.clone());
        assert_ne!(CompareTarget::Value, CompareTarget::Version);
    }

    #[test]
    fn compare_op_clone_and_eq() {
        assert_eq!(CompareOp::Equal, CompareOp::Equal.clone());
        assert_eq!(CompareOp::NotEqual, CompareOp::NotEqual.clone());
        assert_eq!(CompareOp::Greater, CompareOp::Greater.clone());
        assert_eq!(CompareOp::Less, CompareOp::Less.clone());
        assert_ne!(CompareOp::Equal, CompareOp::NotEqual);
    }

    #[test]
    fn txn_compare_clone_and_eq() {
        let cmp = TxnCompare {
            key: "k".into(),
            target: CompareTarget::Value,
            op: CompareOp::Equal,
            value: "v".into(),
        };
        assert_eq!(cmp, cmp.clone());
    }

    #[test]
    fn txn_op_clone_and_eq() {
        let put = TxnOp::Put {
            key: "k".into(),
            value: "v".into(),
        };
        let delete = TxnOp::Delete { key: "k".into() };
        let get = TxnOp::Get { key: "k".into() };
        let range = TxnOp::Range {
            prefix: "p".into(),
            limit: 10,
        };

        assert_eq!(put, put.clone());
        assert_eq!(delete, delete.clone());
        assert_eq!(get, get.clone());
        assert_eq!(range, range.clone());
        assert_ne!(put, delete);
    }

    #[test]
    fn txn_op_result_clone_and_eq() {
        let put_result = TxnOpResult::Put { revision: 42 };
        let delete_result = TxnOpResult::Delete { deleted: 1 };
        let get_result = TxnOpResult::Get { kv: None };
        let range_result = TxnOpResult::Range {
            kvs: vec![],
            more: false,
        };

        assert_eq!(put_result, put_result.clone());
        assert_eq!(delete_result, delete_result.clone());
        assert_eq!(get_result, get_result.clone());
        assert_eq!(range_result, range_result.clone());
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
    fn key_value_with_revision_clone_and_eq() {
        let kv = KeyValueWithRevision {
            key: "k".into(),
            value: "v".into(),
            version: 1,
            create_revision: 10,
            mod_revision: 10,
        };
        assert_eq!(kv, kv.clone());
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
    fn read_consistency_default() {
        assert_eq!(ReadConsistency::default(), ReadConsistency::Linearizable);
    }

    #[test]
    fn read_consistency_clone_and_eq() {
        assert_eq!(ReadConsistency::Linearizable, ReadConsistency::Linearizable.clone());
        assert_eq!(ReadConsistency::Lease, ReadConsistency::Lease.clone());
        assert_eq!(ReadConsistency::Stale, ReadConsistency::Stale.clone());
        assert_ne!(ReadConsistency::Linearizable, ReadConsistency::Stale);
    }

    // ============================================================================
    // Serialization roundtrip tests
    // ============================================================================

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

    #[test]
    fn read_request_serialization_roundtrip() {
        let req = ReadRequest::with_lease("key");
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: ReadRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, deserialized);
    }

    #[test]
    fn scan_request_serialization_roundtrip() {
        let req = ScanRequest {
            prefix: "prefix".into(),
            limit: Some(100),
            continuation_token: Some("token".into()),
        };
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: ScanRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, deserialized);
    }

    #[test]
    fn scan_result_serialization_roundtrip() {
        let result = ScanResult {
            entries: vec![KeyValueWithRevision {
                key: "k".into(),
                value: "v".into(),
                version: 1,
                create_revision: 1,
                mod_revision: 1,
            }],
            count: 1,
            is_truncated: false,
            continuation_token: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: ScanResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result, deserialized);
    }

    #[test]
    fn txn_compare_serialization_roundtrip() {
        let cmp = TxnCompare {
            key: "k".into(),
            target: CompareTarget::Version,
            op: CompareOp::Greater,
            value: "5".into(),
        };
        let json = serde_json::to_string(&cmp).unwrap();
        let deserialized: TxnCompare = serde_json::from_str(&json).unwrap();
        assert_eq!(cmp, deserialized);
    }

    // ============================================================================
    // Debug format tests
    // ============================================================================

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
    fn compare_target_debug() {
        assert_eq!(format!("{:?}", CompareTarget::Value), "Value");
        assert_eq!(format!("{:?}", CompareTarget::Version), "Version");
        assert_eq!(format!("{:?}", CompareTarget::CreateRevision), "CreateRevision");
        assert_eq!(format!("{:?}", CompareTarget::ModRevision), "ModRevision");
    }

    #[test]
    fn compare_op_debug() {
        assert_eq!(format!("{:?}", CompareOp::Equal), "Equal");
        assert_eq!(format!("{:?}", CompareOp::NotEqual), "NotEqual");
        assert_eq!(format!("{:?}", CompareOp::Greater), "Greater");
        assert_eq!(format!("{:?}", CompareOp::Less), "Less");
    }
}
