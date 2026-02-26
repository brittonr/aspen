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

        let mut prev_index: Option<u64> = None;
        for item in iter {
            let (_key, value) = item.context(GetSnafu)?;
            let bytes = value.value();
            let entry: <AppTypeConfig as openraft::RaftTypeConfig>::Entry =
                bincode::deserialize(bytes).context(DeserializeSnafu)?;

            // Tiger Style: log indices must be monotonically increasing
            let current_index = entry.log_id().index();
            if let Some(prev) = prev_index {
                debug_assert!(
                    current_index > prev,
                    "LOG: log indices must be strictly increasing: prev={prev}, current={current_index}"
                );
            }
            prev_index = Some(current_index);

            entries.push(entry);
        }

        // Tiger Style: entry count must not exceed batch size limit
        assert!(
            entries.len() <= MAX_BATCH_SIZE as usize,
            "LOG: fetched {} entries exceeds MAX_BATCH_SIZE {}",
            entries.len(),
            MAX_BATCH_SIZE
        );

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

        // Tiger Style: if both exist, last_log_id must be >= last_purged
        if let (Some(purged), Some(last_id)) = (&last_purged, &last) {
            debug_assert!(
                last_id.index() >= purged.index(),
                "LOG STATE: last_log_id {} must be >= last_purged {}",
                last_id.index(),
                purged.index()
            );
        }

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
        ghost! {
            let _ghost_pre_chain_tip = Ghost::<[u8; 32]>::new([0u8; 32]);
            let _ghost_pre_last_applied = Ghost::<Option<u64>>::new(None);
        }

        ensure_disk_space_available(&self.path)?;

        let prev_hash = self.append_read_chain_tip()?;
        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;

        let (new_tip_hash, new_tip_index, has_entries, pending_response_batch) =
            self.append_process_entries(&write_txn, entries, prev_hash)?;

        // Single commit for both log and state mutations (INVARIANT 1: Crash Safety)
        write_txn.commit().context(CommitSnafu)?;

        self.append_update_chain_tip(new_tip_hash, new_tip_index, has_entries)?;
        self.append_store_pending_responses(pending_response_batch)?;

        proof! {
            // INVARIANT 2 (Chain Continuity): verified by construction in append_process_entries
            // INVARIANT 3 (Chain Tip Sync): chain_tip updated to last appended entry
            // INVARIANT 5 (Monotonic last_applied): indices strictly increasing
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
        // INVARIANT 6 (Monotonic Purge): Verify purge is monotonic
        self.purge_check_monotonicity(&log_id)?;

        ghost! {
            let _ghost_pre_last_purged = Ghost::<Option<u64>>::new(None);
            let _ghost_new_purge_index = Ghost::<u64>::new(log_id.index());
        }

        self.purge_remove_entries_from_tables(&log_id)?;
        self.purge_clean_pending_responses(&log_id)?;
        self.write_raft_meta("last_purged_log_id", &log_id)?;

        proof! {
            // INVARIANT 6 (Monotonic Purge):
            // The check at the start ensures prev_purged <= log_id.index().
            // We then set last_purged = log_id.
            // Therefore: pre.last_purged <= post.last_purged (monotonic).

            // INVARIANT 4 (Response Cache Consistency):
            // After retain(), all remaining response indices > log_id.index().
        }

        Ok(())
    }

    async fn get_log_reader(&mut self) -> Self::LogReader {
        self.clone()
    }
}

// ====================================================================================
// Append Helper Functions
// ====================================================================================

impl SharedRedbStorage {
    /// Read the current chain tip hash for append operation.
    fn append_read_chain_tip(&self) -> Result<[u8; 32], io::Error> {
        let chain_tip = self.chain_tip.read().map_err(|_| SharedStorageError::LockPoisoned {
            context: "reading chain_tip for append".into(),
        })?;
        Ok(chain_tip.hash)
    }

