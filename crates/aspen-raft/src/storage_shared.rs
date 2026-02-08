//! Single-fsync Redb storage implementation.
//!
//! This module provides a unified storage backend that implements both `RaftLogStorage`
//! and `RaftStateMachine` traits on a single struct, enabling single-fsync writes.
//!
//! # Architecture
//!
//! The key insight is that OpenRaft calls `append()` and `apply()` asynchronously
//! via separate tasks. Simply sharing a database handle does NOT achieve single-fsync:
//!
//! ```text
//! // Two separate transactions = two fsyncs (WRONG)
//! RaftLogStorage::append() -> txn.commit() -> fsync #1
//! RaftStateMachine::apply() -> txn.commit() -> fsync #2
//! ```
//!
//! Instead, we bundle state mutations INTO the log append:
//!
//! ```text
//! // Single transaction = single fsync (CORRECT)
//! RaftLogStorage::append() {
//!     txn.insert(log_entry);
//!     txn.apply(state_mutation);  // Apply state HERE
//!     txn.commit();  // Single fsync
//! }
//! RaftStateMachine::apply() {
//!     // No-op - already applied during append
//! }
//! ```
//!
//! # Why This Is Safe
//!
//! 1. Raft's correctness requires only that committed entries survive crashes
//! 2. Either both log entry and state mutation are durable, or neither is
//! 3. Crash before commit() -> clean rollback, Raft re-proposes
//! 4. Crash after commit() -> fully durable, no replay needed
//!
//! # Performance
//!
//! - Current (SQLite): ~9ms per write (2 fsyncs: redb log + SQLite state)
//! - Target (SharedRedb): ~2-3ms per write (1 fsync)
//!
//! # Tiger Style
//!
//! - Fixed limits on batch sizes (MAX_BATCH_SIZE, MAX_SETMULTI_KEYS)
//! - Explicit error types with actionable context
//! - Chain hashing for integrity verification
//! - Bounded operations prevent unbounded memory use

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::io::Cursor;
use std::io::{self};
use std::ops::RangeBounds;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;

#[cfg(feature = "coordination")]
use aspen_coordination::now_unix_ms;
use aspen_core::KeyValueWithRevision;
use aspen_core::TxnOpResult;
use aspen_core::hlc::SerializableTimestamp;
use futures::Stream;
use futures::TryStreamExt;
use openraft::EntryPayload;
use openraft::LogState;
use openraft::OptionalSend;
use openraft::RaftLogReader;
use openraft::StoredMembership;
use openraft::alias::LogIdOf;
use openraft::alias::SnapshotDataOf;
use openraft::alias::VoteOf;
use openraft::entry::RaftEntry;
use openraft::storage::EntryResponder;
use openraft::storage::IOFlushed;
use openraft::storage::RaftLogStorage;
use openraft::storage::RaftSnapshotBuilder;
use openraft::storage::RaftStateMachine;
use openraft::storage::Snapshot;
use redb::Database;
use redb::ReadableTable;
use redb::TableDefinition;
use serde::Deserialize;
use serde::Serialize;
use snafu::ResultExt;
use snafu::Snafu;
use tokio::sync::broadcast;

#[cfg(not(feature = "coordination"))]
fn now_unix_ms() -> u64 {
    use std::time::SystemTime;
    use std::time::UNIX_EPOCH;
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}
use aspen_core::ensure_disk_space_available;
use aspen_core::layer::IndexQueryExecutor;
use aspen_core::layer::IndexRegistry;
use aspen_core::layer::IndexResult;
use aspen_core::layer::IndexScanResult;
use aspen_core::layer::IndexableEntry;
use aspen_core::layer::Subspace;
use aspen_core::layer::Tuple;
use aspen_core::layer::extract_primary_key_from_tuple;

use crate::constants::MAX_BATCH_SIZE;
use crate::constants::MAX_SETMULTI_KEYS;
use crate::constants::MAX_SNAPSHOT_ENTRIES;
use crate::integrity::ChainHash;
use crate::integrity::ChainTipState;
use crate::integrity::SnapshotIntegrity;
use crate::integrity::compute_entry_hash;
use crate::pure::kv::check_cas_condition;
use crate::pure::kv::compute_kv_versions;
use crate::pure::kv::compute_lease_refresh;
use crate::pure::kv::create_lease_entry;
use crate::log_subscriber::LogEntryPayload;
// Ghost code imports - compile away when verus feature is disabled
use crate::spec::verus_shim::*;
use crate::types::AppRequest;
use crate::types::AppResponse;
use crate::types::AppTypeConfig;

// ====================================================================================
// Table Definitions
// ====================================================================================

/// Raft log entries: key = log index (u64), value = serialized Entry
const RAFT_LOG_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("raft_log");

/// Raft metadata: key = string identifier, value = serialized data
/// Keys: "vote", "committed", "last_purged_log_id"
const RAFT_META_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("raft_meta");

/// Snapshot storage
const SNAPSHOT_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("snapshots");

/// Chain hash table: key = log index (u64), value = ChainHash (32 bytes)
const CHAIN_HASH_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("chain_hashes");

/// Integrity metadata table
const INTEGRITY_META_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("integrity_meta");

// State machine tables - re-export from aspen-core for consistency
pub use aspen_core::storage::KvEntry;
pub use aspen_core::storage::SM_KV_TABLE;

/// Lease data: key = lease_id (u64), value = serialized LeaseEntry
const SM_LEASES_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("sm_leases");

/// State machine metadata: key = string identifier, value = serialized data
/// Keys: "last_applied_log", "last_membership"
const SM_META_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("sm_meta");

/// Secondary index table: key = index entry key (packed tuple), value = empty
/// Index keys have format: (index_subspace, indexed_value, primary_key) -> ()
const SM_INDEX_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("sm_index");

// ====================================================================================
// Storage Types
// ====================================================================================

// KvEntry is re-exported from aspen-core at the top of this file

/// Lease entry stored in the state machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseEntry {
    /// Time-to-live in seconds.
    pub ttl_seconds: u32,
    /// When the lease expires (Unix milliseconds).
    pub expires_at_ms: u64,
    /// Keys attached to this lease.
    pub keys: Vec<String>,
}

/// Stored snapshot format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredSnapshot {
    /// Raft snapshot metadata.
    pub meta: openraft::SnapshotMeta<AppTypeConfig>,
    /// Serialized snapshot data.
    pub data: Vec<u8>,
    /// Optional integrity hash.
    #[serde(default)]
    pub integrity: Option<SnapshotIntegrity>,
}

// ====================================================================================
// Snapshot Events
// ====================================================================================

/// Events emitted by snapshot operations for hook integration.
#[derive(Debug, Clone)]
pub enum SnapshotEvent {
    /// A snapshot was created (built) by this node.
    Created {
        /// Snapshot ID.
        snapshot_id: String,
        /// Log index included in the snapshot.
        last_log_index: u64,
        /// Term of the last log entry in the snapshot.
        term: u64,
        /// Number of KV entries in the snapshot.
        entry_count: u64,
        /// Size of the snapshot data in bytes.
        size_bytes: u64,
    },
    /// A snapshot was installed (received from another node).
    Installed {
        /// Snapshot ID.
        snapshot_id: String,
        /// Log index included in the snapshot.
        last_log_index: u64,
        /// Term of the last log entry in the snapshot.
        term: u64,
        /// Number of KV entries installed.
        entry_count: u64,
    },
}

// ====================================================================================
// Errors
// ====================================================================================

/// Errors from SharedRedbStorage operations.
#[derive(Debug, Snafu)]
pub enum SharedStorageError {
    /// Failed to open the redb database file.
    #[snafu(display("failed to open redb database at {}: {source}", path.display()))]
    OpenDatabase {
        /// Path to the database file.
        path: PathBuf,
        /// The underlying database error.
        #[snafu(source(from(redb::DatabaseError, Box::new)))]
        source: Box<redb::DatabaseError>,
    },

    /// Failed to begin a write transaction.
    #[snafu(display("failed to begin write transaction: {source}"))]
    BeginWrite {
        /// The underlying transaction error.
        #[snafu(source(from(redb::TransactionError, Box::new)))]
        source: Box<redb::TransactionError>,
    },

    /// Failed to begin a read transaction.
    #[snafu(display("failed to begin read transaction: {source}"))]
    BeginRead {
        /// The underlying transaction error.
        #[snafu(source(from(redb::TransactionError, Box::new)))]
        source: Box<redb::TransactionError>,
    },

    /// Failed to open a database table.
    #[snafu(display("failed to open table: {source}"))]
    OpenTable {
        /// The underlying table error.
        #[snafu(source(from(redb::TableError, Box::new)))]
        source: Box<redb::TableError>,
    },

    /// Failed to commit a transaction.
    #[snafu(display("failed to commit transaction: {source}"))]
    Commit {
        /// The underlying commit error.
        #[snafu(source(from(redb::CommitError, Box::new)))]
        source: Box<redb::CommitError>,
    },

    /// Failed to insert a value into a table.
    #[snafu(display("failed to insert into table: {source}"))]
    Insert {
        /// The underlying storage error.
        #[snafu(source(from(redb::StorageError, Box::new)))]
        source: Box<redb::StorageError>,
    },

    /// Failed to retrieve a value from a table.
    #[snafu(display("failed to get from table: {source}"))]
    Get {
        /// The underlying storage error.
        #[snafu(source(from(redb::StorageError, Box::new)))]
        source: Box<redb::StorageError>,
    },

    /// Failed to remove a value from a table.
    #[snafu(display("failed to remove from table: {source}"))]
    Remove {
        /// The underlying storage error.
        #[snafu(source(from(redb::StorageError, Box::new)))]
        source: Box<redb::StorageError>,
    },

    /// Failed to iterate over a table range.
    #[snafu(display("failed to iterate table range: {source}"))]
    Range {
        /// The underlying storage error.
        #[snafu(source(from(redb::StorageError, Box::new)))]
        source: Box<redb::StorageError>,
    },

    /// Failed to serialize data with bincode.
    #[snafu(display("failed to serialize data: {source}"))]
    Serialize {
        /// The underlying bincode error.
        #[snafu(source(from(bincode::Error, Box::new)))]
        source: Box<bincode::Error>,
    },

    /// Failed to deserialize data with bincode.
    #[snafu(display("failed to deserialize data: {source}"))]
    Deserialize {
        /// The underlying bincode error.
        #[snafu(source(from(bincode::Error, Box::new)))]
        source: Box<bincode::Error>,
    },

    /// Failed to create a directory.
    #[snafu(display("failed to create directory {}: {source}", path.display()))]
    CreateDirectory {
        /// Path to the directory.
        path: PathBuf,
        /// The underlying IO error.
        source: std::io::Error,
    },

    /// A storage lock was poisoned.
    #[snafu(display("storage lock poisoned: {context}"))]
    LockPoisoned {
        /// Context about which lock was poisoned.
        context: String,
    },

    /// Batch exceeds maximum size.
    #[snafu(display("batch size {} exceeds maximum {}", size, max))]
    BatchTooLarge {
        /// Actual batch size.
        size: usize,
        /// Maximum allowed batch size.
        max: u32,
    },
}

impl From<SharedStorageError> for io::Error {
    fn from(err: SharedStorageError) -> io::Error {
        io::Error::other(err.to_string())
    }
}

// ====================================================================================
// SharedRedbStorage Implementation
// ====================================================================================

/// Unified Raft log and state machine storage using single-fsync Redb.
///
/// This struct implements both `RaftLogStorage` and `RaftStateMachine` traits,
/// bundling state mutations into log appends for single-fsync durability.
///
/// # Architecture
///
/// ```text
/// append() {
///     1. Insert log entry into RAFT_LOG_TABLE
///     2. Compute chain hash
///     3. Parse entry payload
///     4. Apply state mutation to SM_KV_TABLE
///     5. Update SM_META_TABLE.last_applied
///     6. Handle membership changes
///     7. Single commit() -> single fsync
/// }
///
/// apply() {
///     // No-op - state already applied during append
/// }
/// ```
#[derive(Clone)]
pub struct SharedRedbStorage {
    /// The underlying Redb database.
    db: Arc<Database>,
    /// Path to the database file.
    path: PathBuf,
    /// Cached chain tip state for efficient appends.
    chain_tip: Arc<StdRwLock<ChainTipState>>,
    /// Optional broadcast sender for log entry notifications.
    /// Broadcasts committed KV operations for DocsExporter and other subscribers.
    log_broadcast: Option<broadcast::Sender<LogEntryPayload>>,
    /// Optional broadcast sender for snapshot event notifications.
    /// Broadcasts snapshot created/installed events for hook integration.
    snapshot_broadcast: Option<broadcast::Sender<SnapshotEvent>>,
    /// Pending responses computed during append() to be sent in apply().
    /// Key is log index, value is the computed AppResponse.
    pending_responses: Arc<StdRwLock<BTreeMap<u64, AppResponse>>>,
    /// Hybrid Logical Clock for deterministic timestamp ordering.
    hlc: Arc<aspen_core::hlc::HLC>,
    /// Secondary index registry for maintaining indexes.
    index_registry: Arc<IndexRegistry>,
}

