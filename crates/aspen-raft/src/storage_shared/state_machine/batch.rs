//! Batch and ConditionalBatch apply helpers for the state machine.

use redb::ReadableTable;
use snafu::ResultExt;

use super::super::*;

#[inline]
fn empty_response() -> AppResponse {
    AppResponse {
        value: None,
        deleted: None,
        cas_succeeded: None,
        batch_applied: None,
        failed_condition_index: None,
        conditions_met: None,
        lease_id: None,
        ttl_seconds: None,
        keys_deleted: None,
        succeeded: None,
        txn_results: None,
        header_revision: None,
        conflict_key: None,
        conflict_expected_version: None,
        conflict_actual_version: None,
        occ_conflict: None,
        topology_version: None,
    }
}

#[inline]
fn max_setmulti_keys_usize() -> usize {
    match usize::try_from(MAX_SETMULTI_KEYS) {
        Ok(max_keys) => max_keys,
        Err(_) => usize::MAX,
    }
}

#[inline]
fn saturating_count_u32(count_items: usize) -> u32 {
    match u32::try_from(count_items) {
        Ok(count_u32) => count_u32,
        Err(_) => u32::MAX,
    }
}

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
        if operations.len() > max_setmulti_keys_usize() {
            return Err(SharedStorageError::BatchTooLarge {
                size: saturating_count_u32(operations.len()),
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
            batch_applied: Some(saturating_count_u32(operations.len())),
            ..empty_response()
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
        if operations.len() > max_setmulti_keys_usize() {
            return Err(SharedStorageError::BatchTooLarge {
                size: saturating_count_u32(operations.len()),
                max: MAX_SETMULTI_KEYS,
            });
        }

        debug_assert!(
            conditions.len() <= max_setmulti_keys_usize(),
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

            let is_condition_met = match cond_type {
                0 => current.as_ref().map(|e| e.value.as_str() == expected).unwrap_or(false), // ValueEquals
                1 => current.is_some(),                                                       // KeyExists
                2 => current.is_none(),                                                       // KeyNotExists
                _ => {
                    debug_assert!(*cond_type <= 2, "BATCH: invalid condition type {}", cond_type);
                    false
                }
            };

            if !is_condition_met {
                return Ok(AppResponse {
                    conditions_met: Some(false),
                    failed_condition_index: Some(saturating_count_u32(i)),
                    ..empty_response()
                });
            }
        }

        // Apply operations
        Self::apply_batch_in_txn(kv_table, index_table, index_registry, leases_table, operations, log_index)?;

        let response = AppResponse {
            conditions_met: Some(true),
            batch_applied: Some(saturating_count_u32(operations.len())),
            ..empty_response()
        };

        debug_assert!(
            response.conditions_met == Some(true),
            "BATCH: success response must indicate all conditions met"
        );

        Ok(response)
    }
}
