//! KeyValueStore trait implementation for RaftNode.

use aspen_constants::api::DEFAULT_SCAN_LIMIT;
use aspen_constants::api::MAX_SCAN_RESULTS;
use aspen_core::validate_write_command;
use aspen_kv_types::DeleteRequest;
use aspen_kv_types::DeleteResult;
use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::KeyValueWithRevision;
use aspen_kv_types::ReadConsistency;
use aspen_kv_types::ReadRequest;
use aspen_kv_types::ReadResult;
use aspen_kv_types::ScanRequest;
use aspen_kv_types::ScanResult;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_kv_types::WriteResult;
use aspen_raft_types::READ_INDEX_TIMEOUT;
use aspen_traits::KeyValueStore;
use async_trait::async_trait;
use openraft::ReadPolicy;
use openraft::error::ClientWriteError;
use openraft::error::RaftError;
use tracing::instrument;

use super::RaftNode;
use crate::StateMachineVariant;
use crate::types::AppRequest;
use crate::types::AppResponse;
use crate::types::AppTypeConfig;
use crate::write_batcher::command_conversion::write_command_to_app_request;

#[async_trait]
impl KeyValueStore for RaftNode {
    #[instrument(skip(self))]
    async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError> {
        let _permit = self.semaphore().acquire().await.map_err(|_| KeyValueStoreError::Failed {
            reason: "semaphore closed".into(),
        })?;

        self.ensure_initialized_kv()?;

        validate_write_command(&request.command)?;

        // Route simple Set/Delete through batcher if enabled
        if let Some(batcher) = self.write_batcher() {
            match &request.command {
                WriteCommand::Set { .. } | WriteCommand::Delete { .. } => {
                    return batcher.write_shared(request.command).await;
                }
                // Complex operations bypass batcher for correctness
                _ => {}
            }
        }

        // Convert WriteCommand to AppRequest and apply through Raft consensus
        let app_request = write_command_to_app_request(&request.command);
        let result = self.raft().client_write(app_request).await;

        match result {
            Ok(resp) => build_write_result(request.command, resp.data),
            Err(err) => Err(map_raft_write_error(err)),
        }
    }