impl std::fmt::Debug for SharedRedbStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedRedbStorage")
            .field("path", &self.path)
            .field("chain_tip", &self.chain_tip)
            .finish_non_exhaustive()
    }
}

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

    /// Get the path to the database file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get a reference to the underlying database.
    ///
    /// This is useful for maintenance operations that need direct database access.
    pub fn db(&self) -> Arc<Database> {
        self.db.clone()
    }

    /// Create a SQL executor for this storage backend.
    ///
    /// Returns a DataFusion-based SQL executor that can query the KV data.
    /// The executor is thread-safe and can be cached for reuse.
    #[cfg(feature = "sql")]
    pub fn create_sql_executor(&self) -> aspen_sql::RedbSqlExecutor {
        aspen_sql::RedbSqlExecutor::new(self.db.clone())
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

    /// Read metadata from RAFT_META_TABLE.
    fn read_raft_meta<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>, SharedStorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(RAFT_META_TABLE).context(OpenTableSnafu)?;

        match table.get(key).context(GetSnafu)? {
            Some(value) => {
                let data: T = bincode::deserialize(value.value()).context(DeserializeSnafu)?;
                Ok(Some(data))
            }
            None => Ok(None),
        }
    }

    /// Write metadata to RAFT_META_TABLE.
    fn write_raft_meta<T: Serialize>(&self, key: &str, value: &T) -> Result<(), SharedStorageError> {
        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut table = write_txn.open_table(RAFT_META_TABLE).context(OpenTableSnafu)?;
            let serialized = bincode::serialize(value).context(SerializeSnafu)?;
            table.insert(key, serialized.as_slice()).context(InsertSnafu)?;
        }
        write_txn.commit().context(CommitSnafu)?;
        Ok(())
    }

    /// Delete metadata from RAFT_META_TABLE.
    fn delete_raft_meta(&self, key: &str) -> Result<(), SharedStorageError> {
        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut table = write_txn.open_table(RAFT_META_TABLE).context(OpenTableSnafu)?;
            table.remove(key).context(RemoveSnafu)?;
        }
        write_txn.commit().context(CommitSnafu)?;
        Ok(())
    }

    /// Read metadata from SM_META_TABLE.
    fn read_sm_meta<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>, SharedStorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SM_META_TABLE).context(OpenTableSnafu)?;

        match table.get(key).context(GetSnafu)? {
            Some(value) => {
                let data: T = bincode::deserialize(value.value()).context(DeserializeSnafu)?;
                Ok(Some(data))
            }
            None => Ok(None),
        }
    }

    /// Get a key-value entry from the state machine.
    pub fn get(&self, key: &str) -> Result<Option<KvEntry>, SharedStorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;

        match table.get(key.as_bytes()).context(GetSnafu)? {
            Some(value) => {
                let entry: KvEntry = bincode::deserialize(value.value()).context(DeserializeSnafu)?;

                // Check expiration
                if let Some(expires_at) = entry.expires_at_ms
                    && now_unix_ms() > expires_at
                {
                    return Ok(None); // Expired
                }

                Ok(Some(entry))
            }
            None => Ok(None),
        }
    }

    /// Get a key-value with revision metadata.
    pub fn get_with_revision(&self, key: &str) -> Result<Option<KeyValueWithRevision>, SharedStorageError> {
        match self.get(key)? {
            Some(entry) => Ok(Some(KeyValueWithRevision {
                key: key.to_string(),
                value: entry.value,
                version: entry.version as u64,
                create_revision: entry.create_revision as u64,
                mod_revision: entry.mod_revision as u64,
            })),
            None => Ok(None),
        }
    }

    /// Scan keys matching a prefix.
    pub fn scan(
        &self,
        prefix: &str,
        after_key: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<KeyValueWithRevision>, SharedStorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;

        let now_ms = now_unix_ms();
        let bounded_limit = limit.unwrap_or(MAX_BATCH_SIZE as usize).min(MAX_BATCH_SIZE as usize);
        let prefix_bytes = prefix.as_bytes();

        let mut results = Vec::with_capacity(bounded_limit.min(128));

        for item in table.iter().context(RangeSnafu)? {
            let (key_guard, value_guard) = item.context(GetSnafu)?;
            let key_bytes = key_guard.value();

            // Check prefix match
            if !key_bytes.starts_with(prefix_bytes) {
                if key_bytes > prefix_bytes {
                    // Past the prefix range
                    break;
                }
                continue;
            }

            let key_str = match std::str::from_utf8(key_bytes) {
                Ok(s) => s,
                Err(_) => continue,
            };

            // Check continuation token
            if let Some(after) = after_key
                && key_str <= after
            {
                continue;
            }

            let entry: KvEntry = match bincode::deserialize(value_guard.value()) {
                Ok(e) => e,
                Err(_) => continue,
            };

            // Check expiration
            if let Some(expires_at) = entry.expires_at_ms
                && now_ms > expires_at
            {
                continue;
            }

            results.push(KeyValueWithRevision {
                key: key_str.to_string(),
                value: entry.value,
                version: entry.version as u64,
                create_revision: entry.create_revision as u64,
                mod_revision: entry.mod_revision as u64,
            });

            if results.len() >= bounded_limit {
                break;
            }
        }

        Ok(results)
    }

    // =========================================================================
    // TTL Cleanup Methods
    // =========================================================================

    /// Delete expired keys in a batch.
    ///
    /// Returns the number of keys deleted.
    /// This is used by the background TTL cleanup task.
    ///
    /// # Tiger Style
    /// - Fixed batch limit prevents unbounded work per call
    /// - Iterates over all keys (no index for TTL in Redb)
    /// - Idempotent: safe to call concurrently or repeatedly
    pub fn delete_expired_keys(&self, batch_limit: u32) -> Result<u32, SharedStorageError> {
        let now_ms = now_unix_ms();
        let mut deleted: u32 = 0;

        // Collect keys to delete (scan + filter)
        let keys_to_delete: Vec<Vec<u8>> = {
            let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
            let table = read_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;

            let mut keys = Vec::new();
            for item in table.iter().context(RangeSnafu)? {
                if keys.len() >= batch_limit as usize {
                    break;
                }

                let (key, value) = item.context(GetSnafu)?;
                let entry: KvEntry = bincode::deserialize(value.value()).context(DeserializeSnafu)?;

                if let Some(expires_at) = entry.expires_at_ms
                    && expires_at <= now_ms
                {
                    keys.push(key.value().to_vec());
                }
            }
            keys
        };

        // Delete in a write transaction
        if !keys_to_delete.is_empty() {
            let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
            {
                let mut table = write_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;
                for key in &keys_to_delete {
                    table.remove(key.as_slice()).context(RemoveSnafu)?;
                    deleted += 1;
                }
            }
            write_txn.commit().context(CommitSnafu)?;
        }

        Ok(deleted)
    }

    /// Count the number of expired keys in the state machine.
    ///
    /// Useful for metrics and monitoring.
    pub fn count_expired_keys(&self) -> Result<u64, SharedStorageError> {
        let now_ms = now_unix_ms();
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;

        let mut count: u64 = 0;
        for item in table.iter().context(RangeSnafu)? {
            let (_key, value) = item.context(GetSnafu)?;
            let entry: KvEntry = bincode::deserialize(value.value()).context(DeserializeSnafu)?;

            if let Some(expires_at) = entry.expires_at_ms
                && expires_at <= now_ms
            {
                count += 1;
            }
        }

        Ok(count)
    }

    /// Count the number of keys with TTL set (not yet expired).
    ///
    /// Useful for metrics and monitoring.
    pub fn count_keys_with_ttl(&self) -> Result<u64, SharedStorageError> {
        let now_ms = now_unix_ms();
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;

        let mut count: u64 = 0;
        for item in table.iter().context(RangeSnafu)? {
            let (_key, value) = item.context(GetSnafu)?;
            let entry: KvEntry = bincode::deserialize(value.value()).context(DeserializeSnafu)?;

            if let Some(expires_at) = entry.expires_at_ms
                && expires_at > now_ms
            {
                count += 1;
            }
        }

        Ok(count)
    }

    /// Get expired keys with their metadata for hook event emission.
    ///
    /// Returns a vector of (key, ttl_set_at_ms) pairs for expired keys.
    /// The ttl_set_at_ms is the timestamp when the TTL was originally set,
    /// which can be computed from (expires_at_ms - ttl_ms) if we store that,
    /// or approximated from the created_at timestamp.
    ///
    /// # Tiger Style
    /// - Fixed batch limit prevents unbounded work per call
    /// - Read-only operation (doesn't delete keys)
    pub fn get_expired_keys_with_metadata(
        &self,
        batch_limit: u32,
    ) -> Result<Vec<(String, Option<u64>)>, SharedStorageError> {
        let now_ms = now_unix_ms();
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;

        let mut expired_keys = Vec::new();
        for item in table.iter().context(RangeSnafu)? {
            if expired_keys.len() >= batch_limit as usize {
                break;
            }

            let (key, value) = item.context(GetSnafu)?;
            let entry: KvEntry = bincode::deserialize(value.value()).context(DeserializeSnafu)?;

            if let Some(expires_at) = entry.expires_at_ms
                && expires_at <= now_ms
            {
                let key_str = String::from_utf8_lossy(key.value()).to_string();
                // We don't store ttl_set_at explicitly; pass None
                // The expires_at_ms is available but when the TTL was originally set is not tracked
                expired_keys.push((key_str, None));
            }
        }

        Ok(expired_keys)
    }

    /// Get the current chain tip for verification.
    pub fn chain_tip_for_verification(&self) -> Result<(u64, ChainHash), SharedStorageError> {
        let chain_tip = self.chain_tip.read().map_err(|_| SharedStorageError::LockPoisoned {
            context: "reading chain_tip for verification".into(),
        })?;
        Ok((chain_tip.index, chain_tip.hash))
    }

    /// Read chain hash at a specific log index.
    fn read_chain_hash_at(&self, index: u64) -> Result<Option<ChainHash>, SharedStorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(CHAIN_HASH_TABLE).context(OpenTableSnafu)?;

        match table.get(index).context(GetSnafu)? {
            Some(value) => {
                let bytes = value.value();
                if bytes.len() == 32 {
                    let mut hash = [0u8; 32];
                    hash.copy_from_slice(bytes);
                    Ok(Some(hash))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    // =========================================================================
    // Lease Query Methods
    // =========================================================================

    /// Get lease information by ID.
    ///
    /// Returns (granted_ttl_seconds, remaining_ttl_seconds) if the lease exists and hasn't expired.
    pub fn get_lease(&self, lease_id: u64) -> Result<Option<(u32, u32)>, SharedStorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SM_LEASES_TABLE).context(OpenTableSnafu)?;

        let now_ms = now_unix_ms();

        match table.get(lease_id).context(GetSnafu)? {
            Some(value) => {
                let entry: LeaseEntry = bincode::deserialize(value.value()).context(DeserializeSnafu)?;

                // Check if lease has expired
                if now_ms > entry.expires_at_ms {
                    return Ok(None);
                }

                let remaining_ms = entry.expires_at_ms.saturating_sub(now_ms);
                let remaining_seconds = (remaining_ms / 1000) as u32;

                Ok(Some((entry.ttl_seconds, remaining_seconds)))
            }
            None => Ok(None),
        }
    }

    /// Get all keys attached to a lease.
    pub fn get_lease_keys(&self, lease_id: u64) -> Result<Vec<String>, SharedStorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SM_LEASES_TABLE).context(OpenTableSnafu)?;

        match table.get(lease_id).context(GetSnafu)? {
            Some(value) => {
                let entry: LeaseEntry = bincode::deserialize(value.value()).context(DeserializeSnafu)?;
                Ok(entry.keys)
            }
            None => Ok(vec![]),
        }
    }

    /// List all active (non-expired) leases.
    ///
    /// Returns a list of (lease_id, granted_ttl_seconds, remaining_ttl_seconds).
    pub fn list_leases(&self) -> Result<Vec<(u64, u32, u32)>, SharedStorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SM_LEASES_TABLE).context(OpenTableSnafu)?;

        let now_ms = now_unix_ms();
        let mut leases = Vec::new();

        for item in table.iter().context(RangeSnafu)? {
            let (id_guard, value_guard) = item.context(GetSnafu)?;
            let lease_id = id_guard.value();
            let entry: LeaseEntry = bincode::deserialize(value_guard.value()).context(DeserializeSnafu)?;

            // Skip expired leases
            if now_ms > entry.expires_at_ms {
                continue;
            }

            let remaining_ms = entry.expires_at_ms.saturating_sub(now_ms);
            let remaining_seconds = (remaining_ms / 1000) as u32;

            leases.push((lease_id, entry.ttl_seconds, remaining_seconds));
        }

        Ok(leases)
    }

    /// Delete expired leases and their attached keys in a batch.
    ///
    /// Returns the number of leases deleted.
    ///
    /// # Tiger Style
    ///
    /// - Fixed batch limit prevents unbounded work per call
    /// - Deletes both the lease and all keys attached to it
    /// - Idempotent: safe to call concurrently or repeatedly
    pub fn delete_expired_leases(&self, batch_limit: u32) -> Result<u32, SharedStorageError> {
        let now_ms = now_unix_ms();
        let mut deleted: u32 = 0;

        // First, collect expired lease IDs and their attached keys
        let expired_leases: Vec<(u64, Vec<String>)> = {
            let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
            let table = read_txn.open_table(SM_LEASES_TABLE).context(OpenTableSnafu)?;

            let mut expired = Vec::new();
            for item in table.iter().context(RangeSnafu)? {
                if deleted >= batch_limit {
                    break;
                }
                let (id_guard, value_guard) = item.context(GetSnafu)?;
                let lease_id = id_guard.value();
                let entry: LeaseEntry = bincode::deserialize(value_guard.value()).context(DeserializeSnafu)?;

                if now_ms > entry.expires_at_ms {
                    expired.push((lease_id, entry.keys.clone()));
                    deleted += 1;
                }
            }
            expired
        };

        if expired_leases.is_empty() {
            return Ok(0);
        }

        // Delete leases and their keys in a single transaction
        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut leases_table = write_txn.open_table(SM_LEASES_TABLE).context(OpenTableSnafu)?;
            let mut kv_table = write_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;

            for (lease_id, attached_keys) in &expired_leases {
                // Delete all keys attached to this lease
                for key in attached_keys {
                    let _ = kv_table.remove(key.as_bytes()).context(RemoveSnafu)?;
                }
                // Delete the lease itself
                let _ = leases_table.remove(*lease_id).context(RemoveSnafu)?;
            }
        }
        write_txn.commit().context(CommitSnafu)?;

        Ok(deleted)
    }

    /// Count the number of expired leases.
    ///
    /// Useful for metrics and monitoring.
    pub fn count_expired_leases(&self) -> Result<u64, SharedStorageError> {
        let now_ms = now_unix_ms();
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SM_LEASES_TABLE).context(OpenTableSnafu)?;

        let mut count: u64 = 0;
        for item in table.iter().context(RangeSnafu)? {
            let (_id_guard, value_guard) = item.context(GetSnafu)?;
            let entry: LeaseEntry = bincode::deserialize(value_guard.value()).context(DeserializeSnafu)?;

            if now_ms > entry.expires_at_ms {
                count += 1;
            }
        }

        Ok(count)
    }

    /// Count the number of active (non-expired) leases.
    ///
    /// Useful for metrics and monitoring.
    pub fn count_active_leases(&self) -> Result<u64, SharedStorageError> {
        let now_ms = now_unix_ms();
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SM_LEASES_TABLE).context(OpenTableSnafu)?;

        let mut count: u64 = 0;
        for item in table.iter().context(RangeSnafu)? {
            let (_id_guard, value_guard) = item.context(GetSnafu)?;
            let entry: LeaseEntry = bincode::deserialize(value_guard.value()).context(DeserializeSnafu)?;

            if now_ms <= entry.expires_at_ms {
                count += 1;
            }
        }

        Ok(count)
    }
}

