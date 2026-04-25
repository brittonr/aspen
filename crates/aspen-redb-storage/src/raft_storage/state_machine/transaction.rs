//! Transaction (etcd-style) and OptimisticTransaction (FoundationDB-style)
//! apply helpers for the state machine.

use aspen_constants::api::MAX_SCAN_RESULTS;
use aspen_kv_types::KeyValueWithRevision;
use aspen_kv_types::TxnOpResult;
use aspen_layer::IndexRegistry;
use aspen_raft_kv_types::BatchWriteOp;
use aspen_raft_kv_types::RaftKvResponse;
use aspen_raft_kv_types::TxnCompareOp;
use aspen_raft_kv_types::TxnCompareSpec;
use aspen_raft_kv_types::TxnCompareTarget;
use aspen_raft_kv_types::TxnOpSpec;
use aspen_storage_types::KvEntry;
use redb::ReadableTable;
use snafu::ResultExt;

use super::super::GetSnafu;
use super::super::RangeSnafu;
use super::super::RedbKvStorage;
use super::super::SharedStorageError;
use super::set::empty_response;

use aspen_constants::api::MAX_SETMULTI_KEYS;

#[inline]
fn max_setmulti_keys_usize() -> usize {
    usize::try_from(MAX_SETMULTI_KEYS).unwrap_or(usize::MAX)
}

