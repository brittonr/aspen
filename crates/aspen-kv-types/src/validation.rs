//! Validation functions for write commands.

use aspen_constants::api::MAX_KEY_SIZE;
use aspen_constants::api::MAX_SCAN_RESULTS;
use aspen_constants::api::MAX_SETMULTI_KEYS;
use aspen_constants::api::MAX_VALUE_SIZE;

use crate::KeyValueStoreError;
use crate::batch::BatchCondition;
use crate::batch::BatchOperation;
use crate::transaction::TxnOp;
use crate::write::WriteCommand;
use crate::write::WriteOp;

/// Validate a key against size limits.
fn validate_write_command_check_key(key: &str) -> Result<(), KeyValueStoreError> {
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
}

/// Validate a value against size limits.
fn validate_write_command_check_value(value: &str) -> Result<(), KeyValueStoreError> {
    let len = value.len();
    if len > MAX_VALUE_SIZE as usize {
        Err(KeyValueStoreError::ValueTooLarge {
            size: len as u32,
            max: MAX_VALUE_SIZE,
        })
    } else {
        Ok(())
    }
}

/// Validate batch size against limits.
fn validate_write_command_check_batch_size(size: usize) -> Result<(), KeyValueStoreError> {
    if size > MAX_SETMULTI_KEYS as usize {
        Err(KeyValueStoreError::BatchTooLarge {
            size: size as u32,
            max: MAX_SETMULTI_KEYS,
        })
    } else {
        Ok(())
    }
}

/// Validate a key-value pair.
fn validate_write_command_check_pair(key: &str, value: &str) -> Result<(), KeyValueStoreError> {
    validate_write_command_check_key(key)?;
    validate_write_command_check_value(value)
}

/// Validate a batch operation.
fn validate_write_command_check_batch_op(op: &BatchOperation) -> Result<(), KeyValueStoreError> {
    match op {
        BatchOperation::Set { key, value } => validate_write_command_check_pair(key, value),
        BatchOperation::Delete { key } => validate_write_command_check_key(key),
    }
}

/// Validate a batch condition.
fn validate_write_command_check_condition(cond: &BatchCondition) -> Result<(), KeyValueStoreError> {
    match cond {
        BatchCondition::ValueEquals { key, expected } => validate_write_command_check_pair(key, expected),
        BatchCondition::KeyExists { key } | BatchCondition::KeyNotExists { key } => {
            validate_write_command_check_key(key)
        }
    }
}

/// Validate a transaction operation.
fn validate_write_command_check_txn_op(op: &TxnOp) -> Result<(), KeyValueStoreError> {
    match op {
        TxnOp::Put { key, value } => validate_write_command_check_pair(key, value),
        TxnOp::Delete { key } | TxnOp::Get { key } => validate_write_command_check_key(key),
        TxnOp::Range { prefix, limit } => {
            validate_write_command_check_key(prefix)?;
            if *limit > MAX_SCAN_RESULTS {
                return Err(KeyValueStoreError::BatchTooLarge {
                    size: *limit,
                    max: MAX_SCAN_RESULTS,
                });
            }
            Ok(())
        }
    }
}

/// Validate a write operation.
fn validate_write_command_check_write_op(op: &WriteOp) -> Result<(), KeyValueStoreError> {
    match op {
        WriteOp::Set { key, value } => validate_write_command_check_pair(key, value),
        WriteOp::Delete { key } => validate_write_command_check_key(key),
    }
}

/// Validate a write command against fixed size limits.
pub fn validate_write_command(command: &WriteCommand) -> Result<(), KeyValueStoreError> {
    match command {
        WriteCommand::Set { key, value } | WriteCommand::SetWithTTL { key, value, .. } => {
            validate_write_command_check_pair(key, value)
        }
        WriteCommand::SetMulti { pairs } | WriteCommand::SetMultiWithTTL { pairs, .. } => {
            validate_write_command_check_batch_size(pairs.len())?;
            for (key, value) in pairs {
                validate_write_command_check_pair(key, value)?;
            }
            Ok(())
        }
        WriteCommand::Delete { key } => validate_write_command_check_key(key),
        WriteCommand::DeleteMulti { keys } => {
            validate_write_command_check_batch_size(keys.len())?;
            for key in keys {
                validate_write_command_check_key(key)?;
            }
            Ok(())
        }
        WriteCommand::CompareAndSwap {
            key,
            expected,
            new_value,
        } => {
            validate_write_command_check_key(key)?;
            if let Some(exp) = expected {
                validate_write_command_check_value(exp)?;
            }
            validate_write_command_check_value(new_value)
        }
        WriteCommand::CompareAndDelete { key, expected } => validate_write_command_check_pair(key, expected),
        WriteCommand::Batch { operations } => {
            validate_write_command_check_batch_size(operations.len())?;
            for op in operations {
                validate_write_command_check_batch_op(op)?;
            }
            Ok(())
        }
        WriteCommand::ConditionalBatch { conditions, operations } => {
            validate_write_command_check_batch_size(conditions.len() + operations.len())?;
            for cond in conditions {
                validate_write_command_check_condition(cond)?;
            }
            for op in operations {
                validate_write_command_check_batch_op(op)?;
            }
            Ok(())
        }
        WriteCommand::Transaction {
            compare,
            success,
            failure,
        } => {
            validate_write_command_check_batch_size(compare.len() + success.len() + failure.len())?;
            for cmp in compare {
                validate_write_command_check_pair(&cmp.key, &cmp.value)?;
            }
            for op in success.iter().chain(failure.iter()) {
                validate_write_command_check_txn_op(op)?;
            }
            Ok(())
        }
        WriteCommand::SetWithLease { key, value, .. } => validate_write_command_check_pair(key, value),
        WriteCommand::SetMultiWithLease { pairs, .. } => {
            validate_write_command_check_batch_size(pairs.len())?;
            for (key, value) in pairs {
                validate_write_command_check_pair(key, value)?;
            }
            Ok(())
        }
        WriteCommand::LeaseGrant { .. } | WriteCommand::LeaseRevoke { .. } | WriteCommand::LeaseKeepalive { .. } => {
            Ok(())
        }
        WriteCommand::OptimisticTransaction { read_set, write_set } => {
            validate_write_command_check_batch_size(read_set.len())?;
            validate_write_command_check_batch_size(write_set.len())?;
            for (key, _) in read_set {
                validate_write_command_check_key(key)?;
            }
            for op in write_set {
                validate_write_command_check_write_op(op)?;
            }
            Ok(())
        }
        WriteCommand::ShardSplit { split_key, .. } => validate_write_command_check_key(split_key),
        WriteCommand::ShardMerge { .. } => Ok(()),
        WriteCommand::TopologyUpdate { topology_data } => {
            if topology_data.len() > MAX_VALUE_SIZE as usize {
                return Err(KeyValueStoreError::ValueTooLarge {
                    size: topology_data.len() as u32,
                    max: MAX_VALUE_SIZE,
                });
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::batch::BatchCondition;
    use crate::batch::BatchOperation;
    use crate::transaction::CompareOp;
    use crate::transaction::CompareTarget;
    use crate::transaction::TxnCompare;
    use crate::transaction::TxnOp;
    use crate::write::WriteCommand;
    use crate::write::WriteOp;

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
    fn topology_update_over_max_rejected() {
        let cmd = WriteCommand::TopologyUpdate {
            topology_data: vec![0; MAX_VALUE_SIZE as usize + 1],
        };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::ValueTooLarge { .. })));
    }
}