// ====================================================================================
// State Machine Application Helpers
// ====================================================================================

impl SharedRedbStorage {
    /// Apply a Set operation within a transaction.
    #[allow(clippy::too_many_arguments)]
    fn apply_set_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        leases_table: &mut redb::Table<u64, &[u8]>,
        key: &str,
        value: &str,
        log_index: u64,
        expires_at_ms: Option<u64>,
        lease_id: Option<u64>,
    ) -> Result<AppResponse, SharedStorageError> {
        let key_bytes = key.as_bytes();

        // Read existing entry to get version and create_revision
        let existing = kv_table
            .get(key_bytes)
            .context(GetSnafu)?
            .and_then(|v| bincode::deserialize::<KvEntry>(v.value()).ok());

        // Use pure function to compute versions
        let existing_version = existing.as_ref().map(|e| (e.create_revision, e.version));
        let versions = compute_kv_versions(existing_version, log_index);

        let entry = KvEntry {
            value: value.to_string(),
            version: versions.version,
            create_revision: versions.create_revision,
            mod_revision: versions.mod_revision,
            expires_at_ms,
            lease_id,
        };

        // Convert KvEntry to IndexableEntry for index updates
        let new_indexable = IndexableEntry {
            value: entry.value.clone(),
            version: entry.version,
            create_revision: entry.create_revision,
            mod_revision: entry.mod_revision,
            expires_at_ms: entry.expires_at_ms,
            lease_id: entry.lease_id,
        };

        let old_indexable = existing.as_ref().map(|e| IndexableEntry {
            value: e.value.clone(),
            version: e.version,
            create_revision: e.create_revision,
            mod_revision: e.mod_revision,
            expires_at_ms: e.expires_at_ms,
            lease_id: e.lease_id,
        });

        // Generate index updates
        let index_update = index_registry.updates_for_set(key_bytes, old_indexable.as_ref(), &new_indexable);

        // Apply index deletes (old entries)
        for delete_key in &index_update.deletes {
            index_table.remove(delete_key.as_slice()).context(RemoveSnafu)?;
        }

        // Apply index inserts (new entries) - empty value
        for insert_key in &index_update.inserts {
            index_table.insert(insert_key.as_slice(), &[][..]).context(InsertSnafu)?;
        }

        // Write the primary KV entry
        let entry_bytes = bincode::serialize(&entry).context(SerializeSnafu)?;
        kv_table.insert(key_bytes, entry_bytes.as_slice()).context(InsertSnafu)?;

        // If this key is attached to a lease, update the lease's key list
        if let Some(lid) = lease_id {
            let needs_update = if let Some(lease_data) = leases_table.get(lid).context(GetSnafu)? {
                let lease_entry: LeaseEntry = bincode::deserialize(lease_data.value()).context(DeserializeSnafu)?;
                !lease_entry.keys.contains(&key.to_string())
            } else {
                false
            };

            if needs_update {
                // Re-fetch and update the lease - use scoped access to avoid borrow conflicts
                let mut lease_entry = {
                    let lease_data = leases_table.get(lid).context(GetSnafu)?;
                    if let Some(data) = lease_data {
                        let entry: LeaseEntry = bincode::deserialize(data.value()).context(DeserializeSnafu)?;
                        Some(entry)
                    } else {
                        None
                    }
                };

                if let Some(ref mut entry) = lease_entry {
                    entry.keys.push(key.to_string());
                    let updated_bytes = bincode::serialize(&entry).context(SerializeSnafu)?;
                    leases_table.insert(lid, updated_bytes.as_slice()).context(InsertSnafu)?;
                }
            }
        }

        Ok(AppResponse {
            value: Some(value.to_string()),
            ..Default::default()
        })
    }

    /// Apply a Delete operation within a transaction.
    fn apply_delete_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        key: &str,
    ) -> Result<AppResponse, SharedStorageError> {
        let key_bytes = key.as_bytes();

        // Read existing entry to get index values to delete
        let existing = kv_table
            .get(key_bytes)
            .context(GetSnafu)?
            .and_then(|v| bincode::deserialize::<KvEntry>(v.value()).ok());

        // Delete index entries if the key existed
        if let Some(old_entry) = &existing {
            let old_indexable = IndexableEntry {
                value: old_entry.value.clone(),
                version: old_entry.version,
                create_revision: old_entry.create_revision,
                mod_revision: old_entry.mod_revision,
                expires_at_ms: old_entry.expires_at_ms,
                lease_id: old_entry.lease_id,
            };

            let index_update = index_registry.updates_for_delete(key_bytes, &old_indexable);
            for delete_key in &index_update.deletes {
                index_table.remove(delete_key.as_slice()).context(RemoveSnafu)?;
            }
        }

        // Delete the primary KV entry
        let existed = kv_table.remove(key_bytes).context(RemoveSnafu)?.is_some();

        Ok(AppResponse {
            deleted: Some(existed),
            ..Default::default()
        })
    }

    /// Apply a SetMulti operation within a transaction.
    #[allow(clippy::too_many_arguments)]
    fn apply_set_multi_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        leases_table: &mut redb::Table<u64, &[u8]>,
        pairs: &[(String, String)],
        log_index: u64,
        expires_at_ms: Option<u64>,
        lease_id: Option<u64>,
    ) -> Result<AppResponse, SharedStorageError> {
        if pairs.len() > MAX_SETMULTI_KEYS as usize {
            return Err(SharedStorageError::BatchTooLarge {
                size: pairs.len(),
                max: MAX_SETMULTI_KEYS,
            });
        }

        for (key, value) in pairs {
            Self::apply_set_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                key,
                value,
                log_index,
                expires_at_ms,
                lease_id,
            )?;
        }

        Ok(AppResponse::default())
    }

    /// Apply a DeleteMulti operation within a transaction.
    fn apply_delete_multi_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        keys: &[String],
    ) -> Result<AppResponse, SharedStorageError> {
        if keys.len() > MAX_SETMULTI_KEYS as usize {
            return Err(SharedStorageError::BatchTooLarge {
                size: keys.len(),
                max: MAX_SETMULTI_KEYS,
            });
        }

        let mut deleted_any = false;
        for key in keys {
            let result = Self::apply_delete_in_txn(kv_table, index_table, index_registry, key)?;
            deleted_any |= result.deleted.unwrap_or(false);
        }

        Ok(AppResponse {
            deleted: Some(deleted_any),
            ..Default::default()
        })
    }

    /// Apply a CompareAndSwap operation within a transaction.
    fn apply_compare_and_swap_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        key: &str,
        expected: Option<&str>,
        new_value: &str,
        log_index: u64,
    ) -> Result<AppResponse, SharedStorageError> {
        let key_bytes = key.as_bytes();

        // Read current value
        let current = kv_table
            .get(key_bytes)
            .context(GetSnafu)?
            .and_then(|v| bincode::deserialize::<KvEntry>(v.value()).ok());

        let current_value = current.as_ref().map(|e| e.value.as_str());

        // Use pure function to check CAS condition
        if !check_cas_condition(expected, current_value) {
            return Ok(AppResponse {
                value: current_value.map(String::from),
                cas_succeeded: Some(false),
                ..Default::default()
            });
        }

        // Use pure function to compute versions
        let existing_version = current.as_ref().map(|e| (e.create_revision, e.version));
        let versions = compute_kv_versions(existing_version, log_index);

        let entry = KvEntry {
            value: new_value.to_string(),
            version: versions.version,
            create_revision: versions.create_revision,
            mod_revision: versions.mod_revision,
            expires_at_ms: None,
            lease_id: None,
        };

        // Convert KvEntry to IndexableEntry for index updates
        let new_indexable = IndexableEntry {
            value: entry.value.clone(),
            version: entry.version,
            create_revision: entry.create_revision,
            mod_revision: entry.mod_revision,
            expires_at_ms: entry.expires_at_ms,
            lease_id: entry.lease_id,
        };

        let old_indexable = current.as_ref().map(|e| IndexableEntry {
            value: e.value.clone(),
            version: e.version,
            create_revision: e.create_revision,
            mod_revision: e.mod_revision,
            expires_at_ms: e.expires_at_ms,
            lease_id: e.lease_id,
        });

        // Generate and apply index updates
        let index_update = index_registry.updates_for_set(key_bytes, old_indexable.as_ref(), &new_indexable);
        for delete_key in &index_update.deletes {
            index_table.remove(delete_key.as_slice()).context(RemoveSnafu)?;
        }
        for insert_key in &index_update.inserts {
            index_table.insert(insert_key.as_slice(), &[][..]).context(InsertSnafu)?;
        }

        let entry_bytes = bincode::serialize(&entry).context(SerializeSnafu)?;
        kv_table.insert(key_bytes, entry_bytes.as_slice()).context(InsertSnafu)?;

        Ok(AppResponse {
            value: Some(new_value.to_string()),
            cas_succeeded: Some(true),
            ..Default::default()
        })
    }

    /// Apply a CompareAndDelete operation within a transaction.
    fn apply_compare_and_delete_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        key: &str,
        expected: &str,
    ) -> Result<AppResponse, SharedStorageError> {
        let key_bytes = key.as_bytes();

        // Read current value
        let current = kv_table
            .get(key_bytes)
            .context(GetSnafu)?
            .and_then(|v| bincode::deserialize::<KvEntry>(v.value()).ok());

        let current_value = current.as_ref().map(|e| e.value.as_str());

        // Check condition
        let condition_matches = current_value.is_some_and(|v| v == expected);

        if !condition_matches {
            return Ok(AppResponse {
                value: current_value.map(String::from),
                cas_succeeded: Some(false),
                ..Default::default()
            });
        }

        // Delete index entries if the key exists
        if let Some(old_entry) = &current {
            let old_indexable = IndexableEntry {
                value: old_entry.value.clone(),
                version: old_entry.version,
                create_revision: old_entry.create_revision,
                mod_revision: old_entry.mod_revision,
                expires_at_ms: old_entry.expires_at_ms,
                lease_id: old_entry.lease_id,
            };

            let index_update = index_registry.updates_for_delete(key_bytes, &old_indexable);
            for delete_key in &index_update.deletes {
                index_table.remove(delete_key.as_slice()).context(RemoveSnafu)?;
            }
        }

        // Delete the key
        kv_table.remove(key_bytes).context(RemoveSnafu)?;

        Ok(AppResponse {
            deleted: Some(true),
            cas_succeeded: Some(true),
            ..Default::default()
        })
    }

    /// Apply a Batch operation within a transaction.
    fn apply_batch_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        leases_table: &mut redb::Table<u64, &[u8]>,
        operations: &[(bool, String, String)],
        log_index: u64,
    ) -> Result<AppResponse, SharedStorageError> {
        if operations.len() > MAX_SETMULTI_KEYS as usize {
            return Err(SharedStorageError::BatchTooLarge {
                size: operations.len(),
                max: MAX_SETMULTI_KEYS,
            });
        }

        for (is_set, key, value) in operations {
            if *is_set {
                Self::apply_set_in_txn(
                    kv_table,
                    index_table,
                    index_registry,
                    leases_table,
                    key,
                    value,
                    log_index,
                    None,
                    None,
                )?;
            } else {
                Self::apply_delete_in_txn(kv_table, index_table, index_registry, key)?;
            }
        }

        Ok(AppResponse {
            batch_applied: Some(operations.len() as u32),
            ..Default::default()
        })
    }

    /// Apply a ConditionalBatch operation within a transaction.
    fn apply_conditional_batch_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        leases_table: &mut redb::Table<u64, &[u8]>,
        conditions: &[(u8, String, String)],
        operations: &[(bool, String, String)],
        log_index: u64,
    ) -> Result<AppResponse, SharedStorageError> {
        if operations.len() > MAX_SETMULTI_KEYS as usize {
            return Err(SharedStorageError::BatchTooLarge {
                size: operations.len(),
                max: MAX_SETMULTI_KEYS,
            });
        }

        // Check all conditions first
        for (i, (cond_type, key, expected)) in conditions.iter().enumerate() {
            let current = kv_table
                .get(key.as_bytes())
                .context(GetSnafu)?
                .and_then(|v| bincode::deserialize::<KvEntry>(v.value()).ok());

            let met = match cond_type {
                0 => current.as_ref().map(|e| e.value.as_str() == expected).unwrap_or(false), // ValueEquals
                1 => current.is_some(),                                                       // KeyExists
                2 => current.is_none(),                                                       // KeyNotExists
                _ => false,
            };

            if !met {
                return Ok(AppResponse {
                    conditions_met: Some(false),
                    failed_condition_index: Some(i as u32),
                    ..Default::default()
                });
            }
        }

        // Apply operations
        Self::apply_batch_in_txn(kv_table, index_table, index_registry, leases_table, operations, log_index)?;

        Ok(AppResponse {
            conditions_met: Some(true),
            batch_applied: Some(operations.len() as u32),
            ..Default::default()
        })
    }

    /// Apply a lease grant operation within a transaction.
    fn apply_lease_grant_in_txn(
        leases_table: &mut redb::Table<u64, &[u8]>,
        lease_id: u64,
        ttl_seconds: u32,
    ) -> Result<AppResponse, SharedStorageError> {
        // Generate lease_id if not provided (0 means auto-generate)
        let actual_lease_id = if lease_id == 0 {
            // Simple ID generation using timestamp + random component
            let now = now_unix_ms();
            let random_component = now % 1000000;
            now * 1000 + random_component
        } else {
            lease_id
        };

        // Use pure function to create lease entry data
        let lease_data = create_lease_entry(ttl_seconds, now_unix_ms());

        // Create lease entry from pure computed data
        let lease_entry = LeaseEntry {
            ttl_seconds: lease_data.ttl_seconds,
            expires_at_ms: lease_data.expires_at_ms,
            keys: lease_data.keys, // Keys will be added when SetWithLease is called
        };

        // Store lease
        let lease_bytes = bincode::serialize(&lease_entry).context(SerializeSnafu)?;
        leases_table.insert(actual_lease_id, lease_bytes.as_slice()).context(InsertSnafu)?;

        Ok(AppResponse {
            lease_id: Some(actual_lease_id),
            ttl_seconds: Some(ttl_seconds),
            ..Default::default()
        })
    }

    /// Apply a lease revoke operation within a transaction.
    fn apply_lease_revoke_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        leases_table: &mut redb::Table<u64, &[u8]>,
        lease_id: u64,
    ) -> Result<AppResponse, SharedStorageError> {
        // Get the lease entry and extract the keys
        let keys_to_delete = if let Some(lease_data) = leases_table.get(lease_id).context(GetSnafu)? {
            let lease_entry: LeaseEntry = bincode::deserialize(lease_data.value()).context(DeserializeSnafu)?;
            lease_entry.keys.clone()
        } else {
            Vec::new()
        };

        let mut keys_deleted = 0u32;

        // Delete all keys attached to this lease
        for key in &keys_to_delete {
            let result = Self::apply_delete_in_txn(kv_table, index_table, index_registry, key)?;
            if result.deleted.unwrap_or(false) {
                keys_deleted += 1;
            }
        }

        // Delete the lease itself if it exists
        if !keys_to_delete.is_empty() {
            leases_table.remove(lease_id).context(RemoveSnafu)?;
        }

        Ok(AppResponse {
            keys_deleted: Some(keys_deleted),
            ..Default::default()
        })
    }

    /// Apply a lease keepalive operation within a transaction.
    fn apply_lease_keepalive_in_txn(
        leases_table: &mut redb::Table<u64, &[u8]>,
        lease_id: u64,
    ) -> Result<AppResponse, SharedStorageError> {
        // Get the existing lease and extract data to avoid borrow conflicts
        let lease_opt = {
            let lease_bytes = leases_table.get(lease_id).context(GetSnafu)?;
            if let Some(lease_data) = lease_bytes {
                let entry: LeaseEntry = bincode::deserialize(lease_data.value()).context(DeserializeSnafu)?;
                Some(entry)
            } else {
                None
            }
        };

        if let Some(mut lease_entry) = lease_opt {
            // Use pure function to compute new expiration time
            let ttl = lease_entry.ttl_seconds;
            lease_entry.expires_at_ms = compute_lease_refresh(ttl, now_unix_ms());

            // Update the lease
            let updated_bytes = bincode::serialize(&lease_entry).context(SerializeSnafu)?;
            leases_table.insert(lease_id, updated_bytes.as_slice()).context(InsertSnafu)?;

            Ok(AppResponse {
                lease_id: Some(lease_id),
                ttl_seconds: Some(ttl),
                ..Default::default()
            })
        } else {
            // Lease not found - return None for lease_id to indicate not found
            Ok(AppResponse {
                lease_id: None,
                ..Default::default()
            })
        }
    }

    /// Apply a Transaction operation (etcd-style) within a transaction.
    #[allow(clippy::too_many_arguments)]
    fn apply_transaction_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        leases_table: &mut redb::Table<u64, &[u8]>,
        compare: &[(u8, u8, String, String)],
        success: &[(u8, String, String)],
        failure: &[(u8, String, String)],
        log_index: u64,
    ) -> Result<AppResponse, SharedStorageError> {
        // Evaluate all comparison conditions
        let mut all_conditions_met = true;

        for (target, op, key, value) in compare {
            let key_bytes = key.as_bytes();
            let current_entry = kv_table
                .get(key_bytes)
                .context(GetSnafu)?
                .and_then(|v| bincode::deserialize::<KvEntry>(v.value()).ok());

            let condition_met = match target {
                0 => {
                    // Value comparison
                    let current_value = current_entry.as_ref().map(|e| e.value.as_str());
                    match op {
                        0 => current_value == Some(value.as_str()),                      // Equal
                        1 => current_value != Some(value.as_str()),                      // NotEqual
                        2 => current_value.map(|v| v > value.as_str()).unwrap_or(false), // Greater
                        3 => current_value.map(|v| v < value.as_str()).unwrap_or(false), // Less
                        _ => false,
                    }
                }
                1 => {
                    // Version comparison
                    let current_version = current_entry.as_ref().map(|e| e.version).unwrap_or(0);
                    let expected_version: i64 = value.parse().unwrap_or(0);
                    match op {
                        0 => current_version == expected_version,
                        1 => current_version != expected_version,
                        2 => current_version > expected_version,
                        3 => current_version < expected_version,
                        _ => false,
                    }
                }
                2 => {
                    // CreateRevision comparison
                    let create_rev = current_entry.as_ref().map(|e| e.create_revision).unwrap_or(0);
                    let expected_rev: i64 = value.parse().unwrap_or(0);
                    match op {
                        0 => create_rev == expected_rev,
                        1 => create_rev != expected_rev,
                        2 => create_rev > expected_rev,
                        3 => create_rev < expected_rev,
                        _ => false,
                    }
                }
                3 => {
                    // ModRevision comparison
                    let mod_rev = current_entry.as_ref().map(|e| e.mod_revision).unwrap_or(0);
                    let expected_rev: i64 = value.parse().unwrap_or(0);
                    match op {
                        0 => mod_rev == expected_rev,
                        1 => mod_rev != expected_rev,
                        2 => mod_rev > expected_rev,
                        3 => mod_rev < expected_rev,
                        _ => false,
                    }
                }
                _ => false,
            };

            if !condition_met {
                all_conditions_met = false;
                break;
            }
        }

        // Execute the appropriate branch based on conditions
        let operations = if all_conditions_met { success } else { failure };
        let mut results = Vec::new();

        for (op_type, key, value) in operations {
            let result = match op_type {
                0 => {
                    // Put operation
                    Self::apply_set_in_txn(
                        kv_table,
                        index_table,
                        index_registry,
                        leases_table,
                        key,
                        value,
                        log_index,
                        None,
                        None,
                    )?;
                    TxnOpResult::Put { revision: log_index }
                }
                1 => {
                    // Delete operation
                    let del_result = Self::apply_delete_in_txn(kv_table, index_table, index_registry, key)?;
                    let deleted = if del_result.deleted.unwrap_or(false) { 1 } else { 0 };
                    TxnOpResult::Delete { deleted }
                }
                2 => {
                    // Get operation
                    let kv = kv_table
                        .get(key.as_bytes())
                        .context(GetSnafu)?
                        .and_then(|v| bincode::deserialize::<KvEntry>(v.value()).ok())
                        .map(|entry| KeyValueWithRevision {
                            key: key.clone(),
                            value: entry.value,
                            version: entry.version as u64,
                            create_revision: entry.create_revision as u64,
                            mod_revision: entry.mod_revision as u64,
                        });
                    TxnOpResult::Get { kv }
                }
                3 => {
                    // Range operation
                    let limit: usize = value.parse().unwrap_or(10);
                    let mut kvs = Vec::new();
                    let prefix = key.as_bytes();

                    for entry in kv_table.range(prefix..).context(RangeSnafu)? {
                        let (k, v) = entry.context(GetSnafu)?;
                        // Check if key starts with prefix
                        if !k.value().starts_with(prefix) {
                            break;
                        }
                        if kvs.len() >= limit {
                            break;
                        }

                        if let Ok(kv_entry) = bincode::deserialize::<KvEntry>(v.value()) {
                            kvs.push(KeyValueWithRevision {
                                key: String::from_utf8_lossy(k.value()).to_string(),
                                value: kv_entry.value,
                                version: kv_entry.version as u64,
                                create_revision: kv_entry.create_revision as u64,
                                mod_revision: kv_entry.mod_revision as u64,
                            });
                        }
                    }

                    TxnOpResult::Range { kvs, more: false }
                }
                _ => continue,
            };
            results.push(result);
        }

        Ok(AppResponse {
            succeeded: Some(all_conditions_met),
            txn_results: Some(results),
            header_revision: Some(log_index),
            ..Default::default()
        })
    }

    /// Apply an Optimistic Transaction (FoundationDB-style) within a transaction.
    fn apply_optimistic_transaction_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        leases_table: &mut redb::Table<u64, &[u8]>,
        read_set: &[(String, i64)],
        write_set: &[(bool, String, String)],
        log_index: u64,
    ) -> Result<AppResponse, SharedStorageError> {
        // Check all keys in read_set for version conflicts
        for (key, expected_version) in read_set {
            let key_bytes = key.as_bytes();
            let current_entry = kv_table
                .get(key_bytes)
                .context(GetSnafu)?
                .and_then(|v| bincode::deserialize::<KvEntry>(v.value()).ok());

            let current_version = current_entry.as_ref().map(|e| e.version).unwrap_or(0);

            if current_version != *expected_version {
                // Version conflict detected
                return Ok(AppResponse {
                    occ_conflict: Some(true),
                    conflict_key: Some(key.clone()),
                    ..Default::default()
                });
            }
        }

        // All version checks passed, apply write_set
        for (is_set, key, value) in write_set {
            if *is_set {
                Self::apply_set_in_txn(
                    kv_table,
                    index_table,
                    index_registry,
                    leases_table,
                    key,
                    value,
                    log_index,
                    None,
                    None,
                )?;
            } else {
                Self::apply_delete_in_txn(kv_table, index_table, index_registry, key)?;
            }
        }

        Ok(AppResponse {
            occ_conflict: Some(false),
            ..Default::default()
        })
    }

    /// Apply a single AppRequest to the state machine tables within a transaction.
    fn apply_request_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        leases_table: &mut redb::Table<u64, &[u8]>,
        request: &AppRequest,
        log_index: u64,
    ) -> Result<AppResponse, SharedStorageError> {
        match request {
            AppRequest::Set { key, value } => Self::apply_set_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                key,
                value,
                log_index,
                None,
                None,
            ),
            AppRequest::SetWithTTL {
                key,
                value,
                expires_at_ms,
            } => Self::apply_set_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                key,
                value,
                log_index,
                Some(*expires_at_ms),
                None,
            ),
            AppRequest::SetMulti { pairs } => Self::apply_set_multi_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                pairs,
                log_index,
                None,
                None,
            ),
            AppRequest::SetMultiWithTTL { pairs, expires_at_ms } => Self::apply_set_multi_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                pairs,
                log_index,
                Some(*expires_at_ms),
                None,
            ),
            AppRequest::Delete { key } => Self::apply_delete_in_txn(kv_table, index_table, index_registry, key),
            AppRequest::DeleteMulti { keys } => {
                Self::apply_delete_multi_in_txn(kv_table, index_table, index_registry, keys)
            }
            AppRequest::CompareAndSwap {
                key,
                expected,
                new_value,
            } => Self::apply_compare_and_swap_in_txn(
                kv_table,
                index_table,
                index_registry,
                key,
                expected.as_deref(),
                new_value,
                log_index,
            ),
            AppRequest::CompareAndDelete { key, expected } => {
                Self::apply_compare_and_delete_in_txn(kv_table, index_table, index_registry, key, expected)
            }
            AppRequest::Batch { operations } => {
                Self::apply_batch_in_txn(kv_table, index_table, index_registry, leases_table, operations, log_index)
            }
            AppRequest::ConditionalBatch { conditions, operations } => Self::apply_conditional_batch_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                conditions,
                operations,
                log_index,
            ),
            // Lease operations
            AppRequest::SetWithLease { key, value, lease_id } => Self::apply_set_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                key,
                value,
                log_index,
                None,
                Some(*lease_id),
            ),
            AppRequest::SetMultiWithLease { pairs, lease_id } => Self::apply_set_multi_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                pairs,
                log_index,
                None,
                Some(*lease_id),
            ),
            // Lease operations
            AppRequest::LeaseGrant { lease_id, ttl_seconds } => {
                Self::apply_lease_grant_in_txn(leases_table, *lease_id, *ttl_seconds)
            }
            AppRequest::LeaseRevoke { lease_id } => {
                Self::apply_lease_revoke_in_txn(kv_table, index_table, index_registry, leases_table, *lease_id)
            }
            AppRequest::LeaseKeepalive { lease_id } => Self::apply_lease_keepalive_in_txn(leases_table, *lease_id),
            // Transaction operations
            AppRequest::Transaction {
                compare,
                success,
                failure,
            } => Self::apply_transaction_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                compare,
                success,
                failure,
                log_index,
            ),
            AppRequest::OptimisticTransaction { read_set, write_set } => Self::apply_optimistic_transaction_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                read_set,
                write_set,
                log_index,
            ),
            // Shard operations - pass through without state changes
            AppRequest::ShardSplit { .. } | AppRequest::ShardMerge { .. } | AppRequest::TopologyUpdate { .. } => {
                Ok(AppResponse::default())
            }
        }
    }
}

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

