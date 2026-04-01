//! KeyValueStore trait implementation for RaftNode.

use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::Duration;

/// Global counter: total ReadIndex retry attempts across all RaftNode instances.
///
/// Uses a static atomic because RaftNode construction is complex and threading
/// an Arc through the builder chain for metrics-only counters is not worth it.
static READ_INDEX_RETRY_COUNT: AtomicU64 = AtomicU64::new(0);

/// Global counter: ReadIndex operations that succeeded after retrying.
static READ_INDEX_RETRY_SUCCESS_COUNT: AtomicU64 = AtomicU64::new(0);

/// Get the current ReadIndex retry count (for metrics collection).
pub fn read_index_retry_count() -> u64 {
    READ_INDEX_RETRY_COUNT.load(Ordering::Relaxed)
}

/// Get the current ReadIndex retry success count (for metrics collection).
pub fn read_index_retry_success_count() -> u64 {
    READ_INDEX_RETRY_SUCCESS_COUNT.load(Ordering::Relaxed)
}

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
use tracing::debug;
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

        // Route simple Set/Delete through batcher if enabled.
        // Only use the batcher when this node is the leader. The batcher's
        // flush path calls raft.client_write() directly without write
        // forwarding — on a follower this always returns ForwardToLeader.
        // Followers skip the batcher and fall through to the forwarding path.
        if let Some(batcher) = self.write_batcher() {
            let is_leader = self.raft().metrics().borrow().current_leader == Some(self.node_id());
            if is_leader {
                match &request.command {
                    WriteCommand::Set { .. } | WriteCommand::Delete { .. } => {
                        return batcher.write_shared(request.command).await;
                    }
                    // Complex operations bypass batcher for correctness
                    _ => {}
                }
            } else {
                // Follower skips batcher — record for observability
                metrics::counter!("aspen.write_batcher.batcher_skipped_total").increment(1);
            }
        }

        // Convert WriteCommand to AppRequest and apply through Raft consensus
        let app_request = write_command_to_app_request(&request.command);
        let result = self.raft().client_write(app_request).await;

        match result {
            Ok(resp) => build_write_result(request.command, resp.data),
            Err(err) => {
                // If we're a follower with a write forwarder, forward to the leader
                // instead of returning NotLeader. This prevents job ack failures
                // and pipeline stalls during leader elections.
                if let Some(forward_info) = err.forward_to_leader()
                    && let Some(forwarder) = self.write_forwarder()
                    && let Some(leader_id) = forward_info.leader_id
                    && leader_id != self.node_id()
                    && let Some(leader_node) = &forward_info.leader_node
                {
                    let leader_addr = leader_node.iroh_addr.clone();
                    debug!(node_id = self.node_id().0, leader_id = leader_id.0, "forwarding write to leader");
                    metrics::counter!("aspen.write_batcher.forwarded_total").increment(1);
                    return forwarder.forward_write(leader_id, leader_addr, request).await;
                }
                Err(map_raft_write_error(err))
            }
        }
    }

    #[instrument(skip(self))]
    async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError> {
        let _permit = self.semaphore().acquire().await.map_err(|_| KeyValueStoreError::Failed {
            reason: "semaphore closed".into(),
        })?;

        self.ensure_initialized_kv()?;

        // Apply consistency level with retry for transient ReadIndex failures on leader
        if let Err(e) = self.read_ensure_consistency_with_retry(request.consistency).await {
            // Forward to leader if we're a follower
            if matches!(&e, KeyValueStoreError::NotLeader { .. }) {
                // Guard: never forward to self (iroh can't self-connect)
                if let Some(forwarder) = self.write_forwarder()
                    && let Some((leader_id, leader_addr)) = self.current_leader_info()
                    && leader_id != self.node_id()
                {
                    debug!(node_id = self.node_id().0, leader_id = leader_id.0, "forwarding read to leader");
                    return forwarder.forward_read(leader_id, leader_addr, request).await;
                }
            }
            return Err(e);
        }

        // Read directly from state machine (linearizability guaranteed by consistency check above)
        self.read_from_state_machine(&request.key).await
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
                    is_deleted: deleted,
                })
            }
            Err(err) => {
                // Forward to leader if this is a ForwardToLeader error
                if let Some(forward_info) = err.forward_to_leader()
                    && let Some(forwarder) = self.write_forwarder()
                    && let Some(leader_id) = forward_info.leader_id
                    && leader_id != self.node_id()
                    && let Some(leader_node) = &forward_info.leader_node
                {
                    let leader_addr = leader_node.iroh_addr.clone();
                    debug!(node_id = self.node_id().0, leader_id = leader_id.0, "forwarding delete to leader");
                    let write_request = WriteRequest {
                        command: WriteCommand::Delete {
                            key: request.key.clone(),
                        },
                    };
                    let _write_result = forwarder.forward_write(leader_id, leader_addr, write_request).await?;
                    return Ok(DeleteResult {
                        key: request.key,
                        is_deleted: true,
                    });
                }
                Err(map_raft_write_error(err))
            }
        }
    }

    #[instrument(skip(self))]
    async fn scan(&self, _request: ScanRequest) -> Result<ScanResult, KeyValueStoreError> {
        let _permit = self.semaphore().acquire().await.map_err(|_| KeyValueStoreError::Failed {
            reason: "semaphore closed".into(),
        })?;

        self.ensure_initialized_kv()?;

        // Ensure linearizable read with retry for transient ReadIndex failures on leader
        if let Err(e) = self.scan_ensure_linearizable_with_retry().await {
            // Forward to leader if we're a follower
            if matches!(&e, KeyValueStoreError::NotLeader { .. }) {
                // Guard: never forward to self (iroh can't self-connect)
                if let Some(forwarder) = self.write_forwarder()
                    && let Some((leader_id, leader_addr)) = self.current_leader_info()
                    && leader_id != self.node_id()
                {
                    debug!(node_id = self.node_id().0, leader_id = leader_id.0, "forwarding scan to leader");
                    return forwarder.forward_scan(leader_id, leader_addr, _request).await;
                }
            }
            return Err(e);
        }

        // Apply default limit if not specified
        let limit = _request.limit_results.unwrap_or(DEFAULT_SCAN_LIMIT).min(MAX_SCAN_RESULTS) as usize;

        // Tiger Style: scan limit must be bounded
        assert!(
            limit <= MAX_SCAN_RESULTS as usize,
            "SCAN: computed limit {} exceeds MAX_SCAN_RESULTS {}",
            limit,
            MAX_SCAN_RESULTS
        );

        // Scan from appropriate state machine backend
        match self.state_machine() {
            StateMachineVariant::InMemory(sm) => self.scan_from_inmemory(sm, &_request, limit).await,
            StateMachineVariant::Redb(sm) => self.scan_from_redb(sm, &_request, limit),
        }
    }

    /// Scan from the local state machine without linearizability guarantees.
    ///
    /// Reads directly from the follower's replicated state machine, skipping
    /// the ReadIndex protocol that would require a leader round-trip. This
    /// allows followers to scan their local KV data (e.g., for WASM plugin
    /// manifest discovery at startup) even when the leader is unreachable.
    ///
    /// The data is guaranteed to be committed (Raft only applies committed
    /// entries to the state machine), but may lag behind the leader by a
    /// few entries if replication is in-flight.
    #[instrument(skip(self))]
    async fn scan_local(&self, request: ScanRequest) -> Result<ScanResult, KeyValueStoreError> {
        let _permit = self.semaphore().acquire().await.map_err(|_| KeyValueStoreError::Failed {
            reason: "semaphore closed".into(),
        })?;

        self.ensure_initialized_kv()?;

        // No linearizability check — read directly from local state machine.
        // This is the key difference from scan(): followers can serve this
        // without contacting the leader.

        // Apply default limit if not specified
        let limit = request.limit_results.unwrap_or(DEFAULT_SCAN_LIMIT).min(MAX_SCAN_RESULTS) as usize;

        // Tiger Style: scan limit must be bounded
        assert!(
            limit <= MAX_SCAN_RESULTS as usize,
            "SCAN_LOCAL: computed limit {} exceeds MAX_SCAN_RESULTS {}",
            limit,
            MAX_SCAN_RESULTS
        );

        // Scan from appropriate state machine backend
        match self.state_machine() {
            StateMachineVariant::InMemory(sm) => self.scan_from_inmemory(sm, &request, limit).await,
            StateMachineVariant::Redb(sm) => self.scan_from_redb(sm, &request, limit),
        }
    }
}

