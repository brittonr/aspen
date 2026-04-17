//! Delete and DeleteMulti apply helpers for the state machine.

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

impl SharedRedbStorage {
    /// Apply a Delete operation within a transaction.
    pub(in crate::storage_shared) fn apply_delete_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        key: &str,
    ) -> Result<AppResponse, SharedStorageError> {
        // Tiger Style: operation key must not be empty
        assert!(!key.is_empty(), "DELETE: operation key must not be empty");

        let key_bytes = key.as_bytes();

        // Read existing entry to get index values to delete
        let existing = kv_table
            .get(key_bytes)
            .context(GetSnafu)?
            .and_then(|v| bincode::deserialize::<KvEntry>(v.value()).ok());

        // Delete index entries if the key existed
        if let Some(old_entry) = &existing {
            let old_indexable = IndexableEntry {
                value: old_entry.value.clone(),
                version: old_entry.version,
                create_revision: old_entry.create_revision,
                mod_revision: old_entry.mod_revision,
                expires_at_ms: old_entry.expires_at_ms,
                lease_id: old_entry.lease_id,
            };

            let index_update = index_registry.updates_for_delete(key_bytes, &old_indexable);
            for delete_key in &index_update.deletes {
                index_table.remove(delete_key.as_slice()).context(RemoveSnafu)?;
            }
        }

        // Delete the primary KV entry
        let was_deleted = kv_table.remove(key_bytes).context(RemoveSnafu)?.is_some();

        let mut response = empty_response();
        response.deleted = Some(was_deleted);
        Ok(response)
    }

    /// Apply a DeleteMulti operation within a transaction.
    pub(in crate::storage_shared) fn apply_delete_multi_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        keys: &[String],
    ) -> Result<AppResponse, SharedStorageError> {
        if keys.len() > max_setmulti_keys_usize() {
            return Err(SharedStorageError::BatchTooLarge {
                size: keys.len() as u32,
                max: MAX_SETMULTI_KEYS,
            });
        }

        // Tiger Style: all keys in multi-delete must be non-empty
        debug_assert!(keys.iter().all(|k| !k.is_empty()), "DELETE_MULTI: all keys must be non-empty");

        let mut has_deleted_key = false;
        for key in keys {
            let result = Self::apply_delete_in_txn(kv_table, index_table, index_registry, key)?;
            has_deleted_key |= result.deleted.unwrap_or(false);
        }

        let mut response = empty_response();
        response.deleted = Some(has_deleted_key);
        Ok(response)
    }
}
