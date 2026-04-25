//! `RaftStateMachine` trait implementation for `RedbKvStorage`.

use std::collections::BTreeMap;
use std::io;
use std::io::Cursor;

use aspen_storage_types::KvEntry;
use n0_future::Stream;
use n0_future::TryStreamExt;
use openraft::OptionalSend;
use openraft::StoredMembership;
use openraft::alias::LogIdOf;
use openraft::alias::SnapshotDataOf;
use openraft::storage::EntryResponder;
use openraft::storage::RaftStateMachine;
use openraft::storage::Snapshot;
use redb::ReadableTable;
use snafu::ResultExt;

use super::RedbKvStorage;
use super::SharedStorageError;
use super::snapshot::RedbKvSnapshotBuilder;
use super::types::SM_KV_TABLE;
use super::types::SM_META_TABLE;
use super::types::SNAPSHOT_TABLE;
use super::types::StoredSnapshot;
use super::types::SnapshotEvent;
use super::error::BeginReadSnafu;
use super::error::BeginWriteSnafu;
use super::error::CommitSnafu;
use super::error::GetSnafu;
use super::error::InsertSnafu;
use super::error::OpenTableSnafu;
use super::error::RangeSnafu;
use super::error::RemoveSnafu;
use super::error::SerializeSnafu;

use aspen_raft_kv_types::RaftKvTypeConfig;
use crate::MAX_SNAPSHOT_ENTRIES;

#[inline]
fn snapshot_entry_count_u32(entry_count: usize) -> u32 {
    u32::try_from(entry_count).unwrap_or(u32::MAX)
}

#[inline]
fn max_snapshot_entries_usize() -> usize {
    usize::try_from(MAX_SNAPSHOT_ENTRIES).unwrap_or(usize::MAX)
}

// ====================================================================================
// RaftStateMachine Implementation (No-Op Apply)
// ====================================================================================

impl RaftStateMachine<RaftKvTypeConfig> for RedbKvStorage {
    type SnapshotBuilder = RedbKvSnapshotBuilder;

    async fn applied_state(
        &mut self,
    ) -> Result<(Option<LogIdOf<RaftKvTypeConfig>>, StoredMembership<RaftKvTypeConfig>), io::Error> {
        let last_applied: Option<LogIdOf<RaftKvTypeConfig>> = self.read_sm_meta("last_applied_log")?.flatten();
        let membership: Option<StoredMembership<RaftKvTypeConfig>> = self.read_sm_meta("last_membership")?;
        Ok((last_applied, membership.unwrap_or_default()))
    }

    async fn apply<Strm>(&mut self, mut entries: Strm) -> Result<(), io::Error>
    where Strm: Stream<Item = Result<EntryResponder<RaftKvTypeConfig>, io::Error>> + Unpin + OptionalSend {
        let mut latest_log_id: Option<LogIdOf<RaftKvTypeConfig>> = None;

        while let Some((entry, responder_opt)) = entries.try_next().await? {
            let log_index = entry.log_id.index;

            latest_log_id = Some(entry.log_id);

            if let Some(responder) = responder_opt {
                let response = self
                    .pending_responses
                    .write()
                    .map_err(|_| io::Error::other("pending_responses lock poisoned in apply"))?
                    .remove(&log_index)
                    .unwrap_or_default();
                responder.send(response);
            }
        }

        if let Some(new_log_id) = latest_log_id {
            let mut confirmed = self
                .confirmed_last_applied
                .write()
                .map_err(|_| io::Error::other("confirmed_last_applied lock poisoned in apply"))?;
            match *confirmed {
                Some(ref current) if new_log_id.index <= current.index => {}
                _ => {
                    *confirmed = Some(new_log_id);
                }
            }
        }

        Ok(())
    }

    async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
        RedbKvSnapshotBuilder { storage: self.clone() }
    }

    async fn begin_receiving_snapshot(&mut self) -> Result<SnapshotDataOf<RaftKvTypeConfig>, io::Error> {
        Ok(Cursor::new(Vec::new()))
    }

    async fn install_snapshot(
        &mut self,
        meta: &openraft::SnapshotMeta<RaftKvTypeConfig>,
        snapshot: SnapshotDataOf<RaftKvTypeConfig>,
    ) -> Result<(), io::Error> {
        let data = snapshot.into_inner();

        self.install_snapshot_verify_integrity(meta, &data)?;

        let kv_entries = self.install_snapshot_deserialize_data(&data)?;
        let kv_entries_count = snapshot_entry_count_u32(kv_entries.len());

        self.install_snapshot_write_data(meta, kv_entries)?;

        tracing::info!(
            snapshot_id = %meta.snapshot_id,
            last_log_id = ?meta.last_log_id,
            entries = kv_entries_count,
            "installed snapshot"
        );

        self.install_snapshot_broadcast_event(meta, kv_entries_count);

        Ok(())
    }

    async fn get_current_snapshot(&mut self) -> Result<Option<Snapshot<RaftKvTypeConfig>>, io::Error> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SNAPSHOT_TABLE).context(OpenTableSnafu)?;

        match table.get("current").context(GetSnafu)? {
            Some(value) => {
                let stored: StoredSnapshot = bincode::deserialize(value.value()).map_err(|e| {
                    io::Error::other(format!("failed to deserialize stored snapshot: {e}"))
                })?;
                Ok(Some(Snapshot {
                    meta: stored.meta,
                    snapshot: Cursor::new(stored.data),
                }))
            }
            None => Ok(None),
        }
    }
}

