//! CompareAndSwap and CompareAndDelete apply helpers for the state machine.

use redb::ReadableTable;
use snafu::ResultExt;

use super::super::*;

impl SharedRedbStorage {
    /// Apply a CompareAndSwap operation within a transaction.
    pub(in crate::storage_shared) fn apply_compare_and_swap_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        key: &str,
        expected: Option<&str>,
        new_value: &str,
        log_index: u64,
    ) -> Result<AppResponse, SharedStorageError> {
        // Tiger Style: operation key must not be empty
        assert!(!key.is_empty(), "CAS: operation key must not be empty");
        // Tiger Style: log_index must be positive
        assert!(log_index > 0, "CAS: log_index must be positive, got 0");

        let key_bytes = key.as_bytes();

        // Read current value
        let current = kv_table
            .get(key_bytes)
            .context(GetSnafu)?
            .and_then(|v| bincode::deserialize::<KvEntry>(v.value()).ok());

        let current_value = current.as_ref().map(|e| e.value.as_str());

        // Use pure function to check CAS condition
        if !check_cas_condition(expected, current_value) {
            return Ok(AppResponse {
                value: current_value.map(String::from),
                cas_succeeded: Some(false),
                ..Default::default()
            });
        }

        // Use pure function to compute versions
        let existing_version = current.as_ref().map(|e| (e.create_revision, e.version));
        let versions = compute_kv_versions(existing_version, log_index);

        debug_assert!(versions.version >= 1, "CAS: computed version must be at least 1, got {}", versions.version);

        let entry = KvEntry {
            value: new_value.to_string(),
            version: versions.version,
            create_revision: versions.create_revision,
            mod_revision: versions.mod_revision,
            expires_at_ms: None,
            lease_id: None,
        };

        // Convert KvEntry to IndexableEntry for index updates
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

        // Generate and apply index updates
        let index_update = index_registry.updates_for_set(key_bytes, old_indexable.as_ref(), &new_indexable);
        for delete_key in &index_update.deletes {
            index_table.remove(delete_key.as_slice()).context(RemoveSnafu)?;
        }
        for insert_key in &index_update.inserts {
            index_table.insert(insert_key.as_slice(), &[][..]).context(InsertSnafu)?;
        }

        debug_assert!(
            entry.version > 0 && entry.mod_revision == log_index as i64,
            "CAS: new entry must have positive version and correct mod_revision"
        );

        let entry_bytes = bincode::serialize(&entry).context(SerializeSnafu)?;
        kv_table.insert(key_bytes, entry_bytes.as_slice()).context(InsertSnafu)?;

        Ok(AppResponse {
            value: Some(new_value.to_string()),
            cas_succeeded: Some(true),
            ..Default::default()
        })
    }

    /// Apply a CompareAndDelete operation within a transaction.
    pub(in crate::storage_shared) fn apply_compare_and_delete_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        key: &str,
        expected: &str,
    ) -> Result<AppResponse, SharedStorageError> {
        // Tiger Style: operation key must not be empty
        assert!(!key.is_empty(), "CAS DELETE: operation key must not be empty");
        // Tiger Style: expected value must not be empty for compare
        assert!(!expected.is_empty(), "CAS DELETE: expected value must not be empty");

        let key_bytes = key.as_bytes();

        // Read current value
        let current = kv_table
            .get(key_bytes)
            .context(GetSnafu)?
            .and_then(|v| bincode::deserialize::<KvEntry>(v.value()).ok());

        let current_value = current.as_ref().map(|e| e.value.as_str());

        // Check condition
        let condition_matches = current_value.is_some_and(|v| v == expected);

        if !condition_matches {
            return Ok(AppResponse {
                value: current_value.map(String::from),
                cas_succeeded: Some(false),
                ..Default::default()
            });
        }

        debug_assert!(current.is_some(), "CAS DELETE: condition matched implies current value exists");

        // Delete index entries if the key exists
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

        // Delete the key
        kv_table.remove(key_bytes).context(RemoveSnafu)?;

        let response = AppResponse {
            deleted: Some(true),
            cas_succeeded: Some(true),
            ..Default::default()
        };

        debug_assert!(
            response.cas_succeeded == Some(true) && response.deleted == Some(true),
            "CAS DELETE: success path must set both flags"
        );

        Ok(response)
    }
}
