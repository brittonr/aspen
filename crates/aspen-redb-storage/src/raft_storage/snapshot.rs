//! `RedbKvSnapshotBuilder` and snapshot-related code.

use std::collections::BTreeMap;
use std::io::Cursor;

use aspen_storage_types::KvEntry;
use openraft::SnapshotMeta;
use openraft::StoredMembership;
use openraft::alias::LogIdOf;
use openraft::storage::RaftSnapshotBuilder;
use openraft::storage::Snapshot;
use redb::ReadTransaction;
use redb::ReadableTable;
use snafu::ResultExt;

use super::RedbKvStorage;
use super::SharedStorageError;
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
use super::error::SerializeSnafu;

use aspen_raft_kv_types::RaftKvTypeConfig;
use crate::MAX_SNAPSHOT_ENTRIES;
use crate::SnapshotIntegrity;

#[inline]
fn max_snapshot_entries_usize() -> usize {
    usize::try_from(MAX_SNAPSHOT_ENTRIES).unwrap_or(usize::MAX)
}

#[inline]
fn snapshot_entry_count_u32(entry_count: usize) -> u32 {
    u32::try_from(entry_count).unwrap_or(u32::MAX)
}

#[inline]
fn size_bytes_u64(size_bytes: usize) -> u64 {
    u64::try_from(size_bytes).unwrap_or(u64::MAX)
}

#[inline]
fn now_ms() -> u64 {
    aspen_time::current_time_ms()
}

// ====================================================================================
// Snapshot Builder
// ====================================================================================

pub struct RedbKvSnapshotBuilder {
    pub(super) storage: RedbKvStorage,
}

struct SnapshotData {
    last_applied: Option<LogIdOf<RaftKvTypeConfig>>,
    membership: StoredMembership<RaftKvTypeConfig>,
    kv_entries: BTreeMap<String, KvEntry>,
}

impl RedbKvSnapshotBuilder {
    fn build_snapshot_read_data(&self, read_txn: &ReadTransaction) -> Result<SnapshotData, std::io::Error> {
        let kv_table = read_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;
        let sm_meta_table = read_txn.open_table(SM_META_TABLE).context(OpenTableSnafu)?;

        let last_applied =
            *self.storage.confirmed_last_applied.read().map_err(|_| SharedStorageError::LockPoisoned {
                context: "reading confirmed_last_applied for snapshot".into(),
            })?;

        let membership: StoredMembership<RaftKvTypeConfig> = sm_meta_table
            .get("last_membership")
            .context(GetSnafu)?
            .and_then(|v| bincode::deserialize(v.value()).ok())
            .unwrap_or_default();

        let kv_entries = self.build_snapshot_collect_entries(&kv_table)?;

        Ok(SnapshotData {
            last_applied,
            membership,
            kv_entries,
        })
    }

    fn build_snapshot_collect_entries(
        &self,
        kv_table: &redb::ReadOnlyTable<&[u8], &[u8]>,
    ) -> Result<BTreeMap<String, KvEntry>, std::io::Error> {
        let mut kv_entries = BTreeMap::new();
        let current_ms = now_ms();
        let max_snapshot_entries = max_snapshot_entries_usize();

        for item in kv_table.iter().context(RangeSnafu)? {
            let (key_guard, value_guard) = item.context(GetSnafu)?;
            let key_str = match std::str::from_utf8(key_guard.value()) {
                Ok(s) => s.to_string(),
                Err(_) => continue,
            };

            let entry: KvEntry = match bincode::deserialize(value_guard.value()) {
                Ok(e) => e,
                Err(_) => continue,
            };

            if let Some(expires_at) = entry.expires_at_ms
                && current_ms > expires_at
            {
                continue;
            }

            if kv_entries.len() >= max_snapshot_entries {
                tracing::warn!(limit = MAX_SNAPSHOT_ENTRIES, "snapshot truncated at max entries");
                break;
            }

            kv_entries.insert(key_str, entry);
        }

        assert!(
            kv_entries.len() <= max_snapshot_entries,
            "SNAPSHOT: {} entries exceeds MAX_SNAPSHOT_ENTRIES {}",
            kv_entries.len(),
            MAX_SNAPSHOT_ENTRIES
        );

        Ok(kv_entries)
    }

    fn build_snapshot_store(
        &self,
        meta: &SnapshotMeta<RaftKvTypeConfig>,
        data: &[u8],
        entry_count: u32,
    ) -> Result<String, std::io::Error> {
        let snapshot_index = meta.last_log_id.as_ref().map(|l| l.index).unwrap_or(0);
        let chain_hash = self.storage.read_chain_hash_at(snapshot_index)?.unwrap_or([0u8; 32]);
        let meta_bytes = bincode::serialize(meta).context(SerializeSnafu)?;
        let integrity = SnapshotIntegrity::compute(&meta_bytes, data, chain_hash);
        let integrity_hex = integrity.combined_hash_hex();

        let stored = StoredSnapshot {
            meta: meta.clone(),
            data: data.to_vec(),
            integrity: Some(integrity),
        };

        let write_txn = self.storage.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut table = write_txn.open_table(SNAPSHOT_TABLE).context(OpenTableSnafu)?;
            let stored_bytes = bincode::serialize(&stored).context(SerializeSnafu)?;
            table.insert("current", stored_bytes.as_slice()).context(InsertSnafu)?;
        }
        write_txn.commit().context(CommitSnafu)?;

        tracing::info!(
            snapshot_id = %meta.snapshot_id,
            last_log_id = ?meta.last_log_id,
            entries = entry_count,
            size_bytes = data.len(),
            integrity_hash = %integrity_hex,
            "built snapshot with integrity hash"
        );

        Ok(integrity_hex)
    }

    fn build_snapshot_emit_event(&self, meta: &SnapshotMeta<RaftKvTypeConfig>, entry_count: u64, size_bytes: u64) {
        if let Some(ref tx) = self.storage.snapshot_event_tx {
            let event = SnapshotEvent::Created {
                snapshot_id: meta.snapshot_id.clone(),
                last_log_index: meta.last_log_id.as_ref().map(|l| l.index).unwrap_or(0),
                term: meta.last_log_id.as_ref().map(|l| l.leader_id.term).unwrap_or(0),
                entry_count,
                size_bytes,
            };
            if tx.send(event).is_err() {
                tracing::debug!("snapshot event dropped: no broadcast subscribers");
            }
        }
    }
}

impl RaftSnapshotBuilder<RaftKvTypeConfig> for RedbKvSnapshotBuilder {
    async fn build_snapshot(&mut self) -> Result<Snapshot<RaftKvTypeConfig>, std::io::Error> {
        let read_txn = self.storage.db.begin_read().context(BeginReadSnafu)?;
        let snapshot_data = self.build_snapshot_read_data(&read_txn)?;

        let data = bincode::serialize(&snapshot_data.kv_entries).context(SerializeSnafu)?;
        let snapshot_id =
            format!("snapshot-{}-{}", snapshot_data.last_applied.as_ref().map(|l| l.index).unwrap_or(0), now_ms());

        let meta = SnapshotMeta {
            last_log_id: snapshot_data.last_applied,
            last_membership: snapshot_data.membership,
            snapshot_id,
        };

        let entry_count = snapshot_entry_count_u32(snapshot_data.kv_entries.len());
        self.build_snapshot_store(&meta, &data, entry_count)?;
        self.build_snapshot_emit_event(&meta, u64::from(entry_count), size_bytes_u64(data.len()));

        Ok(Snapshot {
            meta,
            snapshot: Cursor::new(data),
        })
    }
}
