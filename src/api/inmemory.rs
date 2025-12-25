//! In-memory implementations of ClusterController and KeyValueStore for testing.
//!
//! Provides deterministic, non-persistent implementations of the core API traits
//! for use in unit tests and deterministic simulation testing. These implementations
//! mirror the behavior of production backends without network or disk I/O.
//!
//! # OCC Support
//!
//! The in-memory backend now supports optimistic concurrency control (OCC) with
//! proper version tracking. Each key stores version, create_revision, and mod_revision
//! metadata to enable conflict detection in OptimisticTransaction operations.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use async_trait::async_trait;
use tokio::sync::Mutex;

use super::AddLearnerRequest;
use super::BatchCondition;
use super::BatchOperation;
use super::ChangeMembershipRequest;
use super::ClusterController;
use super::ClusterState;
use super::ControlPlaneError;
use super::DEFAULT_SCAN_LIMIT;
use super::DeleteRequest;
use super::DeleteResult;
use super::InitRequest;
use super::KeyValueStore;
use super::KeyValueStoreError;
use super::KeyValueWithRevision;
use super::MAX_SCAN_RESULTS;
use super::ReadRequest;
use super::ReadResult;
use super::ScanRequest;
use super::ScanResult;
use super::WriteCommand;
use super::WriteOp;
use super::WriteRequest;
use super::WriteResult;
use super::validate_write_command;

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
/// use aspen::api::{ClusterController, DeterministicClusterController, InitRequest, ClusterNode};
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

    async fn get_metrics(&self) -> Result<super::RaftMetrics<crate::raft::types::AppTypeConfig>, ControlPlaneError> {
        // Deterministic backend is in-memory stub without Raft consensus
        Err(ControlPlaneError::Unsupported {
            backend: "deterministic".into(),
            operation: "get_metrics".into(),
        })
    }

    async fn trigger_snapshot(
        &self,
    ) -> Result<Option<openraft::LogId<crate::raft::types::AppTypeConfig>>, ControlPlaneError> {
        // Deterministic backend is in-memory stub without snapshot support
        Err(ControlPlaneError::Unsupported {
            backend: "deterministic".into(),
            operation: "trigger_snapshot".into(),
        })
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
/// use aspen::api::{KeyValueStore, DeterministicKeyValueStore, WriteRequest, WriteCommand};
///
/// let store = DeterministicKeyValueStore::new();
/// store.write(WriteRequest {
///     command: WriteCommand::Set {
///         key: "test".into(),
///         value: "value".into(),
///     },
/// }).await?;
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
        inner.insert(
            key,
            VersionedValue {
                value,
                version: new_version,
                create_revision: create_rev,
                mod_revision: revision,
            },
        );
    }

    /// Evaluates all comparison predicates for a Transaction operation.
    ///
    /// Returns true if all comparisons pass, false if any fails.
    fn evaluate_transaction_comparisons(
        inner: &HashMap<String, VersionedValue>,
        comparisons: &[super::TxnCompare],
    ) -> bool {
        use super::CompareOp;
        use super::CompareTarget;

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
        operations: &[super::TxnOp],
        revision: u64,
    ) -> Vec<super::TxnOpResult> {
        use super::TxnOp;

        let mut results = Vec::new();
        for op in operations {
            match op {
                TxnOp::Put { key, value } => {
                    Self::insert_versioned(inner, key.clone(), value.clone(), revision);
                    results.push(super::TxnOpResult::Put { revision });
                }
                TxnOp::Delete { key } => {
                    let deleted = if inner.remove(key).is_some() { 1 } else { 0 };
                    results.push(super::TxnOpResult::Delete { deleted });
                }
                TxnOp::Get { key } => {
                    let kv = inner.get(key).map(|v| KeyValueWithRevision {
                        key: key.clone(),
                        value: v.value.clone(),
                        version: v.version as u64,
                        create_revision: v.create_revision,
                        mod_revision: v.mod_revision,
                    });
                    results.push(super::TxnOpResult::Get { kv });
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
                    results.push(super::TxnOpResult::Range { kvs, more });
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
}

#[async_trait]
impl KeyValueStore for DeterministicKeyValueStore {
    async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError> {
        validate_write_command(&request.command)?;

        let revision = self.next_revision();
        let mut inner = self.inner.lock().await;
        match request.command.clone() {
            WriteCommand::Set { key, value } => {
                Self::insert_versioned(&mut inner, key.clone(), value.clone(), revision);
                Ok(WriteResult {
                    command: Some(WriteCommand::Set { key, value }),
                    header_revision: Some(revision),
                    ..Default::default()
                })
            }
            // TTL operations: in-memory store doesn't track TTL, just stores value
            WriteCommand::SetWithTTL { key, value, .. } => {
                Self::insert_versioned(&mut inner, key.clone(), value.clone(), revision);
                Ok(WriteResult {
                    command: Some(WriteCommand::Set { key, value }),
                    header_revision: Some(revision),
                    ..Default::default()
                })
            }
            WriteCommand::SetMulti { ref pairs } => {
                for (key, value) in pairs {
                    Self::insert_versioned(&mut inner, key.clone(), value.clone(), revision);
                }
                Ok(WriteResult {
                    command: Some(request.command.clone()),
                    header_revision: Some(revision),
                    ..Default::default()
                })
            }
            WriteCommand::SetMultiWithTTL { ref pairs, .. } => {
                for (key, value) in pairs {
                    Self::insert_versioned(&mut inner, key.clone(), value.clone(), revision);
                }
                Ok(WriteResult {
                    command: Some(WriteCommand::SetMulti { pairs: pairs.clone() }),
                    header_revision: Some(revision),
                    ..Default::default()
                })
            }
            WriteCommand::Delete { ref key } => {
                inner.remove(key);
                Ok(WriteResult {
                    command: Some(request.command.clone()),
                    ..Default::default()
                })
            }
            WriteCommand::DeleteMulti { ref keys } => {
                for key in keys {
                    inner.remove(key);
                }
                Ok(WriteResult {
                    command: Some(request.command.clone()),
                    ..Default::default()
                })
            }
            WriteCommand::CompareAndSwap {
                key,
                expected,
                new_value,
            } => {
                let current = inner.get(&key).map(|v| v.value.clone());
                let condition_matches = match (&expected, &current) {
                    (None, None) => true,
                    (Some(exp), Some(cur)) => exp == cur,
                    _ => false,
                };
                if condition_matches {
                    Self::insert_versioned(&mut inner, key.clone(), new_value.clone(), revision);
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
            WriteCommand::CompareAndDelete { key, expected } => {
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
            WriteCommand::Batch { ref operations } => {
                for op in operations {
                    match op {
                        BatchOperation::Set { key, value } => {
                            Self::insert_versioned(&mut inner, key.clone(), value.clone(), revision);
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
            WriteCommand::ConditionalBatch {
                ref conditions,
                ref operations,
            } => Ok(Self::handle_conditional_batch(&mut inner, conditions, operations, revision)),
            // Lease operations: in-memory store doesn't track leases, just stores values
            WriteCommand::SetWithLease { key, value, lease_id } => {
                Self::insert_versioned(&mut inner, key.clone(), value.clone(), revision);
                Ok(WriteResult {
                    command: Some(WriteCommand::SetWithLease { key, value, lease_id }),
                    header_revision: Some(revision),
                    ..Default::default()
                })
            }
            WriteCommand::SetMultiWithLease { ref pairs, lease_id } => {
                for (key, value) in pairs {
                    Self::insert_versioned(&mut inner, key.clone(), value.clone(), revision);
                }
                Ok(WriteResult {
                    command: Some(WriteCommand::SetMultiWithLease {
                        pairs: pairs.clone(),
                        lease_id,
                    }),
                    header_revision: Some(revision),
                    ..Default::default()
                })
            }
            WriteCommand::LeaseGrant { lease_id, ttl_seconds } => {
                // In-memory doesn't track leases, just return success
                Ok(WriteResult {
                    lease_id: Some(lease_id),
                    ttl_seconds: Some(ttl_seconds),
                    ..Default::default()
                })
            }
            WriteCommand::LeaseRevoke { lease_id } => {
                // In-memory doesn't track leases, just return success
                Ok(WriteResult {
                    lease_id: Some(lease_id),
                    keys_deleted: Some(0),
                    ..Default::default()
                })
            }
            WriteCommand::LeaseKeepalive { lease_id } => {
                // In-memory doesn't track leases, just return success
                Ok(WriteResult {
                    lease_id: Some(lease_id),
                    ttl_seconds: Some(60), // Dummy value
                    ..Default::default()
                })
            }
            WriteCommand::Transaction {
                compare,
                success,
                failure,
            } => {
                // Evaluate comparisons and execute appropriate branch
                let all_passed = Self::evaluate_transaction_comparisons(&inner, &compare);
                let ops = if all_passed { &success } else { &failure };
                let results = Self::execute_transaction_operations(&mut inner, ops, revision);

                Ok(WriteResult {
                    succeeded: Some(all_passed),
                    txn_results: Some(results),
                    header_revision: Some(revision),
                    ..Default::default()
                })
            }
            WriteCommand::OptimisticTransaction { read_set, write_set } => {
                Ok(Self::handle_optimistic_transaction(&mut inner, &read_set, &write_set, revision))
            }
            // Shard topology operations are not supported in the in-memory store
            // These require the Raft-backed state machine for proper coordination
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
        let deleted = inner.remove(&request.key).is_some();
        Ok(DeleteResult {
            key: request.key,
            deleted,
        })
    }

    async fn scan(&self, request: ScanRequest) -> Result<ScanResult, KeyValueStoreError> {
        let inner = self.inner.lock().await;

        // Apply Tiger Style bounded limit
        let limit = request.limit.unwrap_or(DEFAULT_SCAN_LIMIT).min(MAX_SCAN_RESULTS) as usize;

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

        let count = entries.len() as u32;

        Ok(ScanResult {
            entries,
            count,
            is_truncated,
            continuation_token,
        })
    }
}
