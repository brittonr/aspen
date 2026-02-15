//! Transaction (etcd-style) and OptimisticTransaction (FoundationDB-style)
//! apply helpers for the state machine.

use redb::ReadableTable;
use snafu::ResultExt;

use super::super::*;

impl SharedRedbStorage {
    /// Apply a Transaction operation (etcd-style) within a transaction.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::storage_shared) fn apply_transaction_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        leases_table: &mut redb::Table<u64, &[u8]>,
        compare: &[(u8, u8, String, String)],
        success: &[(u8, String, String)],
        failure: &[(u8, String, String)],
        log_index: u64,
    ) -> Result<AppResponse, SharedStorageError> {
        // Tiger Style: transaction branch sizes must be bounded
        assert!(
            success.len() <= MAX_SETMULTI_KEYS as usize,
            "TXN: success branch has {} ops, exceeds limit {}",
            success.len(),
            MAX_SETMULTI_KEYS
        );
        assert!(
            failure.len() <= MAX_SETMULTI_KEYS as usize,
            "TXN: failure branch has {} ops, exceeds limit {}",
            failure.len(),
            MAX_SETMULTI_KEYS
        );
        // Tiger Style: log_index must be positive
        assert!(log_index > 0, "TXN: log_index must be positive, got 0");

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
    pub(in crate::storage_shared) fn apply_optimistic_transaction_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        leases_table: &mut redb::Table<u64, &[u8]>,
        read_set: &[(String, i64)],
        write_set: &[(bool, String, String)],
        log_index: u64,
    ) -> Result<AppResponse, SharedStorageError> {
        // Tiger Style: write_set size must be bounded
        assert!(
            write_set.len() <= MAX_SETMULTI_KEYS as usize,
            "OCC TXN: write_set has {} ops, exceeds limit {}",
            write_set.len(),
            MAX_SETMULTI_KEYS
        );
        // Tiger Style: log_index must be positive
        assert!(log_index > 0, "OCC TXN: log_index must be positive, got 0");

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
}
