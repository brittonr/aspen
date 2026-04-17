//! Set and SetMulti apply helpers for the state machine.

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

pub(in crate::storage_shared) struct SetOperationInput<'a> {
    pub key: &'a str,
    pub value: &'a str,
    pub log_index: u64,
    pub expires_at_ms: Option<u64>,
    pub lease_id: Option<u64>,
}

pub(in crate::storage_shared) struct SetMultiOperationInput<'a> {
    pub pairs: &'a [(String, String)],
    pub log_index: u64,
    pub expires_at_ms: Option<u64>,
    pub lease_id: Option<u64>,
}

impl SharedRedbStorage {
    /// Apply a Set operation within a transaction.
    pub(in crate::storage_shared) fn apply_set_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        leases_table: &mut redb::Table<u64, &[u8]>,
        input: SetOperationInput<'_>,
    ) -> Result<AppResponse, SharedStorageError> {
        assert!(!input.key.is_empty(), "SET: operation key must not be empty");
        assert!(input.log_index > 0, "SET: log_index must be positive, got 0");

        let key_bytes = input.key.as_bytes();

        let existing = kv_table
            .get(key_bytes)
            .context(GetSnafu)?
            .and_then(|v| bincode::deserialize::<KvEntry>(v.value()).ok());

        let existing_version = existing.as_ref().map(|entry| (entry.create_revision, entry.version));
        let versions = compute_kv_versions(existing_version, input.log_index);

        debug_assert!(versions.version >= 1, "SET: version must be >= 1, got {}", versions.version);
        debug_assert!(
            versions.mod_revision == input.log_index as i64,
            "SET: mod_revision {} must equal log_index {}",
            versions.mod_revision,
            input.log_index
        );

        let entry = KvEntry {
            value: input.value.to_string(),
            version: versions.version,
            create_revision: versions.create_revision,
            mod_revision: versions.mod_revision,
            expires_at_ms: input.expires_at_ms,
            lease_id: input.lease_id,
        };

        let new_indexable = IndexableEntry {
            value: entry.value.clone(),
            version: entry.version,
            create_revision: entry.create_revision,
            mod_revision: entry.mod_revision,
            expires_at_ms: entry.expires_at_ms,
            lease_id: entry.lease_id,
        };

        let old_indexable = existing.as_ref().map(|existing_entry| IndexableEntry {
            value: existing_entry.value.clone(),
            version: existing_entry.version,
            create_revision: existing_entry.create_revision,
            mod_revision: existing_entry.mod_revision,
            expires_at_ms: existing_entry.expires_at_ms,
            lease_id: existing_entry.lease_id,
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

        if let Some(lease_id) = input.lease_id {
            let should_update_lease_keys = if let Some(lease_data) = leases_table.get(lease_id).context(GetSnafu)? {
                let lease_entry: LeaseEntry = bincode::deserialize(lease_data.value()).context(DeserializeSnafu)?;
                !lease_entry.keys.iter().any(|lease_key| lease_key == input.key)
            } else {
                false
            };

            if should_update_lease_keys {
                let mut lease_entry = {
                    let lease_data = leases_table.get(lease_id).context(GetSnafu)?;
                    if let Some(data) = lease_data {
                        let entry: LeaseEntry = bincode::deserialize(data.value()).context(DeserializeSnafu)?;
                        Some(entry)
                    } else {
                        None
                    }
                };

                if let Some(ref mut entry) = lease_entry {
                    entry.keys.push(input.key.to_string());
                    let updated_bytes = bincode::serialize(&entry).context(SerializeSnafu)?;
                    leases_table.insert(lease_id, updated_bytes.as_slice()).context(InsertSnafu)?;
                }
            }
        }

        let mut response = empty_response();
        response.value = Some(input.value.to_string());
        Ok(response)
    }

    /// Apply a SetMulti operation within a transaction.
    pub(in crate::storage_shared) fn apply_set_multi_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        leases_table: &mut redb::Table<u64, &[u8]>,
        input: SetMultiOperationInput<'_>,
    ) -> Result<AppResponse, SharedStorageError> {
        if input.pairs.len() > max_setmulti_keys_usize() {
            return Err(SharedStorageError::BatchTooLarge {
                size: saturating_count_u32(input.pairs.len()),
                max: MAX_SETMULTI_KEYS,
            });
        }

        debug_assert!(input.pairs.iter().all(|(key, _)| !key.is_empty()), "SET_MULTI: all keys must be non-empty");

        for (key, value) in input.pairs {
            Self::apply_set_in_txn(kv_table, index_table, index_registry, leases_table, SetOperationInput {
                key,
                value,
                log_index: input.log_index,
                expires_at_ms: input.expires_at_ms,
                lease_id: input.lease_id,
            })?;
        }

        Ok(empty_response())
    }
}
