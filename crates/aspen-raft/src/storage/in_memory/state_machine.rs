//! In-memory state machine implementation.
//!
//! Provides a non-persistent KV store state machine for testing and simulations.
//! Supports all AppRequest variants including transactions, leases, and batches.

use std::collections::BTreeMap;
use std::io;
use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use aspen_kv_types::KeyValueWithRevision;
use aspen_kv_types::TxnOpResult;
use n0_future::Stream;
use n0_future::TryStreamExt;
use openraft::EntryPayload;
use openraft::OptionalSend;
use openraft::StoredMembership;
use openraft::alias::SnapshotDataOf;
use openraft::storage::EntryResponder;
use openraft::storage::RaftSnapshotBuilder;
use openraft::storage::RaftStateMachine;
use openraft::storage::Snapshot;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::RwLock;
use tracing::Instrument;

use super::StoredSnapshot;
use crate::integrity::GENESIS_HASH;
use crate::integrity::SnapshotIntegrity;
use crate::types::AppRequest;
use crate::types::AppResponse;
use crate::types::AppTypeConfig;

// ====================================================================================
// Helper functions for apply() - extracted for Tiger Style compliance
// ====================================================================================

#[derive(Clone, Copy)]
struct KeyValueRef<'a> {
    key: &'a str,
    value: &'a str,
}

#[derive(Clone, Copy)]
struct DeleteConditionRef<'a> {
    key: &'a str,
    expected_value: &'a str,
}

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
fn saturating_count_u32(count_items: usize) -> u32 {
    u32::try_from(count_items).unwrap_or(u32::MAX)
}

/// Apply a Set operation to the state machine.
fn apply_set(data: &mut BTreeMap<String, String>, entry: KeyValueRef<'_>) -> AppResponse {
    data.insert(entry.key.to_owned(), entry.value.to_owned());
    AppResponse {
        value: Some(entry.value.to_owned()),
        ..empty_response()
    }
}

/// Apply a SetMulti operation to the state machine.
fn apply_set_multi(data: &mut BTreeMap<String, String>, pairs: &[(String, String)]) -> AppResponse {
    for (key, value) in pairs {
        data.insert(key.clone(), value.clone());
    }
    empty_response()
}

/// Apply a Delete operation to the state machine.
fn apply_delete(data: &mut BTreeMap<String, String>, key: &str) -> AppResponse {
    let has_existing_key = data.remove(key).is_some();
    AppResponse {
        deleted: Some(has_existing_key),
        ..empty_response()
    }
}

/// Apply a DeleteMulti operation to the state machine.
fn apply_delete_multi(data: &mut BTreeMap<String, String>, keys: &[String]) -> AppResponse {
    let mut has_deleted_key = false;
    for key in keys {
        has_deleted_key |= data.contains_key(key);
        data.remove(key);
    }
    AppResponse {
        deleted: Some(has_deleted_key),
        ..empty_response()
    }
}

/// Apply a CompareAndSwap operation to the state machine.
fn apply_compare_and_swap(
    data: &mut BTreeMap<String, String>,
    key: &str,
    expected: Option<&String>,
    new_value: &str,
) -> AppResponse {
    let current_value = data.get(key).cloned();
    let is_condition_match = match (expected, &current_value) {
        (None, None) => true,
        (Some(expected_value), Some(actual_value)) => expected_value == actual_value,
        _ => false,
    };
    let response = if is_condition_match {
        data.insert(key.to_owned(), new_value.to_owned());
        AppResponse {
            value: Some(new_value.to_owned()),
            cas_succeeded: Some(true),
            ..empty_response()
        }
    } else {
        AppResponse {
            value: current_value.clone(),
            cas_succeeded: Some(false),
            ..empty_response()
        }
    };
    let expected_response_value = if is_condition_match {
        Some(new_value)
    } else {
        current_value.as_deref()
    };
    debug_assert_eq!(response.cas_succeeded, Some(is_condition_match));
    debug_assert_eq!(response.value.as_deref(), expected_response_value);
    response
}

