//! Copy-on-write branch overlay implementing `KeyValueStore`.

use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use aspen_kv_types::DeleteRequest;
use aspen_kv_types::DeleteResult;
use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::KeyValueWithRevision;
use aspen_kv_types::ReadRequest;
use aspen_kv_types::ReadResult;
use aspen_kv_types::ScanRequest;
use aspen_kv_types::ScanResult;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteOp;
use aspen_kv_types::WriteRequest;
use aspen_kv_types::WriteResult;
use aspen_traits::KeyValueStore;
use async_trait::async_trait;
use dashmap::DashMap;
use tracing::debug;
use tracing::warn;

use crate::commit_result::CommitResult;
use crate::config::BranchConfig;
use crate::config::BranchStats;
use crate::constants::*;
use crate::entry::BranchEntry;
use crate::error::BranchError;
use crate::verified::scan_merge::merge_scan;

/// A copy-on-write overlay that buffers writes in-memory and commits them
/// atomically through the parent `KeyValueStore`.
///
/// Reads fall through to the parent for keys not in the dirty map.
/// Writes and deletes buffer locally. Commit flushes everything as a
/// single `WriteCommand::Batch` or `OptimisticTransaction`. Dropping
/// discards all buffered state with zero Raft interaction.
pub struct BranchOverlay<S: KeyValueStore + ?Sized> {
    /// Human-readable identifier for this branch.
    branch_id: String,
    /// The parent store that reads fall through to and commits target.
    parent: Arc<S>,
    /// Buffered writes and tombstones.
    dirty: DashMap<String, BranchEntry>,
    /// Keys read from parent with their mod_revision at read time.
    read_set: DashMap<String, i64>,
    /// Running total of dirty value bytes (Write entries only).
    dirty_bytes: AtomicU64,
    /// Nesting depth (0 = root branch directly over a real store).
    depth: u8,
    /// Resource limits for this branch.
    config: BranchConfig,
    /// The CommitId of the most recent commit on this branch (commit-dag feature).
    #[cfg(feature = "commit-dag")]
    parent_commit: std::sync::Mutex<Option<aspen_commit_dag::CommitId>>,
}

impl<S: KeyValueStore + ?Sized> BranchOverlay<S> {
    /// Create a new root-level branch overlay wrapping the given store.
    pub fn new(branch_id: impl Into<String>, parent: Arc<S>) -> Self {
        Self {
            branch_id: branch_id.into(),
            parent,
            dirty: DashMap::new(),
            read_set: DashMap::new(),
            dirty_bytes: AtomicU64::new(0),
            depth: 0,
            config: BranchConfig::default(),
            #[cfg(feature = "commit-dag")]
            parent_commit: std::sync::Mutex::new(None),
        }
    }

    /// Create a branch with explicit depth (for nesting).
    #[allow(dead_code)]
    pub(crate) fn with_depth(branch_id: impl Into<String>, parent: Arc<S>, depth: u8) -> Result<Self, BranchError> {
        if depth >= MAX_BRANCH_DEPTH {
            return Err(BranchError::DepthLimitExceeded {
                max_depth: MAX_BRANCH_DEPTH,
            });
        }
        Ok(Self {
            branch_id: branch_id.into(),
            parent,
            dirty: DashMap::new(),
            read_set: DashMap::new(),
            dirty_bytes: AtomicU64::new(0),
            depth,
            config: BranchConfig::default(),
            #[cfg(feature = "commit-dag")]
            parent_commit: std::sync::Mutex::new(None),
        })
    }

    /// Create a branch with custom configuration.
    pub fn with_config(branch_id: impl Into<String>, parent: Arc<S>, config: BranchConfig) -> Self {
        Self {
            branch_id: branch_id.into(),
            parent,
            dirty: DashMap::new(),
            read_set: DashMap::new(),
            dirty_bytes: AtomicU64::new(0),
            depth: 0,
            config,
            #[cfg(feature = "commit-dag")]
            parent_commit: std::sync::Mutex::new(None),
        }
    }