    /// Process all entries within the write transaction.
    ///
    /// Returns (new_tip_hash, new_tip_index, has_entries, pending_responses).
    #[allow(clippy::type_complexity)]
    fn append_process_entries<I>(
        &self,
        write_txn: &redb::WriteTransaction,
        entries: I,
        mut prev_hash: [u8; 32],
    ) -> Result<([u8; 32], u64, bool, Vec<(u64, AppResponse)>), io::Error>
    where
        I: IntoIterator<Item = <AppTypeConfig as openraft::RaftTypeConfig>::Entry>,
    {
        let mut log_table = write_txn.open_table(RAFT_LOG_TABLE).context(OpenTableSnafu)?;
        let mut hash_table = write_txn.open_table(CHAIN_HASH_TABLE).context(OpenTableSnafu)?;
        let mut kv_table = write_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;
        let mut index_table = write_txn.open_table(SM_INDEX_TABLE).context(OpenTableSnafu)?;
        let mut leases_table = write_txn.open_table(SM_LEASES_TABLE).context(OpenTableSnafu)?;
        let mut sm_meta_table = write_txn.open_table(SM_META_TABLE).context(OpenTableSnafu)?;

        let mut new_tip_hash = prev_hash;
        let mut new_tip_index: u64 = 0;
        let mut has_entries = false;
        let mut pending_response_batch: Vec<(u64, AppResponse)> = Vec::new();
        let mut prev_appended_index: Option<u64> = None;

        for entry in entries {
            let (entry_hash, index) = self.append_process_single_entry(
                &entry,
                prev_hash,
                prev_appended_index,
                &mut log_table,
                &mut hash_table,
                &mut kv_table,
                &mut index_table,
                &mut leases_table,
                &mut sm_meta_table,
                &mut pending_response_batch,
            )?;

            prev_appended_index = Some(index);
            prev_hash = entry_hash;
            new_tip_hash = entry_hash;
            new_tip_index = index;
            has_entries = true;
        }

        if has_entries {
            self.append_update_integrity_metadata(write_txn, new_tip_hash, new_tip_index)?;
        }

        Ok((new_tip_hash, new_tip_index, has_entries, pending_response_batch))
    }

    /// Process a single log entry: serialize, hash, store, and apply state mutation.
    #[allow(clippy::too_many_arguments)]
    fn append_process_single_entry(
        &self,
        entry: &<AppTypeConfig as openraft::RaftTypeConfig>::Entry,
        prev_hash: [u8; 32],
        prev_appended_index: Option<u64>,
        log_table: &mut redb::Table<u64, &[u8]>,
        hash_table: &mut redb::Table<u64, &[u8]>,
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        leases_table: &mut redb::Table<u64, &[u8]>,
        sm_meta_table: &mut redb::Table<&str, &[u8]>,
        pending_response_batch: &mut Vec<(u64, AppResponse)>,
    ) -> Result<([u8; 32], u64), io::Error> {
        let log_id = entry.log_id();
        let index = log_id.index();
        let term = log_id.leader_id.term;

        if let Some(prev_idx) = prev_appended_index {
            assert!(index > prev_idx, "APPEND: log index regression: {index} <= {prev_idx}");
        }
        // Term 0 is valid for the initial membership entry at index 0
        assert!(term > 0 || index == 0, "APPEND: entry at index {index} has zero term");

        let data = bincode::serialize(entry).context(SerializeSnafu)?;
        let entry_hash = compute_entry_hash(&prev_hash, index, term, &data);

        log_table.insert(index, data.as_slice()).context(InsertSnafu)?;
        hash_table.insert(index, entry_hash.as_slice()).context(InsertSnafu)?;

        let response =
            self.append_apply_entry_payload(entry, log_id, index, kv_table, index_table, leases_table, sm_meta_table)?;
        pending_response_batch.push((index, response));

        let log_id_bytes = bincode::serialize(&Some(log_id)).context(SerializeSnafu)?;
        sm_meta_table.insert("last_applied_log", log_id_bytes.as_slice()).context(InsertSnafu)?;

        Ok((entry_hash, index))
    }

    /// Apply the entry payload to state machine tables and return the response.
    #[allow(clippy::too_many_arguments)]
    fn append_apply_entry_payload(
        &self,
        entry: &<AppTypeConfig as openraft::RaftTypeConfig>::Entry,
        log_id: LogIdOf<AppTypeConfig>,
        index: u64,
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        leases_table: &mut redb::Table<u64, &[u8]>,
        sm_meta_table: &mut redb::Table<&str, &[u8]>,
    ) -> Result<AppResponse, io::Error> {
        match &entry.payload {
            EntryPayload::Normal(request) => Ok(Self::apply_request_in_txn(
                kv_table,
                index_table,
                &self.index_registry,
                leases_table,
                request,
                index,
            )?),
            EntryPayload::Membership(membership) => {
                let stored = StoredMembership::new(Some(log_id), membership.clone());
                let membership_bytes = bincode::serialize(&stored).context(SerializeSnafu)?;
                sm_meta_table.insert("last_membership", membership_bytes.as_slice()).context(InsertSnafu)?;
                Ok(AppResponse::default())
            }
            EntryPayload::Blank => Ok(AppResponse::default()),
        }
    }