/// Apply a CompareAndDelete operation to the state machine.
fn apply_compare_and_delete(data: &mut BTreeMap<String, String>, request: DeleteConditionRef<'_>) -> AppResponse {
    let current_value = data.get(request.key).cloned();
    let is_condition_match = matches!(&current_value, Some(actual_value) if actual_value == request.expected_value);
    if is_condition_match {
        data.remove(request.key);
        AppResponse {
            deleted: Some(true),
            cas_succeeded: Some(true),
            ..empty_response()
        }
    } else {
        AppResponse {
            value: current_value,
            cas_succeeded: Some(false),
            ..empty_response()
        }
    }
}

/// Apply a Batch operation to the state machine.
fn apply_batch(data: &mut BTreeMap<String, String>, operations: &[(bool, String, String)]) -> AppResponse {
    for (is_set, key, value) in operations {
        if *is_set {
            data.insert(key.clone(), value.clone());
        } else {
            data.remove(key);
        }
    }
    AppResponse {
        batch_applied: Some(saturating_count_u32(operations.len())),
        ..empty_response()
    }
}

/// Apply a ConditionalBatch operation to the state machine.
fn apply_conditional_batch(
    data: &mut BTreeMap<String, String>,
    conditions: &[(u8, String, String)],
    operations: &[(bool, String, String)],
) -> AppResponse {
    // Check all conditions first
    // condition types: 0=ValueEquals, 1=KeyExists, 2=KeyNotExists
    let mut should_apply_batch = true;
    let mut failed_condition_index = None;
    for (condition_index, (cond_type, key, expected)) in conditions.iter().enumerate() {
        let current_value = data.get(key);
        let is_condition_met = match cond_type {
            0 => current_value.map(|value| value == expected).unwrap_or(false), // ValueEquals
            1 => current_value.is_some(),                                       // KeyExists
            2 => current_value.is_none(),                                       // KeyNotExists
            _ => false,
        };
        if !is_condition_met {
            should_apply_batch = false;
            failed_condition_index = u32::try_from(condition_index).ok();
            break;
        }
    }

    let response = if should_apply_batch {
        for (is_set, key, value) in operations {
            if *is_set {
                data.insert(key.clone(), value.clone());
            } else {
                data.remove(key);
            }
        }
        AppResponse {
            batch_applied: Some(saturating_count_u32(operations.len())),
            conditions_met: Some(true),
            ..empty_response()
        }
    } else {
        AppResponse {
            conditions_met: Some(false),
            failed_condition_index,
            ..empty_response()
        }
    };
    debug_assert_eq!(response.conditions_met, Some(should_apply_batch));
    debug_assert_eq!(response.failed_condition_index, failed_condition_index);
    response
}

/// Apply a LeaseGrant operation (no-op for in-memory).
fn apply_lease_grant(lease_id: u64, ttl_seconds: u32) -> AppResponse {
    AppResponse {
        lease_id: Some(lease_id),
        ttl_seconds: Some(ttl_seconds),
        ..empty_response()
    }
}

/// Apply a LeaseRevoke operation (no-op for in-memory).
fn apply_lease_revoke(lease_id: u64) -> AppResponse {
    AppResponse {
        lease_id: Some(lease_id),
        keys_deleted: Some(0),
        ..empty_response()
    }
}

/// Apply a LeaseKeepalive operation (no-op for in-memory).
fn apply_lease_keepalive(lease_id: u64) -> AppResponse {
    AppResponse {
        lease_id: Some(lease_id),
        ttl_seconds: Some(60),
        ..empty_response()
    }
}

/// Apply a Transaction operation to the state machine.
fn apply_transaction(
    data: &mut BTreeMap<String, String>,
    compare: &[(u8, u8, String, String)],
    success: &[(u8, String, String)],
    failure: &[(u8, String, String)],
) -> AppResponse {
    let should_run_success_branch = apply_transaction_evaluate_conditions(data, compare);
    let operations = if should_run_success_branch { success } else { failure };
    let results = apply_transaction_execute_operations(data, operations);

    AppResponse {
        succeeded: Some(should_run_success_branch),
        txn_results: Some(results),
        ..empty_response()
    }
}