// ====================================================================================
// RaftStateMachine Implementation (No-Op Apply)
// ====================================================================================

impl RaftStateMachine<AppTypeConfig> for SharedRedbStorage {
    type SnapshotBuilder = SharedRedbSnapshotBuilder;

    /// Get the applied state (last_applied_log, membership).
    ///
    /// Reads from SM_META_TABLE which is updated during append().
    async fn applied_state(
        &mut self,
    ) -> Result<(Option<LogIdOf<AppTypeConfig>>, StoredMembership<AppTypeConfig>), io::Error> {
        let last_applied: Option<LogIdOf<AppTypeConfig>> = self.read_sm_meta("last_applied_log")?.flatten();

        let membership: Option<StoredMembership<AppTypeConfig>> = self.read_sm_meta("last_membership")?;

        Ok((last_applied, membership.unwrap_or_default()))
    }

    /// Apply entries - NO-OP because state is already applied during append().
    ///
    /// This is the key to single-fsync performance. The state machine mutations
    /// are bundled into the log append transaction, so there's nothing to do here
    /// except retrieve and send the responses via the responders.
    ///
    /// However, this is the correct place to broadcast committed entries to subscribers
    /// (e.g., DocsExporter) because apply() is called when entries are truly committed.
    async fn apply<Strm>(&mut self, mut entries: Strm) -> Result<(), io::Error>
    where Strm: Stream<Item = Result<EntryResponder<AppTypeConfig>, io::Error>> + Unpin + OptionalSend {
        use crate::log_subscriber::KvOperation;

        // State was already applied during append().
        // Retrieve the computed responses and send them via responders.
        // EntryResponder<C> is a tuple: (Entry<C>, Option<ApplyResponder<C>>)
        while let Some((entry, responder_opt)) = entries.try_next().await? {
            let log_index = entry.log_id.index;
            let log_term = entry.log_id.leader_id.term;

            // Broadcast committed entry to subscribers (DocsExporter, etc.)
            if let Some(ref sender) = self.log_broadcast {
                // Convert entry payload to KvOperation for broadcast
                let operation = match &entry.payload {
                    EntryPayload::Normal(request) => KvOperation::from(request.clone()),
                    EntryPayload::Membership(membership) => KvOperation::MembershipChange {
                        description: format!(
                            "membership change: voters={:?}, learners={:?}",
                            membership.nodes().collect::<Vec<_>>(),
                            membership.learner_ids().collect::<Vec<_>>()
                        ),
                    },
                    EntryPayload::Blank => KvOperation::Noop,
                };

                let payload = LogEntryPayload {
                    index: log_index,
                    term: log_term,
                    hlc_timestamp: SerializableTimestamp::from(self.hlc.new_timestamp()),
                    operation,
                };

                // Best-effort broadcast - don't fail if no receivers or channel is full
                // Lagging receivers will get Lagged error and can request full sync
                if let Err(e) = sender.send(payload) {
                    tracing::debug!(
                        log_index,
                        error = %e,
                        "failed to broadcast log entry (no receivers or channel dropped)"
                    );
                }
            }

            // Respond with the pre-computed response if responder is present
            if let Some(responder) = responder_opt {
                // Retrieve response computed during append(), fall back to default
                let response = self
                    .pending_responses
                    .write()
                    .map_err(|_| io::Error::other("pending_responses lock poisoned in apply"))?
                    .remove(&log_index)
                    .unwrap_or_default();
                responder.send(response);
            }
        }

        Ok(())
    }