// ====================================================================================
// read() helper methods (extracted for Tiger Style compliance)
// ====================================================================================

impl RaftNode {
    /// Ensure read consistency based on the requested consistency level.
    async fn read_ensure_consistency(&self, consistency: ReadConsistency) -> Result<(), KeyValueStoreError> {
        match consistency {
            ReadConsistency::Linearizable => {
                // ReadIndex: Strongest consistency via quorum confirmation
                self.read_ensure_linearizable_with_policy(ReadPolicy::ReadIndex).await
            }
            ReadConsistency::Lease => {
                // LeaseRead: Lower latency via leader lease (no quorum confirmation)
                self.read_ensure_linearizable_with_policy(ReadPolicy::LeaseRead).await
            }
            ReadConsistency::Stale => {
                // Stale: Read directly from local state machine without consistency checks
                // WARNING: May return uncommitted or rolled-back data
                Ok(())
            }
        }
    }

    /// Ensure linearizable read with the specified policy.
    async fn read_ensure_linearizable_with_policy(&self, policy: ReadPolicy) -> Result<(), KeyValueStoreError> {
        // Tiger Style: Timeout the linearizer acquisition itself, not just await_ready().
        let linearizer = tokio::time::timeout(READ_INDEX_TIMEOUT, self.raft().get_read_linearizer(policy))
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

        Ok(())
    }

