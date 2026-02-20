//! Lightweight in-memory implementations for Aspen testing.
//!
//! This crate provides deterministic, non-persistent implementations of core Aspen
//! traits for use in unit tests and deterministic simulation testing. These implementations
//! mirror the behavior of production backends without network or disk I/O.
//!
//! # Key Types
//!
//! - [`DeterministicClusterController`]: In-memory cluster controller implementing
//!   [`ClusterController`]
//! - [`DeterministicKeyValueStore`]: In-memory KV store implementing [`KeyValueStore`]
//!
//! # OCC Support
//!
//! The in-memory backend supports optimistic concurrency control (OCC) with
//! proper version tracking. Each key stores version, create_revision, and mod_revision
//! metadata to enable conflict detection in OptimisticTransaction operations.
//!
//! # Example
//!
//! ```ignore
//! use aspen_testing_core::{DeterministicClusterController, DeterministicKeyValueStore};
//! use aspen_traits::{ClusterController, KeyValueStore};
//!
//! // Create in-memory implementations
//! let cluster = DeterministicClusterController::new();
//! let kv = DeterministicKeyValueStore::new();
//!
//! // Use them just like production implementations
//! kv.write(WriteRequest::set("key", "value")).await?;
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use aspen_cluster_types::AddLearnerRequest;
use aspen_cluster_types::ChangeMembershipRequest;
use aspen_cluster_types::ClusterMetrics;
use aspen_cluster_types::ClusterState;
use aspen_cluster_types::ControlPlaneError;
use aspen_cluster_types::InitRequest;
use aspen_cluster_types::SnapshotLogId;
use aspen_constants::api::DEFAULT_SCAN_LIMIT;
use aspen_constants::api::MAX_SCAN_RESULTS;
use aspen_kv_types::BatchCondition;
use aspen_kv_types::BatchOperation;
use aspen_kv_types::CompareOp;
use aspen_kv_types::CompareTarget;
use aspen_kv_types::DeleteRequest;
use aspen_kv_types::DeleteResult;
use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::KeyValueWithRevision;
use aspen_kv_types::ReadRequest;
use aspen_kv_types::ReadResult;
use aspen_kv_types::ScanRequest;
use aspen_kv_types::ScanResult;
use aspen_kv_types::TxnCompare;
use aspen_kv_types::TxnOp;
use aspen_kv_types::TxnOpResult;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteOp;
use aspen_kv_types::WriteRequest;
use aspen_kv_types::WriteResult;
use aspen_kv_types::validate_write_command;
use aspen_traits::ClusterController;
use aspen_traits::KeyValueStore;
use async_trait::async_trait;
use tokio::sync::Mutex;

/// Value with version tracking for OCC support.
#[derive(Clone, Debug)]
struct VersionedValue {
    value: String,
    /// Per-key version counter (1, 2, 3... increments on each write)
    version: i64,
    /// Revision when key was first created
    create_revision: u64,
    /// Revision when key was last modified
    mod_revision: u64,
}

/// In-memory deterministic implementation of [`ClusterController`] for testing.
///
/// This implementation stores cluster state in memory without persistence,
/// making it useful for unit tests and simulation testing where repeatability
/// is more important than durability.
///
/// # Example
///
/// ```ignore
/// use aspen_testing_core::DeterministicClusterController;
/// use aspen_traits::ClusterController;
///
/// let controller = DeterministicClusterController::new();
/// controller.init(InitRequest {
///     initial_members: vec![ClusterNode::new(1, "node1", None)],
/// }).await?;
/// ```
#[derive(Clone, Default)]
pub struct DeterministicClusterController {
    state: Arc<Mutex<ClusterState>>,
}

impl DeterministicClusterController {
    /// Create a new in-memory cluster controller.
    ///
    /// The controller starts with an empty cluster state. Call [`ClusterController::init()`]
    /// to initialize the cluster with founding members.
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
}

#[async_trait]
impl ClusterController for DeterministicClusterController {
    async fn init(&self, request: InitRequest) -> Result<ClusterState, ControlPlaneError> {
        if request.initial_members.is_empty() {
            return Err(ControlPlaneError::InvalidRequest {
                reason: "initial_members must not be empty".into(),
            });
        }
        let mut guard = self.state.lock().await;
        guard.nodes = request.initial_members.clone();
        guard.members = request.initial_members.iter().map(|node| node.id).collect();
        Ok(guard.clone())
    }

    async fn add_learner(&self, request: AddLearnerRequest) -> Result<ClusterState, ControlPlaneError> {
        let mut guard = self.state.lock().await;
        guard.learners.push(request.learner);
        Ok(guard.clone())
    }

