//! Test support utilities for aspen-core internal tests.
//!
//! This module provides minimal deterministic implementations of the core traits
//! for use in aspen-core's own unit tests. For external testing, use the
//! `aspen-testing` crate which provides more full-featured implementations.
//!
//! Note: This is a minimal implementation - only what's needed for internal tests.
//! The full `DeterministicKeyValueStore` and `DeterministicClusterController` are
//! in the `aspen-testing` crate.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::cluster::AddLearnerRequest;
use crate::cluster::ChangeMembershipRequest;
use crate::cluster::ClusterState;
use crate::cluster::InitRequest;
use crate::error::ControlPlaneError;
use crate::error::KeyValueStoreError;
use crate::kv::CompareOp;
use crate::kv::CompareTarget;
use crate::kv::DeleteRequest;
use crate::kv::DeleteResult;
use crate::kv::KeyValueWithRevision;
use crate::kv::ReadRequest;
use crate::kv::ReadResult;
use crate::kv::ScanRequest;
use crate::kv::ScanResult;
use crate::kv::TxnOp;
use crate::kv::WriteCommand;
use crate::kv::WriteRequest;
use crate::kv::WriteResult;
use crate::traits::ClusterController;
use crate::traits::KeyValueStore;
use crate::types::ClusterMetrics;
use crate::types::NodeState;
use crate::types::SnapshotLogId;

/// Versioned value for tracking revisions.
#[derive(Clone)]
struct VersionedValue {
    value: String,
    revision: u64,
}

/// A deterministic in-memory key-value store for testing.
///
/// This implementation is thread-safe and supports all `KeyValueStore` operations
/// with predictable behavior for testing.
pub struct DeterministicKeyValueStore {
    /// Internal data storage
    data: RwLock<BTreeMap<String, VersionedValue>>,
    /// Global revision counter
    revision: RwLock<u64>,
}

impl Default for DeterministicKeyValueStore {
    fn default() -> Self {
        Self::new_inner()
    }
}

impl DeterministicKeyValueStore {
    /// Create a new deterministic store wrapped in Arc.
    pub fn new() -> Arc<Self> {
        Arc::new(Self::new_inner())
    }

    fn new_inner() -> Self {
        Self {
            data: RwLock::new(BTreeMap::new()),
            revision: RwLock::new(0),
        }
    }

    async fn next_revision(&self) -> u64 {
        let mut rev = self.revision.write().await;
        *rev += 1;
        *rev
    }
}

#[async_trait]
impl KeyValueStore for DeterministicKeyValueStore {
    async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError> {
        let data = self.data.read().await;
        match data.get(&request.key) {
            Some(versioned) => Ok(ReadResult {
                kv: Some(KeyValueWithRevision {
                    key: request.key,
                    value: versioned.value.clone(),
                    create_revision: versioned.revision,
                    mod_revision: versioned.revision,
                    version: 1,
                }),
            }),
            None => Ok(ReadResult { kv: None }),
        }
    }