    /// Create a nested child branch. The child's writes are isolated from
    /// this branch until the child commits, at which point they merge into
    /// this branch's dirty map.
    pub fn child(self: &Arc<Self>, branch_id: impl Into<String>) -> Result<BranchOverlay<Self>, BranchError> {
        let child_depth = self.depth.saturating_add(1);
        if child_depth > MAX_BRANCH_DEPTH {
            return Err(BranchError::DepthLimitExceeded {
                max_depth: MAX_BRANCH_DEPTH,
            });
        }
        Ok(BranchOverlay {
            branch_id: branch_id.into(),
            parent: Arc::clone(self),
            dirty: DashMap::new(),
            read_set: DashMap::new(),
            dirty_bytes: AtomicU64::new(0),
            depth: child_depth,
            config: self.config.clone(),
            #[cfg(feature = "commit-dag")]
            parent_commit: std::sync::Mutex::new(None),
        })
    }

    /// Returns the branch identifier.
    pub fn branch_id(&self) -> &str {
        &self.branch_id
    }

    /// Returns the nesting depth.
    pub fn depth(&self) -> u8 {
        self.depth
    }

    /// Returns current branch statistics.
    pub fn stats(&self) -> BranchStats {
        BranchStats {
            dirty_count: self.dirty.len() as u32,
            dirty_bytes: self.dirty_bytes.load(Ordering::Relaxed),
            read_set_size: self.read_set.len() as u32,
            depth: self.depth,
        }
    }

    /// Check resource limits before inserting a new dirty entry.
    fn check_limits(&self, new_value_bytes: u64) -> Result<(), BranchError> {
        let max_keys = self.config.max_dirty_keys.unwrap_or(MAX_BRANCH_DIRTY_KEYS);
        let current_keys = self.dirty.len() as u32;
        if current_keys >= max_keys {
            return Err(BranchError::DirtyKeyLimitExceeded {
                limit: max_keys,
                current: current_keys,
            });
        }

        let max_bytes = self.config.max_total_bytes.unwrap_or(MAX_BRANCH_TOTAL_BYTES);
        let current_bytes = self.dirty_bytes.load(Ordering::Relaxed);
        if current_bytes.saturating_add(new_value_bytes) > max_bytes {
            return Err(BranchError::ByteLimitExceeded {
                limit_bytes: max_bytes,
                current_bytes,
            });
        }

        Ok(())
    }

