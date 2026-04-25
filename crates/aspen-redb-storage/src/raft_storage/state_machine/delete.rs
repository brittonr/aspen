//! Delete and DeleteMulti apply helpers for the state machine.

use aspen_layer::IndexRegistry;
use aspen_layer::IndexableEntry;
use aspen_raft_kv_types::RaftKvResponse;
use aspen_storage_types::KvEntry;
use redb::ReadableTable;
use snafu::ResultExt;

use super::super::GetSnafu;
use super::super::RedbKvStorage;
use super::super::RemoveSnafu;
use super::super::SharedStorageError;
use super::set::empty_response;

use aspen_constants::api::MAX_SETMULTI_KEYS;

#[inline]
fn max_setmulti_keys_usize() -> usize {
    usize::try_from(MAX_SETMULTI_KEYS).unwrap_or(usize::MAX)
}

impl RedbKvStorage {
    pub(in crate::raft_storage) fn apply_delete_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        key: &str,
    ) -> Result<RaftKvResponse, SharedStorageError> {
        assert!(!key.is_empty(), "DELETE: operation key must not be empty");

        let key_bytes = key.as_bytes();

        let existing = kv_table
            .get(key_bytes)
            .context(GetSnafu)?
            .and_then(|v| bincode::deserialize::<KvEntry>(v.value()).ok());

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

        let was_deleted = kv_table.remove(key_bytes).context(RemoveSnafu)?.is_some();

        let mut response = empty_response();
        response.deleted = Some(was_deleted);
        Ok(response)
    }

    pub(in crate::raft_storage) fn apply_delete_multi_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        keys: &[String],
    ) -> Result<RaftKvResponse, SharedStorageError> {
        if keys.len() > max_setmulti_keys_usize() {
            return Err(SharedStorageError::BatchTooLarge {
                size: keys.len() as u32,
                max: MAX_SETMULTI_KEYS,
            });
        }

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
