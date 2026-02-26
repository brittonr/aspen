//! Storage initialization: constructors and database setup.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;

use aspen_core::layer::IndexRegistry;
use aspen_core::layer::Subspace;
use aspen_core::layer::Tuple;
use redb::Database;
use redb::ReadableTable;
use snafu::ResultExt;
use tokio::sync::broadcast;

use super::BeginReadSnafu;
use super::BeginWriteSnafu;
use super::CHAIN_HASH_TABLE;
use super::CommitSnafu;
use super::CreateDirectorySnafu;
use super::GetSnafu;
use super::INTEGRITY_META_TABLE;
use super::OpenDatabaseSnafu;
use super::OpenTableSnafu;
use super::RAFT_LOG_TABLE;
use super::RAFT_META_TABLE;
use super::RangeSnafu;
use super::SM_INDEX_TABLE;
use super::SM_KV_TABLE;
use super::SM_LEASES_TABLE;
use super::SM_META_TABLE;
use super::SNAPSHOT_TABLE;
use super::SharedRedbStorage;
use super::SharedStorageError;
use super::SnapshotEvent;
use crate::integrity::ChainTipState;
use crate::log_subscriber::LogEntryPayload;

impl SharedRedbStorage {
    /// Create or open a SharedRedbStorage at the given path.
    ///
    /// Creates the database file and all required tables if they don't exist.
    ///
    /// # Arguments
    /// * `path` - Path to the database file
    /// * `node_id` - Node identifier for HLC creation (e.g., Raft node ID as string)
    pub fn new(path: impl AsRef<Path>, node_id: &str) -> Result<Self, SharedStorageError> {
        Self::with_broadcasts(path, None, None, node_id)
    }

    /// Create with optional log broadcast channel.
    ///
    /// # Arguments
    /// * `path` - Path to the database file
    /// * `log_broadcast` - Optional broadcast channel for log entry notifications
    /// * `node_id` - Node identifier for HLC creation
    pub fn with_broadcast(
        path: impl AsRef<Path>,
        log_broadcast: Option<broadcast::Sender<LogEntryPayload>>,
        node_id: &str,
    ) -> Result<Self, SharedStorageError> {
        Self::with_broadcasts(path, log_broadcast, None, node_id)
    }

    /// Create with optional log and snapshot broadcast channels.
    ///
    /// # Arguments
    /// * `path` - Path to the database file
    /// * `log_broadcast` - Optional broadcast channel for log entry notifications
    /// * `snapshot_broadcast` - Optional broadcast channel for snapshot event notifications
    /// * `node_id` - Node identifier for HLC creation
    pub fn with_broadcasts(
        path: impl AsRef<Path>,
        log_broadcast: Option<broadcast::Sender<LogEntryPayload>>,
        snapshot_broadcast: Option<broadcast::Sender<SnapshotEvent>>,
        node_id: &str,
    ) -> Result<Self, SharedStorageError> {
        let path = path.as_ref().to_path_buf();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context(CreateDirectorySnafu { path: parent })?;
        }

        // Open or create database
        let db = if path.exists() {
            Database::open(&path).context(OpenDatabaseSnafu { path: &path })?
        } else {
            Database::create(&path).context(OpenDatabaseSnafu { path: &path })?
        };

        // Initialize all tables
        let write_txn = db.begin_write().context(BeginWriteSnafu)?;
        {
            // Log tables
            write_txn.open_table(RAFT_LOG_TABLE).context(OpenTableSnafu)?;
            write_txn.open_table(RAFT_META_TABLE).context(OpenTableSnafu)?;
            write_txn.open_table(SNAPSHOT_TABLE).context(OpenTableSnafu)?;
            write_txn.open_table(CHAIN_HASH_TABLE).context(OpenTableSnafu)?;
            write_txn.open_table(INTEGRITY_META_TABLE).context(OpenTableSnafu)?;

            // State machine tables
            write_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;
            write_txn.open_table(SM_LEASES_TABLE).context(OpenTableSnafu)?;
            write_txn.open_table(SM_META_TABLE).context(OpenTableSnafu)?;

            // Secondary index table
            write_txn.open_table(SM_INDEX_TABLE).context(OpenTableSnafu)?;
        }
        write_txn.commit().context(CommitSnafu)?;

        let db = Arc::new(db);

        // Load chain tip from database
        let chain_tip = Self::load_chain_tip(&db)?;

        // Create HLC for deterministic timestamp ordering
        let hlc = Arc::new(aspen_core::hlc::create_hlc(node_id));

        // Create index registry with built-in indexes
        let idx_subspace = Subspace::new(Tuple::new().push("idx"));
        let index_registry = Arc::new(IndexRegistry::with_builtins(idx_subspace));

        Ok(Self {
            db,
            path,
            chain_tip: Arc::new(StdRwLock::new(chain_tip)),
            log_broadcast,
            snapshot_broadcast,
            pending_responses: Arc::new(StdRwLock::new(BTreeMap::new())),
            hlc,
            index_registry,
        })
    }

    /// Load chain tip state from database.
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
                // Check if we have any chain hashes
                let hash_table = read_txn.open_table(CHAIN_HASH_TABLE).context(OpenTableSnafu)?;

                if let Some(last) = hash_table.iter().context(RangeSnafu)?.last() {
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