// ====================================================================================
// install_snapshot helper methods
// ====================================================================================

impl RedbKvStorage {
    fn install_snapshot_verify_integrity(
        &self,
        meta: &openraft::SnapshotMeta<RaftKvTypeConfig>,
        data: &[u8],
    ) -> Result<(), io::Error> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let Ok(table) = read_txn.open_table(SNAPSHOT_TABLE) else {
            return Ok(());
        };
        let Some(value) = table.get("current").context(GetSnafu)? else {
            return Ok(());
        };
        let Ok(stored) = bincode::deserialize::<StoredSnapshot>(value.value()) else {
            return Ok(());
        };
        let Some(ref integrity) = stored.integrity else {
            return Ok(());
        };

        let meta_bytes = bincode::serialize(meta)
            .map_err(|e| io::Error::other(format!("failed to serialize snapshot metadata for integrity check: {e}")))?;
        if !integrity.verify(&meta_bytes, data) {
            tracing::error!(
                snapshot_id = %meta.snapshot_id,
                "snapshot integrity verification failed"
            );
            return Err(io::Error::other("snapshot integrity verification failed"));
        }
        tracing::debug!(
            snapshot_id = %meta.snapshot_id,
            integrity_hash = %integrity.combined_hash_hex(),
            "snapshot integrity verified"
        );
        Ok(())
    }

    fn install_snapshot_deserialize_data(&self, data: &[u8]) -> Result<BTreeMap<String, KvEntry>, io::Error> {
        let kv_entries: BTreeMap<String, KvEntry> = bincode::deserialize(data).map_err(|e| {
            io::Error::other(format!("failed to deserialize snapshot KV entries ({} bytes): {e}", data.len()))
        })?;

        assert!(
            kv_entries.len() <= max_snapshot_entries_usize(),
            "INSTALL SNAPSHOT: {} entries exceeds MAX_SNAPSHOT_ENTRIES {}",
            kv_entries.len(),
            MAX_SNAPSHOT_ENTRIES
        );
        assert!(!data.is_empty(), "INSTALL SNAPSHOT: snapshot data must not be empty");

        Ok(kv_entries)
    }

    fn install_snapshot_write_data(
        &self,
        meta: &openraft::SnapshotMeta<RaftKvTypeConfig>,
        kv_entries: BTreeMap<String, KvEntry>,
    ) -> Result<(), io::Error> {
        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut kv_table = write_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;
            let mut sm_meta_table = write_txn.open_table(SM_META_TABLE).context(OpenTableSnafu)?;

            let keys: Vec<Vec<u8>> = kv_table
                .iter()
                .context(RangeSnafu)?
                .map(|item| {
                    let (key, _) = item.context(GetSnafu)?;
                    Ok::<_, SharedStorageError>(key.value().to_vec())
                })
                .collect::<Result<Vec<_>, _>>()?;

            for key in keys {
                kv_table.remove(key.as_slice()).context(RemoveSnafu)?;
            }

            for (key, entry) in kv_entries {
                let entry_bytes = bincode::serialize(&entry).context(SerializeSnafu)?;
                kv_table.insert(key.as_bytes(), entry_bytes.as_slice()).context(InsertSnafu)?;
            }

            let log_id_bytes = bincode::serialize(&meta.last_log_id).context(SerializeSnafu)?;
            sm_meta_table.insert("last_applied_log", log_id_bytes.as_slice()).context(InsertSnafu)?;

            let membership_bytes = bincode::serialize(&meta.last_membership).context(SerializeSnafu)?;
            sm_meta_table.insert("last_membership", membership_bytes.as_slice()).context(InsertSnafu)?;
        }
        write_txn.commit().context(CommitSnafu)?;
        Ok(())
    }

    fn install_snapshot_broadcast_event(&self, meta: &openraft::SnapshotMeta<RaftKvTypeConfig>, kv_entries_count: u32) {
        if let Some(ref tx) = self.snapshot_event_tx {
            let event = SnapshotEvent::Installed {
                snapshot_id: meta.snapshot_id.clone(),
                last_log_index: meta.last_log_id.as_ref().map(|l| l.index).unwrap_or(0),
                term: meta.last_log_id.as_ref().map(|l| l.leader_id.term).unwrap_or(0),
                entry_count: u64::from(kv_entries_count),
            };
            if let Err(error) = tx.send(event) {
                tracing::debug!(
                    snapshot_id = %meta.snapshot_id,
                    error = %error,
                    "snapshot install event broadcast dropped"
                );
            }
        }
    }
}
