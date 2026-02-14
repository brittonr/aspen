//! KeyValueStore trait implementation for RaftNode.

use aspen_constants::api::DEFAULT_SCAN_LIMIT;
use aspen_constants::api::MAX_SCAN_RESULTS;
use aspen_core::validate_write_command;
use aspen_kv_types::BatchCondition;
use aspen_kv_types::BatchOperation;
use aspen_kv_types::CompareOp;
use aspen_kv_types::CompareTarget;
use aspen_kv_types::DeleteRequest;
use aspen_kv_types::DeleteResult;
use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::KeyValueWithRevision;
use aspen_kv_types::ReadConsistency;
use aspen_kv_types::ReadRequest;
use aspen_kv_types::ReadResult;
use aspen_kv_types::ScanRequest;
use aspen_kv_types::ScanResult;
use aspen_kv_types::TxnOp;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteOp;
use aspen_kv_types::WriteRequest;
use aspen_kv_types::WriteResult;
use aspen_raft_types::READ_INDEX_TIMEOUT;
use aspen_traits::KeyValueStore;
use async_trait::async_trait;
use openraft::ReadPolicy;
use tracing::instrument;

use super::RaftNode;
use crate::StateMachineVariant;
use crate::types::AppRequest;

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

        // Convert WriteRequest to AppRequest (direct Raft path)
        let app_request = match &request.command {
            WriteCommand::Set { key, value } => AppRequest::Set {
                key: key.clone(),
                value: value.clone(),
            },
            WriteCommand::SetWithTTL {
                key,
                value,
                ttl_seconds,
            } => {
                // Convert TTL in seconds to absolute expiration timestamp in milliseconds
                let now_ms =
                    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()
                        as u64;
                let expires_at_ms = now_ms + (*ttl_seconds as u64 * 1000);
                AppRequest::SetWithTTL {
                    key: key.clone(),
                    value: value.clone(),
                    expires_at_ms,
                }
            }
            WriteCommand::SetMulti { pairs } => AppRequest::SetMulti { pairs: pairs.clone() },
            WriteCommand::SetMultiWithTTL { pairs, ttl_seconds } => {
                let now_ms =
                    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()
                        as u64;
                let expires_at_ms = now_ms + (*ttl_seconds as u64 * 1000);
                AppRequest::SetMultiWithTTL {
                    pairs: pairs.clone(),
                    expires_at_ms,
                }
            }
            WriteCommand::Delete { key } => AppRequest::Delete { key: key.clone() },
            WriteCommand::DeleteMulti { keys } => AppRequest::DeleteMulti { keys: keys.clone() },
            WriteCommand::CompareAndSwap {
                key,
                expected,
                new_value,
            } => AppRequest::CompareAndSwap {
                key: key.clone(),
                expected: expected.clone(),
                new_value: new_value.clone(),
            },
            WriteCommand::CompareAndDelete { key, expected } => AppRequest::CompareAndDelete {
                key: key.clone(),
                expected: expected.clone(),
            },
            WriteCommand::Batch { operations } => {
                // Convert to compact tuple format: (is_set, key, value)
                let ops: Vec<(bool, String, String)> = operations
                    .iter()
                    .map(|op| match op {
                        BatchOperation::Set { key, value } => (true, key.clone(), value.clone()),
                        BatchOperation::Delete { key } => (false, key.clone(), String::new()),
                    })
                    .collect();
                AppRequest::Batch { operations: ops }
            }
            WriteCommand::ConditionalBatch { conditions, operations } => {
                // Convert conditions to compact tuple format: (type, key, expected)
                // Types: 0=ValueEquals, 1=KeyExists, 2=KeyNotExists
                let conds: Vec<(u8, String, String)> = conditions
                    .iter()
                    .map(|c| match c {
                        BatchCondition::ValueEquals { key, expected } => (0, key.clone(), expected.clone()),
                        BatchCondition::KeyExists { key } => (1, key.clone(), String::new()),
                        BatchCondition::KeyNotExists { key } => (2, key.clone(), String::new()),
                    })
                    .collect();
                // Convert operations to compact tuple format
                let ops: Vec<(bool, String, String)> = operations
                    .iter()
                    .map(|op| match op {
                        BatchOperation::Set { key, value } => (true, key.clone(), value.clone()),
                        BatchOperation::Delete { key } => (false, key.clone(), String::new()),
                    })
                    .collect();
                AppRequest::ConditionalBatch {
                    conditions: conds,
                    operations: ops,
                }
            }
            // Lease operations
            WriteCommand::SetWithLease { key, value, lease_id } => AppRequest::SetWithLease {
                key: key.clone(),
                value: value.clone(),
                lease_id: *lease_id,
            },
            WriteCommand::SetMultiWithLease { pairs, lease_id } => AppRequest::SetMultiWithLease {
                pairs: pairs.clone(),
                lease_id: *lease_id,
            },
            WriteCommand::LeaseGrant { lease_id, ttl_seconds } => AppRequest::LeaseGrant {
                lease_id: *lease_id,
                ttl_seconds: *ttl_seconds,
            },
            WriteCommand::LeaseRevoke { lease_id } => AppRequest::LeaseRevoke { lease_id: *lease_id },
            WriteCommand::LeaseKeepalive { lease_id } => AppRequest::LeaseKeepalive { lease_id: *lease_id },
            WriteCommand::Transaction {
                compare,
                success,
                failure,
            } => {
                // Convert compare conditions to compact format:
                // target: 0=Value, 1=Version, 2=CreateRevision, 3=ModRevision
                // op: 0=Equal, 1=NotEqual, 2=Greater, 3=Less
                let cmp: Vec<(u8, u8, String, String)> = compare
                    .iter()
                    .map(|c| {
                        let target = match c.target {
                            CompareTarget::Value => 0,
                            CompareTarget::Version => 1,
                            CompareTarget::CreateRevision => 2,
                            CompareTarget::ModRevision => 3,
                        };
                        let op = match c.op {
                            CompareOp::Equal => 0,
                            CompareOp::NotEqual => 1,
                            CompareOp::Greater => 2,
                            CompareOp::Less => 3,
                        };
                        (target, op, c.key.clone(), c.value.clone())
                    })
                    .collect();

                // Convert operations to compact format:
                // op_type: 0=Put, 1=Delete, 2=Get, 3=Range
                let convert_ops = |ops: &[TxnOp]| -> Vec<(u8, String, String)> {
                    ops.iter()
                        .map(|op| match op {
                            TxnOp::Put { key, value } => (0, key.clone(), value.clone()),
                            TxnOp::Delete { key } => (1, key.clone(), String::new()),
                            TxnOp::Get { key } => (2, key.clone(), String::new()),
                            TxnOp::Range { prefix, limit } => (3, prefix.clone(), limit.to_string()),
                        })
                        .collect()
                };

                AppRequest::Transaction {
                    compare: cmp,
                    success: convert_ops(success),
                    failure: convert_ops(failure),
                }
            }
            WriteCommand::OptimisticTransaction { read_set, write_set } => {
                // Convert WriteOp to compact tuple format: (is_set, key, value)
                let write_ops: Vec<(bool, String, String)> = write_set
                    .iter()
                    .map(|op| match op {
                        WriteOp::Set { key, value } => (true, key.clone(), value.clone()),
                        WriteOp::Delete { key } => (false, key.clone(), String::new()),
                    })
                    .collect();
                AppRequest::OptimisticTransaction {
                    read_set: read_set.clone(),
                    write_set: write_ops,
                }
            }
            // Shard topology operations
            WriteCommand::ShardSplit {
                source_shard,
                split_key,
                new_shard_id,
                topology_version,
            } => AppRequest::ShardSplit {
                source_shard: *source_shard,
                split_key: split_key.clone(),
                new_shard_id: *new_shard_id,
                topology_version: *topology_version,
            },
            WriteCommand::ShardMerge {
                source_shard,
                target_shard,
                topology_version,
            } => AppRequest::ShardMerge {
                source_shard: *source_shard,
                target_shard: *target_shard,
                topology_version: *topology_version,
            },
            WriteCommand::TopologyUpdate { topology_data } => AppRequest::TopologyUpdate {
                topology_data: topology_data.clone(),
            },
        };

        // Apply write through Raft consensus
        let result = self.raft().client_write(app_request).await;

        match result {
            Ok(resp) => {
                // Check if this was a CAS operation that failed its condition
                if let Some(false) = resp.data.cas_succeeded {
                    // CAS condition didn't match - extract key and expected from original command
                    let (key, expected) = match &request.command {
                        WriteCommand::CompareAndSwap { key, expected, .. } => (key.clone(), expected.clone()),
                        WriteCommand::CompareAndDelete { key, expected } => (key.clone(), Some(expected.clone())),
                        _ => unreachable!("cas_succeeded only set for CAS operations"),
                    };
                    return Err(KeyValueStoreError::CompareAndSwapFailed {
                        key,
                        expected,
                        actual: resp.data.value,
                    });
                }
                // Build WriteResult with appropriate fields based on operation type
                Ok(WriteResult {
                    command: Some(request.command),
                    batch_applied: resp.data.batch_applied,
                    conditions_met: resp.data.conditions_met,
                    failed_condition_index: resp.data.failed_condition_index,
                    lease_id: resp.data.lease_id,
                    ttl_seconds: resp.data.ttl_seconds,
                    keys_deleted: resp.data.keys_deleted,
                    succeeded: resp.data.succeeded,
                    txn_results: resp.data.txn_results,
                    header_revision: resp.data.header_revision,
                    occ_conflict: resp.data.occ_conflict,
                    conflict_key: resp.data.conflict_key,
                    conflict_expected_version: resp.data.conflict_expected_version,
                    conflict_actual_version: resp.data.conflict_actual_version,
                })
            }
            Err(err) => {
                // Preserve ForwardToLeader as NotLeader for proper retry handling
                if let Some(forward) = err.forward_to_leader() {
                    return Err(KeyValueStoreError::NotLeader {
                        leader: forward.leader_id.map(|id| id.0),
                        reason: format!(
                            "has to forward request to: {:?}, {:?}",
                            forward.leader_id, forward.leader_node
                        ),
                    });
                }
                Err(KeyValueStoreError::Failed {
                    reason: err.to_string(),
                })
            }
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
