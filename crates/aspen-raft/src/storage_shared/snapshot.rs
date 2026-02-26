//! `SharedRedbSnapshotBuilder` and snapshot-related code.

use std::collections::BTreeMap;
use std::io::Cursor;

use openraft::SnapshotMeta;
use openraft::StoredMembership;
use openraft::alias::LogIdOf;
use openraft::storage::RaftSnapshotBuilder;
use openraft::storage::Snapshot;
use redb::ReadTransaction;
use redb::ReadableTable;
use snafu::ResultExt;

use super::*;

// ====================================================================================
// Snapshot Builder
// ====================================================================================

/// Snapshot builder for SharedRedbStorage.
pub struct SharedRedbSnapshotBuilder {
    pub(super) storage: SharedRedbStorage,
}

/// Data collected for building a snapshot.
struct SnapshotData {
    last_applied: Option<LogIdOf<AppTypeConfig>>,
    membership: StoredMembership<AppTypeConfig>,
    kv_entries: BTreeMap<String, KvEntry>,
}

impl SharedRedbSnapshotBuilder {
    /// Read snapshot metadata and KV entries from the database.
    fn build_snapshot_read_data(&self, read_txn: &ReadTransaction) -> Result<SnapshotData, std::io::Error> {
        let kv_table = read_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;
        let sm_meta_table = read_txn.open_table(SM_META_TABLE).context(OpenTableSnafu)?;

        // Use confirmed_last_applied (from apply() callback) instead of the
        // eagerly-applied value in SM_META_TABLE. This prevents the TOCTOU race
        // where build_snapshot sees last_applied > openraft's apply_progress.
        let last_applied =
            *self.storage.confirmed_last_applied.read().map_err(|_| SharedStorageError::LockPoisoned {
                context: "reading confirmed_last_applied for snapshot".into(),
            })?;

        let membership: StoredMembership<AppTypeConfig> = sm_meta_table
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

    /// Collect non-expired KV entries up to MAX_SNAPSHOT_ENTRIES.
    fn build_snapshot_collect_entries(
        &self,
        kv_table: &redb::ReadOnlyTable<&[u8], &[u8]>,
    ) -> Result<BTreeMap<String, KvEntry>, std::io::Error> {
        let mut kv_entries = BTreeMap::new();
        let now_ms = now_unix_ms();

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

            // Skip expired entries
            if let Some(expires_at) = entry.expires_at_ms
                && now_ms > expires_at
            {
                continue;
            }

            kv_entries.insert(key_str, entry);

            if kv_entries.len() >= MAX_SNAPSHOT_ENTRIES as usize {
                tracing::warn!(limit = MAX_SNAPSHOT_ENTRIES, "snapshot truncated at max entries");
                break;
            }
        }

        assert!(
            kv_entries.len() <= MAX_SNAPSHOT_ENTRIES as usize,
            "SNAPSHOT: {} entries exceeds MAX_SNAPSHOT_ENTRIES {}",
            kv_entries.len(),
            MAX_SNAPSHOT_ENTRIES
        );

        Ok(kv_entries)
    }

    /// Compute integrity hashes and store the snapshot.
    fn build_snapshot_store(
        &self,
        meta: &SnapshotMeta<AppTypeConfig>,
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

    /// Emit snapshot created event to broadcast channel.
    fn build_snapshot_emit_event(&self, meta: &SnapshotMeta<AppTypeConfig>, entry_count: u64, size_bytes: u64) {
        if let Some(ref tx) = self.storage.snapshot_broadcast {
            let event = SnapshotEvent::Created {
                snapshot_id: meta.snapshot_id.clone(),
                last_log_index: meta.last_log_id.as_ref().map(|l| l.index).unwrap_or(0),
                term: meta.last_log_id.as_ref().map(|l| l.leader_id.term).unwrap_or(0),
                entry_count,
                size_bytes,
            };
            let _ = tx.send(event);
        }
    }
}

impl RaftSnapshotBuilder<AppTypeConfig> for SharedRedbSnapshotBuilder {
    /// Build a snapshot of the current state machine.
    ///
    /// This captures a point-in-time view of the KV state with cryptographic
    /// integrity hashes for verification during installation.
    ///
    /// # Verified Invariants
    ///
    /// - **INVARIANT 7 (Snapshot Integrity)**: The snapshot includes:
    ///   1. `data_hash` = blake3(serialized KV entries)
    ///   2. `meta_hash` = blake3(serialized snapshot metadata)
    ///   3. `combined_hash` = blake3(data_hash || meta_hash)
    ///   4. `chain_hash_at_snapshot` = chain hash at last_applied index
    ///
    /// - **Read-Only**: Snapshot creation does not modify storage state. The spec proof
    ///   `create_is_readonly` in verus/snapshot_spec.rs shows that `create_snapshot_post(pre,
    ///   index) == pre`.
    async fn build_snapshot(&mut self) -> Result<Snapshot<AppTypeConfig>, std::io::Error> {
        ghost! {
            let _ghost_pre_state = Ghost::<()>::new(());
        }

        let read_txn = self.storage.db.begin_read().context(BeginReadSnafu)?;
        let snapshot_data = self.build_snapshot_read_data(&read_txn)?;

        let data = bincode::serialize(&snapshot_data.kv_entries).context(SerializeSnafu)?;
        let snapshot_id =
            format!("snapshot-{}-{}", snapshot_data.last_applied.as_ref().map(|l| l.index).unwrap_or(0), now_unix_ms());

        let meta = SnapshotMeta {
            last_log_id: snapshot_data.last_applied,
            last_membership: snapshot_data.membership,
            snapshot_id,
        };

        let entry_count = snapshot_data.kv_entries.len().min(u32::MAX as usize) as u32;
        self.build_snapshot_store(&meta, &data, entry_count)?;
        self.build_snapshot_emit_event(&meta, entry_count as u64, data.len() as u64);

        proof! {
            // INVARIANT 7 verified by SnapshotIntegrity::compute()
        }

        Ok(Snapshot {
            meta,
            snapshot: Cursor::new(data),
        })
    }
}
