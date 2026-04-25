//! `RaftLogStorage` trait implementation for `RedbKvStorage`.

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

use super::RedbKvStorage;
use super::SharedStorageError;
use super::state_machine::set::empty_response;
use crate::ChainTipState;
use crate::EntryHashInput;
use crate::MAX_BATCH_SIZE;
use crate::compute_entry_hash;

use super::types::CHAIN_HASH_TABLE;
use super::types::INTEGRITY_META_TABLE;
use super::types::RAFT_LOG_TABLE;
use super::types::SM_INDEX_TABLE;
use super::types::SM_KV_TABLE;
use super::types::SM_LEASES_TABLE;
use super::types::SM_META_TABLE;
use super::error::BeginReadSnafu;
use super::error::BeginWriteSnafu;
use super::error::CommitSnafu;
use super::error::DeserializeSnafu;
use super::error::GetSnafu;
use super::error::InsertSnafu;
use super::error::OpenTableSnafu;
use super::error::RangeSnafu;
use super::error::RemoveSnafu;
use super::error::SerializeSnafu;

use aspen_raft_kv_types::RaftKvResponse;
use aspen_raft_kv_types::RaftKvTypeConfig;

#[inline]
fn max_batch_size_usize() -> usize {
    usize::try_from(MAX_BATCH_SIZE).unwrap_or(usize::MAX)
}

// ====================================================================================
// RaftLogReader Implementation
// ====================================================================================

impl RaftLogReader<RaftKvTypeConfig> for RedbKvStorage {
    async fn try_get_log_entries<RB>(
        &mut self,
        range: RB,
    ) -> Result<Vec<<RaftKvTypeConfig as openraft::RaftTypeConfig>::Entry>, io::Error>
    where
        RB: RangeBounds<u64> + Clone + Debug + OptionalSend,
    {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(RAFT_LOG_TABLE).context(OpenTableSnafu)?;

        let mut entries = Vec::with_capacity(max_batch_size_usize());
        let iter = table.range(range).context(RangeSnafu)?;

        let mut prev_index: Option<u64> = None;
        for item in iter {
            let (_key, value) = item.context(GetSnafu)?;
            let bytes = value.value();
            let entry: <RaftKvTypeConfig as openraft::RaftTypeConfig>::Entry =
                bincode::deserialize(bytes).context(DeserializeSnafu)?;

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

        assert!(
            entries.len() <= max_batch_size_usize(),
            "LOG: fetched {} entries exceeds MAX_BATCH_SIZE {}",
            entries.len(),
            MAX_BATCH_SIZE
        );

        Ok(entries)
    }

    async fn read_vote(&mut self) -> Result<Option<VoteOf<RaftKvTypeConfig>>, io::Error> {
        Ok(self.read_raft_meta("vote")?)
    }
}

// ====================================================================================
// RaftLogStorage Implementation
// ====================================================================================

impl RaftLogStorage<RaftKvTypeConfig> for RedbKvStorage {
    type LogReader = Self;

    async fn get_log_state(&mut self) -> Result<LogState<RaftKvTypeConfig>, io::Error> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(RAFT_LOG_TABLE).context(OpenTableSnafu)?;

        let last_log_id = table
            .iter()
            .context(RangeSnafu)?
            .next_back()
            .transpose()
            .context(GetSnafu)?
            .map(|(_key, value)| {
                let bytes = value.value();
                let entry: <RaftKvTypeConfig as openraft::RaftTypeConfig>::Entry =
                    bincode::deserialize(bytes).context(DeserializeSnafu)?;
                Ok::<_, SharedStorageError>(entry.log_id())
            })
            .transpose()?;

        let last_purged: Option<LogIdOf<RaftKvTypeConfig>> = self.read_raft_meta("last_purged_log_id")?;
        let last = last_log_id.or(last_purged);

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

