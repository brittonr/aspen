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