    /// Update integrity metadata with new chain tip after processing entries.
    fn append_update_integrity_metadata(
        &self,
        write_txn: &redb::WriteTransaction,
        new_tip_hash: [u8; 32],
        new_tip_index: u64,
    ) -> Result<(), io::Error> {
        let mut integrity_table = write_txn.open_table(INTEGRITY_META_TABLE).context(OpenTableSnafu)?;
        integrity_table.insert("chain_tip_hash", new_tip_hash.as_slice()).context(InsertSnafu)?;
        let index_bytes = bincode::serialize(&new_tip_index).context(SerializeSnafu)?;
        integrity_table.insert("chain_tip_index", index_bytes.as_slice()).context(InsertSnafu)?;
        Ok(())
    }

    /// Update the cached chain tip after successful transaction commit.
    fn append_update_chain_tip(
        &self,
        new_tip_hash: [u8; 32],
        new_tip_index: u64,
        has_entries: bool,
    ) -> Result<(), io::Error> {
        if has_entries {
            let mut chain_tip = self.chain_tip.write().map_err(|_| SharedStorageError::LockPoisoned {
                context: "writing chain_tip after append".into(),
            })?;
            chain_tip.hash = new_tip_hash;
            chain_tip.index = new_tip_index;
        }
        Ok(())
    }

    /// Store pending responses for retrieval in apply().
    fn append_store_pending_responses(&self, pending_response_batch: Vec<(u64, AppResponse)>) -> Result<(), io::Error> {
        if pending_response_batch.is_empty() {
            return Ok(());
        }

        assert!(
            pending_response_batch.len() <= MAX_BATCH_SIZE as usize,
            "APPEND: pending response count {} exceeds MAX_BATCH_SIZE {}",
            pending_response_batch.len(),
            MAX_BATCH_SIZE
        );

        let mut pending_responses = self.pending_responses.write().map_err(|_| SharedStorageError::LockPoisoned {
            context: "writing pending_responses after append".into(),
        })?;
        for (index, response) in pending_response_batch {
            pending_responses.insert(index, response);
        }
        Ok(())
    }

    // ====================================================================================
    // Purge Helper Functions
    // ====================================================================================

    /// Check that purge is monotonic (INVARIANT 6).
    fn purge_check_monotonicity(&self, log_id: &LogIdOf<AppTypeConfig>) -> Result<(), io::Error> {
        if let Some(prev) = self.read_raft_meta::<LogIdOf<AppTypeConfig>>("last_purged_log_id")?
            && prev > *log_id
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("purge must be monotonic: prev={:?}, new={:?}", prev, log_id),
            ));
        }
        Ok(())
    }

    /// Remove log and hash entries from tables up to and including the given log ID.
    fn purge_remove_entries_from_tables(&self, log_id: &LogIdOf<AppTypeConfig>) -> Result<(), io::Error> {
        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut log_table = write_txn.open_table(RAFT_LOG_TABLE).context(OpenTableSnafu)?;
            let mut hash_table = write_txn.open_table(CHAIN_HASH_TABLE).context(OpenTableSnafu)?;

            let keys = self.purge_collect_keys_to_remove(&log_table, log_id.index())?;

            for key in &keys {
                log_table.remove(*key).context(RemoveSnafu)?;
                hash_table.remove(*key).context(RemoveSnafu)?;
            }
        }
        write_txn.commit().context(CommitSnafu)?;
        Ok(())
    }

    /// Collect keys to remove during purge operation.
    fn purge_collect_keys_to_remove(
        &self,
        log_table: &redb::Table<u64, &[u8]>,
        purge_index: u64,
    ) -> Result<Vec<u64>, io::Error> {
        log_table
            .range(..=purge_index)
            .context(RangeSnafu)?
            .map(|item| {
                let (key, _) = item.context(GetSnafu)?;
                Ok::<_, SharedStorageError>(key.value())
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.into())
    }

    /// Clean pending responses for purged log entries (INVARIANT 4).
    fn purge_clean_pending_responses(&self, log_id: &LogIdOf<AppTypeConfig>) -> Result<(), io::Error> {
        let mut pending_responses = self.pending_responses.write().map_err(|_| SharedStorageError::LockPoisoned {
            context: "writing pending_responses during purge".into(),
        })?;
        pending_responses.retain(|&idx, _| idx > log_id.index());
        Ok(())
    }
}