    #[instrument(skip(self))]
    async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError> {
        let _permit = self.semaphore().acquire().await.map_err(|_| KeyValueStoreError::Failed {
            reason: "semaphore closed".into(),
        })?;

        self.ensure_initialized_kv()?;

        // Apply consistency level based on request
        match request.consistency {
            ReadConsistency::Linearizable => {
                // ReadIndex: Strongest consistency via quorum confirmation
                //
                // ReadIndex works by:
                // 1. Leader records its current commit index
                // 2. Leader confirms it's still leader via heartbeat quorum
                // 3. await_ready() waits for our state machine to catch up to that commit index
                //
                // This guarantees linearizability because any read after await_ready()
                // sees all writes committed before get_read_linearizer() was called.
                //
                // Tiger Style: Timeout the linearizer acquisition itself, not just await_ready().
                // The get_read_linearizer() call spawns a background task that waits for heartbeat
                // quorum confirmation. If followers are unreachable, this can hang indefinitely.
                let linearizer =
                    tokio::time::timeout(READ_INDEX_TIMEOUT, self.raft().get_read_linearizer(ReadPolicy::ReadIndex))
                        .await
                        .map_err(|_| KeyValueStoreError::Timeout {
                            duration_ms: READ_INDEX_TIMEOUT.as_millis() as u64,
                        })?
                        .map_err(|err| {
                            let leader_hint = self.raft().metrics().borrow().current_leader.map(|id| id.0);
                            KeyValueStoreError::NotLeader {
                                leader: leader_hint,
                                reason: err.to_string(),
                            }
                        })?;

                // Tiger Style: Explicit timeout prevents indefinite hang during network partition
                tokio::time::timeout(READ_INDEX_TIMEOUT, linearizer.await_ready(self.raft()))
                    .await
                    .map_err(|_| KeyValueStoreError::Timeout {
                        duration_ms: READ_INDEX_TIMEOUT.as_millis() as u64,
                    })?
                    .map_err(|err| {
                        let leader_hint = self.raft().metrics().borrow().current_leader.map(|id| id.0);
                        KeyValueStoreError::NotLeader {
                            leader: leader_hint,
                            reason: err.to_string(),
                        }
                    })?;
            }
            ReadConsistency::Lease => {
                // LeaseRead: Lower latency via leader lease (no quorum confirmation)
                //
                // Uses the leader's lease to serve reads without contacting followers.
                // Safe as long as clock drift is less than the lease duration.
                // Falls back to ReadIndex if the lease has expired.
                //
                // Tiger Style: Timeout for consistency with ReadIndex path, though
                // LeaseRead should return quickly (either from cache or ForwardToLeader).
                let linearizer =
                    tokio::time::timeout(READ_INDEX_TIMEOUT, self.raft().get_read_linearizer(ReadPolicy::LeaseRead))
                        .await
                        .map_err(|_| KeyValueStoreError::Timeout {
                            duration_ms: READ_INDEX_TIMEOUT.as_millis() as u64,
                        })?
                        .map_err(|err| {
                            let leader_hint = self.raft().metrics().borrow().current_leader.map(|id| id.0);
                            KeyValueStoreError::NotLeader {
                                leader: leader_hint,
                                reason: err.to_string(),
                            }
                        })?;

                // Tiger Style: Explicit timeout prevents indefinite hang during network partition
                tokio::time::timeout(READ_INDEX_TIMEOUT, linearizer.await_ready(self.raft()))
                    .await
                    .map_err(|_| KeyValueStoreError::Timeout {
                        duration_ms: READ_INDEX_TIMEOUT.as_millis() as u64,
                    })?
                    .map_err(|err| {
                        let leader_hint = self.raft().metrics().borrow().current_leader.map(|id| id.0);
                        KeyValueStoreError::NotLeader {
                            leader: leader_hint,
                            reason: err.to_string(),
                        }
                    })?;
            }
            ReadConsistency::Stale => {
                // Stale: Read directly from local state machine without consistency checks
                // WARNING: May return uncommitted or rolled-back data
            }
        }

        // Read directly from state machine (linearizability guaranteed by ReadIndex above)
        match self.state_machine() {
            StateMachineVariant::InMemory(sm) => match sm.get(&request.key).await {
                Some(value) => Ok(ReadResult {
                    kv: Some(KeyValueWithRevision {
                        key: request.key,
                        value,
                        version: 1,         // In-memory doesn't track versions
                        create_revision: 0, // In-memory doesn't track revisions
                        mod_revision: 0,
                    }),
                }),
                None => Err(KeyValueStoreError::NotFound { key: request.key }),
            },
            StateMachineVariant::Redb(sm) => match sm.get(&request.key) {
                Ok(Some(entry)) => Ok(ReadResult {
                    kv: Some(KeyValueWithRevision {
                        key: request.key,
                        value: entry.value,
                        version: entry.version as u64,
                        create_revision: entry.create_revision as u64,
                        mod_revision: entry.mod_revision as u64,
                    }),
                }),
                Ok(None) => Err(KeyValueStoreError::NotFound { key: request.key }),
                Err(err) => Err(KeyValueStoreError::Failed {
                    reason: err.to_string(),
                }),
            },
        }
    }

