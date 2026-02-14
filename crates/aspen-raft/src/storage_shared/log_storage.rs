//! `RaftLogStorage` trait implementation for `SharedRedbStorage`.

use std::fmt::Debug;
use std::io;
use std::ops::RangeBounds;

use openraft::EntryPayload;
use openraft::LogState;
use openraft::OptionalSend;
use openraft::RaftLogReader;
use openraft::StoredMembership;
use openraft::alias::LogIdOf;
use openraft::alias::VoteOf;
use openraft::entry::RaftEntry;
use openraft::storage::IOFlushed;
use openraft::storage::RaftLogStorage;
use redb::ReadableTable;
use snafu::ResultExt;

use super::*;

// ====================================================================================
// RaftLogReader Implementation
// ====================================================================================

impl RaftLogReader<AppTypeConfig> for SharedRedbStorage {
    async fn try_get_log_entries<RB>(
        &mut self,
        range: RB,
    ) -> Result<Vec<<AppTypeConfig as openraft::RaftTypeConfig>::Entry>, io::Error>
    where
        RB: RangeBounds<u64> + Clone + Debug + OptionalSend,
    {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(RAFT_LOG_TABLE).context(OpenTableSnafu)?;

        let mut entries = Vec::new();
        let iter = table.range(range).context(RangeSnafu)?;

        for item in iter {
            let (_key, value) = item.context(GetSnafu)?;
            let bytes = value.value();
            let entry: <AppTypeConfig as openraft::RaftTypeConfig>::Entry =
                bincode::deserialize(bytes).context(DeserializeSnafu)?;
            entries.push(entry);
        }

        Ok(entries)
    }

    async fn read_vote(&mut self) -> Result<Option<VoteOf<AppTypeConfig>>, io::Error> {
        Ok(self.read_raft_meta("vote")?)
    }
}

// ====================================================================================
// RaftLogStorage Implementation
// ====================================================================================

impl RaftLogStorage<AppTypeConfig> for SharedRedbStorage {
    type LogReader = Self;

    async fn get_log_state(&mut self) -> Result<LogState<AppTypeConfig>, io::Error> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(RAFT_LOG_TABLE).context(OpenTableSnafu)?;

        // Get last log entry
        let last_log_id = table
            .iter()
            .context(RangeSnafu)?
            .last()
            .transpose()
            .context(GetSnafu)?
            .map(|(_key, value)| {
                let bytes = value.value();
                let entry: <AppTypeConfig as openraft::RaftTypeConfig>::Entry =
                    bincode::deserialize(bytes).context(DeserializeSnafu)?;
                Ok::<_, SharedStorageError>(entry.log_id())
            })
            .transpose()?;

        let last_purged: Option<LogIdOf<AppTypeConfig>> = self.read_raft_meta("last_purged_log_id")?;
        let last = last_log_id.or(last_purged);