/// Evaluate transaction comparison conditions.
fn apply_transaction_evaluate_conditions(
    data: &BTreeMap<String, String>,
    compare: &[(u8, u8, String, String)],
) -> bool {
    debug_assert!(u32::try_from(compare.len()).is_ok());
    for (target, op, key, value) in compare {
        debug_assert!(matches!(*target, 0..=3));
        debug_assert!(matches!(*op, 0..=3));
        let current_value = data.get(key);

        let is_condition_met = match target {
            0 => match op {
                0 => current_value.map(|current| current.as_str()) == Some(value.as_str()),
                1 => current_value.map(|current| current.as_str()) != Some(value.as_str()),
                2 => current_value.map(|current| current.as_str() > value.as_str()).unwrap_or(false),
                3 => current_value.map(|current| current.as_str() < value.as_str()).unwrap_or(false),
                _ => false,
            },
            1..=3 => {
                let current_version: i64 = 0;
                let expected_version: i64 = value.parse().unwrap_or(0);
                match op {
                    0 => current_version == expected_version,
                    1 => current_version != expected_version,
                    2 => current_version > expected_version,
                    3 => current_version < expected_version,
                    _ => false,
                }
            }
            _ => false,
        };

        if !is_condition_met {
            return false;
        }
    }
    true
}

/// Execute transaction operations.
fn apply_transaction_execute_operations(
    data: &mut BTreeMap<String, String>,
    operations: &[(u8, String, String)],
) -> Vec<TxnOpResult> {
    let mut results = Vec::with_capacity(operations.len());

    for (op_type, key, value) in operations {
        let maybe_result = match op_type {
            0 => {
                data.insert(key.clone(), value.clone());
                Some(TxnOpResult::Put { revision: 0 })
            }
            1 => {
                let deleted = u32::from(data.remove(key).is_some());
                Some(TxnOpResult::Delete { deleted })
            }
            2 => {
                let kv = data.get(key).map(|stored_value| KeyValueWithRevision {
                    key: key.clone(),
                    value: stored_value.clone(),
                    version: 0,
                    create_revision: 0,
                    mod_revision: 0,
                });
                Some(TxnOpResult::Get { kv })
            }
            3 => {
                let max_results: usize = value.parse().unwrap_or(10);
                let kvs: Vec<_> = data
                    .iter()
                    .filter(|(stored_key, _)| stored_key.starts_with(key))
                    .take(max_results)
                    .map(|(stored_key, stored_value)| KeyValueWithRevision {
                        key: stored_key.clone(),
                        value: stored_value.clone(),
                        version: 0,
                        create_revision: 0,
                        mod_revision: 0,
                    })
                    .collect();
                Some(TxnOpResult::Range { kvs, more: false })
            }
            _ => None,
        };
        if let Some(result) = maybe_result {
            debug_assert!(matches!(
                (&result, op_type),
                (TxnOpResult::Put { .. }, 0)
                    | (TxnOpResult::Delete { .. }, 1)
                    | (TxnOpResult::Get { .. }, 2)
                    | (TxnOpResult::Range { .. }, 3)
            ));
            results.push(result);
            debug_assert!(results.len() <= operations.len());
        }
    }

    results
}

/// Apply an OptimisticTransaction operation to the state machine.
fn apply_optimistic_transaction(
    data: &mut BTreeMap<String, String>,
    write_set: &[(bool, String, String)],
) -> AppResponse {
    for (is_set, key, value) in write_set {
        if *is_set {
            data.insert(key.clone(), value.clone());
        } else {
            data.remove(key);
        }
    }
    AppResponse {
        occ_conflict: Some(false),
        batch_applied: Some(saturating_count_u32(write_set.len())),
        ..empty_response()
    }
}

/// Internal state machine data for InMemoryStateMachine.
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub(crate) struct StateMachineData {
    /// Last log ID that was applied to the state machine.
    pub last_applied_log: Option<openraft::LogId<AppTypeConfig>>,
    /// Last known membership configuration.
    pub last_membership: StoredMembership<AppTypeConfig>,
    /// Key-value data store.
    pub data: BTreeMap<String, String>,
}