    #[instrument(skip(self))]
    async fn delete(&self, request: DeleteRequest) -> Result<DeleteResult, KeyValueStoreError> {
        let _permit = self.semaphore().acquire().await.map_err(|_| KeyValueStoreError::Failed {
            reason: "semaphore closed".into(),
        })?;

        self.ensure_initialized_kv()?;

        // Tiger Style: delete key must not be empty
        assert!(!request.key.is_empty(), "DELETE: request key must not be empty");

        // Apply delete through Raft consensus
        let app_request = AppRequest::Delete {
            key: request.key.clone(),
        };

        let result = self.raft().client_write(app_request).await;

        match result {
            Ok(resp) => {
                let deleted = resp.data.deleted.unwrap_or(false);
                Ok(DeleteResult {
                    key: request.key,
                    deleted,
                })
            }
            Err(err) => Err(KeyValueStoreError::Failed {
                reason: err.to_string(),
            }),
        }
    }

    #[instrument(skip(self))]
    async fn scan(&self, _request: ScanRequest) -> Result<ScanResult, KeyValueStoreError> {
        let _permit = self.semaphore().acquire().await.map_err(|_| KeyValueStoreError::Failed {
            reason: "semaphore closed".into(),
        })?;

        self.ensure_initialized_kv()?;

        // Use ReadIndex for linearizable scan (see read() for protocol details)
        //
        // Tiger Style: Timeout the linearizer acquisition itself, not just await_ready().
        // The get_read_linearizer() call spawns a background task that waits for heartbeat
        // quorum confirmation. If followers are unreachable, this can hang indefinitely.
        let linearizer =
            tokio::time::timeout(READ_INDEX_TIMEOUT, self.raft().get_read_linearizer(ReadPolicy::ReadIndex))
                .await
                .map_err(|_| KeyValueStoreError::Timeout {
                    duration_ms: READ_INDEX_TIMEOUT.as_millis() as u64,
                })?
                .map_err(|err| {
                    let leader_hint = self.raft().metrics().borrow().current_leader.map(|id| id.0);
                    KeyValueStoreError::NotLeader {
                        leader: leader_hint,
                        reason: err.to_string(),
                    }
                })?;

        // Tiger Style: Explicit timeout prevents indefinite hang during network partition
        tokio::time::timeout(READ_INDEX_TIMEOUT, linearizer.await_ready(self.raft()))
            .await
            .map_err(|_| KeyValueStoreError::Timeout {
                duration_ms: READ_INDEX_TIMEOUT.as_millis() as u64,
            })?
            .map_err(|err| {
                let leader_hint = self.raft().metrics().borrow().current_leader.map(|id| id.0);
                KeyValueStoreError::NotLeader {
                    leader: leader_hint,
                    reason: err.to_string(),
                }
            })?;

        // Scan directly from state machine (linearizability guaranteed by ReadIndex above)
        // Apply default limit if not specified
        let limit = _request.limit.unwrap_or(DEFAULT_SCAN_LIMIT).min(MAX_SCAN_RESULTS) as usize;

        // Tiger Style: scan limit must be bounded
        assert!(
            limit <= MAX_SCAN_RESULTS as usize,
            "SCAN: computed limit {} exceeds MAX_SCAN_RESULTS {}",
            limit,
            MAX_SCAN_RESULTS
        );

        match self.state_machine() {
            StateMachineVariant::InMemory(sm) => {
                // Get all KV pairs matching prefix
                let all_pairs = sm.scan_kv_with_prefix_async(&_request.prefix).await;

                // Handle pagination via continuation token
                //
                // Tiger Style: Use >= comparison and skip the exact token key.
                // This handles the edge case where the continuation token key was
                // deleted between paginated calls - we still return all keys after it.
                // Using only > would skip entries if the token key no longer exists.
                let start_key = _request.continuation_token.as_deref();
                let filtered: Vec<_> = all_pairs
                    .into_iter()
                    .filter(|(k, _)| {
                        // Skip keys before or equal to continuation token
                        start_key.is_none_or(|start| k.as_str() > start)
                    })
                    .collect();

                // Take limit+1 to check if there are more results
                let is_truncated = filtered.len() > limit;
                let entries: Vec<KeyValueWithRevision> = filtered
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

                let continuation_token = if is_truncated {
                    entries.last().map(|e| e.key.clone())
                } else {
                    None
                };

                Ok(ScanResult {
                    count: entries.len() as u32,
                    entries,
                    is_truncated,
                    continuation_token,
                })
            }
            StateMachineVariant::Redb(sm) => {
                // Redb scan with pagination
                let start_key = _request.continuation_token.as_deref();
                match sm.scan(&_request.prefix, start_key, Some(limit + 1)) {
                    Ok(entries_full) => {
                        let is_truncated = entries_full.len() > limit;
                        let entries: Vec<KeyValueWithRevision> = entries_full.into_iter().take(limit).collect();

                        let continuation_token = if is_truncated {
                            entries.last().map(|e| e.key.clone())
                        } else {
                            None
                        };

                        Ok(ScanResult {
                            count: entries.len() as u32,
                            entries,
                            is_truncated,
                            continuation_token,
                        })
                    }
                    Err(err) => Err(KeyValueStoreError::Failed {
                        reason: err.to_string(),
                    }),
                }
            }
        }
    }
}