    /// Commit all buffered mutations to the parent store atomically.
    ///
    /// If the branch has a non-empty read set, uses `OptimisticTransaction`
    /// for conflict detection. Otherwise uses `WriteCommand::Batch`.
    ///
    /// Returns an error if the batch exceeds `MAX_BATCH_SIZE`, the commit
    /// times out, or an optimistic transaction conflict is detected.
    pub async fn commit(&self) -> Result<CommitResult, BranchError> {
        let max_batch = aspen_constants::raft::MAX_BATCH_SIZE;
        let dirty_count = self.dirty.len() as u32;

        if dirty_count > max_batch {
            return Err(BranchError::BatchTooLarge {
                count: dirty_count,
                max: max_batch,
            });
        }

        // Collect dirty entries into write operations.
        let ops: Vec<WriteOp> = self
            .dirty
            .iter()
            .map(|entry| {
                let key = entry.key().clone();
                match entry.value() {
                    BranchEntry::Write { value } => WriteOp::Set {
                        key,
                        value: value.clone(),
                    },
                    BranchEntry::Tombstone => WriteOp::Delete { key },
                }
            })
            .collect();

        if ops.is_empty() {
            debug!(branch_id = %self.branch_id, "commit with no dirty entries");
            return Ok(CommitResult::without_commit_dag(WriteResult {
                command: None,
                batch_applied: Some(0),
                conditions_met: None,
                failed_condition_index: None,
                lease_id: None,
                ttl_seconds: None,
                keys_deleted: None,
                succeeded: Some(true),
                txn_results: None,
                header_revision: None,
                occ_conflict: None,
                conflict_key: None,
                conflict_expected_version: None,
                conflict_actual_version: None,
            }));
        }

        // Build the commit-dag metadata ops (if feature enabled).
        #[cfg(feature = "commit-dag")]
        let (commit_dag_ops, commit_id) = self.build_commit_dag_ops(&ops)?;

        // Choose command based on read set.
        let command = if self.read_set.is_empty() {
            // No reads from parent — use batch (no conflict detection needed).
            #[allow(unused_mut)]
            let mut batch_ops: Vec<aspen_kv_types::batch::BatchOperation> = ops
                .into_iter()
                .map(|op| match op {
                    WriteOp::Set { key, value } => aspen_kv_types::batch::BatchOperation::Set { key, value },
                    WriteOp::Delete { key } => aspen_kv_types::batch::BatchOperation::Delete { key },
                })
                .collect();

            // Append commit-dag metadata to the SAME batch (atomic).
            #[cfg(feature = "commit-dag")]
            batch_ops.extend(commit_dag_ops);

            WriteCommand::Batch { operations: batch_ops }
        } else {
            // Has reads — use optimistic transaction for conflict detection.
            let read_set: Vec<(String, i64)> =
                self.read_set.iter().map(|entry| (entry.key().clone(), *entry.value())).collect();

            #[allow(unused_mut)]
            let mut write_set = ops;

            // Append commit-dag metadata as write ops in the transaction.
            #[cfg(feature = "commit-dag")]
            {
                for op in commit_dag_ops {
                    match op {
                        aspen_kv_types::batch::BatchOperation::Set { key, value } => {
                            write_set.push(WriteOp::Set { key, value });
                        }
                        aspen_kv_types::batch::BatchOperation::Delete { key } => {
                            write_set.push(WriteOp::Delete { key });
                        }
                    }
                }
            }

            WriteCommand::OptimisticTransaction { read_set, write_set }
        };

        let request = WriteRequest { command };

        let timeout_ms = self.config.commit_timeout_ms.unwrap_or(BRANCH_COMMIT_TIMEOUT_MS);

        let result = tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), self.parent.write(request))
            .await
            .map_err(|_| BranchError::CommitTimeout { timeout_ms })?
            .map_err(|e| BranchError::StoreError { reason: e.to_string() })?;

        // Check for OCC conflict.
        if result.occ_conflict == Some(true) {
            let conflict_key = result.conflict_key.clone().unwrap_or_default();
            return Err(BranchError::CommitConflict { key: conflict_key });
        }

        // Update parent_commit tracking.
        #[cfg(feature = "commit-dag")]
        {
            let mut pc = self.parent_commit.lock().unwrap_or_else(|e| e.into_inner());
            *pc = Some(commit_id);
        }

        // Clear dirty state on successful commit.
        self.dirty.clear();
        self.read_set.clear();
        self.dirty_bytes.store(0, Ordering::Relaxed);

        debug!(
            branch_id = %self.branch_id,
            "branch committed successfully"
        );

        #[cfg(feature = "commit-dag")]
        {
            Ok(CommitResult {
                write_result: result,
                commit_id: Some(commit_id),
            })
        }
        #[cfg(not(feature = "commit-dag"))]
        {
            Ok(CommitResult::without_commit_dag(result))
        }
    }

    /// Commit all buffered mutations without checking the read set.
    ///
    /// Use this when the branch is combined with pass-through writes (CAS, batch)
    /// that legitimately modify keys the branch also read. The read set would
    /// report false conflicts in this case.
    pub async fn commit_no_conflict_check(&self) -> Result<CommitResult, BranchError> {
        let max_batch = aspen_constants::raft::MAX_BATCH_SIZE;
        let dirty_count = self.dirty.len() as u32;

        if dirty_count > max_batch {
            return Err(BranchError::BatchTooLarge {
                count: dirty_count,
                max: max_batch,
            });
        }

        // Collect ops first as WriteOps for commit-dag building.
        let write_ops: Vec<WriteOp> = self
            .dirty
            .iter()
            .map(|entry| {
                let key = entry.key().clone();
                match entry.value() {
                    BranchEntry::Write { value } => WriteOp::Set {
                        key,
                        value: value.clone(),
                    },
                    BranchEntry::Tombstone => WriteOp::Delete { key },
                }
            })
            .collect();

        if write_ops.is_empty() {
            debug!(branch_id = %self.branch_id, "commit (no conflict check) with no dirty entries");
            self.read_set.clear();
            return Ok(CommitResult::without_commit_dag(WriteResult {
                command: None,
                batch_applied: Some(0),
                conditions_met: None,
                failed_condition_index: None,
                lease_id: None,
                ttl_seconds: None,
                keys_deleted: None,
                succeeded: Some(true),
                txn_results: None,
                header_revision: None,
                occ_conflict: None,
                conflict_key: None,
                conflict_expected_version: None,
                conflict_actual_version: None,
            }));
        }

        // Build commit-dag metadata.
        #[cfg(feature = "commit-dag")]
        let (commit_dag_ops, commit_id) = self.build_commit_dag_ops(&write_ops)?;

        #[allow(unused_mut)]
        let mut ops: Vec<aspen_kv_types::batch::BatchOperation> = write_ops
            .into_iter()
            .map(|op| match op {
                WriteOp::Set { key, value } => aspen_kv_types::batch::BatchOperation::Set { key, value },
                WriteOp::Delete { key } => aspen_kv_types::batch::BatchOperation::Delete { key },
            })
            .collect();

        // Append commit-dag metadata to the SAME batch (atomic).
        #[cfg(feature = "commit-dag")]
        ops.extend(commit_dag_ops);

        let command = WriteCommand::Batch { operations: ops };
        let request = WriteRequest { command };

        let timeout_ms = self.config.commit_timeout_ms.unwrap_or(BRANCH_COMMIT_TIMEOUT_MS);

        let result = tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), self.parent.write(request))
            .await
            .map_err(|_| BranchError::CommitTimeout { timeout_ms })?
            .map_err(|e| BranchError::StoreError { reason: e.to_string() })?;

        // Update parent_commit tracking.
        #[cfg(feature = "commit-dag")]
        {
            let mut pc = self.parent_commit.lock().unwrap_or_else(|e| e.into_inner());
            *pc = Some(commit_id);
        }

        self.dirty.clear();
        self.read_set.clear();
        self.dirty_bytes.store(0, Ordering::Relaxed);

        debug!(
            branch_id = %self.branch_id,
            "branch committed (no conflict check)"
        );

        #[cfg(feature = "commit-dag")]
        {
            Ok(CommitResult {
                write_result: result,
                commit_id: Some(commit_id),
            })
        }
        #[cfg(not(feature = "commit-dag"))]
        {
            Ok(CommitResult::without_commit_dag(result))
        }
    }

    /// Explicitly abort the branch, discarding all buffered state.
    /// This is equivalent to dropping the branch but allows explicit control flow.
    pub fn abort(&self) {
        let count = self.dirty.len();
        self.dirty.clear();
        self.read_set.clear();
        self.dirty_bytes.store(0, Ordering::Relaxed);
        debug!(branch_id = %self.branch_id, dirty_count = count, "branch aborted");
    }

    /// Build commit-dag metadata operations for inclusion in the Raft batch.
    ///
    /// Returns the batch operations and the computed CommitId.
    #[cfg(feature = "commit-dag")]
    fn build_commit_dag_ops(
        &self,
        ops: &[WriteOp],
    ) -> Result<(Vec<aspen_kv_types::batch::BatchOperation>, aspen_commit_dag::CommitId), BranchError> {
        use aspen_commit_dag::CommitStore;
        use aspen_commit_dag::MutationType;
        use aspen_commit_dag::verified::commit_hash::compute_commit_id;
        use aspen_commit_dag::verified::commit_hash::compute_mutations_hash;

        // Snapshot the dirty map as sorted mutations.
        let mut mutations: Vec<(String, MutationType)> = ops
            .iter()
            .map(|op| match op {
                WriteOp::Set { key, value } => (key.clone(), MutationType::Set(value.clone())),
                WriteOp::Delete { key } => (key.clone(), MutationType::Delete),
            })
            .collect();
        mutations.sort_by(|a, b| a.0.cmp(&b.0));

        let mutations_hash = compute_mutations_hash(&mutations);

        // Read current parent commit.
        let parent = {
            let pc = self.parent_commit.lock().unwrap_or_else(|e| e.into_inner());
            *pc
        };

        let timestamp_ms =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;

        // We don't have the Raft revision yet (it's assigned by the Raft batch),
        // so we use 0 as a placeholder. The chain_hash_at_commit is also [0; 32]
        // since we don't have access to the Raft chain hash at this layer.
        let raft_revision = 0u64;
        let chain_hash_at_commit = [0u8; 32];

        let commit_id = compute_commit_id(&parent, &self.branch_id, &mutations_hash, raft_revision, timestamp_ms);

        let commit = aspen_commit_dag::Commit {
            id: commit_id,
            parent,
            branch_id: self.branch_id.clone(),
            mutations,
            mutations_hash,
            raft_revision,
            chain_hash_at_commit,
            timestamp_ms,
        };

        // Serialize commit for KV storage.
        let commit_value =
            CommitStore::serialize_commit(&commit).map_err(|e| BranchError::StoreError { reason: e.to_string() })?;

        let commit_key = CommitStore::commit_key(&commit_id);
        let tip_key = CommitStore::branch_tip_key(&self.branch_id);
        let tip_value = hex::encode(commit_id);

        let dag_ops = vec![
            aspen_kv_types::batch::BatchOperation::Set {
                key: commit_key,
                value: commit_value,
            },
            aspen_kv_types::batch::BatchOperation::Set {
                key: tip_key,
                value: tip_value,
            },
        ];

        Ok((dag_ops, commit_id))
    }

    /// Get the parent commit (commit-dag feature).
    #[cfg(feature = "commit-dag")]
    pub fn parent_commit(&self) -> Option<aspen_commit_dag::CommitId> {
        let pc = self.parent_commit.lock().unwrap_or_else(|e| e.into_inner());
        *pc
    }

    /// Set the parent commit (for fork_from).
    #[cfg(feature = "commit-dag")]
    pub fn set_parent_commit(&self, commit_id: aspen_commit_dag::CommitId) {
        let mut pc = self.parent_commit.lock().unwrap_or_else(|e| e.into_inner());
        *pc = Some(commit_id);
    }

    /// Access the parent store directly (for nested commit).
    #[allow(dead_code)]
    pub(crate) fn parent(&self) -> &Arc<S> {
        &self.parent
    }

    /// Access the dirty map directly (for nested commit merge).
    #[allow(dead_code)]
    pub(crate) fn dirty_map(&self) -> &DashMap<String, BranchEntry> {
        &self.dirty
    }

    /// Access the read set directly (for nested commit merge).
    #[allow(dead_code)]
    pub(crate) fn read_set_map(&self) -> &DashMap<String, i64> {
        &self.read_set
    }
}