    async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError> {
        let revision = self.next_revision().await;
        let mut data = self.data.write().await;

        // Handle different write commands
        match &request.command {
            WriteCommand::Set { key, value } | WriteCommand::SetWithTTL { key, value, .. } => {
                data.insert(key.clone(), VersionedValue {
                    value: value.clone(),
                    revision,
                });
            }
            WriteCommand::SetMulti { pairs } | WriteCommand::SetMultiWithTTL { pairs, .. } => {
                for (key, value) in pairs {
                    data.insert(key.clone(), VersionedValue {
                        value: value.clone(),
                        revision,
                    });
                }
            }
            WriteCommand::Delete { key } => {
                data.remove(key);
            }
            WriteCommand::DeleteMulti { keys } => {
                for key in keys {
                    data.remove(key);
                }
            }
            WriteCommand::CompareAndSwap {
                key,
                expected,
                new_value,
            } => {
                let current = data.get(key).map(|v| v.value.clone());
                if current.as_ref() == expected.as_ref() {
                    data.insert(key.clone(), VersionedValue {
                        value: new_value.clone(),
                        revision,
                    });
                } else {
                    // Return error for failed CAS
                    return Err(KeyValueStoreError::CompareAndSwapFailed {
                        key: key.clone(),
                        expected: expected.clone(),
                        actual: current,
                    });
                }
            }
            WriteCommand::CompareAndDelete { key, expected } => {
                if let Some(v) = data.get(key) {
                    if &v.value == expected {
                        data.remove(key);
                    }
                }
            }
            WriteCommand::Batch { operations } => {
                for op in operations {
                    match op {
                        crate::kv::BatchOperation::Set { key, value } => {
                            data.insert(key.clone(), VersionedValue {
                                value: value.clone(),
                                revision,
                            });
                        }
                        crate::kv::BatchOperation::Delete { key } => {
                            data.remove(key);
                        }
                    }
                }
            }
            WriteCommand::ConditionalBatch { conditions, operations } => {
                // Check all conditions
                let mut conditions_met = true;
                for condition in conditions {
                    match condition {
                        crate::kv::BatchCondition::KeyExists { key } => {
                            if !data.contains_key(key) {
                                conditions_met = false;
                                break;
                            }
                        }
                        crate::kv::BatchCondition::KeyNotExists { key } => {
                            if data.contains_key(key) {
                                conditions_met = false;
                                break;
                            }
                        }
                        crate::kv::BatchCondition::ValueEquals { key, expected } => match data.get(key) {
                            Some(v) if v.value == *expected => {}
                            _ => {
                                conditions_met = false;
                                break;
                            }
                        },
                    }
                }

                if conditions_met {
                    for op in operations {
                        match op {
                            crate::kv::BatchOperation::Set { key, value } => {
                                data.insert(key.clone(), VersionedValue {
                                    value: value.clone(),
                                    revision,
                                });
                            }
                            crate::kv::BatchOperation::Delete { key } => {
                                data.remove(key);
                            }
                        }
                    }
                }

                return Ok(WriteResult {
                    command: None,
                    batch_applied: None,
                    conditions_met: Some(conditions_met),
                    failed_condition_index: None,
                    lease_id: None,
                    ttl_seconds: None,
                    keys_deleted: None,
                    succeeded: Some(conditions_met),
                    txn_results: None,
                    header_revision: Some(revision),
                    occ_conflict: None,
                    conflict_key: None,
                    conflict_expected_version: None,
                    conflict_actual_version: None,
                });
            }
            WriteCommand::Transaction {
                compare,
                success,
                failure,
            } => {
                // Evaluate all compare conditions
                let mut all_met = true;
                for cmp in compare {
                    let actual = data.get(&cmp.key);
                    let met = match cmp.target {
                        CompareTarget::Version => {
                            let actual_version = actual.map(|_| 1u64).unwrap_or(0);
                            let expected: u64 = cmp.value.parse().unwrap_or(0);
                            match cmp.op {
                                CompareOp::Equal => actual_version == expected,
                                CompareOp::NotEqual => actual_version != expected,
                                CompareOp::Greater => actual_version > expected,
                                CompareOp::Less => actual_version < expected,
                            }
                        }
                        CompareTarget::Value => {
                            let actual_value = actual.map(|v| v.value.as_str()).unwrap_or("");
                            let expected = cmp.value.as_str();
                            match cmp.op {
                                CompareOp::Equal => actual_value == expected,
                                CompareOp::NotEqual => actual_value != expected,
                                CompareOp::Greater => actual_value > expected,
                                CompareOp::Less => actual_value < expected,
                            }
                        }
                        CompareTarget::CreateRevision | CompareTarget::ModRevision => {
                            let actual_rev = actual.map(|v| v.revision).unwrap_or(0);
                            let expected: u64 = cmp.value.parse().unwrap_or(0);
                            match cmp.op {
                                CompareOp::Equal => actual_rev == expected,
                                CompareOp::NotEqual => actual_rev != expected,
                                CompareOp::Greater => actual_rev > expected,
                                CompareOp::Less => actual_rev < expected,
                            }
                        }
                    };
                    if !met {
                        all_met = false;
                        break;
                    }
                }

                // Execute success or failure branch
                let ops = if all_met { success } else { failure };
                for op in ops {
                    match op {
                        TxnOp::Put { key, value } => {
                            data.insert(key.clone(), VersionedValue {
                                value: value.clone(),
                                revision,
                            });
                        }
                        TxnOp::Delete { key } => {
                            data.remove(key);
                        }
                        TxnOp::Get { .. } | TxnOp::Range { .. } => {
                            // Read ops in txn - ignored in test store
                        }
                    }
                }

                return Ok(WriteResult {
                    command: None,
                    batch_applied: None,
                    conditions_met: Some(all_met),
                    failed_condition_index: None,
                    lease_id: None,
                    ttl_seconds: None,
                    keys_deleted: None,
                    succeeded: Some(all_met),
                    txn_results: None,
                    header_revision: Some(revision),
                    occ_conflict: None,
                    conflict_key: None,
                    conflict_expected_version: None,
                    conflict_actual_version: None,
                });
            }
            _ => {
                // Other commands - just succeed for now
            }
        }

        Ok(WriteResult {
            command: None,
            batch_applied: None,
            conditions_met: None,
            failed_condition_index: None,
            lease_id: None,
            ttl_seconds: None,
            keys_deleted: None,
            succeeded: Some(true),
            txn_results: None,
            header_revision: Some(revision),
            occ_conflict: None,
            conflict_key: None,
            conflict_expected_version: None,
            conflict_actual_version: None,
        })
    }