    async fn change_membership(&self, request: ChangeMembershipRequest) -> Result<ClusterState, ControlPlaneError> {
        if request.members.is_empty() {
            return Err(ControlPlaneError::InvalidRequest {
                reason: "members must include at least one voter".into(),
            });
        }
        let mut guard = self.state.lock().await;
        guard.members = request.members;
        Ok(guard.clone())
    }

    async fn current_state(&self) -> Result<ClusterState, ControlPlaneError> {
        Ok(self.state.lock().await.clone())
    }

    async fn get_metrics(&self) -> Result<ClusterMetrics, ControlPlaneError> {
        // Deterministic backend is in-memory stub without Raft consensus
        Err(ControlPlaneError::Unsupported {
            backend: "deterministic".into(),
            operation: "get_metrics".into(),
        })
    }

    async fn trigger_snapshot(&self) -> Result<Option<SnapshotLogId>, ControlPlaneError> {
        // Deterministic backend is in-memory stub without snapshot support
        Err(ControlPlaneError::Unsupported {
            backend: "deterministic".into(),
            operation: "trigger_snapshot".into(),
        })
    }

    async fn get_leader(&self) -> Result<Option<u64>, ControlPlaneError> {
        // Deterministic backend has no leader concept
        Ok(None)
    }

    fn is_initialized(&self) -> bool {
        // In-memory deterministic backend is always considered "initialized"
        // since it doesn't have the Raft bootstrapping concept
        true
    }
}

/// In-memory deterministic implementation of [`KeyValueStore`] for testing.
///
/// This implementation stores key-value pairs in a HashMap without persistence,
/// making it useful for unit tests, property-based testing, and simulation testing.
/// Unlike the production Raft-backed implementation, this provides instant operations
/// without network I/O or consensus overhead.
///
/// # OCC Support
///
/// This backend now supports optimistic concurrency control with proper version tracking:
/// - Each key tracks `version` (increments on each write), `create_revision`, and `mod_revision`
/// - OptimisticTransaction validates read_set versions before applying writes
/// - Conflicts return detailed error info (key, expected version, actual version)
///
/// # Limitations
///
/// - No TTL or lease tracking (values stored indefinitely)
/// - No persistence across restarts
/// - Single-node only (no replication)
///
/// # Example
///
/// ```ignore
/// use aspen_testing_core::DeterministicKeyValueStore;
/// use aspen_traits::KeyValueStore;
///
/// let store = DeterministicKeyValueStore::new();
/// store.write(WriteRequest::set("key", "value")).await?;
/// ```
pub struct DeterministicKeyValueStore {
    inner: Arc<Mutex<HashMap<String, VersionedValue>>>,
    /// Global revision counter (simulates Raft log index)
    revision: AtomicU64,
}

impl Clone for DeterministicKeyValueStore {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            revision: AtomicU64::new(self.revision.load(Ordering::SeqCst)),
        }
    }
}

impl Default for DeterministicKeyValueStore {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            revision: AtomicU64::new(0),
        }
    }
}

