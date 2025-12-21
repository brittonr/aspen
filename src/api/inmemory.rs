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
    AddLearnerRequest, ChangeMembershipRequest, ClusterController, ClusterState, ControlPlaneError,
    DEFAULT_SCAN_LIMIT, DeleteRequest, DeleteResult, InitRequest, KeyValueStore,
    KeyValueStoreError, MAX_SCAN_RESULTS, ReadRequest, ReadResult, ScanEntry, ScanRequest,
    ScanResult, WriteCommand, WriteRequest, WriteResult, validate_write_command,
};

#[derive(Clone, Default)]
pub struct DeterministicClusterController {
    state: Arc<Mutex<ClusterState>>,
}

impl DeterministicClusterController {
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

#[derive(Clone, Default)]
pub struct DeterministicKeyValueStore {
    inner: Arc<Mutex<HashMap<String, String>>>,
}

impl DeterministicKeyValueStore {
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
                    command: WriteCommand::Set { key, value },
                })
            }
            WriteCommand::SetMulti { ref pairs } => {
                for (key, value) in pairs {
                    inner.insert(key.clone(), value.clone());
                }
                Ok(WriteResult {
                    command: request.command.clone(),
                })
            }
            WriteCommand::Delete { ref key } => {
                inner.remove(key);
                Ok(WriteResult {
                    command: request.command.clone(),
                })
            }
            WriteCommand::DeleteMulti { ref keys } => {
                for key in keys {
                    inner.remove(key);
                }
                Ok(WriteResult {
                    command: request.command.clone(),
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
                        command: WriteCommand::CompareAndSwap {
                            key,
                            expected,
                            new_value,
                        },
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
                        command: WriteCommand::CompareAndDelete { key, expected },
                    })
                } else {
                    Err(KeyValueStoreError::CompareAndSwapFailed {
                        key,
                        expected: Some(expected),
                        actual: current,
                    })
                }
            }
        }
    }

    async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError> {
        let guard = self.inner.lock().await;
        match guard.get(&request.key) {
            Some(value) => Ok(ReadResult {
                key: request.key,
                value: value.clone(),
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
        let entries: Vec<ScanEntry> = matching
            .into_iter()
            .take(limit)
            .map(|(key, value)| ScanEntry { key, value })
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