    async fn delete(&self, request: DeleteRequest) -> Result<DeleteResult, KeyValueStoreError> {
        let _revision = self.next_revision().await;
        let mut data = self.data.write().await;

        let is_deleted = data.remove(&request.key).is_some();

        Ok(DeleteResult {
            key: request.key,
            is_deleted,
        })
    }

    async fn scan(&self, request: ScanRequest) -> Result<ScanResult, KeyValueStoreError> {
        let data = self.data.read().await;

        let prefix = &request.prefix;
        let limit = request.limit_results.unwrap_or(100) as usize;

        let entries: Vec<_> = data
            .iter()
            .filter(|(k, _)| k.starts_with(prefix))
            .take(limit + 1)
            .map(|(k, v)| KeyValueWithRevision {
                key: k.clone(),
                value: v.value.clone(),
                create_revision: v.revision,
                mod_revision: v.revision,
                version: 1,
            })
            .collect();

        let (entries, is_truncated) = if entries.len() > limit {
            (entries[..limit].to_vec(), true)
        } else {
            (entries.clone(), false)
        };

        Ok(ScanResult {
            result_count: entries.len() as u32,
            entries,
            is_truncated,
            continuation_token: None,
        })
    }
}

/// A deterministic in-memory cluster controller for testing.
pub struct DeterministicClusterController {
    state: RwLock<Option<ClusterState>>,
}

impl Default for DeterministicClusterController {
    fn default() -> Self {
        Self::new_inner()
    }
}

impl DeterministicClusterController {
    /// Create a new deterministic controller wrapped in Arc.
    pub fn new() -> Arc<Self> {
        Arc::new(Self::new_inner())
    }

    fn new_inner() -> Self {
        Self {
            state: RwLock::new(None),
        }
    }
}

#[async_trait]
impl ClusterController for DeterministicClusterController {
    async fn init(&self, request: InitRequest) -> Result<ClusterState, ControlPlaneError> {
        let mut state = self.state.write().await;
        let member_ids: Vec<u64> = request.initial_members.iter().map(|n| n.id).collect();
        let new_state = ClusterState {
            nodes: request.initial_members.clone(),
            members: member_ids,
            learners: vec![],
        };
        *state = Some(new_state.clone());
        Ok(new_state)
    }

    async fn add_learner(&self, request: AddLearnerRequest) -> Result<ClusterState, ControlPlaneError> {
        let mut state = self.state.write().await;
        if let Some(ref mut s) = *state {
            s.learners.push(request.learner.clone());
            s.nodes.push(request.learner);
            Ok(s.clone())
        } else {
            Err(ControlPlaneError::NotInitialized)
        }
    }

    async fn change_membership(&self, request: ChangeMembershipRequest) -> Result<ClusterState, ControlPlaneError> {
        let mut state = self.state.write().await;
        if let Some(ref mut s) = *state {
            s.members = request.members.clone();
            // Promote learners that are now members
            let member_ids: std::collections::HashSet<u64> = s.members.iter().copied().collect();
            s.learners.retain(|node| !member_ids.contains(&node.id));
            Ok(s.clone())
        } else {
            Err(ControlPlaneError::NotInitialized)
        }
    }

    async fn current_state(&self) -> Result<ClusterState, ControlPlaneError> {
        let state = self.state.read().await;
        state.clone().ok_or(ControlPlaneError::NotInitialized)
    }

    async fn get_metrics(&self) -> Result<ClusterMetrics, ControlPlaneError> {
        let state = self.state.read().await;
        match &*state {
            Some(s) => Ok(ClusterMetrics {
                id: 1,
                state: NodeState::Leader,
                current_leader: s.members.first().copied(),
                current_term: 1,
                last_log_index: Some(0),
                last_applied_index: Some(0),
                snapshot_index: None,
                replication: Some(BTreeMap::new()),
                voters: s.members.clone(),
                learners: s.learners.iter().map(|n| n.id).collect(),
            }),
            None => Ok(ClusterMetrics {
                id: 1,
                state: NodeState::Follower,
                current_leader: None,
                current_term: 0,
                last_log_index: None,
                last_applied_index: None,
                snapshot_index: None,
                replication: None,
                voters: vec![],
                learners: vec![],
            }),
        }
    }

    async fn trigger_snapshot(&self) -> Result<Option<SnapshotLogId>, ControlPlaneError> {
        // For testing, just return None (no snapshot triggered)
        Ok(None)
    }

    fn is_initialized(&self) -> bool {
        // Use blocking read since we can't await in a sync fn
        // For a simple test implementation, we can use try_read
        self.state.try_read().map(|guard| guard.is_some()).unwrap_or(false)
    }
}
