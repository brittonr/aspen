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

use super::StoredSnapshot;
use crate::integrity::GENESIS_HASH;
use crate::integrity::SnapshotIntegrity;
use crate::types::AppRequest;
use crate::types::AppResponse;
use crate::types::AppTypeConfig;

// ====================================================================================
// Helper functions for apply() - extracted for Tiger Style compliance
// ====================================================================================

/// Apply a Set operation to the state machine.
fn apply_set(data: &mut BTreeMap<String, String>, key: &str, value: &str) -> AppResponse {
    data.insert(key.to_owned(), value.to_owned());
    AppResponse {
        value: Some(value.to_owned()),
        ..Default::default()
    }
}

/// Apply a SetMulti operation to the state machine.
fn apply_set_multi(data: &mut BTreeMap<String, String>, pairs: &[(String, String)]) -> AppResponse {
    for (key, value) in pairs {
        data.insert(key.clone(), value.clone());
    }
    AppResponse::default()
}

/// Apply a Delete operation to the state machine.
fn apply_delete(data: &mut BTreeMap<String, String>, key: &str) -> AppResponse {
    let existed = data.remove(key).is_some();
    AppResponse {
        deleted: Some(existed),
        ..Default::default()
    }
}

/// Apply a DeleteMulti operation to the state machine.
fn apply_delete_multi(data: &mut BTreeMap<String, String>, keys: &[String]) -> AppResponse {
    let mut deleted_any = false;
    for key in keys {
        deleted_any |= data.contains_key(key);
        data.remove(key);
    }
    AppResponse {
        deleted: Some(deleted_any),
        ..Default::default()
    }
}

/// Apply a CompareAndSwap operation to the state machine.
fn apply_compare_and_swap(
    data: &mut BTreeMap<String, String>,
    key: &str,
    expected: Option<&String>,
    new_value: &str,
) -> AppResponse {
    let current = data.get(key).cloned();
    let condition_matches = match (expected, &current) {
        (None, None) => true,
        (Some(exp), Some(cur)) => exp == cur,
        _ => false,
    };
    if condition_matches {
        data.insert(key.to_owned(), new_value.to_owned());
        AppResponse {
            value: Some(new_value.to_owned()),
            cas_succeeded: Some(true),
            ..Default::default()
        }
    } else {
        AppResponse {
            value: current,
            cas_succeeded: Some(false),
            ..Default::default()
        }
    }
}