impl DeterministicKeyValueStore {
    /// Create a new in-memory key-value store.
    ///
    /// The store starts empty. All operations are performed in memory
    /// with no persistence or replication.
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Get the next revision number (simulates Raft log index).
    fn next_revision(&self) -> u64 {
        self.revision.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Insert or update a key with proper version tracking.
    fn insert_versioned(inner: &mut HashMap<String, VersionedValue>, key: String, value: String, revision: u64) {
        let existing = inner.get(&key);
        let (new_version, create_rev) = match existing {
            Some(v) => (v.version + 1, v.create_revision),
            None => (1, revision),
        };
        inner.insert(key, VersionedValue {
            value,
            version: new_version,
            create_revision: create_rev,
            mod_revision: revision,
        });
    }

    /// Evaluates all comparison predicates for a Transaction operation.
    ///
    /// Returns true if all comparisons pass, false if any fails.
    fn evaluate_transaction_comparisons(inner: &HashMap<String, VersionedValue>, comparisons: &[TxnCompare]) -> bool {
        for cmp in comparisons {
            let current_versioned = inner.get(&cmp.key);
            let passed = match cmp.target {
                CompareTarget::Value => {
                    let actual = current_versioned.map(|v| v.value.clone()).unwrap_or_default();
                    match cmp.op {
                        CompareOp::Equal => actual == cmp.value,
                        CompareOp::NotEqual => actual != cmp.value,
                        CompareOp::Greater => actual > cmp.value,
                        CompareOp::Less => actual < cmp.value,
                    }
                }
                CompareTarget::Version => {
                    let compare_val: i64 = cmp.value.parse().unwrap_or(0);
                    let actual = current_versioned.map(|v| v.version).unwrap_or(0);
                    match cmp.op {
                        CompareOp::Equal => actual == compare_val,
                        CompareOp::NotEqual => actual != compare_val,
                        CompareOp::Greater => actual > compare_val,
                        CompareOp::Less => actual < compare_val,
                    }
                }
                CompareTarget::CreateRevision => {
                    let compare_val: u64 = cmp.value.parse().unwrap_or(0);
                    let actual = current_versioned.map(|v| v.create_revision).unwrap_or(0);
                    match cmp.op {
                        CompareOp::Equal => actual == compare_val,
                        CompareOp::NotEqual => actual != compare_val,
                        CompareOp::Greater => actual > compare_val,
                        CompareOp::Less => actual < compare_val,
                    }
                }
                CompareTarget::ModRevision => {
                    let compare_val: u64 = cmp.value.parse().unwrap_or(0);
                    let actual = current_versioned.map(|v| v.mod_revision).unwrap_or(0);
                    match cmp.op {
                        CompareOp::Equal => actual == compare_val,
                        CompareOp::NotEqual => actual != compare_val,
                        CompareOp::Greater => actual > compare_val,
                        CompareOp::Less => actual < compare_val,
                    }
                }
            };
            if !passed {
                return false;
            }
        }
        true
    }

    /// Executes a list of transaction operations on the store.
    ///
    /// Returns a vector of results, one per operation.
    fn execute_transaction_operations(
        inner: &mut HashMap<String, VersionedValue>,
        operations: &[TxnOp],
        revision: u64,
    ) -> Vec<TxnOpResult> {
        let mut results = Vec::new();
        for op in operations {
            match op {
                TxnOp::Put { key, value } => {
                    Self::insert_versioned(inner, key.clone(), value.clone(), revision);
                    results.push(TxnOpResult::Put { revision });
                }
                TxnOp::Delete { key } => {
                    let deleted = if inner.remove(key).is_some() { 1 } else { 0 };
                    results.push(TxnOpResult::Delete { deleted });
                }
                TxnOp::Get { key } => {
                    let kv = inner.get(key).map(|v| KeyValueWithRevision {
                        key: key.clone(),
                        value: v.value.clone(),
                        version: v.version as u64,
                        create_revision: v.create_revision,
                        mod_revision: v.mod_revision,
                    });
                    results.push(TxnOpResult::Get { kv });
                }
                TxnOp::Range { prefix, limit } => {
                    let matching: Vec<_> = inner.iter().filter(|(k, _)| k.starts_with(prefix)).collect();
                    let more = matching.len() > *limit as usize;
                    let kvs: Vec<_> = matching
                        .into_iter()
                        .take(*limit as usize)
                        .map(|(k, v)| KeyValueWithRevision {
                            key: k.clone(),
                            value: v.value.clone(),
                            version: v.version as u64,
                            create_revision: v.create_revision,
                            mod_revision: v.mod_revision,
                        })
                        .collect();
                    results.push(TxnOpResult::Range { kvs, more });
                }
            }
        }
        results
    }

    /// Handles a ConditionalBatch operation.
    ///
    /// Checks all conditions first, then applies operations if all pass.
    fn handle_conditional_batch(
        inner: &mut HashMap<String, VersionedValue>,
        conditions: &[BatchCondition],
        operations: &[BatchOperation],
        revision: u64,
    ) -> WriteResult {
        // Check all conditions first
        let mut failed_index = None;
        for (i, cond) in conditions.iter().enumerate() {
            let met = match cond {
                BatchCondition::ValueEquals { key, expected } => {
                    inner.get(key).map(|v| &v.value == expected).unwrap_or(false)
                }
                BatchCondition::KeyExists { key } => inner.contains_key(key),
                BatchCondition::KeyNotExists { key } => !inner.contains_key(key),
            };
            if !met {
                failed_index = Some(i as u32);
                break;
            }
        }

        if failed_index.is_none() {
            // All conditions passed - apply operations
            for op in operations {
                match op {
                    BatchOperation::Set { key, value } => {
                        Self::insert_versioned(inner, key.clone(), value.clone(), revision);
                    }
                    BatchOperation::Delete { key } => {
                        inner.remove(key);
                    }
                }
            }
            WriteResult {
                batch_applied: Some(operations.len() as u32),
                conditions_met: Some(true),
                header_revision: Some(revision),
                ..Default::default()
            }
        } else {
            WriteResult {
                conditions_met: Some(false),
                failed_condition_index: failed_index,
                ..Default::default()
            }
        }
    }

    /// Handles an OptimisticTransaction operation.
    ///
    /// Validates read set versions, then applies write set if all match.
    fn handle_optimistic_transaction(
        inner: &mut HashMap<String, VersionedValue>,
        read_set: &[(String, i64)],
        write_set: &[WriteOp],
        revision: u64,
    ) -> WriteResult {
        // Phase 1: Validate read set versions
        for (key, expected_version) in read_set {
            let current_version = inner.get(key).map(|v| v.version).unwrap_or(0);
            if current_version != *expected_version {
                // Conflict detected
                return WriteResult {
                    occ_conflict: Some(true),
                    conflict_key: Some(key.clone()),
                    conflict_expected_version: Some(*expected_version),
                    conflict_actual_version: Some(current_version),
                    ..Default::default()
                };
            }
        }

        // Phase 2: Apply write set
        for op in write_set {
            match op {
                WriteOp::Set { key, value } => {
                    Self::insert_versioned(inner, key.clone(), value.clone(), revision);
                }
                WriteOp::Delete { key } => {
                    inner.remove(key);
                }
            }
        }

        WriteResult {
            occ_conflict: Some(false),
            batch_applied: Some(write_set.len() as u32),
            header_revision: Some(revision),
            ..Default::default()
        }
    }

    fn write_handle_set(
        inner: &mut HashMap<String, VersionedValue>,
        key: String,
        value: String,
        revision: u64,
    ) -> Result<WriteResult, KeyValueStoreError> {
        Self::insert_versioned(inner, key.clone(), value.clone(), revision);
        Ok(WriteResult {
            command: Some(WriteCommand::Set { key, value }),
            header_revision: Some(revision),
            ..Default::default()
        })
    }

    fn write_handle_set_with_ttl(
        inner: &mut HashMap<String, VersionedValue>,
        key: String,
        value: String,
        revision: u64,
    ) -> Result<WriteResult, KeyValueStoreError> {
        Self::insert_versioned(inner, key.clone(), value.clone(), revision);
        Ok(WriteResult {
            command: Some(WriteCommand::Set { key, value }),
            header_revision: Some(revision),
            ..Default::default()
        })
    }

    fn write_handle_set_multi(
        inner: &mut HashMap<String, VersionedValue>,
        pairs: &[(String, String)],
        revision: u64,
        command: WriteCommand,
    ) -> Result<WriteResult, KeyValueStoreError> {
        for (key, value) in pairs {
            Self::insert_versioned(inner, key.clone(), value.clone(), revision);
        }
        Ok(WriteResult {
            command: Some(command),
            header_revision: Some(revision),
            ..Default::default()
        })
    }

    fn write_handle_delete(
        inner: &mut HashMap<String, VersionedValue>,
        key: &str,
        command: WriteCommand,
    ) -> Result<WriteResult, KeyValueStoreError> {
        inner.remove(key);
        Ok(WriteResult {
            command: Some(command),
            ..Default::default()
        })
    }

    fn write_handle_delete_multi(
        inner: &mut HashMap<String, VersionedValue>,
        keys: &[String],
        command: WriteCommand,
    ) -> Result<WriteResult, KeyValueStoreError> {
        for key in keys {
            inner.remove(key);
        }
        Ok(WriteResult {
            command: Some(command),
            ..Default::default()
        })
    }

    fn write_handle_compare_and_swap(
        inner: &mut HashMap<String, VersionedValue>,
        key: String,
        expected: Option<String>,
        new_value: String,
        revision: u64,
    ) -> Result<WriteResult, KeyValueStoreError> {
        let current = inner.get(&key).map(|v| v.value.clone());
        let condition_matches = match (&expected, &current) {
            (None, None) => true,
            (Some(exp), Some(cur)) => exp == cur,
            _ => false,
        };
        if condition_matches {
            Self::insert_versioned(inner, key.clone(), new_value.clone(), revision);
            Ok(WriteResult {
                command: Some(WriteCommand::CompareAndSwap {
                    key,
                    expected,
                    new_value,
                }),
                header_revision: Some(revision),
                ..Default::default()
            })
        } else {
            Err(KeyValueStoreError::CompareAndSwapFailed {
                key,
                expected,
                actual: current,
            })
        }
    }

    fn write_handle_compare_and_delete(
        inner: &mut HashMap<String, VersionedValue>,
        key: String,
        expected: String,
        revision: u64,
    ) -> Result<WriteResult, KeyValueStoreError> {
        let current = inner.get(&key).map(|v| v.value.clone());
        let condition_matches = matches!(&current, Some(cur) if cur == &expected);
        if condition_matches {
            inner.remove(&key);
            Ok(WriteResult {
                command: Some(WriteCommand::CompareAndDelete { key, expected }),
                header_revision: Some(revision),
                ..Default::default()
            })
        } else {
            Err(KeyValueStoreError::CompareAndSwapFailed {
                key,
                expected: Some(expected),
                actual: current,
            })
        }
    }

    fn write_handle_batch(
        inner: &mut HashMap<String, VersionedValue>,
        operations: &[BatchOperation],
        revision: u64,
    ) -> Result<WriteResult, KeyValueStoreError> {
        for op in operations {
            match op {
                BatchOperation::Set { key, value } => {
                    Self::insert_versioned(inner, key.clone(), value.clone(), revision);
                }
                BatchOperation::Delete { key } => {
                    inner.remove(key);
                }
            }
        }
        Ok(WriteResult {
            batch_applied: Some(operations.len() as u32),
            header_revision: Some(revision),
            ..Default::default()
        })
    }

    fn write_handle_set_with_lease(
        inner: &mut HashMap<String, VersionedValue>,
        key: String,
        value: String,
        lease_id: u64,
        revision: u64,
    ) -> Result<WriteResult, KeyValueStoreError> {
        Self::insert_versioned(inner, key.clone(), value.clone(), revision);
        Ok(WriteResult {
            command: Some(WriteCommand::SetWithLease { key, value, lease_id }),
            header_revision: Some(revision),
            ..Default::default()
        })
    }

    fn write_handle_set_multi_with_lease(
        inner: &mut HashMap<String, VersionedValue>,
        pairs: &[(String, String)],
        lease_id: u64,
        revision: u64,
    ) -> Result<WriteResult, KeyValueStoreError> {
        for (key, value) in pairs {
            Self::insert_versioned(inner, key.clone(), value.clone(), revision);
        }
        Ok(WriteResult {
            command: Some(WriteCommand::SetMultiWithLease {
                pairs: pairs.to_vec(),
                lease_id,
            }),
            header_revision: Some(revision),
            ..Default::default()
        })
    }

    fn write_handle_lease_grant(lease_id: u64, ttl_seconds: u32) -> Result<WriteResult, KeyValueStoreError> {
        Ok(WriteResult {
            lease_id: Some(lease_id),
            ttl_seconds: Some(ttl_seconds),
            ..Default::default()
        })
    }

    fn write_handle_lease_revoke(lease_id: u64) -> Result<WriteResult, KeyValueStoreError> {
        Ok(WriteResult {
            lease_id: Some(lease_id),
            keys_deleted: Some(0),
            ..Default::default()
        })
    }

    fn write_handle_lease_keepalive(lease_id: u64) -> Result<WriteResult, KeyValueStoreError> {
        Ok(WriteResult {
            lease_id: Some(lease_id),
            ttl_seconds: Some(60),
            ..Default::default()
        })
    }

    fn write_handle_transaction(
        inner: &mut HashMap<String, VersionedValue>,
        compare: &[TxnCompare],
        success: &[TxnOp],
        failure: &[TxnOp],
        revision: u64,
    ) -> Result<WriteResult, KeyValueStoreError> {
        let all_passed = Self::evaluate_transaction_comparisons(inner, compare);
        let ops = if all_passed { success } else { failure };
        let results = Self::execute_transaction_operations(inner, ops, revision);

        Ok(WriteResult {
            succeeded: Some(all_passed),
            txn_results: Some(results),
            header_revision: Some(revision),
            ..Default::default()
        })
    }
}

#[async_trait]
impl KeyValueStore for DeterministicKeyValueStore {
    async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError> {
        validate_write_command(&request.command)?;

        let revision = self.next_revision();
        let mut inner = self.inner.lock().await;
        match request.command.clone() {
            WriteCommand::Set { key, value } => Self::write_handle_set(&mut inner, key, value, revision),
            WriteCommand::SetWithTTL { key, value, .. } => {
                Self::write_handle_set_with_ttl(&mut inner, key, value, revision)
            }
            WriteCommand::SetMulti { ref pairs } => {
                Self::write_handle_set_multi(&mut inner, pairs, revision, request.command.clone())
            }
            WriteCommand::SetMultiWithTTL { ref pairs, .. } => {
                Self::write_handle_set_multi(&mut inner, pairs, revision, WriteCommand::SetMulti {
                    pairs: pairs.clone(),
                })
            }
            WriteCommand::Delete { ref key } => Self::write_handle_delete(&mut inner, key, request.command.clone()),
            WriteCommand::DeleteMulti { ref keys } => {
                Self::write_handle_delete_multi(&mut inner, keys, request.command.clone())
            }
            WriteCommand::CompareAndSwap {
                key,
                expected,
                new_value,
            } => Self::write_handle_compare_and_swap(&mut inner, key, expected, new_value, revision),
            WriteCommand::CompareAndDelete { key, expected } => {
                Self::write_handle_compare_and_delete(&mut inner, key, expected, revision)
            }
            WriteCommand::Batch { ref operations } => Self::write_handle_batch(&mut inner, operations, revision),
            WriteCommand::ConditionalBatch {
                ref conditions,
                ref operations,
            } => Ok(Self::handle_conditional_batch(&mut inner, conditions, operations, revision)),
            WriteCommand::SetWithLease { key, value, lease_id } => {
                Self::write_handle_set_with_lease(&mut inner, key, value, lease_id, revision)
            }
            WriteCommand::SetMultiWithLease { ref pairs, lease_id } => {
                Self::write_handle_set_multi_with_lease(&mut inner, pairs, lease_id, revision)
            }
            WriteCommand::LeaseGrant { lease_id, ttl_seconds } => Self::write_handle_lease_grant(lease_id, ttl_seconds),
            WriteCommand::LeaseRevoke { lease_id } => Self::write_handle_lease_revoke(lease_id),
            WriteCommand::LeaseKeepalive { lease_id } => Self::write_handle_lease_keepalive(lease_id),
            WriteCommand::Transaction {
                compare,
                success,
                failure,
            } => Self::write_handle_transaction(&mut inner, &compare, &success, &failure, revision),
            WriteCommand::OptimisticTransaction { read_set, write_set } => {
                Ok(Self::handle_optimistic_transaction(&mut inner, &read_set, &write_set, revision))
            }
            WriteCommand::ShardSplit { .. } | WriteCommand::ShardMerge { .. } | WriteCommand::TopologyUpdate { .. } => {
                Err(KeyValueStoreError::Failed {
                    reason: "shard topology operations not supported in in-memory store".to_string(),
                })
            }
        }
    }

