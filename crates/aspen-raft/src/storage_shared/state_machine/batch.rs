//! Batch and ConditionalBatch apply helpers for the state machine.

use redb::ReadableTable;
use snafu::ResultExt;

use super::super::*;

impl SharedRedbStorage {
    /// Apply a Batch operation within a transaction.
    pub(in crate::storage_shared) fn apply_batch_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        leases_table: &mut redb::Table<u64, &[u8]>,
        operations: &[(bool, String, String)],
        log_index: u64,
    ) -> Result<AppResponse, SharedStorageError> {
        if operations.len() > MAX_SETMULTI_KEYS as usize {
            return Err(SharedStorageError::BatchTooLarge {
                size: operations.len(),
                max: MAX_SETMULTI_KEYS,
            });
        }

        // Tiger Style: all operation keys must be non-empty
        debug_assert!(operations.iter().all(|(_, k, _)| !k.is_empty()), "BATCH: all operation keys must be non-empty");
        // Tiger Style: log_index must be positive
        assert!(log_index > 0, "BATCH: log_index must be positive, got 0");

        for (is_set, key, value) in operations {
            if *is_set {
                Self::apply_set_in_txn(
                    kv_table,
                    index_table,
                    index_registry,
                    leases_table,
                    key,
                    value,
                    log_index,
                    None,
                    None,
                )?;
            } else {
                Self::apply_delete_in_txn(kv_table, index_table, index_registry, key)?;
            }
        }

        Ok(AppResponse {
            batch_applied: Some(operations.len() as u32),
            ..Default::default()
        })
    }

    /// Apply a ConditionalBatch operation within a transaction.
    pub(in crate::storage_shared) fn apply_conditional_batch_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        leases_table: &mut redb::Table<u64, &[u8]>,
        conditions: &[(u8, String, String)],
        operations: &[(bool, String, String)],
        log_index: u64,
    ) -> Result<AppResponse, SharedStorageError> {
        if operations.len() > MAX_SETMULTI_KEYS as usize {
            return Err(SharedStorageError::BatchTooLarge {
                size: operations.len(),
                max: MAX_SETMULTI_KEYS,
            });
        }

        debug_assert!(
            conditions.len() <= MAX_SETMULTI_KEYS as usize,
            "BATCH: conditions count {} must not exceed operations limit {}",
            conditions.len(),
            MAX_SETMULTI_KEYS
        );

        // Check all conditions first
        for (i, (cond_type, key, expected)) in conditions.iter().enumerate() {
            let current = kv_table
                .get(key.as_bytes())
                .context(GetSnafu)?
                .and_then(|v| bincode::deserialize::<KvEntry>(v.value()).ok());

            let met = match cond_type {
                0 => current.as_ref().map(|e| e.value.as_str() == expected).unwrap_or(false), // ValueEquals
                1 => current.is_some(),                                                       // KeyExists
                2 => current.is_none(),                                                       // KeyNotExists
                _ => {
                    debug_assert!(*cond_type <= 2, "BATCH: invalid condition type {}", cond_type);
                    false
                }
            };

            if !met {
                return Ok(AppResponse {
                    conditions_met: Some(false),
                    failed_condition_index: Some(i as u32),
                    ..Default::default()
                });
            }
        }

        // Apply operations
        Self::apply_batch_in_txn(kv_table, index_table, index_registry, leases_table, operations, log_index)?;

        let response = AppResponse {
            conditions_met: Some(true),
            batch_applied: Some(operations.len() as u32),
            ..Default::default()
        };

        debug_assert!(
            response.conditions_met == Some(true),
            "BATCH: success response must indicate all conditions met"
        );

        Ok(response)
    }
}