    /// Read a key from the state machine.
    async fn read_from_state_machine(&self, key: &str) -> Result<ReadResult, KeyValueStoreError> {
        match self.state_machine() {
            StateMachineVariant::InMemory(sm) => match sm.get(key).await {
                Some(value) => Ok(ReadResult {
                    kv: Some(KeyValueWithRevision {
                        key: key.to_owned(),
                        value,
                        version: 1,         // In-memory doesn't track versions
                        create_revision: 0, // In-memory doesn't track revisions
                        mod_revision: 0,
                    }),
                }),
                None => Err(KeyValueStoreError::NotFound { key: key.to_owned() }),
            },
            StateMachineVariant::Redb(sm) => match sm.get(key) {
                Ok(Some(entry)) => Ok(ReadResult {
                    kv: Some(KeyValueWithRevision {
                        key: key.to_owned(),
                        value: entry.value,
                        version: entry.version as u64,
                        create_revision: entry.create_revision as u64,
                        mod_revision: entry.mod_revision as u64,
                    }),
                }),
                Ok(None) => Err(KeyValueStoreError::NotFound { key: key.to_owned() }),
                Err(err) => Err(KeyValueStoreError::Failed {
                    reason: err.to_string(),
                }),
            },
        }
    }
}

// ====================================================================================
// scan() helper methods (extracted for Tiger Style compliance)
// ====================================================================================

impl RaftNode {
    /// Ensure linearizable read via ReadIndex protocol.
    async fn scan_ensure_linearizable(&self) -> Result<(), KeyValueStoreError> {
        self.read_ensure_linearizable_with_policy(ReadPolicy::ReadIndex).await
    }

    /// Read consistency check with adaptive retry for transient leader-side ReadIndex failures.
    ///
    /// Uses Raft metrics (leader identity, log lag) to decide whether a retry is
    /// warranted. Only retries when this node believes it is still the leader and
    /// the log is reasonably current. Jittered backoff prevents thundering herd.
    ///
    /// With QUIC stream priorities (heartbeats scheduled ahead of bulk data),
    /// these retries should rarely fire — but they remain as a safety net for
    /// genuine contention (e.g., during snapshot install).
    async fn read_ensure_consistency_with_retry(&self, consistency: ReadConsistency) -> Result<(), KeyValueStoreError> {
        self.retry_read_index(|this| Box::pin(this.read_ensure_consistency(consistency)), "read").await
    }

    /// Scan linearizable check with adaptive retry.
    async fn scan_ensure_linearizable_with_retry(&self) -> Result<(), KeyValueStoreError> {
        self.retry_read_index(|this| Box::pin(this.scan_ensure_linearizable()), "scan").await
    }