/// Simple in-memory state machine that mirrors the openraft memstore example.
///
/// Provides a non-persistent KV store for testing and simulations. All data
/// is stored in a `BTreeMap` and lost when the state machine is dropped.
#[derive(Debug, Default)]
pub struct InMemoryStateMachine {
    /// State machine data (last applied log, membership, KV data).
    state_machine: RwLock<StateMachineData>,
    /// Counter for generating unique snapshot IDs.
    snapshot_idx: AtomicU64,
    /// Currently held snapshot.
    ///
    /// Lock ordering: when both locks are needed, acquire `state_machine` before
    /// `current_snapshot`.
    current_snapshot: RwLock<Option<StoredSnapshot>>,
}

impl InMemoryStateMachine {
    /// Create a new in-memory state machine wrapped in an Arc.
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Wrap an existing shared state machine for openraft storage traits.
    pub fn store(inner: Arc<Self>) -> InMemoryStateMachineStore {
        InMemoryStateMachineStore(inner)
    }

    /// Test/helper shim for the openraft `apply()` trait method.
    pub async fn apply<Strm>(self: &Arc<Self>, entries: Strm) -> Result<(), io::Error>
    where Strm: Stream<Item = Result<EntryResponder<AppTypeConfig>, io::Error>> + Unpin + OptionalSend {
        let mut store = Self::store(self.clone());
        RaftStateMachine::<AppTypeConfig>::apply(&mut store, entries).await
    }

    /// Test/helper shim for the openraft `applied_state()` trait method.
    pub async fn applied_state(
        self: &Arc<Self>,
    ) -> Result<(Option<openraft::LogId<AppTypeConfig>>, StoredMembership<AppTypeConfig>), io::Error> {
        let mut store = Self::store(self.clone());
        RaftStateMachine::<AppTypeConfig>::applied_state(&mut store).await
    }

    /// Test/helper shim for the openraft `get_snapshot_builder()` trait method.
    pub async fn get_snapshot_builder(self: &Arc<Self>) -> InMemoryStateMachineStore {
        let mut store = Self::store(self.clone());
        RaftStateMachine::<AppTypeConfig>::get_snapshot_builder(&mut store).await
    }

    /// Test/helper shim for the openraft `begin_receiving_snapshot()` trait method.
    pub async fn begin_receiving_snapshot(self: &Arc<Self>) -> Result<SnapshotDataOf<AppTypeConfig>, io::Error> {
        let mut store = Self::store(self.clone());
        RaftStateMachine::<AppTypeConfig>::begin_receiving_snapshot(&mut store).await
    }

    /// Test/helper shim for the openraft `get_current_snapshot()` trait method.
    pub async fn get_current_snapshot(self: &Arc<Self>) -> Result<Option<Snapshot<AppTypeConfig>>, io::Error> {
        let mut store = Self::store(self.clone());
        RaftStateMachine::<AppTypeConfig>::get_current_snapshot(&mut store).await
    }

    /// Test/helper shim for the openraft `build_snapshot()` trait method.
    pub async fn build_snapshot(self: &Arc<Self>) -> Result<Snapshot<AppTypeConfig>, io::Error> {
        let mut store = Self::store(self.clone());
        RaftSnapshotBuilder::<AppTypeConfig>::build_snapshot(&mut store).await
    }

    /// Get a value from the state machine by key.
    ///
    /// Returns `None` if the key does not exist.
    pub async fn get(&self, key: &str) -> Option<String> {
        let sm = self.state_machine.read().await;
        sm.data.get(key).cloned()
    }

    /// Scan all keys that start with the given prefix (async version).
    ///
    /// Returns a list of full key names.
    ///
    /// # Arguments
    ///
    /// * `prefix` - Key prefix to match
    pub async fn scan_keys_with_prefix(&self, prefix: &str) -> Vec<String> {
        let sm = self.state_machine.read().await;
        sm.data.keys().filter(|k| k.starts_with(prefix)).cloned().collect()
    }

