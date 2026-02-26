//! Set and SetMulti apply helpers for the state machine.

use redb::ReadableTable;
use snafu::ResultExt;

use super::super::*;

impl SharedRedbStorage {
    /// Apply a Set operation within a transaction.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::storage_shared) fn apply_set_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        leases_table: &mut redb::Table<u64, &[u8]>,
        key: &str,
        value: &str,
        log_index: u64,
        expires_at_ms: Option<u64>,
        lease_id: Option<u64>,
    ) -> Result<AppResponse, SharedStorageError> {
        // Tiger Style: operation key must not be empty
        assert!(!key.is_empty(), "SET: operation key must not be empty");
        // Tiger Style: log_index must be positive for applied entries
        assert!(log_index > 0, "SET: log_index must be positive, got 0");

        let key_bytes = key.as_bytes();

        // Read existing entry to get version and create_revision
        let existing = kv_table
            .get(key_bytes)
            .context(GetSnafu)?
            .and_then(|v| bincode::deserialize::<KvEntry>(v.value()).ok());

        // Use pure function to compute versions
        let existing_version = existing.as_ref().map(|e| (e.create_revision, e.version));
        let versions = compute_kv_versions(existing_version, log_index);

        // Tiger Style: computed versions must be valid
        debug_assert!(versions.version >= 1, "SET: version must be >= 1, got {}", versions.version);
        debug_assert!(
            versions.mod_revision == log_index as i64,
            "SET: mod_revision {} must equal log_index {}",
            versions.mod_revision,
            log_index
        );

        let entry = KvEntry {
            value: value.to_string(),
            version: versions.version,
            create_revision: versions.create_revision,
            mod_revision: versions.mod_revision,
            expires_at_ms,
            lease_id,
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

        let old_indexable = existing.as_ref().map(|e| IndexableEntry {
            value: e.value.clone(),
            version: e.version,
            create_revision: e.create_revision,
            mod_revision: e.mod_revision,
            expires_at_ms: e.expires_at_ms,
            lease_id: e.lease_id,
        });

        // Generate index updates
        let index_update = index_registry.updates_for_set(key_bytes, old_indexable.as_ref(), &new_indexable);

        // Apply index deletes (old entries)
        for delete_key in &index_update.deletes {
            index_table.remove(delete_key.as_slice()).context(RemoveSnafu)?;
        }

        // Apply index inserts (new entries) - empty value
        for insert_key in &index_update.inserts {
            index_table.insert(insert_key.as_slice(), &[][..]).context(InsertSnafu)?;
        }

        // Write the primary KV entry
        let entry_bytes = bincode::serialize(&entry).context(SerializeSnafu)?;
        kv_table.insert(key_bytes, entry_bytes.as_slice()).context(InsertSnafu)?;

        // If this key is attached to a lease, update the lease's key list
        if let Some(lid) = lease_id {
            let needs_update = if let Some(lease_data) = leases_table.get(lid).context(GetSnafu)? {
                let lease_entry: LeaseEntry = bincode::deserialize(lease_data.value()).context(DeserializeSnafu)?;
                !lease_entry.keys.contains(&key.to_string())
            } else {
                false
            };

            if needs_update {
                // Re-fetch and update the lease - use scoped access to avoid borrow conflicts
                let mut lease_entry = {
                    let lease_data = leases_table.get(lid).context(GetSnafu)?;
                    if let Some(data) = lease_data {
                        let entry: LeaseEntry = bincode::deserialize(data.value()).context(DeserializeSnafu)?;
                        Some(entry)
                    } else {
                        None
                    }
                };

                if let Some(ref mut entry) = lease_entry {
                    entry.keys.push(key.to_string());
                    let updated_bytes = bincode::serialize(&entry).context(SerializeSnafu)?;
                    leases_table.insert(lid, updated_bytes.as_slice()).context(InsertSnafu)?;
                }
            }
        }

        Ok(AppResponse {
            value: Some(value.to_string()),
            ..Default::default()
        })
    }

    /// Apply a SetMulti operation within a transaction.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::storage_shared) fn apply_set_multi_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        leases_table: &mut redb::Table<u64, &[u8]>,
        pairs: &[(String, String)],
        log_index: u64,
        expires_at_ms: Option<u64>,
        lease_id: Option<u64>,
    ) -> Result<AppResponse, SharedStorageError> {
        if pairs.len() > MAX_SETMULTI_KEYS as usize {
            return Err(SharedStorageError::BatchTooLarge {
                size: pairs.len() as u32,
                max: MAX_SETMULTI_KEYS,
            });
        }

        // Tiger Style: all keys in multi-set must be non-empty
        debug_assert!(pairs.iter().all(|(k, _)| !k.is_empty()), "SET_MULTI: all keys must be non-empty");

        for (key, value) in pairs {
            Self::apply_set_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                key,
                value,
                log_index,
                expires_at_ms,
                lease_id,
            )?;
        }

        Ok(AppResponse::default())
    }
}
