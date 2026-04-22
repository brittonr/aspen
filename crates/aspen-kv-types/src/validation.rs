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

#[derive(Clone, Copy)]
struct WritePairRef<'a> {
    key: &'a str,
    value: &'a str,
}

/// Validate a key against size limits.
fn validate_write_command_check_key(key: &str) -> Result<(), KeyValueStoreError> {
    if key.is_empty() {
        return Err(KeyValueStoreError::EmptyKey);
    }
    let key_len_bytes = usize_to_u32_limit_checked(key.len(), MAX_KEY_SIZE);
    if key_len_bytes > MAX_KEY_SIZE {
        return Err(KeyValueStoreError::KeyTooLarge {
            size: key_len_bytes,
            max: MAX_KEY_SIZE,
        });
    }
    Ok(())
}

/// Validate a value against size limits.
fn validate_write_command_check_value(value: &str) -> Result<(), KeyValueStoreError> {
    let value_len_bytes = usize_to_u32_limit_checked(value.len(), MAX_VALUE_SIZE);
    if value_len_bytes > MAX_VALUE_SIZE {
        return Err(KeyValueStoreError::ValueTooLarge {
            size: value_len_bytes,
            max: MAX_VALUE_SIZE,
        });
    }
    Ok(())
}

/// Validate batch size against limits.
fn validate_write_command_check_batch_size(size_count: usize) -> Result<(), KeyValueStoreError> {
    let batch_len_items = usize_to_u32_limit_checked(size_count, MAX_SETMULTI_KEYS);
    if batch_len_items > MAX_SETMULTI_KEYS {
        return Err(KeyValueStoreError::BatchTooLarge {
            size: batch_len_items,
            max: MAX_SETMULTI_KEYS,
        });
    }
    Ok(())
}

/// Validate a key-value pair.
fn validate_write_command_check_pair(pair: WritePairRef<'_>) -> Result<(), KeyValueStoreError> {
    validate_write_command_check_key(pair.key)?;
    validate_write_command_check_value(pair.value)
}

/// Validate a batch operation.
fn validate_write_command_check_batch_op(op: &BatchOperation) -> Result<(), KeyValueStoreError> {
    match op {
        BatchOperation::Set { key, value } => validate_write_command_check_pair(WritePairRef { key, value }),
        BatchOperation::Delete { key } => validate_write_command_check_key(key),
    }
}

/// Validate a batch condition.
fn validate_write_command_check_condition(cond: &BatchCondition) -> Result<(), KeyValueStoreError> {
    match cond {
        BatchCondition::ValueEquals { key, expected } => {
            validate_write_command_check_pair(WritePairRef { key, value: expected })
        }
        BatchCondition::KeyExists { key } | BatchCondition::KeyNotExists { key } => {
            validate_write_command_check_key(key)
        }
    }
}

/// Validate a transaction operation.
fn validate_write_command_check_txn_op(op: &TxnOp) -> Result<(), KeyValueStoreError> {
    match op {
        TxnOp::Put { key, value } => validate_write_command_check_pair(WritePairRef { key, value }),
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
        WriteOp::Set { key, value } => validate_write_command_check_pair(WritePairRef { key, value }),
        WriteOp::Delete { key } => validate_write_command_check_key(key),
    }
}

fn validate_write_command_check_pair_list(pairs: &[(String, String)]) -> Result<(), KeyValueStoreError> {
    validate_write_command_check_batch_size(pairs.len())?;
    for (key, value) in pairs {
        validate_write_command_check_pair(WritePairRef { key, value })?;
    }
    Ok(())
}

fn validate_write_command_check_key_list(keys: &[String]) -> Result<(), KeyValueStoreError> {
    validate_write_command_check_batch_size(keys.len())?;
    for key in keys {
        validate_write_command_check_key(key)?;
    }
    Ok(())
}

fn validate_write_command_check_batch_ops(operations: &[BatchOperation]) -> Result<(), KeyValueStoreError> {
    validate_write_command_check_batch_size(operations.len())?;
    for op in operations {
        validate_write_command_check_batch_op(op)?;
    }
    Ok(())
}

fn validate_write_command_check_conditional_batch(
    conditions: &[BatchCondition],
    operations: &[BatchOperation],
) -> Result<(), KeyValueStoreError> {
    let total_count = conditions.len().saturating_add(operations.len());
    validate_write_command_check_batch_size(total_count)?;
    for cond in conditions {
        validate_write_command_check_condition(cond)?;
    }
    for op in operations {
        validate_write_command_check_batch_op(op)?;
    }
    Ok(())
}

fn validate_write_command_check_transaction(
    compare: &[crate::transaction::TxnCompare],
    success: &[TxnOp],
    failure: &[TxnOp],
) -> Result<(), KeyValueStoreError> {
    let compare_and_success = compare.len().saturating_add(success.len());
    let total_count = compare_and_success.saturating_add(failure.len());
    validate_write_command_check_batch_size(total_count)?;
    for cmp in compare {
        validate_write_command_check_pair(WritePairRef {
            key: &cmp.key,
            value: &cmp.value,
        })?;
    }
    for op in success.iter().chain(failure.iter()) {
        validate_write_command_check_txn_op(op)?;
    }
    Ok(())
}