    /// Scan all key-value pairs that start with the given prefix.
    ///
    /// Returns a list of (key, value) pairs.
    ///
    /// # Arguments
    ///
    /// * `prefix` - Key prefix to match
    pub async fn scan_kv_with_prefix(&self, prefix: &str) -> Vec<(String, String)> {
        let sm = self.state_machine.read().await;
        sm.data.iter().filter(|(k, _)| k.starts_with(prefix)).map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    /// Async version of scan_kv_with_prefix for use in async contexts.
    ///
    /// Returns a list of (key, value) pairs matching the prefix.
    ///
    /// # Arguments
    ///
    /// * `prefix` - Key prefix to match
    pub async fn scan_kv_with_prefix_async(&self, prefix: &str) -> Vec<(String, String)> {
        let sm = self.state_machine.read().await;
        sm.data.iter().filter(|(k, _)| k.starts_with(prefix)).map(|(k, v)| (k.clone(), v.clone())).collect()
    }
}

#[derive(Clone, Debug)]
pub struct InMemoryStateMachineStore(Arc<InMemoryStateMachine>);

impl InMemoryStateMachineStore {
    pub fn into_inner(self) -> Arc<InMemoryStateMachine> {
        self.0
    }
}

impl std::ops::Deref for InMemoryStateMachineStore {
    type Target = InMemoryStateMachine;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl RaftSnapshotBuilder<AppTypeConfig> for InMemoryStateMachineStore {
    async fn build_snapshot(&mut self) -> Result<Snapshot<AppTypeConfig>, io::Error> {
        async move {
            let state_machine = self.state_machine.read().await;
            let data = serde_json::to_vec(&state_machine.data)
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
            let last_applied_log = state_machine.last_applied_log;
            let last_membership = state_machine.last_membership.clone();
            let mut current_snapshot = self.current_snapshot.write().await;
            drop(state_machine);

            let previous_snapshot_idx = self.snapshot_idx.fetch_add(1, Ordering::Relaxed);
            let snapshot_idx = previous_snapshot_idx.saturating_add(1);
            let snapshot_id = if let Some(last) = last_applied_log {
                format!("{}-{}-{snapshot_idx}", last.committed_leader_id(), last.index())
            } else {
                format!("--{snapshot_idx}")
            };

            let meta = openraft::SnapshotMeta {
                last_log_id: last_applied_log,
                last_membership,
                snapshot_id,
            };

            // Compute snapshot integrity hash (Tiger Style: verify data corruption)
            let meta_bytes = bincode::serialize(&meta).map_err(|err| io::Error::other(err.to_string()))?;
            let integrity = SnapshotIntegrity::compute(&meta_bytes, &data, GENESIS_HASH);

            let snapshot = StoredSnapshot {
                meta: meta.clone(),
                data: data.clone(),
                integrity: Some(integrity),
            };
            *current_snapshot = Some(snapshot);

            Ok(Snapshot {
                meta,
                snapshot: Cursor::new(data),
            })
        }
        .instrument(tracing::trace_span!("build_snapshot"))
        .await
    }
}

impl RaftStateMachine<AppTypeConfig> for InMemoryStateMachineStore {
    type SnapshotBuilder = Self;

    async fn applied_state(
        &mut self,
    ) -> Result<(Option<openraft::LogId<AppTypeConfig>>, StoredMembership<AppTypeConfig>), io::Error> {
        let state_machine = self.state_machine.read().await;
        Ok((state_machine.last_applied_log, state_machine.last_membership.clone()))
    }

    async fn apply<Strm>(&mut self, mut entries: Strm) -> Result<(), io::Error>
    where Strm: Stream<Item = Result<EntryResponder<AppTypeConfig>, io::Error>> + Unpin + OptionalSend {
        async move {
            let mut sm = self.state_machine.write().await;
            for _entry_index in 0..u32::MAX {
                let Some((entry, responder)) = entries.try_next().await? else {
                    break;
                };
                sm.last_applied_log = Some(entry.log_id);
                let response = match entry.payload {
                    EntryPayload::Blank => empty_response(),
                    EntryPayload::Normal(ref req) => apply_app_request(&mut sm.data, req),
                    EntryPayload::Membership(ref membership) => {
                        sm.last_membership = StoredMembership::new(Some(entry.log_id), membership.clone());
                        empty_response()
                    }
                };
                if let Some(responder) = responder {
                    responder.send(response);
                }
            }
            Ok(())
        }
        .instrument(tracing::trace_span!("apply_entries"))
        .await
    }