    async fn save_committed(&mut self, committed: Option<LogIdOf<RaftKvTypeConfig>>) -> Result<(), io::Error> {
        if let Some(ref c) = committed {
            self.write_raft_meta("committed", c)?;
        } else {
            self.delete_raft_meta("committed")?;
        }
        Ok(())
    }

    async fn read_committed(&mut self) -> Result<Option<LogIdOf<RaftKvTypeConfig>>, io::Error> {
        Ok(self.read_raft_meta("committed")?)
    }

    async fn save_vote(&mut self, vote: &VoteOf<RaftKvTypeConfig>) -> Result<(), io::Error> {
        self.write_raft_meta("vote", vote)?;
        Ok(())
    }

    async fn append<I>(&mut self, entries: I, callback: IOFlushed<RaftKvTypeConfig>) -> Result<(), io::Error>
    where
        I: IntoIterator<Item = <RaftKvTypeConfig as openraft::RaftTypeConfig>::Entry> + OptionalSend,
        I::IntoIter: OptionalSend,
    {
        let prev_hash = self.append_read_chain_tip()?;
        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;

        let (new_tip_hash, new_tip_index, has_entries, pending_response_batch) =
            self.append_process_entries(&write_txn, entries, prev_hash)?;

        write_txn.commit().context(CommitSnafu)?;

        self.append_update_chain_tip(new_tip_hash, new_tip_index, has_entries)?;
        self.append_store_pending_responses(pending_response_batch)?;

        callback.io_completed(Ok(()));
        Ok(())
    }