/// Apply a CompareAndDelete operation to the state machine.
fn apply_compare_and_delete(data: &mut BTreeMap<String, String>, key: &str, expected: &str) -> AppResponse {
    let current = data.get(key).cloned();
    let condition_matches = matches!(&current, Some(cur) if cur == expected);
    if condition_matches {
        data.remove(key);
        AppResponse {
            deleted: Some(true),
            cas_succeeded: Some(true),
            ..Default::default()
        }
    } else {
        AppResponse {
            value: current,
            cas_succeeded: Some(false),
            ..Default::default()
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
        batch_applied: Some(operations.len() as u32),
        ..Default::default()
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
    let mut conditions_met = true;
    let mut failed_index = None;
    for (i, (cond_type, key, expected)) in conditions.iter().enumerate() {
        let current = data.get(key);
        let met = match cond_type {
            0 => current.map(|v| v == expected).unwrap_or(false), // ValueEquals
            1 => current.is_some(),                               // KeyExists
            2 => current.is_none(),                               // KeyNotExists
            _ => false,
        };
        if !met {
            conditions_met = false;
            failed_index = Some(i as u32);
            break;
        }
    }

    if conditions_met {
        // Apply all operations
        for (is_set, key, value) in operations {
            if *is_set {
                data.insert(key.clone(), value.clone());
            } else {
                data.remove(key);
            }
        }
        AppResponse {
            batch_applied: Some(operations.len() as u32),
            conditions_met: Some(true),
            ..Default::default()
        }
    } else {
        AppResponse {
            conditions_met: Some(false),
            failed_condition_index: failed_index,
            ..Default::default()
        }
    }
}

/// Apply a LeaseGrant operation (no-op for in-memory).
fn apply_lease_grant(lease_id: u64, ttl_seconds: u32) -> AppResponse {
    AppResponse {
        lease_id: Some(lease_id),
        ttl_seconds: Some(ttl_seconds),
        ..Default::default()
    }
}

/// Apply a LeaseRevoke operation (no-op for in-memory).
fn apply_lease_revoke(lease_id: u64) -> AppResponse {
    AppResponse {
        lease_id: Some(lease_id),
        keys_deleted: Some(0),
        ..Default::default()
    }
}

/// Apply a LeaseKeepalive operation (no-op for in-memory).
fn apply_lease_keepalive(lease_id: u64) -> AppResponse {
    AppResponse {
        lease_id: Some(lease_id),
        ttl_seconds: Some(60), // Dummy value
        ..Default::default()
    }
}

/// Apply a Transaction operation to the state machine.
fn apply_transaction(
    data: &mut BTreeMap<String, String>,
    compare: &[(u8, u8, String, String)],
    success: &[(u8, String, String)],
    failure: &[(u8, String, String)],
) -> AppResponse {
    // Evaluate all comparison conditions
    let all_conditions_met = apply_transaction_evaluate_conditions(data, compare);

    // Execute the appropriate branch based on conditions
    let operations = if all_conditions_met { success } else { failure };
    let results = apply_transaction_execute_operations(data, operations);

    AppResponse {
        succeeded: Some(all_conditions_met),
        txn_results: Some(results),
        ..Default::default()
    }
}

/// Evaluate transaction comparison conditions.
fn apply_transaction_evaluate_conditions(
    data: &BTreeMap<String, String>,
    compare: &[(u8, u8, String, String)],
) -> bool {
    for (target, op, key, value) in compare {
        let current_value = data.get(key);

        let condition_met = match target {
            0 => {
                // Value comparison
                match op {
                    0 => current_value.map(|v| v.as_str()) == Some(value.as_str()), // Equal
                    1 => current_value.map(|v| v.as_str()) != Some(value.as_str()), // NotEqual
                    2 => current_value.map(|v| v.as_str() > value.as_str()).unwrap_or(false), // Greater
                    3 => current_value.map(|v| v.as_str() < value.as_str()).unwrap_or(false), // Less
                    _ => false,
                }
            }
            1..=3 => {
                // Version/CreateRevision/ModRevision comparison
                // In-memory doesn't track versions, treat as 0
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

        if !condition_met {
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
    let mut results = Vec::new();

    for (op_type, key, value) in operations {
        let result = match op_type {
            0 => {
                // Put operation
                data.insert(key.clone(), value.clone());
                TxnOpResult::Put { revision: 0 }
            }
            1 => {
                // Delete operation
                let deleted = if data.remove(key).is_some() { 1 } else { 0 };
                TxnOpResult::Delete { deleted }
            }
            2 => {
                // Get operation
                let kv = data.get(key).map(|v| KeyValueWithRevision {
                    key: key.clone(),
                    value: v.clone(),
                    version: 0,
                    create_revision: 0,
                    mod_revision: 0,
                });
                TxnOpResult::Get { kv }
            }
            3 => {
                // Range operation
                let limit: usize = value.parse().unwrap_or(10);
                let prefix = key;
                let kvs: Vec<_> = data
                    .iter()
                    .filter(|(k, _)| k.starts_with(prefix))
                    .take(limit)
                    .map(|(k, v)| KeyValueWithRevision {
                        key: k.clone(),
                        value: v.clone(),
                        version: 0,
                        create_revision: 0,
                        mod_revision: 0,
                    })
                    .collect();
                TxnOpResult::Range { kvs, more: false }
            }
            _ => continue,
        };
        results.push(result);
    }

    results
}

/// Apply an OptimisticTransaction operation to the state machine.
fn apply_optimistic_transaction(
    data: &mut BTreeMap<String, String>,
    write_set: &[(bool, String, String)],
) -> AppResponse {
    // In-memory state machine doesn't track versions, just apply the writes
    for (is_set, key, value) in write_set {
        if *is_set {
            data.insert(key.clone(), value.clone());
        } else {
            data.remove(key);
        }
    }
    AppResponse {
        occ_conflict: Some(false),
        batch_applied: Some(write_set.len() as u32),
        ..Default::default()
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
    current_snapshot: RwLock<Option<StoredSnapshot>>,
}

impl InMemoryStateMachine {
    /// Create a new in-memory state machine wrapped in an Arc.
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
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

impl RaftSnapshotBuilder<AppTypeConfig> for Arc<InMemoryStateMachine> {
    #[tracing::instrument(level = "trace", skip(self))]
    async fn build_snapshot(&mut self) -> Result<Snapshot<AppTypeConfig>, io::Error> {
        let state_machine = self.state_machine.read().await;
        let data =
            serde_json::to_vec(&state_machine.data).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        let last_applied_log = state_machine.last_applied_log;
        let last_membership = state_machine.last_membership.clone();
        let mut current_snapshot = self.current_snapshot.write().await;
        drop(state_machine);

        let snapshot_idx = self.snapshot_idx.fetch_add(1, Ordering::Relaxed) + 1;
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
}

impl RaftStateMachine<AppTypeConfig> for Arc<InMemoryStateMachine> {
    type SnapshotBuilder = Self;

    async fn applied_state(
        &mut self,
    ) -> Result<(Option<openraft::LogId<AppTypeConfig>>, StoredMembership<AppTypeConfig>), io::Error> {
        let state_machine = self.state_machine.read().await;
        Ok((state_machine.last_applied_log, state_machine.last_membership.clone()))
    }

    #[tracing::instrument(level = "trace", skip(self, entries))]
    async fn apply<Strm>(&mut self, mut entries: Strm) -> Result<(), io::Error>
    where Strm: Stream<Item = Result<EntryResponder<AppTypeConfig>, io::Error>> + Unpin + OptionalSend {
        let mut sm = self.state_machine.write().await;
        while let Some((entry, responder)) = entries.try_next().await? {
            sm.last_applied_log = Some(entry.log_id);
            let response = match entry.payload {
                EntryPayload::Blank => AppResponse::default(),
                EntryPayload::Normal(ref req) => apply_app_request(&mut sm.data, req),
                EntryPayload::Membership(ref membership) => {
                    sm.last_membership = StoredMembership::new(Some(entry.log_id), membership.clone());
                    AppResponse::default()
                }
            };
            if let Some(responder) = responder {
                responder.send(response);
            }
        }
        Ok(())
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
        AppRequest::Set { key, value } => apply_set(data, key, value),
        // TTL operations: store value but don't track expiration (testing only)
        AppRequest::SetWithTTL { key, value, .. } => apply_set(data, key, value),
        AppRequest::SetMulti { pairs } => apply_set_multi(data, pairs),
        AppRequest::SetMultiWithTTL { pairs, .. } => apply_set_multi(data, pairs),
        AppRequest::Delete { key } => apply_delete(data, key),
        AppRequest::DeleteMulti { keys } => apply_delete_multi(data, keys),
        AppRequest::CompareAndSwap {
            key,
            expected,
            new_value,
        } => apply_compare_and_swap(data, key, expected.as_ref(), new_value),
        AppRequest::CompareAndDelete { key, expected } => apply_compare_and_delete(data, key, expected),
        AppRequest::Batch { operations } => apply_batch(data, operations),
        AppRequest::ConditionalBatch { conditions, operations } => {
            apply_conditional_batch(data, conditions, operations)
        }
        // Lease operations: store values but don't track leases (testing only)
        AppRequest::SetWithLease { key, value, .. } => apply_set(data, key, value),
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
        // Shard topology operations: no-op for in-memory (testing only)
        AppRequest::ShardSplit { .. } | AppRequest::ShardMerge { .. } | AppRequest::TopologyUpdate { .. } => {
            AppResponse::default()
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
        let mut sm = InMemoryStateMachine::new();
        let (last_applied, membership) = sm.applied_state().await.unwrap();

        assert_eq!(last_applied, None);
        // Membership is an Option - check inner membership is empty
        assert!(membership.membership().nodes().next().is_none());
    }

    #[tokio::test]
    async fn test_inmemory_state_machine_get_snapshot_builder() {
        let mut sm = InMemoryStateMachine::new();
        let _builder = sm.get_snapshot_builder().await;
        // Builder should be a clone of self
    }

    #[tokio::test]
    async fn test_inmemory_state_machine_begin_receiving_snapshot() {
        let mut sm = InMemoryStateMachine::new();
        let cursor = sm.begin_receiving_snapshot().await.unwrap();
        // Should return empty cursor initially
        assert_eq!(cursor.get_ref().len(), 0);
    }

    #[tokio::test]
    async fn test_inmemory_state_machine_get_current_snapshot_none() {
        let mut sm = InMemoryStateMachine::new();
        let snapshot = sm.get_current_snapshot().await.unwrap();
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
