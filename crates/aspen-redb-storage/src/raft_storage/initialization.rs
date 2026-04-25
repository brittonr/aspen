//! Storage initialization: constructors and database setup.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;

use aspen_layer::IndexRegistry;
use aspen_layer::Subspace;
use aspen_layer::Tuple;
use redb::Database;
use redb::ReadableTable;
use snafu::ResultExt;
use tokio::sync::broadcast;

use super::BeginReadSnafu;
use super::BeginWriteSnafu;
use super::CommitSnafu;
use super::CreateDirectorySnafu;
use super::GetSnafu;
use super::OpenDatabaseSnafu;
use super::OpenTableSnafu;
use super::RangeSnafu;
use super::RedbKvStorage;
use super::SharedStorageError;
use super::SnapshotEvent;
use super::CHAIN_HASH_TABLE;
use super::INTEGRITY_META_TABLE;
use super::RAFT_LOG_TABLE;
use super::RAFT_META_TABLE;
use super::SM_INDEX_TABLE;
use super::SM_KV_TABLE;
use super::SM_LEASES_TABLE;
use super::SM_META_TABLE;
use super::SNAPSHOT_TABLE;

use crate::ChainTipState;

impl RedbKvStorage {
    pub fn new(path: impl AsRef<Path>, node_id: &str) -> Result<Self, SharedStorageError> {
        Self::with_options(path, None, node_id)
    }

    pub fn with_options(
        path: impl AsRef<Path>,
        snapshot_broadcast: Option<broadcast::Sender<SnapshotEvent>>,
        node_id: &str,
    ) -> Result<Self, SharedStorageError> {
        let path = path.as_ref().to_path_buf();

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context(CreateDirectorySnafu { path: parent })?;
        }

        let db = if path.exists() {
            Database::open(&path).context(OpenDatabaseSnafu { path: &path })?
        } else {
            Database::create(&path).context(OpenDatabaseSnafu { path: &path })?
        };

        let write_txn = db.begin_write().context(BeginWriteSnafu)?;
        {
            write_txn.open_table(RAFT_LOG_TABLE).context(OpenTableSnafu)?;
            write_txn.open_table(RAFT_META_TABLE).context(OpenTableSnafu)?;
            write_txn.open_table(SNAPSHOT_TABLE).context(OpenTableSnafu)?;
            write_txn.open_table(CHAIN_HASH_TABLE).context(OpenTableSnafu)?;
            write_txn.open_table(INTEGRITY_META_TABLE).context(OpenTableSnafu)?;
            write_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;
            write_txn.open_table(SM_LEASES_TABLE).context(OpenTableSnafu)?;
            write_txn.open_table(SM_META_TABLE).context(OpenTableSnafu)?;
            write_txn.open_table(SM_INDEX_TABLE).context(OpenTableSnafu)?;
        }
        write_txn.commit().context(CommitSnafu)?;

        let db = Arc::new(db);
        let local_node_id = node_id.parse::<u64>().ok();
        let chain_tip = Self::load_chain_tip(&db)?;

        let idx_subspace = Subspace::new(Tuple::new().push("idx"));
        let index_registry = Arc::new(IndexRegistry::with_builtins(idx_subspace));

        Ok(Self {
            db,
            path,
            local_node_id,
            chain_tip: Arc::new(StdRwLock::new(chain_tip)),
            pending_responses: Arc::new(StdRwLock::new(BTreeMap::new())),
            index_registry,
            confirmed_last_applied: Arc::new(StdRwLock::new(None)),
            snapshot_event_tx: snapshot_broadcast,
        })
    }

    fn load_chain_tip(db: &Arc<Database>) -> Result<ChainTipState, SharedStorageError> {
        let read_txn = db.begin_read().context(BeginReadSnafu)?;
        let meta_table = read_txn.open_table(INTEGRITY_META_TABLE).context(OpenTableSnafu)?;

        let tip_hash = meta_table.get("chain_tip_hash").context(GetSnafu)?.and_then(|v| {
            let bytes = v.value();
            if bytes.len() == 32 {
                let mut hash = [0u8; 32];
                hash.copy_from_slice(bytes);
                Some(hash)
            } else {
                None
            }
        });

        let tip_index: Option<u64> = meta_table
            .get("chain_tip_index")
            .context(GetSnafu)?
            .and_then(|v| bincode::deserialize(v.value()).ok());

        match (tip_hash, tip_index) {
            (Some(hash), Some(index)) => Ok(ChainTipState { hash, index }),
            _ => {
                let hash_table = read_txn.open_table(CHAIN_HASH_TABLE).context(OpenTableSnafu)?;

                if let Some(last) = hash_table.iter().context(RangeSnafu)?.next_back() {
                    let (key, value) = last.context(GetSnafu)?;
                    let index = key.value();
                    let bytes = value.value();
                    if bytes.len() == 32 {
                        let mut hash = [0u8; 32];
                        hash.copy_from_slice(bytes);
                        return Ok(ChainTipState { hash, index });
                    }
                }

                Ok(ChainTipState::default())
            }
        }
    }
}