    /// Generic adaptive ReadIndex retry loop.
    ///
    /// Calls the provided async function, retrying on transient NotLeader errors
    /// when this node is the leader with a reasonably current log.
    async fn retry_read_index<F>(&self, op: F, label: &str) -> Result<(), KeyValueStoreError>
    where F: Fn(
            &Self,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), KeyValueStoreError>> + Send + '_>> {
        use aspen_constants::network::READ_INDEX_MAX_RETRIES;
        use aspen_constants::network::READ_INDEX_RETRY_BASE_MS;

        use crate::verified::should_retry_read_index;

        let mut last_err = None;
        for attempt in 0..READ_INDEX_MAX_RETRIES {
            match op(self).await {
                Ok(()) => {
                    if attempt > 0 {
                        READ_INDEX_RETRY_SUCCESS_COUNT.fetch_add(1, Ordering::Relaxed);
                        debug!(node_id = self.node_id().0, attempt, label, "ReadIndex succeeded after retry");
                    }
                    return Ok(());
                }
                Err(e @ KeyValueStoreError::NotLeader { .. }) => {
                    // Use Raft metrics to decide if retry is warranted
                    let metrics = self.raft().metrics().borrow().clone();
                    let current_leader = metrics.current_leader.map(|id| id.0);
                    let last_log_index = metrics.last_log_index.unwrap_or(0);
                    let committed_index = metrics.last_applied.map(|li| li.index).unwrap_or(0);

                    let should_retry =
                        should_retry_read_index(current_leader, self.node_id().0, last_log_index, committed_index);

                    if should_retry && attempt + 1 < READ_INDEX_MAX_RETRIES {
                        READ_INDEX_RETRY_COUNT.fetch_add(1, Ordering::Relaxed);
                        // Jittered backoff: random between base and 2*base
                        let jitter = rand::random_range(0..=READ_INDEX_RETRY_BASE_MS);
                        let backoff_ms = READ_INDEX_RETRY_BASE_MS + jitter;
                        debug!(
                            node_id = self.node_id().0,
                            attempt = attempt + 1,
                            backoff_ms,
                            log_gap = last_log_index.saturating_sub(committed_index),
                            label,
                            "ReadIndex failed on leader, retrying with jittered backoff"
                        );
                        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                        last_err = Some(e);
                        continue;
                    }
                    return Err(e);
                }
                Err(e) => return Err(e),
            }
        }
        Err(last_err.unwrap_or_else(|| KeyValueStoreError::Failed {
            reason: format!("{} consistency retries exhausted", label),
        }))
    }

    /// Scan from in-memory state machine with pagination.
    async fn scan_from_inmemory(
        &self,
        sm: &std::sync::Arc<crate::InMemoryStateMachine>,
        request: &ScanRequest,
        limit: usize,
    ) -> Result<ScanResult, KeyValueStoreError> {
        // Get all KV pairs matching prefix
        let all_pairs = sm.scan_kv_with_prefix_async(&request.prefix).await;

        // Handle pagination via continuation token
        let start_key = request.continuation_token.as_deref();
        let filtered: Vec<_> =
            all_pairs.into_iter().filter(|(k, _)| start_key.is_none_or(|start| k.as_str() > start)).collect();

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
            result_count: entries.len() as u32,
            entries,
            is_truncated,
            continuation_token,
        })
    }

    /// Scan from Redb state machine with pagination.
    fn scan_from_redb(
        &self,
        sm: &crate::SharedRedbStorage,
        request: &ScanRequest,
        limit: usize,
    ) -> Result<ScanResult, KeyValueStoreError> {
        let start_key = request.continuation_token.as_deref();
        let limit_u32 = limit.min(u32::MAX as usize - 1) as u32;
        match sm.scan(&request.prefix, start_key, Some(limit_u32 + 1)) {
            Ok(entries_full) => {
                let is_truncated = entries_full.len() > limit;
                let entries: Vec<KeyValueWithRevision> = entries_full.into_iter().take(limit).collect();

                let continuation_token = if is_truncated {
                    entries.last().map(|e| e.key.clone())
                } else {
                    None
                };

                Ok(ScanResult {
                    result_count: entries.len() as u32,
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