    async fn truncate(&mut self, log_id: LogIdOf<RaftKvTypeConfig>) -> Result<(), io::Error> {
        let truncate_from = log_id.index();

        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut log_table = write_txn.open_table(RAFT_LOG_TABLE).context(OpenTableSnafu)?;
            let mut hash_table = write_txn.open_table(CHAIN_HASH_TABLE).context(OpenTableSnafu)?;

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

        let new_tip = if truncate_from > 0 {
            let previous_index = truncate_from.saturating_sub(1);
            self.read_chain_hash_at(previous_index)?
                .map(|hash| ChainTipState {
                    hash,
                    index: previous_index,
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

        Ok(())
    }

    async fn purge(&mut self, log_id: LogIdOf<RaftKvTypeConfig>) -> Result<(), io::Error> {
        self.purge_check_monotonicity(&log_id)?;
        self.purge_remove_entries_from_tables(&log_id)?;
        self.purge_clean_pending_responses(&log_id)?;
        self.write_raft_meta("last_purged_log_id", &log_id)?;
        Ok(())
    }

    async fn get_log_reader(&mut self) -> Self::LogReader {
        self.clone()
    }
}

// ====================================================================================
// Append Helper Functions
// ====================================================================================

impl RedbKvStorage {
    fn append_read_chain_tip(&self) -> Result<[u8; 32], io::Error> {
        let chain_tip = self.chain_tip.read().map_err(|_| SharedStorageError::LockPoisoned {
            context: "reading chain_tip for append".into(),
        })?;
        Ok(chain_tip.hash)
    }

    #[allow(clippy::type_complexity)]
    fn append_process_entries<I>(
        &self,
        write_txn: &redb::WriteTransaction,
        entries: I,
        mut prev_hash: [u8; 32],
    ) -> Result<([u8; 32], u64, bool, Vec<(u64, RaftKvResponse)>), io::Error>
    where
        I: IntoIterator<Item = <RaftKvTypeConfig as openraft::RaftTypeConfig>::Entry>,
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
        let mut pending_response_batch: Vec<(u64, RaftKvResponse)> = Vec::with_capacity(max_batch_size_usize());
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

    #[allow(clippy::too_many_arguments)]
    fn append_process_single_entry(
        &self,
        entry: &<RaftKvTypeConfig as openraft::RaftTypeConfig>::Entry,
        prev_hash: [u8; 32],
        prev_appended_index: Option<u64>,
        log_table: &mut redb::Table<u64, &[u8]>,
        hash_table: &mut redb::Table<u64, &[u8]>,
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        leases_table: &mut redb::Table<u64, &[u8]>,
        sm_meta_table: &mut redb::Table<&str, &[u8]>,
        pending_response_batch: &mut Vec<(u64, RaftKvResponse)>,
    ) -> Result<([u8; 32], u64), io::Error> {
        let log_id = entry.log_id();
        let index = log_id.index();
        let term = log_id.leader_id.term;

        if let Some(prev_idx) = prev_appended_index {
            assert!(index > prev_idx, "APPEND: log index regression: {index} <= {prev_idx}");
        }
        if index > 0 {
            assert!(term > 0, "APPEND: entry at index {index} has zero term");
        }

        let data = bincode::serialize(entry).context(SerializeSnafu)?;
        let entry_hash = compute_entry_hash(EntryHashInput {
            prev_hash: &prev_hash,
            log_index: index,
            term,
            entry_bytes: &data,
        });

        log_table.insert(index, data.as_slice()).context(InsertSnafu)?;
        hash_table.insert(index, entry_hash.as_slice()).context(InsertSnafu)?;

        let response = self.append_apply_entry_payload(
            entry,
            log_id,
            index,
            kv_table,
            index_table,
            leases_table,
            sm_meta_table,
        )?;
        pending_response_batch.push((index, response));

        let log_id_bytes = bincode::serialize(&Some(log_id)).context(SerializeSnafu)?;
        sm_meta_table.insert("last_applied_log", log_id_bytes.as_slice()).context(InsertSnafu)?;

        Ok((entry_hash, index))
    }

    #[allow(clippy::too_many_arguments)]
    fn append_apply_entry_payload(
        &self,
        entry: &<RaftKvTypeConfig as openraft::RaftTypeConfig>::Entry,
        log_id: LogIdOf<RaftKvTypeConfig>,
        index: u64,
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        leases_table: &mut redb::Table<u64, &[u8]>,
        sm_meta_table: &mut redb::Table<&str, &[u8]>,
    ) -> Result<RaftKvResponse, io::Error> {
        match &entry.payload {
            EntryPayload::Normal(request) => Ok(self.apply_request_in_txn(
                kv_table,
                index_table,
                &self.index_registry,
                leases_table,
                sm_meta_table,
                request,
                index,
            )?),
            EntryPayload::Membership(membership) => {
                let stored = StoredMembership::new(Some(log_id), membership.clone());
                let membership_bytes = bincode::serialize(&stored).context(SerializeSnafu)?;
                sm_meta_table.insert("last_membership", membership_bytes.as_slice()).context(InsertSnafu)?;
                Ok(empty_response())
            }
            EntryPayload::Blank => Ok(empty_response()),
        }
    }

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

    fn append_store_pending_responses(&self, pending_response_batch: Vec<(u64, RaftKvResponse)>) -> Result<(), io::Error> {
        if pending_response_batch.is_empty() {
            return Ok(());
        }

        assert!(
            pending_response_batch.len() <= max_batch_size_usize(),
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

    fn purge_check_monotonicity(&self, log_id: &LogIdOf<RaftKvTypeConfig>) -> Result<(), io::Error> {
        if let Some(prev) = self.read_raft_meta::<LogIdOf<RaftKvTypeConfig>>("last_purged_log_id")?
            && prev > *log_id
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("purge must be monotonic: prev={:?}, new={:?}", prev, log_id),
            ));
        }
        Ok(())
    }

    fn purge_remove_entries_from_tables(&self, log_id: &LogIdOf<RaftKvTypeConfig>) -> Result<(), io::Error> {
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

    fn purge_clean_pending_responses(&self, log_id: &LogIdOf<RaftKvTypeConfig>) -> Result<(), io::Error> {
        let mut pending_responses = self.pending_responses.write().map_err(|_| SharedStorageError::LockPoisoned {
            context: "writing pending_responses during purge".into(),
        })?;
        pending_responses.retain(|&idx, _| idx > log_id.index());
        Ok(())
    }
}
