//! In-memory implementations of ClusterController and KeyValueStore for testing.
//!
//! Provides deterministic, non-persistent implementations of the core API traits
//! for use in unit tests and deterministic simulation testing. These implementations
//! mirror the behavior of production backends without network or disk I/O.
//!
//! # Test Coverage
//!
//! TODO: Add unit tests for DeterministicClusterController:
//!       - init() with empty members (should error)
//!       - add_learner() idempotency
//!       - change_membership() with invalid node IDs
//!       Coverage: 5.00% function coverage - used as test fixture, not tested directly
//!
//! TODO: Add unit tests for DeterministicKeyValueStore:
//!       - WriteCommand validation boundary testing
//!       - scan() with continuation tokens
//!       - delete() idempotency verification
//!       Coverage: 7.14% line coverage - tested via proptest generators

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use super::{
    AddLearnerRequest, BatchCondition, BatchOperation, ChangeMembershipRequest, ClusterController,
    ClusterState, ControlPlaneError, DEFAULT_SCAN_LIMIT, DeleteRequest, DeleteResult, InitRequest,
    KeyValueStore, KeyValueStoreError, KeyValueWithRevision, MAX_SCAN_RESULTS, ReadRequest,
    ReadResult, ScanRequest, ScanResult, WriteCommand, WriteOp, WriteRequest, WriteResult,
    validate_write_command,
};

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

    async fn add_learner(
        &self,
        request: AddLearnerRequest,
    ) -> Result<ClusterState, ControlPlaneError> {
        let mut guard = self.state.lock().await;
        guard.learners.push(request.learner);
        Ok(guard.clone())
    }

    async fn change_membership(
        &self,
        request: ChangeMembershipRequest,
    ) -> Result<ClusterState, ControlPlaneError> {
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

    async fn get_metrics(
        &self,
    ) -> Result<super::RaftMetrics<crate::raft::types::AppTypeConfig>, ControlPlaneError> {
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
}

/// In-memory deterministic implementation of [`KeyValueStore`] for testing.
///
/// This implementation stores key-value pairs in a HashMap without persistence,
/// making it useful for unit tests, property-based testing, and simulation testing.
/// Unlike the production Raft-backed implementation, this provides instant operations
/// without network I/O or consensus overhead.
///
/// # Limitations
///
/// - No TTL or lease tracking (values stored indefinitely)
/// - No revision metadata (version/create_revision/mod_revision always return defaults)
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
#[derive(Clone, Default)]
pub struct DeterministicKeyValueStore {
    inner: Arc<Mutex<HashMap<String, String>>>,
}

impl DeterministicKeyValueStore {
    /// Create a new in-memory key-value store.
    ///
    /// The store starts empty. All operations are performed in memory
    /// with no persistence or replication.
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
}

#[async_trait]
impl KeyValueStore for DeterministicKeyValueStore {
    async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError> {
        validate_write_command(&request.command)?;

        let mut inner = self.inner.lock().await;
        match request.command.clone() {
            WriteCommand::Set { key, value } => {
                inner.insert(key.clone(), value.clone());
                Ok(WriteResult {
                    command: Some(WriteCommand::Set { key, value }),
                    ..Default::default()
                })
            }
            // TTL operations: in-memory store doesn't track TTL, just stores value
            WriteCommand::SetWithTTL { key, value, .. } => {
                inner.insert(key.clone(), value.clone());
                Ok(WriteResult {
                    command: Some(WriteCommand::Set { key, value }),
                    ..Default::default()
                })
            }
            WriteCommand::SetMulti { ref pairs } => {
                for (key, value) in pairs {
                    inner.insert(key.clone(), value.clone());
                }
                Ok(WriteResult {
                    command: Some(request.command.clone()),
                    ..Default::default()
                })
            }
            WriteCommand::SetMultiWithTTL { ref pairs, .. } => {
                for (key, value) in pairs {
                    inner.insert(key.clone(), value.clone());
                }
                Ok(WriteResult {
                    command: Some(WriteCommand::SetMulti {
                        pairs: pairs.clone(),
                    }),
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
                let current = inner.get(&key).cloned();
                let condition_matches = match (&expected, &current) {
                    (None, None) => true,
                    (Some(exp), Some(cur)) => exp == cur,
                    _ => false,
                };
                if condition_matches {
                    inner.insert(key.clone(), new_value.clone());
                    Ok(WriteResult {
                        command: Some(WriteCommand::CompareAndSwap {
                            key,
                            expected,
                            new_value,
                        }),
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
                let current = inner.get(&key).cloned();
                let condition_matches = matches!(&current, Some(cur) if cur == &expected);
                if condition_matches {
                    inner.remove(&key);
                    Ok(WriteResult {
                        command: Some(WriteCommand::CompareAndDelete { key, expected }),
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
                            inner.insert(key.clone(), value.clone());
                        }
                        BatchOperation::Delete { key } => {
                            inner.remove(key);
                        }
                    }
                }
                Ok(WriteResult {
                    batch_applied: Some(operations.len() as u32),
                    ..Default::default()
                })
            }
            WriteCommand::ConditionalBatch {
                ref conditions,
                ref operations,
            } => {
                // Check all conditions first
                let mut conditions_met = true;
                let mut failed_index = None;
                for (i, cond) in conditions.iter().enumerate() {
                    let met = match cond {
                        BatchCondition::ValueEquals { key, expected } => {
                            inner.get(key).map(|v| v == expected).unwrap_or(false)
                        }
                        BatchCondition::KeyExists { key } => inner.contains_key(key),
                        BatchCondition::KeyNotExists { key } => !inner.contains_key(key),
                    };
                    if !met {
                        conditions_met = false;
                        failed_index = Some(i as u32);
                        break;
                    }
                }

                if conditions_met {
                    for op in operations {
                        match op {
                            BatchOperation::Set { key, value } => {
                                inner.insert(key.clone(), value.clone());
                            }
                            BatchOperation::Delete { key } => {
                                inner.remove(key);
                            }
                        }
                    }
                    Ok(WriteResult {
                        batch_applied: Some(operations.len() as u32),
                        conditions_met: Some(true),
                        ..Default::default()
                    })
                } else {
                    Ok(WriteResult {
                        conditions_met: Some(false),
                        failed_condition_index: failed_index,
                        ..Default::default()
                    })
                }
            }
            // Lease operations: in-memory store doesn't track leases, just stores values
            WriteCommand::SetWithLease {
                key,
                value,
                lease_id,
            } => {
                inner.insert(key.clone(), value.clone());
                Ok(WriteResult {
                    command: Some(WriteCommand::SetWithLease {
                        key,
                        value,
                        lease_id,
                    }),
                    ..Default::default()
                })
            }
            WriteCommand::SetMultiWithLease {
                ref pairs,
                lease_id,
            } => {
                for (key, value) in pairs {
                    inner.insert(key.clone(), value.clone());
                }
                Ok(WriteResult {
                    command: Some(WriteCommand::SetMultiWithLease {
                        pairs: pairs.clone(),
                        lease_id,
                    }),
                    ..Default::default()
                })
            }
            WriteCommand::LeaseGrant {
                lease_id,
                ttl_seconds,
            } => {
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
                use crate::api::{CompareOp, CompareTarget, TxnOp, TxnOpResult};

                // Evaluate all comparisons
                let mut all_passed = true;
                for cmp in &compare {
                    let current_value = inner.get(&cmp.key).cloned();
                    let passed = match cmp.target {
                        CompareTarget::Value => {
                            let actual = current_value.unwrap_or_default();
                            match cmp.op {
                                CompareOp::Equal => actual == cmp.value,
                                CompareOp::NotEqual => actual != cmp.value,
                                CompareOp::Greater => actual > cmp.value,
                                CompareOp::Less => actual < cmp.value,
                            }
                        }
                        // In-memory doesn't track versions/revisions - use defaults
                        CompareTarget::Version
                        | CompareTarget::CreateRevision
                        | CompareTarget::ModRevision => {
                            let compare_val: u64 = cmp.value.parse().unwrap_or(0);
                            let actual = if current_value.is_some() { 1u64 } else { 0 };
                            match cmp.op {
                                CompareOp::Equal => actual == compare_val,
                                CompareOp::NotEqual => actual != compare_val,
                                CompareOp::Greater => actual > compare_val,
                                CompareOp::Less => actual < compare_val,
                            }
                        }
                    };
                    if !passed {
                        all_passed = false;
                        break;
                    }
                }

                // Execute the appropriate branch
                let ops = if all_passed { &success } else { &failure };
                let mut results = Vec::new();

                for op in ops {
                    match op {
                        TxnOp::Put { key, value } => {
                            inner.insert(key.clone(), value.clone());
                            results.push(TxnOpResult::Put { revision: 0 }); // In-memory doesn't track revisions
                        }
                        TxnOp::Delete { key } => {
                            let deleted = if inner.remove(key).is_some() { 1 } else { 0 };
                            results.push(TxnOpResult::Delete { deleted });
                        }
                        TxnOp::Get { key } => {
                            let kv = inner.get(key).map(|v| KeyValueWithRevision {
                                key: key.clone(),
                                value: v.clone(),
                                version: 1,
                                create_revision: 0,
                                mod_revision: 0,
                            });
                            results.push(TxnOpResult::Get { kv });
                        }
                        TxnOp::Range { prefix, limit } => {
                            let matching: Vec<_> = inner
                                .iter()
                                .filter(|(k, _)| k.starts_with(prefix))
                                .collect();
                            let more = matching.len() > *limit as usize;
                            let kvs: Vec<_> = matching
                                .into_iter()
                                .take(*limit as usize)
                                .map(|(k, v)| KeyValueWithRevision {
                                    key: k.clone(),
                                    value: v.clone(),
                                    version: 1,
                                    create_revision: 0,
                                    mod_revision: 0,
                                })
                                .collect();
                            results.push(TxnOpResult::Range { kvs, more });
                        }
                    }
                }

                Ok(WriteResult {
                    succeeded: Some(all_passed),
                    txn_results: Some(results),
                    header_revision: Some(0), // In-memory doesn't track revisions
                    ..Default::default()
                })
            }
            WriteCommand::OptimisticTransaction {
                read_set: _,
                write_set,
            } => {
                // In-memory doesn't track versions, so we can't do proper OCC validation
                // Just apply the writes and return success
                for op in &write_set {
                    match op {
                        WriteOp::Set { key, value } => {
                            inner.insert(key.clone(), value.clone());
                        }
                        WriteOp::Delete { key } => {
                            inner.remove(key);
                        }
                    }
                }
                Ok(WriteResult {
                    occ_conflict: Some(false),
                    batch_applied: Some(write_set.len() as u32),
                    ..Default::default()
                })
            }
        }
    }

    async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError> {
        let guard = self.inner.lock().await;
        match guard.get(&request.key) {
            Some(value) => Ok(ReadResult {
                kv: Some(KeyValueWithRevision {
                    key: request.key,
                    value: value.clone(),
                    version: 1,         // In-memory doesn't track versions
                    create_revision: 0, // In-memory doesn't track revisions
                    mod_revision: 0,
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
        let limit = request
            .limit
            .unwrap_or(DEFAULT_SCAN_LIMIT)
            .min(MAX_SCAN_RESULTS) as usize;

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
            .map(|(key, value)| KeyValueWithRevision {
                key,
                value,
                version: 1,         // In-memory doesn't track versions
                create_revision: 0, // In-memory doesn't track revisions
                mod_revision: 0,
            })
            .collect();

        // Generate continuation token if truncated
        let continuation_token = if is_truncated {
            entries
                .last()
                .map(|e| base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &e.key))
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
