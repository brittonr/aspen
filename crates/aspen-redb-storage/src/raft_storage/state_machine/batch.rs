//! Batch and ConditionalBatch apply helpers for the state machine.

use aspen_layer::IndexRegistry;
use aspen_raft_kv_types::BatchCondition;
use aspen_raft_kv_types::BatchWriteOp;
use aspen_raft_kv_types::RaftKvResponse;
use aspen_storage_types::KvEntry;
use redb::ReadableTable;
use snafu::ResultExt;

use super::super::GetSnafu;
use super::super::RedbKvStorage;
use super::super::SharedStorageError;
use super::set::empty_response;

use aspen_constants::api::MAX_SETMULTI_KEYS;

#[inline]
fn max_setmulti_keys_usize() -> usize {
    usize::try_from(MAX_SETMULTI_KEYS).unwrap_or(usize::MAX)
}

#[inline]
fn saturating_count_u32(count_items: usize) -> u32 {
    u32::try_from(count_items).unwrap_or(u32::MAX)
}

impl RedbKvStorage {
    pub(in crate::raft_storage) fn apply_batch_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        leases_table: &mut redb::Table<u64, &[u8]>,
        operations: &[BatchWriteOp],
        log_index: u64,
    ) -> Result<RaftKvResponse, SharedStorageError> {
        if operations.len() > max_setmulti_keys_usize() {
            return Err(SharedStorageError::BatchTooLarge {
                size: saturating_count_u32(operations.len()),
                max: MAX_SETMULTI_KEYS,
            });
        }

        assert!(log_index > 0, "BATCH: log_index must be positive, got 0");

        for op in operations {
            match op {
                BatchWriteOp::Put { key, value } => {
                    Self::apply_set_in_txn(
                        kv_table,
                        index_table,
                        index_registry,
                        leases_table,
                        super::set::SetOperationInput {
                            key,
                            value,
                            log_index,
                            expires_at_ms: None,
                            lease_id: None,
                        },
                    )?;
                }
                BatchWriteOp::Delete { key } => {
                    Self::apply_delete_in_txn(kv_table, index_table, index_registry, key)?;
                }
            }
        }

        Ok(RaftKvResponse {
            batch_applied: Some(saturating_count_u32(operations.len())),
            ..empty_response()
        })
    }

    pub(in crate::raft_storage) fn apply_conditional_batch_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        leases_table: &mut redb::Table<u64, &[u8]>,
        conditions: &[BatchCondition],
        operations: &[BatchWriteOp],
        log_index: u64,
    ) -> Result<RaftKvResponse, SharedStorageError> {
        if operations.len() > max_setmulti_keys_usize() {
            return Err(SharedStorageError::BatchTooLarge {
                size: saturating_count_u32(operations.len()),
                max: MAX_SETMULTI_KEYS,
            });
        }

        for (i, condition) in conditions.iter().enumerate() {
            let (key, is_condition_met) = match condition {
                BatchCondition::ValueEquals { key, expected } => {
                    let current = kv_table
                        .get(key.as_bytes())
                        .context(GetSnafu)?
                        .and_then(|v| bincode::deserialize::<KvEntry>(v.value()).ok());
                    (key, current.as_ref().map(|e| e.value.as_str() == expected).unwrap_or(false))
                }
                BatchCondition::KeyExists { key } => {
                    let current = kv_table.get(key.as_bytes()).context(GetSnafu)?;
                    (key, current.is_some())
                }
                BatchCondition::KeyNotExists { key } => {
                    let current = kv_table.get(key.as_bytes()).context(GetSnafu)?;
                    (key, current.is_none())
                }
            };

            let _ = key;
            if !is_condition_met {
                return Ok(RaftKvResponse {
                    conditions_met: Some(false),
                    failed_condition_index: Some(saturating_count_u32(i)),
                    ..empty_response()
                });
            }
        }

        Self::apply_batch_in_txn(kv_table, index_table, index_registry, leases_table, operations, log_index)?;

        Ok(RaftKvResponse {
            conditions_met: Some(true),
            batch_applied: Some(saturating_count_u32(operations.len())),
            ..empty_response()
        })
    }
}