    async fn begin_receiving_snapshot(&mut self) -> Result<SnapshotDataOf<AppTypeConfig>, io::Error> {
        let mut current_snapshot = self.current_snapshot.write().await;
        Ok(match current_snapshot.take() {
            Some(snapshot) => Cursor::new(snapshot.data),
            None => Cursor::new(Vec::new()),
        })
    }

    async fn install_snapshot(
        &mut self,
        meta: &openraft::SnapshotMeta<AppTypeConfig>,
        mut snapshot: SnapshotDataOf<AppTypeConfig>,
    ) -> Result<(), io::Error> {
        // Read snapshot data
        let mut snapshot_data = Vec::new();
        std::io::copy(&mut snapshot, &mut snapshot_data)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

        let new_data: BTreeMap<String, String> =
            serde_json::from_slice(&snapshot_data).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

        // Update state machine
        let mut sm = self.state_machine.write().await;
        sm.data = new_data;
        sm.last_applied_log = meta.last_log_id;
        sm.last_membership = meta.last_membership.clone();
        drop(sm);

        // Compute integrity hash for the installed snapshot (Tiger Style)
        let meta_bytes = bincode::serialize(meta).map_err(|err| io::Error::other(err.to_string()))?;
        let integrity = SnapshotIntegrity::compute(&meta_bytes, &snapshot_data, GENESIS_HASH);

        // Store the installed snapshot so get_current_snapshot() returns it
        let mut current_snapshot = self.current_snapshot.write().await;
        *current_snapshot = Some(StoredSnapshot {
            meta: meta.clone(),
            data: snapshot_data,
            integrity: Some(integrity),
        });

        Ok(())
    }

    async fn get_current_snapshot(&mut self) -> Result<Option<Snapshot<AppTypeConfig>>, io::Error> {
        let snapshot = self.current_snapshot.read().await;
        Ok(snapshot.as_ref().map(|snap| Snapshot {
            meta: snap.meta.clone(),
            snapshot: Cursor::new(snap.data.clone()),
        }))
    }