#[async_trait]
impl<S: KeyValueStore + ?Sized> KeyValueStore for BranchOverlay<S> {
    async fn read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueStoreError> {
        let key = &request.key;

        // Check dirty map first.
        if let Some(entry) = self.dirty.get(key) {
            return match entry.value() {
                BranchEntry::Write { value } => Ok(ReadResult {
                    kv: Some(KeyValueWithRevision {
                        key: key.clone(),
                        value: value.clone(),
                        version: 0,
                        create_revision: 0,
                        mod_revision: 0,
                    }),
                }),
                BranchEntry::Tombstone => Err(KeyValueStoreError::NotFound { key: key.clone() }),
            };
        }

        // Fall through to parent.
        let result = self.parent.read(request).await?;

        // Record the mod_revision in read set for conflict detection.
        if let Some(ref kv) = result.kv {
            self.read_set.insert(kv.key.clone(), kv.mod_revision as i64);
        }

        Ok(result)
    }

    async fn write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueStoreError> {
        // Extract key and value from the write command.
        let (key, value) = match &request.command {
            WriteCommand::Set { key, value } => (key.clone(), value.clone()),
            WriteCommand::SetWithTTL { key, value, .. } => (key.clone(), value.clone()),
            _ => {
                // For complex commands (batch, transaction, etc.), pass through
                // to parent directly. Branch overlay only intercepts single-key writes.
                return self.parent.write(request).await;
            }
        };

        let new_bytes = value.len() as u64;

        // Check if we're replacing an existing dirty entry.
        let old_bytes = self.dirty.get(&key).map(|e| e.value().value_bytes()).unwrap_or(0);

        // Only check limits for genuinely new keys.
        if !self.dirty.contains_key(&key) {
            self.check_limits(new_bytes).map_err(|e| KeyValueStoreError::Failed { reason: e.to_string() })?;
        }

        // Update dirty bytes: subtract old, add new.
        if old_bytes > 0 {
            self.dirty_bytes.fetch_sub(old_bytes, Ordering::Relaxed);
        }
        self.dirty_bytes.fetch_add(new_bytes, Ordering::Relaxed);

        self.dirty.insert(key.clone(), BranchEntry::Write { value: value.clone() });

        Ok(WriteResult {
            command: Some(request.command),
            batch_applied: None,
            conditions_met: None,
            failed_condition_index: None,
            lease_id: None,
            ttl_seconds: None,
            keys_deleted: None,
            succeeded: Some(true),
            txn_results: None,
            header_revision: None,
            occ_conflict: None,
            conflict_key: None,
            conflict_expected_version: None,
            conflict_actual_version: None,
        })
    }