    async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError> {
        let guard = self.inner.lock().await;
        match guard.get(&request.key) {
            Some(versioned) => Ok(ReadResult {
                kv: Some(KeyValueWithRevision {
                    key: request.key,
                    value: versioned.value.clone(),
                    version: versioned.version as u64,
                    create_revision: versioned.create_revision,
                    mod_revision: versioned.mod_revision,
                }),
            }),
            None => Err(KeyValueStoreError::NotFound { key: request.key }),
        }
    }

    async fn delete(&self, request: DeleteRequest) -> Result<DeleteResult, KeyValueStoreError> {
        let mut inner = self.inner.lock().await;
        let is_deleted = inner.remove(&request.key).is_some();
        Ok(DeleteResult {
            key: request.key,
            is_deleted,
        })
    }

    async fn scan(&self, request: ScanRequest) -> Result<ScanResult, KeyValueStoreError> {
        let inner = self.inner.lock().await;

        // Apply Tiger Style bounded limit
        let limit = request.limit_results.unwrap_or(DEFAULT_SCAN_LIMIT).min(MAX_SCAN_RESULTS) as usize;

        // Decode continuation token (format: base64(last_key))
        let start_after = request.continuation_token.as_ref().and_then(|token| {
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, token)
                .ok()
                .and_then(|bytes| String::from_utf8(bytes).ok())
        });