/// Build a `WriteResult` from a successful Raft response, handling CAS failure detection.
///
/// If the response indicates a CAS operation that failed its condition (cas_succeeded == false),
/// returns a `CompareAndSwapFailed` error with the key, expected value, and actual value.
fn build_write_result(command: WriteCommand, resp: AppResponse) -> Result<WriteResult, KeyValueStoreError> {
    // Tiger Style: if batch_applied is set, it must be bounded
    if let Some(count) = resp.batch_applied {
        debug_assert!(
            count <= MAX_SCAN_RESULTS,
            "BUILD_RESULT: batch_applied {} exceeds MAX_SCAN_RESULTS {}",
            count,
            MAX_SCAN_RESULTS
        );
    }
    // Check if this was a CAS operation that failed its condition
    if let Some(false) = resp.cas_succeeded {
        // CAS condition didn't match - extract key and expected from original command
        let (key, expected) = match &command {
            WriteCommand::CompareAndSwap { key, expected, .. } => (key.clone(), expected.clone()),
            WriteCommand::CompareAndDelete { key, expected } => (key.clone(), Some(expected.clone())),
            _ => {
                return Err(KeyValueStoreError::Failed {
                    reason: "unexpected cas_succeeded flag on non-CAS operation".into(),
                });
            }
        };
        return Err(KeyValueStoreError::CompareAndSwapFailed {
            key,
            expected,
            actual: resp.value,
        });
    }

    // Build WriteResult with appropriate fields based on operation type
    Ok(WriteResult {
        command: Some(command),
        batch_applied: resp.batch_applied,
        conditions_met: resp.conditions_met,
        failed_condition_index: resp.failed_condition_index,
        lease_id: resp.lease_id,
        ttl_seconds: resp.ttl_seconds,
        keys_deleted: resp.keys_deleted,
        succeeded: resp.succeeded,
        txn_results: resp.txn_results,
        header_revision: resp.header_revision,
        occ_conflict: resp.occ_conflict,
        conflict_key: resp.conflict_key,
        conflict_expected_version: resp.conflict_expected_version,
        conflict_actual_version: resp.conflict_actual_version,
    })
}

/// Map a Raft client write error to a `KeyValueStoreError`.
///
/// Preserves `ForwardToLeader` as `NotLeader` for proper client-side retry handling.
fn map_raft_write_error(err: RaftError<AppTypeConfig, ClientWriteError<AppTypeConfig>>) -> KeyValueStoreError {
    if let Some(forward) = err.forward_to_leader() {
        return KeyValueStoreError::NotLeader {
            leader: forward.leader_id.map(|id| id.0),
            reason: format!("has to forward request to: {:?}, {:?}", forward.leader_id, forward.leader_node),
        };
    }
    KeyValueStoreError::Failed {
        reason: err.to_string(),
    }
}