    async fn delete(&self, request: DeleteRequest) -> Result<DeleteResult, KeyValueStoreError> {
        let key = &request.key;

        // Adjust dirty bytes if replacing a Write entry.
        if let Some(entry) = self.dirty.get(key) {
            let old_bytes = entry.value().value_bytes();
            if old_bytes > 0 {
                self.dirty_bytes.fetch_sub(old_bytes, Ordering::Relaxed);
            }
        } else {
            // New key in dirty map — check limits (tombstones are 0 bytes).
            self.check_limits(0).map_err(|e| KeyValueStoreError::Failed { reason: e.to_string() })?;
        }

        self.dirty.insert(key.clone(), BranchEntry::Tombstone);

        Ok(DeleteResult {
            key: key.clone(),
            is_deleted: true,
        })
    }

    async fn scan(&self, request: ScanRequest) -> Result<ScanResult, KeyValueStoreError> {
        // Get parent results.
        let parent_result = self.parent.scan(request.clone()).await?;

        // Collect dirty entries sorted by key.
        let mut dirty_entries: Vec<(String, BranchEntry)> =
            self.dirty.iter().map(|entry| (entry.key().clone(), entry.value().clone())).collect();
        dirty_entries.sort_by(|a, b| a.0.cmp(&b.0));

        let max_results = request.limit_results.unwrap_or(aspen_constants::api::MAX_SCAN_RESULTS);

        // Use verified merge function.
        let merged = merge_scan(&dirty_entries, &parent_result.entries, &request.prefix, max_results);

        let result_count = merged.len() as u32;
        Ok(ScanResult {
            entries: merged,
            result_count,
            is_truncated: parent_result.is_truncated || result_count >= max_results,
            continuation_token: parent_result.continuation_token,
        })
    }
}

impl<S: KeyValueStore + ?Sized> Drop for BranchOverlay<S> {
    fn drop(&mut self) {
        let dirty_count = self.dirty.len();
        if dirty_count > 0 {
            warn!(
                branch_id = %self.branch_id,
                dirty_count,
                "branch dropped with uncommitted entries"
            );
        }
    }
}