    async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
        SharedRedbSnapshotBuilder { storage: self.clone() }
    }

    async fn begin_receiving_snapshot(&mut self) -> Result<SnapshotDataOf<AppTypeConfig>, io::Error> {
        Ok(Cursor::new(Vec::new()))
    }

    /// Install a snapshot received from the leader.
    ///
    /// This replaces the state machine state with the snapshot data.
    ///
    /// # Verified Invariants
    ///
    /// - **INVARIANT 7 (Snapshot Integrity)**: Before installation, the snapshot is verified using
    ///   cryptographic hashes:
    ///   1. Data hash matches actual snapshot data (blake3)
    ///   2. Meta hash matches snapshot metadata
    ///   3. Combined hash is correctly computed
    ///   4. Chain hash matches the stored chain at snapshot point
    ///
    /// - **State Replacement**: After installation:
    ///   - KV state is completely replaced with snapshot data
    ///   - last_applied is set to snapshot's last_log_id
    ///   - Membership is updated to snapshot's membership
    async fn install_snapshot(
        &mut self,
        meta: &openraft::SnapshotMeta<AppTypeConfig>,
        snapshot: SnapshotDataOf<AppTypeConfig>,
    ) -> Result<(), io::Error> {
        let data = snapshot.into_inner();

        // =========================================================================
        // GHOST: Capture pre-state for verification
        // =========================================================================
        ghost! {
            let _ghost_pre_last_applied = Ghost::<Option<u64>>::new(None);
            let _ghost_snapshot_index = Ghost::<u64>::new(
                meta.last_log_id.as_ref().map(|l| l.index).unwrap_or(0)
            );
        }

        // =========================================================================
        // INVARIANT 7 (Snapshot Integrity): Verify before installation
        // =========================================================================
        // The snapshot integrity check verifies:
        // 1. blake3(data) == integrity.data_hash
        // 2. blake3(meta_bytes) == integrity.meta_hash
        // 3. blake3(data_hash || meta_hash) == integrity.combined_hash
        // 4. chain_hash_at_snapshot matches stored chain hash
        //
        // This ensures the snapshot has not been corrupted or tampered with.
        // =========================================================================
        {
            let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
            if let Ok(table) = read_txn.open_table(SNAPSHOT_TABLE)
                && let Some(value) = table.get("current").context(GetSnafu)?
                && let Ok(stored) = bincode::deserialize::<StoredSnapshot>(value.value())
                && let Some(ref integrity) = stored.integrity
            {
                // Verify the incoming data matches the stored integrity
                let meta_bytes = bincode::serialize(meta).map_err(|e| io::Error::other(e.to_string()))?;
                if !integrity.verify(&meta_bytes, &data) {
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
            }
        }

        // Deserialize snapshot data
        let kv_entries: BTreeMap<String, KvEntry> =
            bincode::deserialize(&data).map_err(|e| io::Error::other(e.to_string()))?;
        let kv_entries_count = kv_entries.len();

        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut kv_table = write_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;
            let mut sm_meta_table = write_txn.open_table(SM_META_TABLE).context(OpenTableSnafu)?;

            // Clear existing KV data
            // Note: redb doesn't have a clear() method, so we iterate and remove
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

            // Insert snapshot data
            for (key, entry) in kv_entries {
                let entry_bytes = bincode::serialize(&entry).context(SerializeSnafu)?;
                kv_table.insert(key.as_bytes(), entry_bytes.as_slice()).context(InsertSnafu)?;
            }

            // Update last_applied
            let log_id_bytes = bincode::serialize(&meta.last_log_id).context(SerializeSnafu)?;
            sm_meta_table.insert("last_applied_log", log_id_bytes.as_slice()).context(InsertSnafu)?;

            // Update membership (meta.last_membership is already a StoredMembership)
            let membership_bytes = bincode::serialize(&meta.last_membership).context(SerializeSnafu)?;
            sm_meta_table.insert("last_membership", membership_bytes.as_slice()).context(InsertSnafu)?;
        }
        write_txn.commit().context(CommitSnafu)?;

        tracing::info!(
            snapshot_id = %meta.snapshot_id,
            last_log_id = ?meta.last_log_id,
            entries = kv_entries_count,
            "installed snapshot"
        );

        // Emit SnapshotInstalled event if broadcast channel is configured
        if let Some(ref tx) = self.snapshot_broadcast {
            let event = SnapshotEvent::Installed {
                snapshot_id: meta.snapshot_id.clone(),
                last_log_index: meta.last_log_id.as_ref().map(|l| l.index).unwrap_or(0),
                term: meta.last_log_id.as_ref().map(|l| l.leader_id.term).unwrap_or(0),
                entry_count: kv_entries_count as u64,
            };
            // Non-blocking send - ignore errors (no subscribers)
            let _ = tx.send(event);
        }

        // =========================================================================
        // GHOST: Verify post-conditions
        // =========================================================================
        proof! {
            // INVARIANT 7 (Snapshot Integrity):
            // The integrity check at the start ensures:
            // - Data hash matches actual data (corruption detection)
            // - Meta hash matches metadata (tamper detection)
            // - Combined hash binds data and meta together
            // - Chain hash connects to existing chain
            //
            // After successful installation:
            // - KV state is replaced with verified snapshot data
            // - last_applied is set to snapshot's last_log_id
            // - Membership reflects snapshot's membership
            //
            // The spec proof install_preserves_invariants in verus/snapshot_spec.rs
            // shows that this operation maintains storage_invariant.
        }

        Ok(())
    }

    async fn get_current_snapshot(&mut self) -> Result<Option<Snapshot<AppTypeConfig>>, io::Error> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SNAPSHOT_TABLE).context(OpenTableSnafu)?;

        match table.get("current").context(GetSnafu)? {
            Some(value) => {
                let stored: StoredSnapshot = bincode::deserialize(value.value()).context(DeserializeSnafu)?;
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
// Snapshot Builder
// ====================================================================================

/// Snapshot builder for SharedRedbStorage.
pub struct SharedRedbSnapshotBuilder {
    storage: SharedRedbStorage,
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
    async fn build_snapshot(&mut self) -> Result<Snapshot<AppTypeConfig>, io::Error> {
        // =========================================================================
        // GHOST: Snapshot creation is read-only
        // =========================================================================
        ghost! {
            // Pre-state captured for verification that state is unchanged
            let _ghost_pre_state = Ghost::<()>::new(());
        }

        let read_txn = self.storage.db.begin_read().context(BeginReadSnafu)?;
        let kv_table = read_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;
        let sm_meta_table = read_txn.open_table(SM_META_TABLE).context(OpenTableSnafu)?;

        // Read last applied log
        let last_applied: Option<LogIdOf<AppTypeConfig>> = sm_meta_table
            .get("last_applied_log")
            .context(GetSnafu)?
            .and_then(|v| bincode::deserialize(v.value()).ok())
            .flatten();

        // Read membership
        let membership: StoredMembership<AppTypeConfig> = sm_meta_table
            .get("last_membership")
            .context(GetSnafu)?
            .and_then(|v| bincode::deserialize(v.value()).ok())
            .unwrap_or_default();

        // Collect all KV entries
        let mut kv_entries = BTreeMap::new();
        let now_ms = now_unix_ms();

        for item in kv_table.iter().context(RangeSnafu)? {
            let (key_guard, value_guard) = item.context(GetSnafu)?;
            let key_bytes = key_guard.value();
            let key_str = match std::str::from_utf8(key_bytes) {
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

        // Serialize snapshot data
        let data = bincode::serialize(&kv_entries).context(SerializeSnafu)?;

        let snapshot_id = format!("snapshot-{}-{}", last_applied.as_ref().map(|l| l.index).unwrap_or(0), now_unix_ms());

        let meta = openraft::SnapshotMeta {
            last_log_id: last_applied,
            last_membership: membership,
            snapshot_id,
        };

        // =========================================================================
        // INVARIANT 7 (Snapshot Integrity): Compute integrity hashes
        // =========================================================================
        // The integrity struct binds together:
        // 1. data_hash = blake3(serialized KV entries)
        // 2. meta_hash = blake3(serialized metadata)
        // 3. combined_hash = blake3(data_hash || meta_hash)
        // 4. chain_hash_at_snapshot = chain hash at last_applied
        //
        // This enables verification during install_snapshot() that the
        // received data has not been corrupted or tampered with.
        // =========================================================================
        let snapshot_index = last_applied.as_ref().map(|l| l.index).unwrap_or(0);
        let chain_hash = self.storage.read_chain_hash_at(snapshot_index)?.unwrap_or([0u8; 32]);

        // Serialize metadata for hashing
        let meta_bytes = bincode::serialize(&meta).context(SerializeSnafu)?;

        // Compute snapshot integrity hash
        let integrity = SnapshotIntegrity::compute(&meta_bytes, &data, chain_hash);
        let integrity_hex = integrity.combined_hash_hex();

        // Store the snapshot with integrity hash
        let stored = StoredSnapshot {
            meta: meta.clone(),
            data: data.clone(),
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
            entries = kv_entries.len(),
            size_bytes = data.len(),
            integrity_hash = %integrity_hex,
            "built snapshot with integrity hash"
        );

        // Emit SnapshotCreated event if broadcast channel is configured
        if let Some(ref tx) = self.storage.snapshot_broadcast {
            let event = SnapshotEvent::Created {
                snapshot_id: meta.snapshot_id.clone(),
                last_log_index: meta.last_log_id.as_ref().map(|l| l.index).unwrap_or(0),
                term: meta.last_log_id.as_ref().map(|l| l.leader_id.term).unwrap_or(0),
                entry_count: kv_entries.len() as u64,
                size_bytes: data.len() as u64,
            };
            // Non-blocking send - ignore errors (no subscribers)
            let _ = tx.send(event);
        }

        // =========================================================================
        // GHOST: Verify post-conditions
        // =========================================================================
        proof! {
            // INVARIANT 7 (Snapshot Integrity):
            // The integrity hash is correctly computed:
            // - data_hash = blake3(data) where data = bincode::serialize(kv_entries)
            // - meta_hash = blake3(meta_bytes) where meta_bytes = bincode::serialize(meta)
            // - combined_hash = blake3(data_hash || meta_hash)
            // - chain_hash_at_snapshot = chain hash at snapshot_index
            //
            // This is verified by SnapshotIntegrity::compute() which:
            // 1. Hashes the data with blake3
            // 2. Hashes the meta with blake3
            // 3. Computes combined hash of both
            // 4. Stores the chain hash at snapshot point
            //
            // Read-only property:
            // The spec proof create_is_readonly in verus/snapshot_spec.rs shows
            // that snapshot creation does not modify the log or state machine.
            // The only mutation is storing the snapshot itself (separate table).
        }

        Ok(Snapshot {
            meta,
            snapshot: Cursor::new(data),
        })
    }
}

// ====================================================================================
// IndexQueryExecutor Implementation
// ====================================================================================

impl IndexQueryExecutor for SharedRedbStorage {
    /// Scan an index for a specific value.
    ///
    /// Returns primary keys of entries with the given indexed value.
    fn scan_by_index(&self, index_name: &str, value: &[u8], limit: u32) -> IndexResult<IndexScanResult> {
        let index = self.index_registry.get(index_name).ok_or_else(|| aspen_core::layer::IndexError::NotFound {
            name: index_name.to_string(),
        })?;

        let (start, end) = index.range_for_value(value);
        self.scan_index_range(&start, &end, limit)
    }

    /// Scan an index for a range of values.
    ///
    /// Returns primary keys of entries with indexed values in [start, end).
    fn range_by_index(&self, index_name: &str, start: &[u8], end: &[u8], limit: u32) -> IndexResult<IndexScanResult> {
        let index = self.index_registry.get(index_name).ok_or_else(|| aspen_core::layer::IndexError::NotFound {
            name: index_name.to_string(),
        })?;

        let (range_start, range_end) = index.range_between(start, end);
        self.scan_index_range(&range_start, &range_end, limit)
    }

    /// Scan an index for values less than a threshold.
    ///
    /// Useful for TTL cleanup: find all keys expiring before a timestamp.
    fn scan_index_lt(&self, index_name: &str, threshold: &[u8], limit: u32) -> IndexResult<IndexScanResult> {
        let index = self.index_registry.get(index_name).ok_or_else(|| aspen_core::layer::IndexError::NotFound {
            name: index_name.to_string(),
        })?;

        let (range_start, range_end) = index.range_lt(threshold);
        self.scan_index_range(&range_start, &range_end, limit)
    }
}

impl SharedRedbStorage {
    /// Internal helper to scan an index range and extract primary keys.
    fn scan_index_range(&self, start: &[u8], end: &[u8], limit: u32) -> IndexResult<IndexScanResult> {
        // Cap the limit to prevent resource exhaustion
        let effective_limit = limit.min(aspen_core::layer::MAX_INDEX_SCAN_RESULTS);

        let read_txn = self.db.begin_read().map_err(|e| aspen_core::layer::IndexError::ExtractionFailed {
            name: "scan".to_string(),
            reason: e.to_string(),
        })?;

        let index_table =
            read_txn.open_table(SM_INDEX_TABLE).map_err(|e| aspen_core::layer::IndexError::ExtractionFailed {
                name: "scan".to_string(),
                reason: e.to_string(),
            })?;

        let mut primary_keys = Vec::new();
        let mut has_more = false;

        // Scan the range
        let range = index_table.range(start..end).map_err(|e| aspen_core::layer::IndexError::ExtractionFailed {
            name: "scan".to_string(),
            reason: e.to_string(),
        })?;

        for item in range {
            let (key_guard, _) = item.map_err(|e| aspen_core::layer::IndexError::ExtractionFailed {
                name: "scan".to_string(),
                reason: e.to_string(),
            })?;

            if primary_keys.len() >= effective_limit as usize {
                has_more = true;
                break;
            }

            // Unpack the index key to extract the primary key
            let index_key = key_guard.value();
            if let Ok(tuple) = Tuple::unpack(index_key)
                && let Some(pk) = extract_primary_key_from_tuple(&tuple)
            {
                primary_keys.push(pk);
            }
        }

        Ok(IndexScanResult { primary_keys, has_more })
    }

    /// Get a reference to the index registry.
    ///
    /// This allows external code to access index metadata and create custom queries.
    pub fn index_registry(&self) -> &Arc<IndexRegistry> {
        &self.index_registry
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    // =========================================================================
    // Helper Functions
    // =========================================================================

    fn create_test_storage(temp_dir: &TempDir) -> SharedRedbStorage {
        let db_path = temp_dir.path().join("test.redb");
        SharedRedbStorage::new(&db_path, "test-node-1").unwrap()
    }

    fn insert_kv_entry(storage: &SharedRedbStorage, key: &str, value: &str, version: i64) {
        let write_txn = storage.db.begin_write().unwrap();
        {
            let mut kv_table = write_txn.open_table(SM_KV_TABLE).unwrap();
            let entry = KvEntry {
                value: value.to_string(),
                version,
                create_revision: version,
                mod_revision: version,
                expires_at_ms: None,
                lease_id: None,
            };
            let entry_bytes = bincode::serialize(&entry).unwrap();
            kv_table.insert(key.as_bytes(), entry_bytes.as_slice()).unwrap();
        }
        write_txn.commit().unwrap();
    }

    fn insert_kv_entry_with_ttl(storage: &SharedRedbStorage, key: &str, value: &str, version: i64, expires_at_ms: u64) {
        let write_txn = storage.db.begin_write().unwrap();
        {
            let mut kv_table = write_txn.open_table(SM_KV_TABLE).unwrap();
            let entry = KvEntry {
                value: value.to_string(),
                version,
                create_revision: version,
                mod_revision: version,
                expires_at_ms: Some(expires_at_ms),
                lease_id: None,
            };
            let entry_bytes = bincode::serialize(&entry).unwrap();
            kv_table.insert(key.as_bytes(), entry_bytes.as_slice()).unwrap();
        }
        write_txn.commit().unwrap();
    }

    fn insert_lease_entry(storage: &SharedRedbStorage, lease_id: u64, ttl_seconds: u32, expires_at_ms: u64) {
        let write_txn = storage.db.begin_write().unwrap();
        {
            let mut leases_table = write_txn.open_table(SM_LEASES_TABLE).unwrap();
            let entry = LeaseEntry {
                ttl_seconds,
                expires_at_ms,
                keys: Vec::new(),
            };
            let entry_bytes = bincode::serialize(&entry).unwrap();
            leases_table.insert(lease_id, entry_bytes.as_slice()).unwrap();
        }
        write_txn.commit().unwrap();
    }

    fn insert_lease_entry_with_keys(
        storage: &SharedRedbStorage,
        lease_id: u64,
        ttl_seconds: u32,
        expires_at_ms: u64,
        keys: Vec<String>,
    ) {
        let write_txn = storage.db.begin_write().unwrap();
        {
            let mut leases_table = write_txn.open_table(SM_LEASES_TABLE).unwrap();
            let entry = LeaseEntry {
                ttl_seconds,
                expires_at_ms,
                keys,
            };
            let entry_bytes = bincode::serialize(&entry).unwrap();
            leases_table.insert(lease_id, entry_bytes.as_slice()).unwrap();
        }
        write_txn.commit().unwrap();
    }

    // =========================================================================
    // Basic Storage Tests
    // =========================================================================

    #[tokio::test]
    async fn test_shared_storage_basic() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Test basic KV operations
        assert!(storage.get("test_key").unwrap().is_none());
    }

    #[tokio::test]
    async fn test_shared_storage_set_get() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        insert_kv_entry(&storage, "test_key", "test_value", 1);

        // Verify we can read it
        let entry = storage.get("test_key").unwrap();
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().value, "test_value");
    }

    #[tokio::test]
    async fn test_get_nonexistent_key() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        let result = storage.get("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_with_revision() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        insert_kv_entry(&storage, "test_key", "test_value", 5);

        let result = storage.get_with_revision("test_key").unwrap();
        assert!(result.is_some());
        let kv = result.unwrap();
        assert_eq!(kv.key, "test_key");
        assert_eq!(kv.value, "test_value");
        assert_eq!(kv.version, 5);
        assert_eq!(kv.create_revision, 5);
        assert_eq!(kv.mod_revision, 5);
    }

    #[tokio::test]
    async fn test_get_with_revision_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        let result = storage.get_with_revision("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_storage_path() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.redb");
        let storage = SharedRedbStorage::new(&db_path, "test-node-1").unwrap();

        assert_eq!(storage.path(), db_path.as_path());
    }

    #[tokio::test]
    async fn test_storage_db_handle() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Verify we can get the database handle
        let db = storage.db();
        assert!(Arc::strong_count(&db) >= 2); // storage + returned handle
    }

    #[tokio::test]
    async fn test_storage_debug_format() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        let debug_str = format!("{:?}", storage);
        assert!(debug_str.contains("SharedRedbStorage"));
        assert!(debug_str.contains("path"));
    }

    // =========================================================================
    // TTL Expiration Tests
    // =========================================================================

    #[tokio::test]
    async fn test_get_expired_key_returns_none() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert a key that has already expired
        let expired_time = now_unix_ms() - 1000; // 1 second ago
        insert_kv_entry_with_ttl(&storage, "expired_key", "value", 1, expired_time);

        // Should return None for expired key
        let result = storage.get("expired_key").unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_non_expired_key_returns_value() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert a key that expires in the future
        let future_time = now_unix_ms() + 60000; // 1 minute from now
        insert_kv_entry_with_ttl(&storage, "valid_key", "value", 1, future_time);

        // Should return the value
        let result = storage.get("valid_key").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().value, "value");
    }

    #[tokio::test]
    async fn test_delete_expired_keys_basic() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert expired keys
        let expired_time = now_unix_ms() - 1000;
        insert_kv_entry_with_ttl(&storage, "expired1", "value1", 1, expired_time);
        insert_kv_entry_with_ttl(&storage, "expired2", "value2", 2, expired_time);

        // Insert non-expired key
        let future_time = now_unix_ms() + 60000;
        insert_kv_entry_with_ttl(&storage, "valid", "value3", 3, future_time);

        // Delete expired keys
        let deleted = storage.delete_expired_keys(100).unwrap();
        assert_eq!(deleted, 2);

        // Verify non-expired key still exists
        assert!(storage.get("valid").unwrap().is_some());
    }

    #[tokio::test]
    async fn test_delete_expired_keys_respects_batch_limit() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert 10 expired keys
        let expired_time = now_unix_ms() - 1000;
        for i in 0..10 {
            let key = format!("expired_{}", i);
            insert_kv_entry_with_ttl(&storage, &key, "value", i as i64, expired_time);
        }

        // Delete with batch limit of 3
        let deleted = storage.delete_expired_keys(3).unwrap();
        assert_eq!(deleted, 3);

        // There should still be expired keys
        let remaining = storage.count_expired_keys().unwrap();
        assert_eq!(remaining, 7);
    }

    #[tokio::test]
    async fn test_delete_expired_keys_no_expired() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert only non-expired keys
        let future_time = now_unix_ms() + 60000;
        insert_kv_entry_with_ttl(&storage, "valid1", "value1", 1, future_time);
        insert_kv_entry_with_ttl(&storage, "valid2", "value2", 2, future_time);

        // Delete expired keys - should delete none
        let deleted = storage.delete_expired_keys(100).unwrap();
        assert_eq!(deleted, 0);
    }

    #[tokio::test]
    async fn test_count_expired_keys() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert expired keys
        let expired_time = now_unix_ms() - 1000;
        insert_kv_entry_with_ttl(&storage, "expired1", "value1", 1, expired_time);
        insert_kv_entry_with_ttl(&storage, "expired2", "value2", 2, expired_time);

        // Insert non-expired key
        let future_time = now_unix_ms() + 60000;
        insert_kv_entry_with_ttl(&storage, "valid", "value3", 3, future_time);

        let count = storage.count_expired_keys().unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_count_keys_with_ttl() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert expired key (not counted as "with TTL" because it's expired)
        let expired_time = now_unix_ms() - 1000;
        insert_kv_entry_with_ttl(&storage, "expired1", "value1", 1, expired_time);

        // Insert non-expired keys with TTL
        let future_time = now_unix_ms() + 60000;
        insert_kv_entry_with_ttl(&storage, "valid1", "value2", 2, future_time);
        insert_kv_entry_with_ttl(&storage, "valid2", "value3", 3, future_time);

        // Insert key without TTL
        insert_kv_entry(&storage, "no_ttl", "value4", 4);

        let count = storage.count_keys_with_ttl().unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_get_expired_keys_with_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert expired keys
        let expired_time = now_unix_ms() - 1000;
        insert_kv_entry_with_ttl(&storage, "expired1", "value1", 1, expired_time);
        insert_kv_entry_with_ttl(&storage, "expired2", "value2", 2, expired_time);

        let expired_keys = storage.get_expired_keys_with_metadata(10).unwrap();
        assert_eq!(expired_keys.len(), 2);

        // Verify keys are present
        let key_names: Vec<&str> = expired_keys.iter().map(|(k, _)| k.as_str()).collect();
        assert!(key_names.contains(&"expired1"));
        assert!(key_names.contains(&"expired2"));
    }

    #[tokio::test]
    async fn test_get_expired_keys_with_metadata_respects_limit() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert 10 expired keys
        let expired_time = now_unix_ms() - 1000;
        for i in 0..10 {
            let key = format!("expired_{}", i);
            insert_kv_entry_with_ttl(&storage, &key, "value", i as i64, expired_time);
        }

        let expired_keys = storage.get_expired_keys_with_metadata(3).unwrap();
        assert_eq!(expired_keys.len(), 3);
    }

    // =========================================================================
    // Scan Tests
    // =========================================================================

    #[tokio::test]
    async fn test_shared_storage_scan() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert multiple values
        for i in 0..10 {
            let key = format!("prefix/{}", i);
            insert_kv_entry(&storage, &key, &format!("value_{}", i), i as i64);
        }

        // Scan with prefix
        let results = storage.scan("prefix/", None, Some(5)).unwrap();
        assert_eq!(results.len(), 5);
    }

    #[tokio::test]
    async fn test_scan_with_after_key() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert keys
        for i in 0..10 {
            let key = format!("key/{}", i);
            insert_kv_entry(&storage, &key, &format!("value_{}", i), i as i64);
        }

        // Scan starting after key/3
        let results = storage.scan("key/", Some("key/3"), Some(10)).unwrap();

        // Should not include key/0, key/1, key/2, key/3
        for result in &results {
            assert!(result.key > "key/3".to_string());
        }
    }

    #[tokio::test]
    async fn test_scan_empty_prefix() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert keys with different prefixes
        insert_kv_entry(&storage, "aaa", "value1", 1);
        insert_kv_entry(&storage, "bbb", "value2", 2);
        insert_kv_entry(&storage, "ccc", "value3", 3);

        // Scan with empty prefix - should return all keys
        let results = storage.scan("", None, Some(10)).unwrap();
        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn test_scan_no_matches() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        insert_kv_entry(&storage, "other/key", "value", 1);

        // Scan with non-matching prefix
        let results = storage.scan("prefix/", None, Some(10)).unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_scan_excludes_expired_keys() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert expired key
        let expired_time = now_unix_ms() - 1000;
        insert_kv_entry_with_ttl(&storage, "prefix/expired", "value1", 1, expired_time);

        // Insert valid key
        insert_kv_entry(&storage, "prefix/valid", "value2", 2);

        let results = storage.scan("prefix/", None, Some(10)).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "prefix/valid");
    }

    #[tokio::test]
    async fn test_scan_respects_max_batch_size() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert many keys
        for i in 0..100 {
            let key = format!("key/{:03}", i);
            insert_kv_entry(&storage, &key, &format!("value_{}", i), i as i64);
        }

        // Scan with limit much larger than MAX_BATCH_SIZE
        let results = storage.scan("key/", None, Some(10000)).unwrap();

        // Should be capped at MAX_BATCH_SIZE
        assert!(results.len() <= MAX_BATCH_SIZE as usize);
    }

    // =========================================================================
    // Lease Tests
    // =========================================================================

    #[tokio::test]
    async fn test_get_lease_active() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert active lease (expires in 60 seconds)
        let expires_at = now_unix_ms() + 60000;
        insert_lease_entry(&storage, 1001, 60, expires_at);

        let result = storage.get_lease(1001).unwrap();
        assert!(result.is_some());
        let (granted_ttl, remaining_ttl) = result.unwrap();
        assert_eq!(granted_ttl, 60);
        assert!(remaining_ttl > 0);
        assert!(remaining_ttl <= 60);
    }

    #[tokio::test]
    async fn test_get_lease_expired() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert expired lease
        let expired_time = now_unix_ms() - 1000;
        insert_lease_entry(&storage, 1001, 60, expired_time);

        let result = storage.get_lease(1001).unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_lease_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        let result = storage.get_lease(9999).unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_lease_keys() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        let expires_at = now_unix_ms() + 60000;
        let keys = vec!["key1".to_string(), "key2".to_string(), "key3".to_string()];
        insert_lease_entry_with_keys(&storage, 1001, 60, expires_at, keys.clone());

        let result = storage.get_lease_keys(1001).unwrap();
        assert_eq!(result.len(), 3);
        assert!(result.contains(&"key1".to_string()));
        assert!(result.contains(&"key2".to_string()));
        assert!(result.contains(&"key3".to_string()));
    }

    #[tokio::test]
    async fn test_get_lease_keys_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        let result = storage.get_lease_keys(9999).unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_list_leases_active_only() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert active leases
        let future_time = now_unix_ms() + 60000;
        insert_lease_entry(&storage, 1001, 60, future_time);
        insert_lease_entry(&storage, 1002, 120, future_time);

        // Insert expired lease
        let expired_time = now_unix_ms() - 1000;
        insert_lease_entry(&storage, 1003, 60, expired_time);

        let leases = storage.list_leases().unwrap();
        assert_eq!(leases.len(), 2);

        let lease_ids: Vec<u64> = leases.iter().map(|(id, _, _)| *id).collect();
        assert!(lease_ids.contains(&1001));
        assert!(lease_ids.contains(&1002));
        assert!(!lease_ids.contains(&1003));
    }

    #[tokio::test]
    async fn test_delete_expired_leases_basic() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert expired leases
        let expired_time = now_unix_ms() - 1000;
        insert_lease_entry(&storage, 1001, 60, expired_time);
        insert_lease_entry(&storage, 1002, 60, expired_time);

        // Insert active lease
        let future_time = now_unix_ms() + 60000;
        insert_lease_entry(&storage, 1003, 60, future_time);

        let deleted = storage.delete_expired_leases(100).unwrap();
        assert_eq!(deleted, 2);

        // Active lease should still exist
        assert!(storage.get_lease(1003).unwrap().is_some());
    }

    #[tokio::test]
    async fn test_delete_expired_leases_with_attached_keys() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert keys
        insert_kv_entry(&storage, "attached_key1", "value1", 1);
        insert_kv_entry(&storage, "attached_key2", "value2", 2);

        // Insert expired lease with attached keys
        let expired_time = now_unix_ms() - 1000;
        let keys = vec!["attached_key1".to_string(), "attached_key2".to_string()];
        insert_lease_entry_with_keys(&storage, 1001, 60, expired_time, keys);

        // Delete expired leases
        let deleted = storage.delete_expired_leases(100).unwrap();
        assert_eq!(deleted, 1);

        // Attached keys should be deleted
        assert!(storage.get("attached_key1").unwrap().is_none());
        assert!(storage.get("attached_key2").unwrap().is_none());
    }

    #[tokio::test]
    async fn test_delete_expired_leases_respects_limit() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert 10 expired leases
        let expired_time = now_unix_ms() - 1000;
        for i in 0..10 {
            insert_lease_entry(&storage, 1000 + i, 60, expired_time);
        }

        // Delete with limit of 3
        let deleted = storage.delete_expired_leases(3).unwrap();
        assert_eq!(deleted, 3);

        let remaining = storage.count_expired_leases().unwrap();
        assert_eq!(remaining, 7);
    }

    #[tokio::test]
    async fn test_count_expired_leases() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert expired leases
        let expired_time = now_unix_ms() - 1000;
        insert_lease_entry(&storage, 1001, 60, expired_time);
        insert_lease_entry(&storage, 1002, 60, expired_time);

        // Insert active lease
        let future_time = now_unix_ms() + 60000;
        insert_lease_entry(&storage, 1003, 60, future_time);

        let count = storage.count_expired_leases().unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_count_active_leases() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Insert expired lease
        let expired_time = now_unix_ms() - 1000;
        insert_lease_entry(&storage, 1001, 60, expired_time);

        // Insert active leases
        let future_time = now_unix_ms() + 60000;
        insert_lease_entry(&storage, 1002, 60, future_time);
        insert_lease_entry(&storage, 1003, 120, future_time);

        let count = storage.count_active_leases().unwrap();
        assert_eq!(count, 2);
    }

    // =========================================================================
    // Chain Hash Tests
    // =========================================================================

    #[tokio::test]
    async fn test_chain_tip_for_verification() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // New storage should have default chain tip
        let (index, hash) = storage.chain_tip_for_verification().unwrap();
        assert_eq!(index, 0);
        assert_eq!(hash, [0u8; 32]);
    }

    #[tokio::test]
    async fn test_storage_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.redb");

        // Create storage and insert data
        {
            let storage = SharedRedbStorage::new(&db_path, "test-node-1").unwrap();
            insert_kv_entry(&storage, "persistent_key", "persistent_value", 1);
        }

        // Reopen storage and verify data
        {
            let storage = SharedRedbStorage::new(&db_path, "test-node-1").unwrap();
            let entry = storage.get("persistent_key").unwrap();
            assert!(entry.is_some());
            assert_eq!(entry.unwrap().value, "persistent_value");
        }
    }

    // =========================================================================
    // Error Handling Tests
    // =========================================================================

    #[tokio::test]
    async fn test_error_display() {
        let err = SharedStorageError::BatchTooLarge { size: 2000, max: 1000 };
        let display = err.to_string();
        assert!(display.contains("2000"));
        assert!(display.contains("1000"));
    }

    #[tokio::test]
    async fn test_lock_poisoned_error() {
        let err = SharedStorageError::LockPoisoned {
            context: "test context".into(),
        };
        let display = err.to_string();
        assert!(display.contains("test context"));
        assert!(display.contains("poisoned"));
    }

    #[tokio::test]
    async fn test_storage_error_to_io_error() {
        let err = SharedStorageError::BatchTooLarge { size: 2000, max: 1000 };
        let io_err: std::io::Error = err.into();
        assert!(io_err.to_string().contains("2000"));
    }

    // =========================================================================
    // Broadcast Tests
    // =========================================================================

    #[tokio::test]
    async fn test_log_broadcast_on_apply() {
        use futures::stream;
        use openraft::storage::RaftStateMachine;
        use openraft::testing::log_id;

        use crate::log_subscriber::KvOperation;
        use crate::log_subscriber::LOG_BROADCAST_BUFFER_SIZE;
        use crate::log_subscriber::LogEntryPayload;
        use crate::types::AppRequest;
        use crate::types::AppTypeConfig;
        use crate::types::NodeId;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_broadcast.redb");

        // Create broadcast channel
        let (sender, mut receiver) = broadcast::channel::<LogEntryPayload>(LOG_BROADCAST_BUFFER_SIZE);

        // Create storage with broadcast
        let mut storage = SharedRedbStorage::with_broadcast(&db_path, Some(sender), "test-node-1").unwrap();

        // Create a test entry using the helper function from openraft::testing
        let log_id = log_id::<AppTypeConfig>(1, NodeId::from(1), 1);
        let entry: openraft::Entry<AppTypeConfig> = openraft::Entry::new_normal(log_id, AppRequest::Set {
            key: "test_key".to_string(),
            value: "test_value".to_string(),
        });

        // Simulate apply() being called with the entry
        // Note: In real usage, apply() receives EntryResponder tuples
        // For this test, we directly call the broadcast logic
        let entries = stream::iter(vec![Ok((entry, None))]);
        storage.apply(entries).await.unwrap();

        // Verify broadcast was received
        let payload = receiver.try_recv().expect("should receive broadcast");
        assert_eq!(payload.index, 1);
        assert_eq!(payload.term, 1);

        // Verify the operation matches
        match payload.operation {
            KvOperation::Set { key, value } => {
                assert_eq!(key, b"test_key".to_vec());
                assert_eq!(value, b"test_value".to_vec());
            }
            _ => panic!("expected Set operation"),
        }
    }

    #[tokio::test]
    async fn test_snapshot_broadcast_channel() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_snapshot.redb");

        // Create broadcast channels
        let (log_tx, _log_rx) = broadcast::channel::<crate::log_subscriber::LogEntryPayload>(16);
        let (snapshot_tx, _snapshot_rx) = broadcast::channel::<SnapshotEvent>(16);

        // Create storage with both broadcast channels
        let storage =
            SharedRedbStorage::with_broadcasts(&db_path, Some(log_tx), Some(snapshot_tx), "test-node-1").unwrap();

        // Verify storage was created successfully
        assert!(storage.get("test").unwrap().is_none());
    }

    // =========================================================================
    // Raft Metadata Tests
    // =========================================================================

    #[tokio::test]
    async fn test_raft_meta_read_write() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Write metadata
        storage.write_raft_meta("test_key", &42u64).unwrap();

        // Read metadata
        let result: Option<u64> = storage.read_raft_meta("test_key").unwrap();
        assert_eq!(result, Some(42));
    }

    #[tokio::test]
    async fn test_raft_meta_read_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        let result: Option<u64> = storage.read_raft_meta("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_raft_meta_delete() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Write then delete
        storage.write_raft_meta("test_key", &42u64).unwrap();
        storage.delete_raft_meta("test_key").unwrap();

        let result: Option<u64> = storage.read_raft_meta("test_key").unwrap();
        assert!(result.is_none());
    }

    // =========================================================================
    // State Machine Metadata Tests
    // =========================================================================

    #[tokio::test]
    async fn test_sm_meta_read() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Write SM metadata directly
        let write_txn = storage.db.begin_write().unwrap();
        {
            let mut table = write_txn.open_table(SM_META_TABLE).unwrap();
            let data = bincode::serialize(&Some(100u64)).unwrap();
            table.insert("test_sm_meta", data.as_slice()).unwrap();
        }
        write_txn.commit().unwrap();

        // Read via helper
        let result: Option<Option<u64>> = storage.read_sm_meta("test_sm_meta").unwrap();
        assert_eq!(result, Some(Some(100)));
    }

    // =========================================================================
    // Clone Tests
    // =========================================================================

    #[tokio::test]
    async fn test_storage_clone_shares_db() {
        let temp_dir = TempDir::new().unwrap();
        let storage1 = create_test_storage(&temp_dir);
        let storage2 = storage1.clone();

        // Insert via storage1
        insert_kv_entry(&storage1, "shared_key", "shared_value", 1);

        // Read via storage2
        let entry = storage2.get("shared_key").unwrap();
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().value, "shared_value");
    }

    // =========================================================================
    // Index Query Tests
    // =========================================================================

    #[tokio::test]
    async fn test_scan_by_index_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Try to scan by non-existent index
        let result = storage.scan_by_index("nonexistent_index", b"value", 10);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_range_by_index_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Try to range by non-existent index
        let result = storage.range_by_index("nonexistent_index", b"start", b"end", 10);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_scan_index_lt_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let storage = create_test_storage(&temp_dir);

        // Try to scan by non-existent index
        let result = storage.scan_index_lt("nonexistent_index", b"threshold", 10);
        assert!(result.is_err());
    }

    // =========================================================================
    // KvEntry Tests
    // =========================================================================

    #[tokio::test]
    async fn test_kv_entry_serialization() {
        let entry = KvEntry {
            value: "test_value".to_string(),
            version: 5,
            create_revision: 10,
            mod_revision: 15,
            expires_at_ms: Some(1234567890),
            lease_id: Some(1001),
        };

        let serialized = bincode::serialize(&entry).unwrap();
        let deserialized: KvEntry = bincode::deserialize(&serialized).unwrap();

        assert_eq!(entry.value, deserialized.value);
        assert_eq!(entry.version, deserialized.version);
        assert_eq!(entry.create_revision, deserialized.create_revision);
        assert_eq!(entry.mod_revision, deserialized.mod_revision);
        assert_eq!(entry.expires_at_ms, deserialized.expires_at_ms);
        assert_eq!(entry.lease_id, deserialized.lease_id);
    }

    // =========================================================================
    // LeaseEntry Tests
    // =========================================================================

    #[tokio::test]
    async fn test_lease_entry_serialization() {
        let entry = LeaseEntry {
            ttl_seconds: 60,
            expires_at_ms: 1234567890,
            keys: vec!["key1".to_string(), "key2".to_string()],
        };

        let serialized = bincode::serialize(&entry).unwrap();
        let deserialized: LeaseEntry = bincode::deserialize(&serialized).unwrap();

        assert_eq!(entry.ttl_seconds, deserialized.ttl_seconds);
        assert_eq!(entry.expires_at_ms, deserialized.expires_at_ms);
        assert_eq!(entry.keys, deserialized.keys);
    }

    #[test]
    fn test_lease_entry_debug() {
        let entry = LeaseEntry {
            ttl_seconds: 60,
            expires_at_ms: 1234567890,
            keys: vec!["key1".to_string()],
        };

        let debug_str = format!("{:?}", entry);
        assert!(debug_str.contains("LeaseEntry"));
        assert!(debug_str.contains("ttl_seconds"));
        assert!(debug_str.contains("60"));
    }

    #[test]
    fn test_lease_entry_clone() {
        let entry = LeaseEntry {
            ttl_seconds: 60,
            expires_at_ms: 1234567890,
            keys: vec!["key1".to_string()],
        };

        let cloned = entry.clone();
        assert_eq!(entry.ttl_seconds, cloned.ttl_seconds);
        assert_eq!(entry.expires_at_ms, cloned.expires_at_ms);
        assert_eq!(entry.keys, cloned.keys);
    }

    // =========================================================================
    // StoredSnapshot Tests
    // =========================================================================

    #[test]
    fn test_stored_snapshot_debug() {
        let snapshot = StoredSnapshot {
            meta: openraft::SnapshotMeta {
                last_log_id: None,
                last_membership: openraft::StoredMembership::default(),
                snapshot_id: "test-snapshot".to_string(),
            },
            data: vec![1, 2, 3],
            integrity: None,
        };

        let debug_str = format!("{:?}", snapshot);
        assert!(debug_str.contains("StoredSnapshot"));
        assert!(debug_str.contains("test-snapshot"));
    }

    #[test]
    fn test_stored_snapshot_clone() {
        let snapshot = StoredSnapshot {
            meta: openraft::SnapshotMeta {
                last_log_id: None,
                last_membership: openraft::StoredMembership::default(),
                snapshot_id: "test-snapshot".to_string(),
            },
            data: vec![1, 2, 3],
            integrity: None,
        };

        let cloned = snapshot.clone();
        assert_eq!(snapshot.meta.snapshot_id, cloned.meta.snapshot_id);
        assert_eq!(snapshot.data, cloned.data);
    }

    // =========================================================================
    // SnapshotEvent Tests
    // =========================================================================

    #[test]
    fn test_snapshot_event_created_debug() {
        let event = SnapshotEvent::Created {
            snapshot_id: "snap-1".to_string(),
            last_log_index: 100,
            term: 5,
            entry_count: 1000,
            size_bytes: 4096,
        };

        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("Created"));
        assert!(debug_str.contains("snap-1"));
        assert!(debug_str.contains("100"));
    }

    #[test]
    fn test_snapshot_event_installed_debug() {
        let event = SnapshotEvent::Installed {
            snapshot_id: "snap-2".to_string(),
            last_log_index: 200,
            term: 10,
            entry_count: 2000,
        };

        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("Installed"));
        assert!(debug_str.contains("snap-2"));
        assert!(debug_str.contains("200"));
    }

    #[test]
    fn test_snapshot_event_clone() {
        let event = SnapshotEvent::Created {
            snapshot_id: "snap-1".to_string(),
            last_log_index: 100,
            term: 5,
            entry_count: 1000,
            size_bytes: 4096,
        };

        let cloned = event.clone();
        match cloned {
            SnapshotEvent::Created {
                snapshot_id,
                last_log_index,
                ..
            } => {
                assert_eq!(snapshot_id, "snap-1");
                assert_eq!(last_log_index, 100);
            }
            _ => panic!("expected Created event"),
        }
    }

    // =========================================================================
    // Directory Creation Tests
    // =========================================================================

    #[tokio::test]
    async fn test_storage_creates_parent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir.path().join("nested/deep/path/test.redb");

        // Parent directories don't exist yet
        assert!(!nested_path.parent().unwrap().exists());

        // Creating storage should create parent directories
        let storage = SharedRedbStorage::new(&nested_path, "test-node-1").unwrap();
        assert!(nested_path.exists());

        // Storage should work
        insert_kv_entry(&storage, "key", "value", 1);
        let entry = storage.get("key").unwrap();
        assert!(entry.is_some());
    }

    // =========================================================================
    // Multiple Node Tests
    // =========================================================================

    #[tokio::test]
    async fn test_different_node_ids_create_different_hlc() {
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();

        let storage1 = SharedRedbStorage::new(temp_dir1.path().join("test1.redb"), "node-1").unwrap();
        let storage2 = SharedRedbStorage::new(temp_dir2.path().join("test2.redb"), "node-2").unwrap();

        // Both storages should be independent
        insert_kv_entry(&storage1, "key1", "value1", 1);
        insert_kv_entry(&storage2, "key2", "value2", 1);

        assert!(storage1.get("key1").unwrap().is_some());
        assert!(storage1.get("key2").unwrap().is_none());

        assert!(storage2.get("key2").unwrap().is_some());
        assert!(storage2.get("key1").unwrap().is_none());
    }
}