fn validate_write_command_check_optimistic_transaction(
    read_set: &[(String, i64)],
    write_set: &[WriteOp],
) -> Result<(), KeyValueStoreError> {
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

fn validate_write_command_check_topology_data(topology_data: &[u8]) -> Result<(), KeyValueStoreError> {
    let topology_size_bytes = usize_to_u32_limit_checked(topology_data.len(), MAX_VALUE_SIZE);
    if topology_size_bytes > MAX_VALUE_SIZE {
        return Err(KeyValueStoreError::ValueTooLarge {
            size: topology_size_bytes,
            max: MAX_VALUE_SIZE,
        });
    }
    Ok(())
}

fn usize_to_u32_limit_checked(value: usize, max_allowed: u32) -> u32 {
    match u32::try_from(value) {
        Ok(converted_value) => converted_value,
        Err(_) => max_allowed.saturating_add(1),
    }
}

/// Validate a write command against fixed size limits.
pub fn validate_write_command(command: &WriteCommand) -> Result<(), KeyValueStoreError> {
    match command {
        WriteCommand::Set { key, value }
        | WriteCommand::SetWithTTL { key, value, .. }
        | WriteCommand::SetWithLease { key, value, .. } => {
            validate_write_command_check_pair(WritePairRef { key, value })
        }
        WriteCommand::SetMulti { pairs }
        | WriteCommand::SetMultiWithTTL { pairs, .. }
        | WriteCommand::SetMultiWithLease { pairs, .. } => validate_write_command_check_pair_list(pairs),
        WriteCommand::Delete { key } | WriteCommand::ShardSplit { split_key: key, .. } => {
            validate_write_command_check_key(key)
        }
        WriteCommand::DeleteMulti { keys } => validate_write_command_check_key_list(keys),
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
        WriteCommand::CompareAndDelete { key, expected } => {
            validate_write_command_check_pair(WritePairRef { key, value: expected })
        }
        WriteCommand::Batch { operations } => validate_write_command_check_batch_ops(operations),
        WriteCommand::ConditionalBatch { conditions, operations } => {
            validate_write_command_check_conditional_batch(conditions, operations)
        }
        WriteCommand::Transaction {
            compare,
            success,
            failure,
        } => validate_write_command_check_transaction(compare, success, failure),
        WriteCommand::LeaseGrant { .. }
        | WriteCommand::LeaseRevoke { .. }
        | WriteCommand::LeaseKeepalive { .. }
        | WriteCommand::ShardMerge { .. } => Ok(()),
        WriteCommand::OptimisticTransaction { read_set, write_set } => {
            validate_write_command_check_optimistic_transaction(read_set, write_set)
        }
        WriteCommand::TopologyUpdate { topology_data } => validate_write_command_check_topology_data(topology_data),
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

    fn max_key_size_usize() -> usize {
        usize::try_from(MAX_KEY_SIZE).unwrap_or(usize::MAX)
    }

    fn max_value_size_usize() -> usize {
        usize::try_from(MAX_VALUE_SIZE).unwrap_or(usize::MAX)
    }

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
        let key = "k".repeat(max_key_size_usize());
        let cmd = WriteCommand::Set { key, value: "v".into() };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn key_over_max_size_rejected() {
        let key = "k".repeat(max_key_size_usize().saturating_add(1));
        let cmd = WriteCommand::Set { key, value: "v".into() };
        match validate_write_command(&cmd) {
            Err(KeyValueStoreError::KeyTooLarge { size, max }) => {
                assert_eq!(size, MAX_KEY_SIZE.saturating_add(1));
                assert_eq!(max, MAX_KEY_SIZE);
            }
            other => panic!("unexpected result: {:?}", other),
        }
    }

    #[test]
    fn value_at_max_size_accepted() {
        let value = "v".repeat(max_value_size_usize());
        let cmd = WriteCommand::Set { key: "k".into(), value };
        assert!(validate_write_command(&cmd).is_ok());
    }

    #[test]
    fn value_over_max_size_rejected() {
        let value = "v".repeat(max_value_size_usize().saturating_add(1));
        let cmd = WriteCommand::Set { key: "k".into(), value };
        match validate_write_command(&cmd) {
            Err(KeyValueStoreError::ValueTooLarge { size, max }) => {
                assert_eq!(size, MAX_VALUE_SIZE.saturating_add(1));
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
                assert_eq!(size, MAX_SETMULTI_KEYS.saturating_add(1));
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
                limit: MAX_SCAN_RESULTS.saturating_add(1),
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
            topology_data: vec![0; max_value_size_usize().saturating_add(1)],
        };
        assert!(matches!(validate_write_command(&cmd), Err(KeyValueStoreError::ValueTooLarge { .. })));
    }
}