impl RedbKvStorage {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::raft_storage) fn apply_transaction_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        leases_table: &mut redb::Table<u64, &[u8]>,
        compare: &[TxnCompareSpec],
        success: &[TxnOpSpec],
        failure: &[TxnOpSpec],
        log_index: u64,
    ) -> Result<RaftKvResponse, SharedStorageError> {
        assert!(
            success.len() <= max_setmulti_keys_usize(),
            "TXN: success branch has {} ops, exceeds limit {}",
            success.len(),
            MAX_SETMULTI_KEYS
        );
        assert!(
            failure.len() <= max_setmulti_keys_usize(),
            "TXN: failure branch has {} ops, exceeds limit {}",
            failure.len(),
            MAX_SETMULTI_KEYS
        );
        assert!(log_index > 0, "TXN: log_index must be positive, got 0");

        let mut should_run_success_branch = true;

        for spec in compare {
            let key_bytes = spec.key.as_bytes();
            let current_entry = kv_table
                .get(key_bytes)
                .context(GetSnafu)?
                .and_then(|v| bincode::deserialize::<KvEntry>(v.value()).ok());

            let is_condition_met = match spec.target {
                TxnCompareTarget::Value => {
                    let current_value = current_entry.as_ref().map(|e| e.value.as_str());
                    match spec.op {
                        TxnCompareOp::Equal => current_value == Some(spec.value.as_str()),
                        TxnCompareOp::NotEqual => current_value != Some(spec.value.as_str()),
                        TxnCompareOp::Greater => current_value.map(|v| v > spec.value.as_str()).unwrap_or(false),
                        TxnCompareOp::Less => current_value.map(|v| v < spec.value.as_str()).unwrap_or(false),
                    }
                }
                TxnCompareTarget::Version => {
                    let current_version = current_entry.as_ref().map(|e| e.version).unwrap_or(0);
                    let expected_version: i64 = spec.value.parse().unwrap_or(0);
                    match spec.op {
                        TxnCompareOp::Equal => current_version == expected_version,
                        TxnCompareOp::NotEqual => current_version != expected_version,
                        TxnCompareOp::Greater => current_version > expected_version,
                        TxnCompareOp::Less => current_version < expected_version,
                    }
                }
                TxnCompareTarget::CreateRevision => {
                    let create_rev = current_entry.as_ref().map(|e| e.create_revision).unwrap_or(0);
                    let expected_rev: i64 = spec.value.parse().unwrap_or(0);
                    match spec.op {
                        TxnCompareOp::Equal => create_rev == expected_rev,
                        TxnCompareOp::NotEqual => create_rev != expected_rev,
                        TxnCompareOp::Greater => create_rev > expected_rev,
                        TxnCompareOp::Less => create_rev < expected_rev,
                    }
                }
                TxnCompareTarget::ModRevision => {
                    let mod_rev = current_entry.as_ref().map(|e| e.mod_revision).unwrap_or(0);
                    let expected_rev: i64 = spec.value.parse().unwrap_or(0);
                    match spec.op {
                        TxnCompareOp::Equal => mod_rev == expected_rev,
                        TxnCompareOp::NotEqual => mod_rev != expected_rev,
                        TxnCompareOp::Greater => mod_rev > expected_rev,
                        TxnCompareOp::Less => mod_rev < expected_rev,
                    }
                }
            };

            if !is_condition_met {
                should_run_success_branch = false;
                break;
            }
        }

        let operations = if should_run_success_branch { success } else { failure };
        let mut results = Vec::with_capacity(operations.len());

        for op in operations {
            let result = match op {
                TxnOpSpec::Put { key, value } => {
                    Self::apply_set_in_txn(
                        kv_table,
                        index_table,
                        index_registry,
                        leases_table,
                        super::set::SetOperationInput {
                            key,
                            value,
                            log_index,
                            expires_at_ms: None,
                            lease_id: None,
                        },
                    )?;
                    TxnOpResult::Put { revision: log_index }
                }
                TxnOpSpec::Delete { key } => {
                    let del_result = Self::apply_delete_in_txn(kv_table, index_table, index_registry, key)?;
                    let deleted = if del_result.deleted.unwrap_or(false) { 1 } else { 0 };
                    TxnOpResult::Delete { deleted }
                }
                TxnOpSpec::Get { key } => {
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
                TxnOpSpec::Range { key, limit } => {
                    let max_results_items_u32 = (*limit).min(MAX_SCAN_RESULTS);
                    let max_results_items = usize::try_from(max_results_items_u32).unwrap_or(usize::MAX);
                    let mut kvs = Vec::with_capacity(max_results_items);
                    let prefix = key.as_bytes();

                    for entry in kv_table.range(prefix..).context(RangeSnafu)? {
                        let (k, v) = entry.context(GetSnafu)?;
                        if !k.value().starts_with(prefix) {
                            break;
                        }
                        if kvs.len() >= max_results_items {
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
            };
            results.push(result);
        }

        Ok(RaftKvResponse {
            succeeded: Some(should_run_success_branch),
            txn_results: Some(results),
            header_revision: Some(log_index),
            ..empty_response()
        })
    }

    pub(in crate::raft_storage) fn apply_optimistic_transaction_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        leases_table: &mut redb::Table<u64, &[u8]>,
        read_set: &[(String, i64)],
        write_set: &[BatchWriteOp],
        log_index: u64,
    ) -> Result<RaftKvResponse, SharedStorageError> {
        assert!(
            write_set.len() <= max_setmulti_keys_usize(),
            "OCC TXN: write_set has {} ops, exceeds limit {}",
            write_set.len(),
            MAX_SETMULTI_KEYS
        );
        assert!(log_index > 0, "OCC TXN: log_index must be positive, got 0");

        for (key, expected_version) in read_set {
            let key_bytes = key.as_bytes();
            let current_entry = kv_table
                .get(key_bytes)
                .context(GetSnafu)?
                .and_then(|v| bincode::deserialize::<KvEntry>(v.value()).ok());

            let current_version = current_entry.as_ref().map(|e| e.version).unwrap_or(0);

            if current_version != *expected_version {
                return Ok(RaftKvResponse {
                    occ_conflict: Some(true),
                    conflict_key: Some(key.clone()),
                    ..empty_response()
                });
            }
        }

        for op in write_set {
            match op {
                BatchWriteOp::Put { key, value } => {
                    Self::apply_set_in_txn(
                        kv_table,
                        index_table,
                        index_registry,
                        leases_table,
                        super::set::SetOperationInput {
                            key,
                            value,
                            log_index,
                            expires_at_ms: None,
                            lease_id: None,
                        },
                    )?;
                }
                BatchWriteOp::Delete { key } => {
                    Self::apply_delete_in_txn(kv_table, index_table, index_registry, key)?;
                }
            }
        }

        Ok(RaftKvResponse {
            occ_conflict: Some(false),
            ..empty_response()
        })
    }
}
