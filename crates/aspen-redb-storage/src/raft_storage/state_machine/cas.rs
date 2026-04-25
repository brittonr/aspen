//! CompareAndSwap and CompareAndDelete apply helpers for the state machine.

use aspen_layer::IndexRegistry;
use aspen_layer::IndexableEntry;
use aspen_raft_kv_types::RaftKvResponse;
use aspen_storage_types::KvEntry;
use redb::ReadableTable;
use snafu::ResultExt;

use super::super::GetSnafu;
use super::super::InsertSnafu;
use super::super::RedbKvStorage;
use super::super::RemoveSnafu;
use super::super::SerializeSnafu;
use super::super::SharedStorageError;
use super::set::empty_response;

use crate::check_cas_condition;
use crate::compute_kv_versions;

impl RedbKvStorage {
    pub(in crate::raft_storage) fn apply_compare_and_swap_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        key: &str,
        expected: Option<&str>,
        new_value: &str,
        log_index: u64,
    ) -> Result<RaftKvResponse, SharedStorageError> {
        assert!(!key.is_empty(), "CAS: operation key must not be empty");
        assert!(log_index > 0, "CAS: log_index must be positive, got 0");

        let key_bytes = key.as_bytes();

        let current = kv_table
            .get(key_bytes)
            .context(GetSnafu)?
            .and_then(|v| bincode::deserialize::<KvEntry>(v.value()).ok());

        let current_value = current.as_ref().map(|e| e.value.as_str());

        if !check_cas_condition(expected, current_value) {
            return Ok(RaftKvResponse {
                value: current_value.map(String::from),
                cas_succeeded: Some(false),
                ..empty_response()
            });
        }

        let existing_version = current.as_ref().map(|e| (e.create_revision, e.version));
        let versions = compute_kv_versions(existing_version, log_index);

        let entry = KvEntry {
            value: new_value.to_string(),
            version: versions.version,
            create_revision: versions.create_revision,
            mod_revision: versions.mod_revision,
            expires_at_ms: None,
            lease_id: None,
        };

        let new_indexable = IndexableEntry {
            value: entry.value.clone(),
            version: entry.version,
            create_revision: entry.create_revision,
            mod_revision: entry.mod_revision,
            expires_at_ms: entry.expires_at_ms,
            lease_id: entry.lease_id,
        };

        let old_indexable = current.as_ref().map(|e| IndexableEntry {
            value: e.value.clone(),
            version: e.version,
            create_revision: e.create_revision,
            mod_revision: e.mod_revision,
            expires_at_ms: e.expires_at_ms,
            lease_id: e.lease_id,
        });

        let index_update = index_registry.updates_for_set(key_bytes, old_indexable.as_ref(), &new_indexable);
        for delete_key in &index_update.deletes {
            index_table.remove(delete_key.as_slice()).context(RemoveSnafu)?;
        }
        for insert_key in &index_update.inserts {
            index_table.insert(insert_key.as_slice(), &[][..]).context(InsertSnafu)?;
        }

        let entry_bytes = bincode::serialize(&entry).context(SerializeSnafu)?;
        kv_table.insert(key_bytes, entry_bytes.as_slice()).context(InsertSnafu)?;

        Ok(RaftKvResponse {
            value: Some(new_value.to_string()),
            cas_succeeded: Some(true),
            ..empty_response()
        })
    }

    pub(in crate::raft_storage) fn apply_compare_and_delete_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        key: &str,
        expected: &str,
    ) -> Result<RaftKvResponse, SharedStorageError> {
        assert!(!key.is_empty(), "CAS DELETE: operation key must not be empty");
        assert!(!expected.is_empty(), "CAS DELETE: expected value must not be empty");

        let key_bytes = key.as_bytes();

        let current = kv_table
            .get(key_bytes)
            .context(GetSnafu)?
            .and_then(|v| bincode::deserialize::<KvEntry>(v.value()).ok());

        let current_value = current.as_ref().map(|e| e.value.as_str());
        let is_condition_match = current_value.is_some_and(|value| value == expected);

        if !is_condition_match {
            return Ok(RaftKvResponse {
                value: current_value.map(String::from),
                cas_succeeded: Some(false),
                ..empty_response()
            });
        }

        if let Some(old_entry) = &current {
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

        kv_table.remove(key_bytes).context(RemoveSnafu)?;

        Ok(RaftKvResponse {
            deleted: Some(true),
            cas_succeeded: Some(true),
            ..empty_response()
        })
    }
}