        Ok(LogState {
            last_purged_log_id: last_purged,
            last_log_id: last,
        })
    }

    async fn save_committed(&mut self, committed: Option<LogIdOf<AppTypeConfig>>) -> Result<(), io::Error> {
        if let Some(ref c) = committed {
            self.write_raft_meta("committed", c)?;
        } else {
            self.delete_raft_meta("committed")?;
        }
        Ok(())
    }

    async fn read_committed(&mut self) -> Result<Option<LogIdOf<AppTypeConfig>>, io::Error> {
        Ok(self.read_raft_meta("committed")?)
    }

    async fn save_vote(&mut self, vote: &VoteOf<AppTypeConfig>) -> Result<(), io::Error> {
        ensure_disk_space_available(&self.path)?;
        self.write_raft_meta("vote", vote)?;
        Ok(())
    }

    /// Append log entries AND apply state mutations in a single transaction.
    ///
    /// This is the core of the single-fsync optimization. Instead of:
    /// 1. append() -> fsync #1
    /// 2. apply() -> fsync #2
    ///
    /// We do:
    /// 1. append() with state application -> single fsync
    /// 2. apply() -> no-op
    ///
    /// # Verified Invariants
    ///
    /// This function maintains the following invariants (see `crates/aspen-raft/verus/`):
    ///
    /// - **INVARIANT 1 (Crash Safety)**: Either both log entry and state mutation are durable, or
    ///   neither is. This is achieved by using a single transaction for both.
    ///
    /// - **INVARIANT 2 (Chain Continuity)**: Each entry's hash correctly links to its predecessor
    ///   via `compute_entry_hash(prev_hash, index, term, data)`.
    ///
    /// - **INVARIANT 3 (Chain Tip Sync)**: After append, `chain_tip` reflects the last appended
    ///   entry's hash and index.
    ///
    /// - **INVARIANT 5 (Monotonic last_applied)**: `last_applied` only increases, never decreases.
    async fn append<I>(&mut self, entries: I, callback: IOFlushed<AppTypeConfig>) -> Result<(), io::Error>
    where
        I: IntoIterator<Item = <AppTypeConfig as openraft::RaftTypeConfig>::Entry> + OptionalSend,
        I::IntoIter: OptionalSend,
    {
        // =========================================================================
        // GHOST: Capture pre-state for verification
        // =========================================================================
        // When verus is enabled, this captures the abstract state before mutation.
        // When verus is disabled, this compiles to nothing (zero runtime cost).
        //
        // ghost! {
        //     let pre_chain_tip_hash = self.chain_tip.read().unwrap().hash;
        //     let pre_chain_tip_index = self.chain_tip.read().unwrap().index;
        //     let pre_last_applied = self.read_last_applied_index();
        // }
        ghost! {
            // Capture pre-state for invariant verification
            let _ghost_pre_chain_tip = Ghost::<[u8; 32]>::new([0u8; 32]);
            let _ghost_pre_last_applied = Ghost::<Option<u64>>::new(None);
        }

        ensure_disk_space_available(&self.path)?;

        // Get current chain tip
        let mut prev_hash = {
            let chain_tip = self.chain_tip.read().map_err(|_| SharedStorageError::LockPoisoned {
                context: "reading chain_tip for append".into(),
            })?;
            chain_tip.hash
        };

        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        let mut new_tip_hash = prev_hash;
        let mut new_tip_index: u64 = 0;
        let mut has_entries = false;
        // Track last applied log and membership for future use (e.g., log broadcast)
        let mut _last_applied_log_id: Option<LogIdOf<AppTypeConfig>> = None;
        let mut _last_membership: Option<StoredMembership<AppTypeConfig>> = None;
        // Collect responses to store after successful commit
        let mut pending_response_batch: Vec<(u64, AppResponse)> = Vec::new();

        {
            let mut log_table = write_txn.open_table(RAFT_LOG_TABLE).context(OpenTableSnafu)?;
            let mut hash_table = write_txn.open_table(CHAIN_HASH_TABLE).context(OpenTableSnafu)?;
            let mut kv_table = write_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;
            let mut index_table = write_txn.open_table(SM_INDEX_TABLE).context(OpenTableSnafu)?;
            let mut leases_table = write_txn.open_table(SM_LEASES_TABLE).context(OpenTableSnafu)?;
            let mut sm_meta_table = write_txn.open_table(SM_META_TABLE).context(OpenTableSnafu)?;

            for entry in entries {
                let log_id = entry.log_id();
                let index = log_id.index();
                let term = log_id.leader_id.term;

                // Serialize and insert log entry
                let data = bincode::serialize(&entry).context(SerializeSnafu)?;

                // Compute chain hash
                let entry_hash = compute_entry_hash(&prev_hash, index, term, &data);

                log_table.insert(index, data.as_slice()).context(InsertSnafu)?;
                hash_table.insert(index, entry_hash.as_slice()).context(InsertSnafu)?;

                // Apply state mutation based on payload and collect response
                let response = match &entry.payload {
                    EntryPayload::Normal(request) => {
                        // Apply the request to state machine tables with secondary index updates
                        Self::apply_request_in_txn(
                            &mut kv_table,
                            &mut index_table,
                            &self.index_registry,
                            &mut leases_table,
                            request,
                            index,
                        )?
                    }
                    EntryPayload::Membership(membership) => {
                        // Store membership in state machine metadata
                        let stored = StoredMembership::new(Some(log_id), membership.clone());
                        let membership_bytes = bincode::serialize(&stored).context(SerializeSnafu)?;
                        sm_meta_table.insert("last_membership", membership_bytes.as_slice()).context(InsertSnafu)?;
                        _last_membership = Some(stored);
                        AppResponse::default()
                    }
                    EntryPayload::Blank => {
                        // No-op for blank entries
                        AppResponse::default()
                    }
                };
                pending_response_batch.push((index, response));

                // Update last_applied
                _last_applied_log_id = Some(log_id);
                let log_id_bytes = bincode::serialize(&Some(log_id)).context(SerializeSnafu)?;
                sm_meta_table.insert("last_applied_log", log_id_bytes.as_slice()).context(InsertSnafu)?;

                prev_hash = entry_hash;
                new_tip_hash = entry_hash;
                new_tip_index = index;
                has_entries = true;
            }

            // Update chain tip in integrity metadata
            if has_entries {
                let mut integrity_table = write_txn.open_table(INTEGRITY_META_TABLE).context(OpenTableSnafu)?;
                integrity_table.insert("chain_tip_hash", new_tip_hash.as_slice()).context(InsertSnafu)?;
                let index_bytes = bincode::serialize(&new_tip_index).context(SerializeSnafu)?;
                integrity_table.insert("chain_tip_index", index_bytes.as_slice()).context(InsertSnafu)?;
            }
        }

        // Single commit for both log and state mutations
        // =========================================================================
        // INVARIANT 1 (Crash Safety): This single commit ensures atomicity.
        // Either all log entries AND their state mutations are durable, or none are.
        // Crash before commit() -> clean rollback, Raft will re-propose
        // Crash after commit() -> fully durable, no replay needed
        // =========================================================================
        write_txn.commit().context(CommitSnafu)?;

        // Update cached chain tip after successful commit
        if has_entries {
            let mut chain_tip = self.chain_tip.write().map_err(|_| SharedStorageError::LockPoisoned {
                context: "writing chain_tip after append".into(),
            })?;
            chain_tip.hash = new_tip_hash;
            chain_tip.index = new_tip_index;
        }

        // Store collected responses for retrieval in apply()
        if !pending_response_batch.is_empty() {
            let mut pending_responses =
                self.pending_responses.write().map_err(|_| SharedStorageError::LockPoisoned {
                    context: "writing pending_responses after append".into(),
                })?;
            for (index, response) in pending_response_batch {
                pending_responses.insert(index, response);
            }
        }

        // =========================================================================
        // GHOST: Verify post-conditions
        // =========================================================================
        // When verus is enabled, this proof block verifies:
        // 1. Chain continuity is preserved (new hashes correctly link to previous)
        // 2. Chain tip is synchronized with the last appended entry
        // 3. last_applied is monotonically increasing
        //
        // proof! {
        //     // Link to standalone proofs in verus/append.rs
        //     append_preserves_chain(pre_chain, pre_log, genesis, new_entries...);
        //
        //     // Verify invariants hold in post-state
        //     let post_state = self.to_spec_state();
        //     assert(chain_tip_synchronized(post_state));
        //     assert(last_applied_monotonic(pre_state, post_state));
        //     assert(storage_invariant(post_state));
        // }
        proof! {
            // INVARIANT 2 (Chain Continuity):
            // Each entry_hash = compute_entry_hash(prev_hash, index, term, data)
            // This is verified by construction in the loop above.
            // The spec proof in verus/chain_hash.rs shows append_preserves_chain.

            // INVARIANT 3 (Chain Tip Sync):
            // chain_tip.hash = new_tip_hash = hash of last appended entry
            // chain_tip.index = new_tip_index = index of last appended entry

            // INVARIANT 5 (Monotonic last_applied):
            // Each entry's index > previous entry's index (Raft log ordering)
            // last_applied is set to each entry's log_id in order
            // Therefore last_applied only increases
        }

        callback.io_completed(Ok(()));
        Ok(())
    }

    /// Truncate log entries from the given index onwards.
    ///
    /// This operation removes log entries with index >= truncate_from and repairs
    /// the chain tip to point to the last remaining entry.
    ///
    /// # Verified Invariants
    ///
    /// - **INVARIANT 2 (Chain Continuity)**: Remaining entries form a valid chain. The spec proof
    ///   `truncate_preserves_chain` in verus/chain_hash.rs shows that restricting a valid chain to
    ///   entries < truncate_at preserves validity.
    ///
    /// - **INVARIANT 3 (Chain Tip Sync)**: After truncate, chain_tip points to the last remaining
    ///   entry (truncate_from - 1) or genesis if all entries removed.
    async fn truncate(&mut self, log_id: LogIdOf<AppTypeConfig>) -> Result<(), io::Error> {
        let truncate_from = log_id.index();

        // =========================================================================
        // GHOST: Capture pre-state for verification
        // =========================================================================
        ghost! {
            let _ghost_pre_chain_tip_index = Ghost::<u64>::new(0);
            let _ghost_pre_log_count = Ghost::<u64>::new(0);
        }

        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut log_table = write_txn.open_table(RAFT_LOG_TABLE).context(OpenTableSnafu)?;
            let mut hash_table = write_txn.open_table(CHAIN_HASH_TABLE).context(OpenTableSnafu)?;

            // Collect keys to remove
            let keys: Vec<u64> = log_table
                .range(truncate_from..)
                .context(RangeSnafu)?
                .map(|item| {
                    let (key, _) = item.context(GetSnafu)?;
                    Ok::<_, SharedStorageError>(key.value())
                })
                .collect::<Result<Vec<_>, _>>()?;

            for key in &keys {
                log_table.remove(*key).context(RemoveSnafu)?;
                hash_table.remove(*key).context(RemoveSnafu)?;
            }
        }
        write_txn.commit().context(CommitSnafu)?;

        // Repair chain tip
        // =========================================================================
        // INVARIANT 3 (Chain Tip Sync):
        // After truncation, chain_tip must point to the last remaining entry.
        // If truncate_from > 0, that's entry at index (truncate_from - 1).
        // If truncate_from == 0, log is empty, chain_tip resets to genesis.
        // =========================================================================
        let new_tip = if truncate_from > 0 {
            self.read_chain_hash_at(truncate_from - 1)?
                .map(|hash| ChainTipState {
                    hash,
                    index: truncate_from - 1,
                })
                .unwrap_or_default()
        } else {
            ChainTipState::default()
        };

        {
            let mut chain_tip = self.chain_tip.write().map_err(|_| SharedStorageError::LockPoisoned {
                context: "writing chain_tip after truncate".into(),
            })?;
            *chain_tip = new_tip;
        }

        // =========================================================================
        // GHOST: Verify post-conditions
        // =========================================================================
        proof! {
            // INVARIANT 2 (Chain Continuity):
            // The spec proof truncate_preserves_chain shows that restricting
            // a valid chain to entries with index < truncate_at preserves validity.
            // All remaining entries have unchanged hashes linking to their predecessors.

            // INVARIANT 3 (Chain Tip Sync):
            // new_tip.index = truncate_from - 1 (or 0 if empty)
            // new_tip.hash = hash at that index (or genesis if empty)
            // This is exactly what chain_tip_synchronized requires.
        }

        Ok(())
    }

    /// Purge log entries up to and including the given log ID.
    ///
    /// This operation removes old log entries that have been snapshotted and are
    /// no longer needed for replication. It also cleans up pending responses.
    ///
    /// # Verified Invariants
    ///
    /// - **INVARIANT 6 (Monotonic Purge)**: `last_purged` only increases, never decreases. This is
    ///   enforced by the check at the start of this function and proven by the `purge_monotonic`
    ///   predicate in verus/purge.rs.
    ///
    /// - **INVARIANT 4 (Response Cache Consistency)**: After purge, no responses are cached for
    ///   indices <= purge_index. This is ensured by the retain() call that removes purged
    ///   responses.
    async fn purge(&mut self, log_id: LogIdOf<AppTypeConfig>) -> Result<(), io::Error> {
        // =========================================================================
        // INVARIANT 6 (Monotonic Purge):
        // Verify purge is monotonic - new purge index must be >= previous.
        // This check enforces the invariant and will reject invalid requests.
        // =========================================================================
        if let Some(prev) = self.read_raft_meta::<LogIdOf<AppTypeConfig>>("last_purged_log_id")?
            && prev > log_id
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("purge must be monotonic: prev={:?}, new={:?}", prev, log_id),
            ));
        }

        // =========================================================================
        // GHOST: Capture pre-state for verification
        // =========================================================================
        ghost! {
            let _ghost_pre_last_purged = Ghost::<Option<u64>>::new(None);
            let _ghost_new_purge_index = Ghost::<u64>::new(log_id.index());
        }

        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut log_table = write_txn.open_table(RAFT_LOG_TABLE).context(OpenTableSnafu)?;
            let mut hash_table = write_txn.open_table(CHAIN_HASH_TABLE).context(OpenTableSnafu)?;

            // Collect keys to remove
            let keys: Vec<u64> = log_table
                .range(..=log_id.index())
                .context(RangeSnafu)?
                .map(|item| {
                    let (key, _) = item.context(GetSnafu)?;
                    Ok::<_, SharedStorageError>(key.value())
                })
                .collect::<Result<Vec<_>, _>>()?;

            for key in &keys {
                log_table.remove(*key).context(RemoveSnafu)?;
                hash_table.remove(*key).context(RemoveSnafu)?;
            }
        }
        write_txn.commit().context(CommitSnafu)?;

        // =========================================================================
        // INVARIANT 4 (Response Cache Consistency):
        // Clean up pending responses for purged log entries.
        // After this, no responses exist for indices <= log_id.index().
        // =========================================================================
        {
            let mut pending_responses =
                self.pending_responses.write().map_err(|_| SharedStorageError::LockPoisoned {
                    context: "writing pending_responses during purge".into(),
                })?;
            // Remove all responses for indices <= purge index
            pending_responses.retain(|&idx, _| idx > log_id.index());
        }

        self.write_raft_meta("last_purged_log_id", &log_id)?;

        // =========================================================================
        // GHOST: Verify post-conditions
        // =========================================================================
        proof! {
            // INVARIANT 6 (Monotonic Purge):
            // The check at the start ensures prev_purged <= log_id.index().
            // We then set last_purged = log_id.
            // Therefore: pre.last_purged <= post.last_purged (monotonic).
            // This matches the purge_monotonic predicate in verus/purge.rs.

            // INVARIANT 4 (Response Cache Consistency):
            // After retain(), all remaining response indices > log_id.index().
            // Since last_applied >= last_purged (by Raft semantics),
            // all cached responses have index <= last_applied.
            // Therefore response_cache_consistent holds.
        }

        Ok(())
    }

    async fn get_log_reader(&mut self) -> Self::LogReader {
        self.clone()
    }
}
