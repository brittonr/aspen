//! State machine apply helpers: `apply_*_in_txn` methods that handle applying
//! Raft log entries to the state.

use redb::ReadableTable;
use snafu::ResultExt;

use super::*;

impl SharedRedbStorage {
    /// Apply a Set operation within a transaction.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn apply_set_in_txn(
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
        let key_bytes = key.as_bytes();

        // Read existing entry to get version and create_revision
        let existing = kv_table
            .get(key_bytes)
            .context(GetSnafu)?
            .and_then(|v| bincode::deserialize::<KvEntry>(v.value()).ok());

        // Use pure function to compute versions
        let existing_version = existing.as_ref().map(|e| (e.create_revision, e.version));
        let versions = compute_kv_versions(existing_version, log_index);

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

    /// Apply a Delete operation within a transaction.
    pub(super) fn apply_delete_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        key: &str,
    ) -> Result<AppResponse, SharedStorageError> {
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
        let existed = kv_table.remove(key_bytes).context(RemoveSnafu)?.is_some();

        Ok(AppResponse {
            deleted: Some(existed),
            ..Default::default()
        })
    }

    /// Apply a SetMulti operation within a transaction.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn apply_set_multi_in_txn(
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
                size: pairs.len(),
                max: MAX_SETMULTI_KEYS,
            });
        }

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

    /// Apply a DeleteMulti operation within a transaction.
    pub(super) fn apply_delete_multi_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        keys: &[String],
    ) -> Result<AppResponse, SharedStorageError> {
        if keys.len() > MAX_SETMULTI_KEYS as usize {
            return Err(SharedStorageError::BatchTooLarge {
                size: keys.len(),
                max: MAX_SETMULTI_KEYS,
            });
        }

        let mut deleted_any = false;
        for key in keys {
            let result = Self::apply_delete_in_txn(kv_table, index_table, index_registry, key)?;
            deleted_any |= result.deleted.unwrap_or(false);
        }

        Ok(AppResponse {
            deleted: Some(deleted_any),
            ..Default::default()
        })
    }

    /// Apply a CompareAndSwap operation within a transaction.
    pub(super) fn apply_compare_and_swap_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        key: &str,
        expected: Option<&str>,
        new_value: &str,
        log_index: u64,
    ) -> Result<AppResponse, SharedStorageError> {
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

        let entry_bytes = bincode::serialize(&entry).context(SerializeSnafu)?;
        kv_table.insert(key_bytes, entry_bytes.as_slice()).context(InsertSnafu)?;

        Ok(AppResponse {
            value: Some(new_value.to_string()),
            cas_succeeded: Some(true),
            ..Default::default()
        })
    }

    /// Apply a CompareAndDelete operation within a transaction.
    pub(super) fn apply_compare_and_delete_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        key: &str,
        expected: &str,
    ) -> Result<AppResponse, SharedStorageError> {
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

        Ok(AppResponse {
            deleted: Some(true),
            cas_succeeded: Some(true),
            ..Default::default()
        })
    }

    /// Apply a Batch operation within a transaction.
    pub(super) fn apply_batch_in_txn(
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
    pub(super) fn apply_conditional_batch_in_txn(
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
                _ => false,
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

        Ok(AppResponse {
            conditions_met: Some(true),
            batch_applied: Some(operations.len() as u32),
            ..Default::default()
        })
    }

    /// Apply a lease grant operation within a transaction.
    pub(super) fn apply_lease_grant_in_txn(
        leases_table: &mut redb::Table<u64, &[u8]>,
        lease_id: u64,
        ttl_seconds: u32,
    ) -> Result<AppResponse, SharedStorageError> {
        // Generate lease_id if not provided (0 means auto-generate)
        let actual_lease_id = if lease_id == 0 {
            // Simple ID generation using timestamp + random component
            let now = now_unix_ms();
            let random_component = now % 1000000;
            now * 1000 + random_component
        } else {
            lease_id
        };

        // Use pure function to create lease entry data
        let lease_data = create_lease_entry(ttl_seconds, now_unix_ms());

        // Create lease entry from pure computed data
        let lease_entry = LeaseEntry {
            ttl_seconds: lease_data.ttl_seconds,
            expires_at_ms: lease_data.expires_at_ms,
            keys: lease_data.keys, // Keys will be added when SetWithLease is called
        };

        // Store lease
        let lease_bytes = bincode::serialize(&lease_entry).context(SerializeSnafu)?;
        leases_table.insert(actual_lease_id, lease_bytes.as_slice()).context(InsertSnafu)?;

        Ok(AppResponse {
            lease_id: Some(actual_lease_id),
            ttl_seconds: Some(ttl_seconds),
            ..Default::default()
        })
    }

    /// Apply a lease revoke operation within a transaction.
    pub(super) fn apply_lease_revoke_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        leases_table: &mut redb::Table<u64, &[u8]>,
        lease_id: u64,
    ) -> Result<AppResponse, SharedStorageError> {
        // Get the lease entry and extract the keys
        let keys_to_delete = if let Some(lease_data) = leases_table.get(lease_id).context(GetSnafu)? {
            let lease_entry: LeaseEntry = bincode::deserialize(lease_data.value()).context(DeserializeSnafu)?;
            lease_entry.keys.clone()
        } else {
            Vec::new()
        };

        let mut keys_deleted = 0u32;

        // Delete all keys attached to this lease
        for key in &keys_to_delete {
            let result = Self::apply_delete_in_txn(kv_table, index_table, index_registry, key)?;
            if result.deleted.unwrap_or(false) {
                keys_deleted += 1;
            }
        }

        // Delete the lease itself if it exists
        if !keys_to_delete.is_empty() {
            leases_table.remove(lease_id).context(RemoveSnafu)?;
        }

        Ok(AppResponse {
            keys_deleted: Some(keys_deleted),
            ..Default::default()
        })
    }

    /// Apply a lease keepalive operation within a transaction.
    pub(super) fn apply_lease_keepalive_in_txn(
        leases_table: &mut redb::Table<u64, &[u8]>,
        lease_id: u64,
    ) -> Result<AppResponse, SharedStorageError> {
        // Get the existing lease and extract data to avoid borrow conflicts
        let lease_opt = {
            let lease_bytes = leases_table.get(lease_id).context(GetSnafu)?;
            if let Some(lease_data) = lease_bytes {
                let entry: LeaseEntry = bincode::deserialize(lease_data.value()).context(DeserializeSnafu)?;
                Some(entry)
            } else {
                None
            }
        };

        if let Some(mut lease_entry) = lease_opt {
            // Use pure function to compute new expiration time
            let ttl = lease_entry.ttl_seconds;
            lease_entry.expires_at_ms = compute_lease_refresh(ttl, now_unix_ms());

            // Update the lease
            let updated_bytes = bincode::serialize(&lease_entry).context(SerializeSnafu)?;
            leases_table.insert(lease_id, updated_bytes.as_slice()).context(InsertSnafu)?;

            Ok(AppResponse {
                lease_id: Some(lease_id),
                ttl_seconds: Some(ttl),
                ..Default::default()
            })
        } else {
            // Lease not found - return None for lease_id to indicate not found
            Ok(AppResponse {
                lease_id: None,
                ..Default::default()
            })
        }
    }

    /// Apply a Transaction operation (etcd-style) within a transaction.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn apply_transaction_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        leases_table: &mut redb::Table<u64, &[u8]>,
        compare: &[(u8, u8, String, String)],
        success: &[(u8, String, String)],
        failure: &[(u8, String, String)],
        log_index: u64,
    ) -> Result<AppResponse, SharedStorageError> {
        // Evaluate all comparison conditions
        let mut all_conditions_met = true;

        for (target, op, key, value) in compare {
            let key_bytes = key.as_bytes();
            let current_entry = kv_table
                .get(key_bytes)
                .context(GetSnafu)?
                .and_then(|v| bincode::deserialize::<KvEntry>(v.value()).ok());

            let condition_met = match target {
                0 => {
                    // Value comparison
                    let current_value = current_entry.as_ref().map(|e| e.value.as_str());
                    match op {
                        0 => current_value == Some(value.as_str()),                      // Equal
                        1 => current_value != Some(value.as_str()),                      // NotEqual
                        2 => current_value.map(|v| v > value.as_str()).unwrap_or(false), // Greater
                        3 => current_value.map(|v| v < value.as_str()).unwrap_or(false), // Less
                        _ => false,
                    }
                }
                1 => {
                    // Version comparison
                    let current_version = current_entry.as_ref().map(|e| e.version).unwrap_or(0);
                    let expected_version: i64 = value.parse().unwrap_or(0);
                    match op {
                        0 => current_version == expected_version,
                        1 => current_version != expected_version,
                        2 => current_version > expected_version,
                        3 => current_version < expected_version,
                        _ => false,
                    }
                }
                2 => {
                    // CreateRevision comparison
                    let create_rev = current_entry.as_ref().map(|e| e.create_revision).unwrap_or(0);
                    let expected_rev: i64 = value.parse().unwrap_or(0);
                    match op {
                        0 => create_rev == expected_rev,
                        1 => create_rev != expected_rev,
                        2 => create_rev > expected_rev,
                        3 => create_rev < expected_rev,
                        _ => false,
                    }
                }
                3 => {
                    // ModRevision comparison
                    let mod_rev = current_entry.as_ref().map(|e| e.mod_revision).unwrap_or(0);
                    let expected_rev: i64 = value.parse().unwrap_or(0);
                    match op {
                        0 => mod_rev == expected_rev,
                        1 => mod_rev != expected_rev,
                        2 => mod_rev > expected_rev,
                        3 => mod_rev < expected_rev,
                        _ => false,
                    }
                }
                _ => false,
            };

            if !condition_met {
                all_conditions_met = false;
                break;
            }
        }

        // Execute the appropriate branch based on conditions
        let operations = if all_conditions_met { success } else { failure };
        let mut results = Vec::new();

        for (op_type, key, value) in operations {
            let result = match op_type {
                0 => {
                    // Put operation
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
                    TxnOpResult::Put { revision: log_index }
                }
                1 => {
                    // Delete operation
                    let del_result = Self::apply_delete_in_txn(kv_table, index_table, index_registry, key)?;
                    let deleted = if del_result.deleted.unwrap_or(false) { 1 } else { 0 };
                    TxnOpResult::Delete { deleted }
                }
                2 => {
                    // Get operation
                    let kv = kv_table
                        .get(key.as_bytes())
                        .context(GetSnafu)?
                        .and_then(|v| bincode::deserialize::<KvEntry>(v.value()).ok())
                        .map(|entry| KeyValueWithRevision {
                            key: key.clone(),
                            value: entry.value,
                            version: entry.version as u64,
                            create_revision: entry.create_revision as u64,
                            mod_revision: entry.mod_revision as u64,
                        });
                    TxnOpResult::Get { kv }
                }
                3 => {
                    // Range operation
                    let limit: usize = value.parse().unwrap_or(10);
                    let mut kvs = Vec::new();
                    let prefix = key.as_bytes();

                    for entry in kv_table.range(prefix..).context(RangeSnafu)? {
                        let (k, v) = entry.context(GetSnafu)?;
                        // Check if key starts with prefix
                        if !k.value().starts_with(prefix) {
                            break;
                        }
                        if kvs.len() >= limit {
                            break;
                        }

                        if let Ok(kv_entry) = bincode::deserialize::<KvEntry>(v.value()) {
                            kvs.push(KeyValueWithRevision {
                                key: String::from_utf8_lossy(k.value()).to_string(),
                                value: kv_entry.value,
                                version: kv_entry.version as u64,
                                create_revision: kv_entry.create_revision as u64,
                                mod_revision: kv_entry.mod_revision as u64,
                            });
                        }
                    }

                    TxnOpResult::Range { kvs, more: false }
                }
                _ => continue,
            };
            results.push(result);
        }

        Ok(AppResponse {
            succeeded: Some(all_conditions_met),
            txn_results: Some(results),
            header_revision: Some(log_index),
            ..Default::default()
        })
    }

    /// Apply an Optimistic Transaction (FoundationDB-style) within a transaction.
    pub(super) fn apply_optimistic_transaction_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        leases_table: &mut redb::Table<u64, &[u8]>,
        read_set: &[(String, i64)],
        write_set: &[(bool, String, String)],
        log_index: u64,
    ) -> Result<AppResponse, SharedStorageError> {
        // Check all keys in read_set for version conflicts
        for (key, expected_version) in read_set {
            let key_bytes = key.as_bytes();
            let current_entry = kv_table
                .get(key_bytes)
                .context(GetSnafu)?
                .and_then(|v| bincode::deserialize::<KvEntry>(v.value()).ok());

            let current_version = current_entry.as_ref().map(|e| e.version).unwrap_or(0);

            if current_version != *expected_version {
                // Version conflict detected
                return Ok(AppResponse {
                    occ_conflict: Some(true),
                    conflict_key: Some(key.clone()),
                    ..Default::default()
                });
            }
        }

        // All version checks passed, apply write_set
        for (is_set, key, value) in write_set {
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
            occ_conflict: Some(false),
            ..Default::default()
        })
    }

    /// Apply a single AppRequest to the state machine tables within a transaction.
    pub(super) fn apply_request_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        leases_table: &mut redb::Table<u64, &[u8]>,
        request: &AppRequest,
        log_index: u64,
    ) -> Result<AppResponse, SharedStorageError> {
        match request {
            AppRequest::Set { key, value } => Self::apply_set_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                key,
                value,
                log_index,
                None,
                None,
            ),
            AppRequest::SetWithTTL {
                key,
                value,
                expires_at_ms,
            } => Self::apply_set_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                key,
                value,
                log_index,
                Some(*expires_at_ms),
                None,
            ),
            AppRequest::SetMulti { pairs } => Self::apply_set_multi_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                pairs,
                log_index,
                None,
                None,
            ),
            AppRequest::SetMultiWithTTL { pairs, expires_at_ms } => Self::apply_set_multi_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                pairs,
                log_index,
                Some(*expires_at_ms),
                None,
            ),
            AppRequest::Delete { key } => Self::apply_delete_in_txn(kv_table, index_table, index_registry, key),
            AppRequest::DeleteMulti { keys } => {
                Self::apply_delete_multi_in_txn(kv_table, index_table, index_registry, keys)
            }
            AppRequest::CompareAndSwap {
                key,
                expected,
                new_value,
            } => Self::apply_compare_and_swap_in_txn(
                kv_table,
                index_table,
                index_registry,
                key,
                expected.as_deref(),
                new_value,
                log_index,
            ),
            AppRequest::CompareAndDelete { key, expected } => {
                Self::apply_compare_and_delete_in_txn(kv_table, index_table, index_registry, key, expected)
            }
            AppRequest::Batch { operations } => {
                Self::apply_batch_in_txn(kv_table, index_table, index_registry, leases_table, operations, log_index)
            }
            AppRequest::ConditionalBatch { conditions, operations } => Self::apply_conditional_batch_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                conditions,
                operations,
                log_index,
            ),
            // Lease operations
            AppRequest::SetWithLease { key, value, lease_id } => Self::apply_set_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                key,
                value,
                log_index,
                None,
                Some(*lease_id),
            ),
            AppRequest::SetMultiWithLease { pairs, lease_id } => Self::apply_set_multi_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                pairs,
                log_index,
                None,
                Some(*lease_id),
            ),
            // Lease operations
            AppRequest::LeaseGrant { lease_id, ttl_seconds } => {
                Self::apply_lease_grant_in_txn(leases_table, *lease_id, *ttl_seconds)
            }
            AppRequest::LeaseRevoke { lease_id } => {
                Self::apply_lease_revoke_in_txn(kv_table, index_table, index_registry, leases_table, *lease_id)
            }
            AppRequest::LeaseKeepalive { lease_id } => Self::apply_lease_keepalive_in_txn(leases_table, *lease_id),
            // Transaction operations
            AppRequest::Transaction {
                compare,
                success,
                failure,
            } => Self::apply_transaction_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                compare,
                success,
                failure,
                log_index,
            ),
            AppRequest::OptimisticTransaction { read_set, write_set } => Self::apply_optimistic_transaction_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                read_set,
                write_set,
                log_index,
            ),
            // Shard operations - pass through without state changes
            AppRequest::ShardSplit { .. } | AppRequest::ShardMerge { .. } | AppRequest::TopologyUpdate { .. } => {
                Ok(AppResponse::default())
            }
        }
    }
}