        // Collect matching keys and sort them
        let mut matching: Vec<_> = inner
            .iter()
            .filter(|(k, _)| k.starts_with(&request.prefix))
            .filter(|(k, _)| {
                // Skip keys <= continuation token
                match &start_after {
                    Some(after) => k.as_str() > after.as_str(),
                    None => true,
                }
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // Sort by key for consistent ordering
        matching.sort_by(|a, b| a.0.cmp(&b.0));

        // Take limit + 1 to check if there are more results
        let is_truncated = matching.len() > limit;
        let entries: Vec<KeyValueWithRevision> = matching
            .into_iter()
            .take(limit)
            .map(|(key, versioned)| KeyValueWithRevision {
                key,
                value: versioned.value,
                version: versioned.version as u64,
                create_revision: versioned.create_revision,
                mod_revision: versioned.mod_revision,
            })
            .collect();

        // Generate continuation token if truncated
        let continuation_token = if is_truncated {
            entries.last().map(|e| base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &e.key))
        } else {
            None
        };

        let result_count = entries.len() as u32;

        Ok(ScanResult {
            entries,
            result_count,
            is_truncated,
            continuation_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use aspen_core::ClusterNode;

    use super::*;

    // ============================================================================
    // DeterministicClusterController tests
    // ============================================================================

    #[tokio::test]
    async fn cluster_controller_new() {
        let controller = DeterministicClusterController::new();
        assert!(controller.is_initialized()); // Always true for deterministic
    }

    #[tokio::test]
    async fn cluster_controller_init_success() {
        let controller = DeterministicClusterController::new();
        let result = controller
            .init(InitRequest {
                initial_members: vec![ClusterNode::new(1, "node1", None)],
            })
            .await;

        assert!(result.is_ok());
        let state = result.unwrap();
        assert_eq!(state.nodes.len(), 1);
        assert_eq!(state.members, vec![1]);
    }

    #[tokio::test]
    async fn cluster_controller_init_empty_members_fails() {
        let controller = DeterministicClusterController::new();
        let result = controller
            .init(InitRequest {
                initial_members: vec![],
            })
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ControlPlaneError::InvalidRequest { reason } => {
                assert!(reason.contains("must not be empty"));
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[tokio::test]
    async fn cluster_controller_add_learner() {
        let controller = DeterministicClusterController::new();
        controller
            .init(InitRequest {
                initial_members: vec![ClusterNode::new(1, "node1", None)],
            })
            .await
            .unwrap();

        let result = controller
            .add_learner(AddLearnerRequest {
                learner: ClusterNode::new(2, "learner", None),
            })
            .await;

        assert!(result.is_ok());
        let state = result.unwrap();
        assert_eq!(state.learners.len(), 1);
        assert_eq!(state.learners[0].id, 2);
    }

    #[tokio::test]
    async fn cluster_controller_change_membership() {
        let controller = DeterministicClusterController::new();
        controller
            .init(InitRequest {
                initial_members: vec![ClusterNode::new(1, "node1", None)],
            })
            .await
            .unwrap();

        let result = controller.change_membership(ChangeMembershipRequest { members: vec![1, 2, 3] }).await;

        assert!(result.is_ok());
        let state = result.unwrap();
        assert_eq!(state.members, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn cluster_controller_change_membership_empty_fails() {
        let controller = DeterministicClusterController::new();
        controller
            .init(InitRequest {
                initial_members: vec![ClusterNode::new(1, "node1", None)],
            })
            .await
            .unwrap();

        let result = controller.change_membership(ChangeMembershipRequest { members: vec![] }).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ControlPlaneError::InvalidRequest { reason } => {
                assert!(reason.contains("at least one voter"));
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[tokio::test]
    async fn cluster_controller_current_state() {
        let controller = DeterministicClusterController::new();
        controller
            .init(InitRequest {
                initial_members: vec![ClusterNode::new(1, "node1", None)],
            })
            .await
            .unwrap();

        let state = controller.current_state().await.unwrap();
        assert_eq!(state.nodes.len(), 1);
        assert_eq!(state.members, vec![1]);
    }

    #[tokio::test]
    async fn cluster_controller_get_metrics_unsupported() {
        let controller = DeterministicClusterController::new();
        let result = controller.get_metrics().await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ControlPlaneError::Unsupported { backend, operation } => {
                assert_eq!(backend, "deterministic");
                assert_eq!(operation, "get_metrics");
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[tokio::test]
    async fn cluster_controller_trigger_snapshot_unsupported() {
        let controller = DeterministicClusterController::new();
        let result = controller.trigger_snapshot().await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ControlPlaneError::Unsupported { backend, operation } => {
                assert_eq!(backend, "deterministic");
                assert_eq!(operation, "trigger_snapshot");
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[tokio::test]
    async fn cluster_controller_get_leader() {
        let controller = DeterministicClusterController::new();
        let leader = controller.get_leader().await.unwrap();
        assert!(leader.is_none()); // Deterministic backend has no leader
    }

    // ============================================================================
    // DeterministicKeyValueStore basic operations
    // ============================================================================

    #[tokio::test]
    async fn kv_store_new() {
        let store = DeterministicKeyValueStore::new();
        // Store should be empty initially
        let result = store
            .scan(ScanRequest {
                prefix: "".to_string(),
                limit_results: None,
                continuation_token: None,
            })
            .await
            .unwrap();
        assert_eq!(result.result_count, 0);
    }

    #[tokio::test]
    async fn kv_store_set_and_read() {
        let store = DeterministicKeyValueStore::new();

        // Write
        store.write(WriteRequest::set("key1", "value1")).await.unwrap();

        // Read
        let result = store.read(ReadRequest::new("key1")).await.unwrap();
        assert!(result.kv.is_some());
        let kv = result.kv.unwrap();
        assert_eq!(kv.key, "key1");
        assert_eq!(kv.value, "value1");
        assert_eq!(kv.version, 1);
    }

    #[tokio::test]
    async fn kv_store_read_not_found() {
        let store = DeterministicKeyValueStore::new();
        let result = store.read(ReadRequest::new("nonexistent")).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            KeyValueStoreError::NotFound { key } => {
                assert_eq!(key, "nonexistent");
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[tokio::test]
    async fn kv_store_update_increments_version() {
        let store = DeterministicKeyValueStore::new();

        store.write(WriteRequest::set("key", "v1")).await.unwrap();
        let kv1 = store.read(ReadRequest::new("key")).await.unwrap().kv.unwrap();
        assert_eq!(kv1.version, 1);

        store.write(WriteRequest::set("key", "v2")).await.unwrap();
        let kv2 = store.read(ReadRequest::new("key")).await.unwrap().kv.unwrap();
        assert_eq!(kv2.version, 2);
        assert_eq!(kv2.value, "v2");

        // create_revision should stay the same
        assert_eq!(kv1.create_revision, kv2.create_revision);
        // mod_revision should change
        assert!(kv2.mod_revision > kv1.mod_revision);
    }

    #[tokio::test]
    async fn kv_store_delete() {
        let store = DeterministicKeyValueStore::new();

        store.write(WriteRequest::set("key", "value")).await.unwrap();
        let result = store.delete(DeleteRequest::new("key")).await.unwrap();
        assert!(result.is_deleted);

        // Key should be gone
        let read_result = store.read(ReadRequest::new("key")).await;
        assert!(read_result.is_err());
    }

    #[tokio::test]
    async fn kv_store_delete_nonexistent() {
        let store = DeterministicKeyValueStore::new();
        let result = store.delete(DeleteRequest::new("nonexistent")).await.unwrap();
        assert!(!result.is_deleted);
    }

    #[tokio::test]
    async fn kv_store_compare_and_swap_success() {
        let store = DeterministicKeyValueStore::new();

        store.write(WriteRequest::set("counter", "10")).await.unwrap();

        let result = store.write(WriteRequest::compare_and_swap("counter", Some("10".to_string()), "11")).await;

        assert!(result.is_ok());
        let kv = store.read(ReadRequest::new("counter")).await.unwrap().kv.unwrap();
        assert_eq!(kv.value, "11");
    }

    #[tokio::test]
    async fn kv_store_compare_and_swap_failure() {
        let store = DeterministicKeyValueStore::new();

        store.write(WriteRequest::set("counter", "10")).await.unwrap();

        let result = store.write(WriteRequest::compare_and_swap("counter", Some("5".to_string()), "11")).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            KeyValueStoreError::CompareAndSwapFailed { key, expected, actual } => {
                assert_eq!(key, "counter");
                assert_eq!(expected, Some("5".to_string()));
                assert_eq!(actual, Some("10".to_string()));
            }
            other => panic!("unexpected error: {:?}", other),
        }

        // Value should be unchanged
        let kv = store.read(ReadRequest::new("counter")).await.unwrap().kv.unwrap();
        assert_eq!(kv.value, "10");
    }

    #[tokio::test]
    async fn kv_store_scan_with_prefix() {
        let store = DeterministicKeyValueStore::new();

        store.write(WriteRequest::set("users/1", "alice")).await.unwrap();
        store.write(WriteRequest::set("users/2", "bob")).await.unwrap();
        store.write(WriteRequest::set("orders/1", "order1")).await.unwrap();

        let result = store
            .scan(ScanRequest {
                prefix: "users/".to_string(),
                limit_results: None,
                continuation_token: None,
            })
            .await
            .unwrap();

        assert_eq!(result.result_count, 2);
        assert!(!result.is_truncated);
    }

    #[tokio::test]
    async fn kv_store_clone_shares_state() {
        let store1 = DeterministicKeyValueStore::new();
        let store2 = store1.clone();

        store1.write(WriteRequest::set("shared", "value")).await.unwrap();

        // store2 should see the write
        let result = store2.read(ReadRequest::new("shared")).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().kv.unwrap().value, "value");
    }
}