    async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
        self.clone()
    }
}

/// Apply a single request to the state machine data.
///
/// This is extracted from apply() for Tiger Style compliance (function length < 70 lines).
fn apply_app_request(data: &mut BTreeMap<String, String>, req: &AppRequest) -> AppResponse {
    match req {
        AppRequest::Set { key, value } => apply_set(data, KeyValueRef { key, value }),
        // TTL operations: store value but don't track expiration (testing only)
        AppRequest::SetWithTTL { key, value, .. } => apply_set(data, KeyValueRef { key, value }),
        AppRequest::SetMulti { pairs } => apply_set_multi(data, pairs),
        AppRequest::SetMultiWithTTL { pairs, .. } => apply_set_multi(data, pairs),
        AppRequest::Delete { key } => apply_delete(data, key),
        AppRequest::DeleteMulti { keys } => apply_delete_multi(data, keys),
        AppRequest::CompareAndSwap {
            key,
            expected,
            new_value,
        } => apply_compare_and_swap(data, key, expected.as_ref(), new_value),
        AppRequest::CompareAndDelete { key, expected } => apply_compare_and_delete(data, DeleteConditionRef {
            key,
            expected_value: expected,
        }),
        AppRequest::Batch { operations } => apply_batch(data, operations),
        AppRequest::ConditionalBatch { conditions, operations } => {
            apply_conditional_batch(data, conditions, operations)
        }
        // Lease operations: store values but don't track leases (testing only)
        AppRequest::SetWithLease { key, value, .. } => apply_set(data, KeyValueRef { key, value }),
        AppRequest::SetMultiWithLease { pairs, .. } => apply_set_multi(data, pairs),
        AppRequest::LeaseGrant { lease_id, ttl_seconds } => apply_lease_grant(*lease_id, *ttl_seconds),
        AppRequest::LeaseRevoke { lease_id } => apply_lease_revoke(*lease_id),
        AppRequest::LeaseKeepalive { lease_id } => apply_lease_keepalive(*lease_id),
        // Transaction: etcd-style conditional transactions
        AppRequest::Transaction {
            compare,
            success,
            failure,
        } => apply_transaction(data, compare, success, failure),
        // OptimisticTransaction: in-memory doesn't track versions
        AppRequest::OptimisticTransaction { write_set, .. } => apply_optimistic_transaction(data, write_set),
        AppRequest::TrustInitialize(_) => empty_response(),
        AppRequest::TrustReconfiguration(_) => empty_response(),
        // Shard topology operations: no-op for in-memory (testing only)
        AppRequest::ShardSplit { .. } | AppRequest::ShardMerge { .. } | AppRequest::TopologyUpdate { .. } => {
            empty_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inmemory_state_machine_new() {
        let sm = InMemoryStateMachine::new();
        // Should be wrapped in Arc
        let _cloned = Arc::clone(&sm);
    }

    #[tokio::test]
    async fn test_inmemory_state_machine_get_nonexistent() {
        let sm = InMemoryStateMachine::new();
        let value = sm.get("nonexistent").await;
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_inmemory_state_machine_scan_keys_empty() {
        let sm = InMemoryStateMachine::new();
        let keys = sm.scan_keys_with_prefix("test:").await;
        assert!(keys.is_empty());
    }

    #[tokio::test]
    async fn test_inmemory_state_machine_scan_kv_empty() {
        let sm = InMemoryStateMachine::new();
        let pairs = sm.scan_kv_with_prefix("test:").await;
        assert!(pairs.is_empty());
    }

    #[tokio::test]
    async fn test_inmemory_state_machine_scan_kv_async_empty() {
        let sm = InMemoryStateMachine::new();
        let pairs = sm.scan_kv_with_prefix_async("test:").await;
        assert!(pairs.is_empty());
    }

    #[tokio::test]
    async fn test_inmemory_state_machine_applied_state_initial() {
        let sm = InMemoryStateMachine::new();
        let mut store = InMemoryStateMachine::store(sm);
        let (last_applied, membership) = store.applied_state().await.unwrap();

        assert_eq!(last_applied, None);
        // Membership is an Option - check inner membership is empty
        assert!(membership.membership().nodes().next().is_none());
    }

    #[tokio::test]
    async fn test_inmemory_state_machine_get_snapshot_builder() {
        let sm = InMemoryStateMachine::new();
        let mut store = InMemoryStateMachine::store(sm);
        let _builder = store.get_snapshot_builder().await;
        // Builder should be a clone of self
    }

    #[tokio::test]
    async fn test_inmemory_state_machine_begin_receiving_snapshot() {
        let sm = InMemoryStateMachine::new();
        let mut store = InMemoryStateMachine::store(sm);
        let cursor = store.begin_receiving_snapshot().await.unwrap();
        // Should return empty cursor initially
        assert_eq!(cursor.get_ref().len(), 0);
    }

    #[tokio::test]
    async fn test_inmemory_state_machine_get_current_snapshot_none() {
        let sm = InMemoryStateMachine::new();
        let mut store = InMemoryStateMachine::store(sm);
        let snapshot = store.get_current_snapshot().await.unwrap();
        assert!(snapshot.is_none());
    }

    // =========================================================================
    // StateMachineData Tests
    // =========================================================================

    #[test]
    fn test_state_machine_data_default() {
        let data = StateMachineData::default();
        assert_eq!(data.last_applied_log, None);
        assert!(data.data.is_empty());
    }

    #[test]
    fn test_state_machine_data_clone() {
        let mut data = StateMachineData::default();
        data.data.insert("key".to_string(), "value".to_string());

        let cloned = data.clone();
        assert_eq!(cloned.data.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_state_machine_data_serde() {
        let mut data = StateMachineData::default();
        data.data.insert("test".to_string(), "data".to_string());

        let serialized = bincode::serialize(&data).expect("serialize");
        let deserialized: StateMachineData = bincode::deserialize(&serialized).expect("deserialize");

        assert_eq!(deserialized.data.get("test"), Some(&"data".to_string()));
    }
}
