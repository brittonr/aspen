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

use aspen_core::{KeyValueWithRevision, TxnOpResult};
#[cfg(feature = "coordination")]
use aspen_coordination::now_unix_ms;

#[cfg(not(feature = "coordination"))]
fn now_unix_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
use crate::constants::MAX_BATCH_SIZE;
use crate::constants::MAX_SETMULTI_KEYS;
use crate::constants::MAX_SNAPSHOT_ENTRIES;
use crate::integrity::ChainHash;
use crate::integrity::ChainTipState;
use crate::integrity::SnapshotIntegrity;
use crate::integrity::compute_entry_hash;
use crate::log_subscriber::LogEntryPayload;
use crate::types::AppRequest;
use crate::types::AppResponse;
use crate::types::AppTypeConfig;
use aspen_core::ensure_disk_space_available;

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
#[derive(Clone, Debug)]
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
    /// Pending responses computed during append() to be sent in apply().
    /// Key is log index, value is the computed AppResponse.
    pending_responses: Arc<StdRwLock<BTreeMap<u64, AppResponse>>>,
}

impl SharedRedbStorage {
    /// Create or open a SharedRedbStorage at the given path.
    ///
    /// Creates the database file and all required tables if they don't exist.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, SharedStorageError> {
        Self::with_broadcast(path, None)
    }

    /// Create with optional log broadcast channel.
    pub fn with_broadcast(
        path: impl AsRef<Path>,
        log_broadcast: Option<broadcast::Sender<LogEntryPayload>>,
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
        }
        write_txn.commit().context(CommitSnafu)?;

        let db = Arc::new(db);

        // Load chain tip from database
        let chain_tip = Self::load_chain_tip(&db)?;

        Ok(Self {
            db,
            path,
            chain_tip: Arc::new(StdRwLock::new(chain_tip)),
            log_broadcast,
            pending_responses: Arc::new(StdRwLock::new(BTreeMap::new())),
        })
    }

    /// Get the path to the database file.
    pub fn path(&self) -> &Path {
        &self.path
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
    fn apply_set_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
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

        let (version, create_revision) = match existing {
            Some(e) => (e.version + 1, e.create_revision),
            None => (1, log_index as i64),
        };

        let entry = KvEntry {
            value: value.to_string(),
            version,
            create_revision,
            mod_revision: log_index as i64,
            expires_at_ms,
            lease_id,
        };

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
        key: &str,
    ) -> Result<AppResponse, SharedStorageError> {
        let existed = kv_table.remove(key.as_bytes()).context(RemoveSnafu)?.is_some();
        Ok(AppResponse {
            deleted: Some(existed),
            ..Default::default()
        })
    }

    /// Apply a SetMulti operation within a transaction.
    fn apply_set_multi_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
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
            Self::apply_set_in_txn(kv_table, leases_table, key, value, log_index, expires_at_ms, lease_id)?;
        }

        Ok(AppResponse::default())
    }

    /// Apply a DeleteMulti operation within a transaction.
    fn apply_delete_multi_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
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
            let existed = kv_table.remove(key.as_bytes()).context(RemoveSnafu)?.is_some();
            deleted_any |= existed;
        }

        Ok(AppResponse {
            deleted: Some(deleted_any),
            ..Default::default()
        })
    }

    /// Apply a CompareAndSwap operation within a transaction.
    fn apply_compare_and_swap_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
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

        // Check condition
        let condition_matches = match (expected, current_value) {
            (None, None) => true,                 // Expected no key, found no key
            (Some(exp), Some(cur)) => exp == cur, // Values match
            _ => false,                           // Mismatch
        };

        if !condition_matches {
            return Ok(AppResponse {
                value: current_value.map(String::from),
                cas_succeeded: Some(false),
                ..Default::default()
            });
        }

        // Apply the write
        let (version, create_revision) = match current {
            Some(ref e) => (e.version + 1, e.create_revision),
            None => (1, log_index as i64),
        };

        let entry = KvEntry {
            value: new_value.to_string(),
            version,
            create_revision,
            mod_revision: log_index as i64,
            expires_at_ms: None,
            lease_id: None,
        };

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
                Self::apply_set_in_txn(kv_table, leases_table, key, value, log_index, None, None)?;
            } else {
                Self::apply_delete_in_txn(kv_table, key)?;
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
        Self::apply_batch_in_txn(kv_table, leases_table, operations, log_index)?;

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
            let random_component = (now % 1000000) as u64;
            now * 1000 + random_component
        } else {
            lease_id
        };

        // Calculate expiration time
        let expires_at_ms = now_unix_ms() + (ttl_seconds as u64 * 1000);

        // Create lease entry
        let lease_entry = LeaseEntry {
            ttl_seconds,
            expires_at_ms,
            keys: Vec::new(), // Keys will be added when SetWithLease is called
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
            if kv_table.remove(key.as_bytes()).context(RemoveSnafu)?.is_some() {
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
            // Refresh the expiration time
            lease_entry.expires_at_ms = now_unix_ms() + (lease_entry.ttl_seconds as u64 * 1000);
            let ttl = lease_entry.ttl_seconds;

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
    fn apply_transaction_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
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
                        0 => current_value == Some(value.as_str()), // Equal
                        1 => current_value != Some(value.as_str()), // NotEqual
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
                    Self::apply_set_in_txn(kv_table, leases_table, key, value, log_index, None, None)?;
                    TxnOpResult::Put { revision: log_index }
                }
                1 => {
                    // Delete operation
                    let deleted = if kv_table.remove(key.as_bytes()).context(RemoveSnafu)?.is_some() {
                        1
                    } else {
                        0
                    };
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
                Self::apply_set_in_txn(kv_table, leases_table, key, value, log_index, None, None)?;
            } else {
                Self::apply_delete_in_txn(kv_table, key)?;
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
        _leases_table: &mut redb::Table<u64, &[u8]>,
        request: &AppRequest,
        log_index: u64,
    ) -> Result<AppResponse, SharedStorageError> {
        match request {
            AppRequest::Set { key, value } => Self::apply_set_in_txn(kv_table, _leases_table, key, value, log_index, None, None),
            AppRequest::SetWithTTL {
                key,
                value,
                expires_at_ms,
            } => Self::apply_set_in_txn(kv_table, _leases_table, key, value, log_index, Some(*expires_at_ms), None),
            AppRequest::SetMulti { pairs } => Self::apply_set_multi_in_txn(kv_table, _leases_table, pairs, log_index, None, None),
            AppRequest::SetMultiWithTTL { pairs, expires_at_ms } => {
                Self::apply_set_multi_in_txn(kv_table, _leases_table, pairs, log_index, Some(*expires_at_ms), None)
            }
            AppRequest::Delete { key } => Self::apply_delete_in_txn(kv_table, key),
            AppRequest::DeleteMulti { keys } => Self::apply_delete_multi_in_txn(kv_table, keys),
            AppRequest::CompareAndSwap {
                key,
                expected,
                new_value,
            } => Self::apply_compare_and_swap_in_txn(kv_table, key, expected.as_deref(), new_value, log_index),
            AppRequest::CompareAndDelete { key, expected } => {
                Self::apply_compare_and_delete_in_txn(kv_table, key, expected)
            }
            AppRequest::Batch { operations } => Self::apply_batch_in_txn(kv_table, _leases_table, operations, log_index),
            AppRequest::ConditionalBatch { conditions, operations } => {
                Self::apply_conditional_batch_in_txn(kv_table, _leases_table, conditions, operations, log_index)
            }
            // Lease operations
            AppRequest::SetWithLease { key, value, lease_id } => {
                Self::apply_set_in_txn(kv_table, _leases_table, key, value, log_index, None, Some(*lease_id))
            }
            AppRequest::SetMultiWithLease { pairs, lease_id } => {
                Self::apply_set_multi_in_txn(kv_table, _leases_table, pairs, log_index, None, Some(*lease_id))
            }
            // Lease operations
            AppRequest::LeaseGrant { lease_id, ttl_seconds } => {
                Self::apply_lease_grant_in_txn(_leases_table, *lease_id, *ttl_seconds)
            }
            AppRequest::LeaseRevoke { lease_id } => {
                Self::apply_lease_revoke_in_txn(kv_table, _leases_table, *lease_id)
            }
            AppRequest::LeaseKeepalive { lease_id } => {
                Self::apply_lease_keepalive_in_txn(_leases_table, *lease_id)
            }
            // Transaction operations
            AppRequest::Transaction {
                compare,
                success,
                failure,
            } => Self::apply_transaction_in_txn(kv_table, _leases_table, compare, success, failure, log_index),
            AppRequest::OptimisticTransaction {
                read_set,
                write_set,
            } => Self::apply_optimistic_transaction_in_txn(kv_table, _leases_table, read_set, write_set, log_index),
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
    async fn append<I>(&mut self, entries: I, callback: IOFlushed<AppTypeConfig>) -> Result<(), io::Error>
    where
        I: IntoIterator<Item = <AppTypeConfig as openraft::RaftTypeConfig>::Entry> + OptionalSend,
        I::IntoIter: OptionalSend,
    {
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
                        // Apply the request to state machine tables
                        Self::apply_request_in_txn(&mut kv_table, &mut leases_table, request, index)?
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

        callback.io_completed(Ok(()));
        Ok(())
    }

    async fn truncate(&mut self, log_id: LogIdOf<AppTypeConfig>) -> Result<(), io::Error> {
        let truncate_from = log_id.index();

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

        Ok(())
    }

    async fn purge(&mut self, log_id: LogIdOf<AppTypeConfig>) -> Result<(), io::Error> {
        // Verify purge is monotonic
        if let Some(prev) = self.read_raft_meta::<LogIdOf<AppTypeConfig>>("last_purged_log_id")?
            && prev > log_id
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("purge must be monotonic: prev={:?}, new={:?}", prev, log_id),
            ));
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

        // Clean up pending responses for purged log entries to prevent memory leak
        {
            let mut pending_responses =
                self.pending_responses.write().map_err(|_| SharedStorageError::LockPoisoned {
                    context: "writing pending_responses during purge".into(),
                })?;
            // Remove all responses for indices <= purge index
            pending_responses.retain(|&idx, _| idx > log_id.index());
        }

        self.write_raft_meta("last_purged_log_id", &log_id)?;
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
    where
        Strm: Stream<Item = Result<EntryResponder<AppTypeConfig>, io::Error>> + Unpin + OptionalSend,
    {
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
                    committed_at_ms: aspen_core::utils::current_time_ms(),
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

    async fn install_snapshot(
        &mut self,
        meta: &openraft::SnapshotMeta<AppTypeConfig>,
        snapshot: SnapshotDataOf<AppTypeConfig>,
    ) -> Result<(), io::Error> {
        let data = snapshot.into_inner();

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
    async fn build_snapshot(&mut self) -> Result<Snapshot<AppTypeConfig>, io::Error> {
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

        // Store the snapshot
        let stored = StoredSnapshot {
            meta: meta.clone(),
            data: data.clone(),
            integrity: None, // TODO: Add integrity hash
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
            "built snapshot"
        );

        Ok(Snapshot {
            meta,
            snapshot: Cursor::new(data),
        })
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[tokio::test]
    async fn test_shared_storage_basic() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.redb");

        let storage = SharedRedbStorage::new(&db_path).unwrap();

        // Test basic KV operations
        assert!(storage.get("test_key").unwrap().is_none());
    }

    #[tokio::test]
    async fn test_shared_storage_set_get() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.redb");

        let storage = SharedRedbStorage::new(&db_path).unwrap();

        // Manually insert a value
        let write_txn = storage.db.begin_write().unwrap();
        {
            let mut kv_table = write_txn.open_table(SM_KV_TABLE).unwrap();
            let entry = KvEntry {
                value: "test_value".to_string(),
                version: 1,
                create_revision: 1,
                mod_revision: 1,
                expires_at_ms: None,
                lease_id: None,
            };
            let entry_bytes = bincode::serialize(&entry).unwrap();
            kv_table.insert(b"test_key".as_slice(), entry_bytes.as_slice()).unwrap();
        }
        write_txn.commit().unwrap();

        // Verify we can read it
        let entry = storage.get("test_key").unwrap();
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().value, "test_value");
    }

    #[tokio::test]
    async fn test_shared_storage_scan() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.redb");

        let storage = SharedRedbStorage::new(&db_path).unwrap();

        // Insert multiple values
        let write_txn = storage.db.begin_write().unwrap();
        {
            let mut kv_table = write_txn.open_table(SM_KV_TABLE).unwrap();
            for i in 0..10 {
                let key = format!("prefix/{}", i);
                let entry = KvEntry {
                    value: format!("value_{}", i),
                    version: 1,
                    create_revision: i as i64,
                    mod_revision: i as i64,
                    expires_at_ms: None,
                    lease_id: None,
                };
                let entry_bytes = bincode::serialize(&entry).unwrap();
                kv_table.insert(key.as_bytes(), entry_bytes.as_slice()).unwrap();
            }
        }
        write_txn.commit().unwrap();

        // Scan with prefix
        let results = storage.scan("prefix/", None, Some(5)).unwrap();
        assert_eq!(results.len(), 5);
    }

    #[tokio::test]
    async fn test_log_broadcast_on_apply() {
        use crate::log_subscriber::KvOperation;
        use crate::log_subscriber::LOG_BROADCAST_BUFFER_SIZE;
        use crate::log_subscriber::LogEntryPayload;
        use crate::types::AppRequest;
        use crate::types::AppTypeConfig;
        use crate::types::NodeId;
        use futures::stream;
        use openraft::entry::RaftEntry;
        use openraft::storage::RaftStateMachine;
        use openraft::testing::log_id;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_broadcast.redb");

        // Create broadcast channel
        let (sender, mut receiver) = broadcast::channel::<LogEntryPayload>(LOG_BROADCAST_BUFFER_SIZE);

        // Create storage with broadcast
        let mut storage = SharedRedbStorage::with_broadcast(&db_path, Some(sender)).unwrap();

        // Create a test entry using the helper function from openraft::testing
        let log_id = log_id::<AppTypeConfig>(1, NodeId::from(1), 1);
        let entry: openraft::Entry<AppTypeConfig> = openraft::Entry::new_normal(
            log_id,
            AppRequest::Set {
                key: "test_key".to_string(),
                value: "test_value".to_string(),
            },
        );

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
}
