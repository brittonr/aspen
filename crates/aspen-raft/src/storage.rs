//! Raft log and state machine storage backends.
//!
//! Provides pluggable storage implementations for openraft's log and state machine,
//! supporting both in-memory (for testing) and persistent (redb) backends.
//! The unified Redb backend implements both log storage and state machine in a single
//! database file, enabling single-fsync writes for optimal performance (~2-3ms latency).
//!
//! # Key Components
//!
//! - `StorageBackend`: Enum selecting backend (InMemory or Redb)
//! - `SharedRedbStorage`: Unified log+state machine with single-fsync writes (default production)
//! - `RedbLogStore`: Standalone log store (for legacy/testing)
//! - `InMemoryLogStore`: Non-durable log for testing and development
//! - `InMemoryStateMachine`: In-memory state machine implementation
//! - Snapshot management with bounded in-memory snapshots
//!
//! # Tiger Style
//!
//! - Fixed limits: MAX_BATCH_SIZE (1024 entries), MAX_SETMULTI_KEYS (100 keys)
//! - Explicit types: u64 for log indices (portable across architectures)
//! - Resource bounds: Snapshot data capped to prevent unbounded memory growth
//! - Disk space checks: Pre-flight validation before writing snapshots
//! - Error handling: Explicit SNAFU errors for each failure mode
//!
//! # Test Coverage
//!
//! RedbLogStore persistence tests are comprehensive (50+ tests):
//! - Log append/read across process restarts (12 tests)
//! - Vote persistence and recovery (8 tests)
//! - Committed index persistence (6 tests)
//! - Chain integrity and hash verification (17 tests)
//! - Truncation and purge edge cases (9 tests)
//!
//! InMemoryStateMachine supports all AppRequest variants including:
//!       - Basic KV: Set, Delete, SetWithTTL, SetMulti, DeleteMulti
//!       - Compare-and-swap: CompareAndSwap, CompareAndDelete
//!       - Batch operations: Batch, ConditionalBatch
//!       - Transactions: Transaction (etcd-style), OptimisticTransaction
//!       - Leases: LeaseGrant, LeaseRevoke, LeaseKeepalive (stub responses)
//!       Coverage: Tested via router tests and 85+ unit tests
//!
//! # Example
//!
//! ```ignore
//! use aspen::raft::storage::{RedbLogStore, StorageBackend};
//! use aspen::raft::storage_shared::SharedRedbStorage;
//!
//! // Create unified Redb storage (single-fsync, production default)
//! let storage = SharedRedbStorage::new("./data/shared.redb")?;
//!
//! // Or use backend selection
//! let backend = StorageBackend::Redb;
//! ```

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::io;
use std::io::Cursor;
use std::ops::RangeBounds;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use aspen_core::KeyValueWithRevision;
use aspen_core::TxnOpResult;
use aspen_core::ensure_disk_space_available;
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
use tokio::sync::Mutex;
use tokio::sync::RwLock;

use crate::constants::INTEGRITY_VERSION;
use crate::constants::MAX_BATCH_SIZE;
use crate::integrity::ChainHash;
use crate::integrity::ChainTipState;
use crate::integrity::GENESIS_HASH;
use crate::integrity::SnapshotIntegrity;
use crate::integrity::compute_entry_hash;
use crate::integrity::hash_to_hex;
use crate::types::AppRequest;
use crate::types::AppResponse;
use crate::types::AppTypeConfig;

// ====================================================================================
// Storage Backend Configuration
// ====================================================================================

/// Storage backend selection for Raft log and state machine.
///
/// Aspen supports two storage backends:
/// - **Redb**: Single-fsync storage using shared redb for both log and state machine (default)
/// - **InMemory**: Fast, deterministic storage for testing and simulations
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema
)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum StorageBackend {
    /// In-memory storage using BTreeMap. Data is lost on restart.
    /// Use for: unit tests, madsim simulations, development.
    InMemory,
    /// Single-fsync storage using shared redb for both log and state machine.
    /// Default storage backend for production deployments.
    /// Bundles state mutations into log appends for single-fsync durability.
    /// Write latency: ~2-3ms (single fsync)
    #[default]
    Redb,
}

impl std::str::FromStr for StorageBackend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "inmemory" | "in-memory" | "memory" => Ok(StorageBackend::InMemory),
            "redb" | "single-fsync" | "fast" | "persistent" | "disk" => Ok(StorageBackend::Redb),
            _ => Err(format!("Invalid storage backend '{}'. Valid options: inmemory, redb", s)),
        }
    }
}

impl std::fmt::Display for StorageBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageBackend::InMemory => write!(f, "inmemory"),
            StorageBackend::Redb => write!(f, "redb"),
        }
    }
}

// ====================================================================================
// Redb Table Definitions (Tiger Style: explicitly named, typed tables)
// ====================================================================================

/// Raft log entries: key = log index (u64), value = serialized Entry
const RAFT_LOG_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("raft_log");

/// Raft metadata: key = string identifier, value = serialized data
/// Keys: "vote", "committed", "last_purged_log_id"
const RAFT_META_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("raft_meta");

// State machine tables removed (were only used by deprecated RedbStateMachine)

/// Snapshot storage
const SNAPSHOT_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("snapshots");

/// Chain hash table: key = log index (u64), value = ChainHash (32 bytes).
///
/// Stores chain hashes separately from log entries to enable fast chain
/// verification without deserializing entries. Each hash depends on the
/// previous hash, creating an unbreakable integrity chain.
const CHAIN_HASH_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("chain_hashes");

/// Integrity metadata table: key = string identifier, value = serialized data.
///
/// Keys:
/// - "integrity_version": Schema version for migration detection
/// - "chain_tip_hash": Hash of the most recent entry
/// - "chain_tip_index": Index of the most recent entry
/// - "snapshot_chain_hash": Chain hash at last snapshot point
const INTEGRITY_META_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("integrity_meta");

// ====================================================================================
// Redb Storage Errors
// ====================================================================================

/// Errors that can occur during Raft storage operations.
#[derive(Debug, Snafu)]
pub enum StorageError {
    /// Failed to open the redb database file.
    #[snafu(display("failed to open redb database at {}: {source}", path.display()))]
    OpenDatabase {
        /// Path to the database file.
        path: PathBuf,
        /// Underlying redb error.
        #[snafu(source(from(redb::DatabaseError, Box::new)))]
        source: Box<redb::DatabaseError>,
    },

    /// Failed to begin a write transaction.
    #[snafu(display("failed to begin write transaction: {source}"))]
    BeginWrite {
        /// Underlying redb transaction error.
        #[snafu(source(from(redb::TransactionError, Box::new)))]
        source: Box<redb::TransactionError>,
    },

    /// Failed to begin a read transaction.
    #[snafu(display("failed to begin read transaction: {source}"))]
    BeginRead {
        /// Underlying redb transaction error.
        #[snafu(source(from(redb::TransactionError, Box::new)))]
        source: Box<redb::TransactionError>,
    },

    /// Failed to open a database table.
    #[snafu(display("failed to open table: {source}"))]
    OpenTable {
        /// Underlying redb table error.
        #[snafu(source(from(redb::TableError, Box::new)))]
        source: Box<redb::TableError>,
    },

    /// Failed to commit a transaction.
    #[snafu(display("failed to commit transaction: {source}"))]
    Commit {
        /// Underlying redb commit error.
        #[snafu(source(from(redb::CommitError, Box::new)))]
        source: Box<redb::CommitError>,
    },

    /// Failed to insert a value into a table.
    #[snafu(display("failed to insert into table: {source}"))]
    Insert {
        /// Underlying redb storage error.
        #[snafu(source(from(redb::StorageError, Box::new)))]
        source: Box<redb::StorageError>,
    },

    /// Failed to retrieve a value from a table.
    #[snafu(display("failed to get from table: {source}"))]
    Get {
        /// Underlying redb storage error.
        #[snafu(source(from(redb::StorageError, Box::new)))]
        source: Box<redb::StorageError>,
    },

    /// Failed to remove a value from a table.
    #[snafu(display("failed to remove from table: {source}"))]
    Remove {
        /// Underlying redb storage error.
        #[snafu(source(from(redb::StorageError, Box::new)))]
        source: Box<redb::StorageError>,
    },

    /// Failed to iterate over a table range.
    #[snafu(display("failed to iterate table range: {source}"))]
    Range {
        /// Underlying redb storage error.
        #[snafu(source(from(redb::StorageError, Box::new)))]
        source: Box<redb::StorageError>,
    },

    /// Failed to serialize data with bincode.
    #[snafu(display("failed to serialize data: {source}"))]
    Serialize {
        /// Underlying bincode error.
        #[snafu(source(from(bincode::Error, Box::new)))]
        source: Box<bincode::Error>,
    },

    /// Failed to deserialize data with bincode.
    #[snafu(display("failed to deserialize data: {source}"))]
    Deserialize {
        /// Underlying bincode error.
        #[snafu(source(from(bincode::Error, Box::new)))]
        source: Box<bincode::Error>,
    },

    /// Failed to create a directory for the database.
    #[snafu(display("failed to create directory {}: {source}", path.display()))]
    CreateDirectory {
        /// Path to the directory that could not be created.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },

    /// Chain integrity violation detected during verification.
    ///
    /// This indicates that a log entry's chain hash does not match the expected
    /// value based on the previous hash. This could indicate hardware corruption
    /// or tampering.
    #[snafu(display("chain integrity violation at index {index}: expected {expected}, found {found}"))]
    ChainIntegrityViolation {
        /// Log index where the violation was detected.
        index: u64,
        /// Expected hash value (hex-encoded).
        expected: String,
        /// Actual hash value found (hex-encoded).
        found: String,
    },

    /// Snapshot integrity verification failed.
    #[snafu(display("snapshot integrity verification failed: {reason}"))]
    SnapshotIntegrityFailed {
        /// Human-readable reason for the failure.
        reason: String,
    },

    /// Chain hash is missing at the specified log index.
    ///
    /// This should not occur in normal operation and indicates incomplete
    /// migration or database corruption.
    #[snafu(display("chain hash missing at index {index}"))]
    ChainHashMissing {
        /// Log index where the chain hash is missing.
        index: u64,
    },

    /// A storage lock was poisoned due to a panic in another thread.
    #[snafu(display("storage lock poisoned: {context}"))]
    LockPoisoned {
        /// Context describing which lock was poisoned.
        context: String,
    },
}

impl From<StorageError> for io::Error {
    fn from(err: StorageError) -> io::Error {
        io::Error::other(err.to_string())
    }
}

/// In-memory Raft log backed by a simple `BTreeMap`.
///
/// Provides fast, non-persistent log storage for testing and simulations.
/// All data is lost when the store is dropped.
#[derive(Clone, Debug, Default)]
pub struct InMemoryLogStore {
    /// Shared mutable state protected by an async mutex.
    inner: Arc<Mutex<LogStoreInner>>,
}

/// Internal state for InMemoryLogStore.
#[derive(Debug, Default)]
struct LogStoreInner {
    /// Last log ID that was purged (for snapshot compaction).
    last_purged_log_id: Option<LogIdOf<AppTypeConfig>>,
    /// Map of log index to log entry.
    log: BTreeMap<u64, <AppTypeConfig as openraft::RaftTypeConfig>::Entry>,
    /// Last committed log ID.
    committed: Option<LogIdOf<AppTypeConfig>>,
    /// Current vote state.
    vote: Option<VoteOf<AppTypeConfig>>,
}

impl LogStoreInner {
    async fn try_get_log_entries<RB>(
        &mut self,
        range: RB,
    ) -> Result<Vec<<AppTypeConfig as openraft::RaftTypeConfig>::Entry>, io::Error>
    where
        RB: RangeBounds<u64> + Clone + Debug,
        <AppTypeConfig as openraft::RaftTypeConfig>::Entry: Clone,
    {
        Ok(self.log.range(range).map(|(_, entry)| entry.clone()).collect())
    }

    async fn get_log_state(&mut self) -> Result<LogState<AppTypeConfig>, io::Error> {
        let last_log_id = self.log.iter().next_back().map(|(_, entry)| entry.log_id());
        let last_purged = self.last_purged_log_id;
        let last = last_log_id.or(last_purged);
        Ok(LogState {
            last_purged_log_id: last_purged,
            last_log_id: last,
        })
    }

    async fn save_committed(&mut self, committed: Option<LogIdOf<AppTypeConfig>>) -> Result<(), io::Error> {
        self.committed = committed;
        Ok(())
    }

    async fn read_committed(&mut self) -> Result<Option<LogIdOf<AppTypeConfig>>, io::Error> {
        Ok(self.committed)
    }

    async fn save_vote(&mut self, vote: &VoteOf<AppTypeConfig>) -> Result<(), io::Error> {
        self.vote = Some(*vote);
        Ok(())
    }

    async fn read_vote(&mut self) -> Result<Option<VoteOf<AppTypeConfig>>, io::Error> {
        Ok(self.vote)
    }

    async fn append<I>(&mut self, entries: I, callback: IOFlushed<AppTypeConfig>) -> Result<(), io::Error>
    where I: IntoIterator<Item = <AppTypeConfig as openraft::RaftTypeConfig>::Entry> {
        for entry in entries {
            self.log.insert(entry.log_id().index(), entry);
        }
        callback.io_completed(Ok(()));
        Ok(())
    }

    async fn truncate(&mut self, log_id: LogIdOf<AppTypeConfig>) -> Result<(), io::Error> {
        let keys = self.log.range(log_id.index()..).map(|(k, _)| *k).collect::<Vec<_>>();
        for key in keys {
            self.log.remove(&key);
        }
        Ok(())
    }

    async fn purge(&mut self, log_id: LogIdOf<AppTypeConfig>) -> Result<(), io::Error> {
        if let Some(prev) = &self.last_purged_log_id {
            assert!(prev <= &log_id);
        }
        self.last_purged_log_id = Some(log_id);
        let keys = self.log.range(..=log_id.index()).map(|(k, _)| *k).collect::<Vec<_>>();
        for key in keys {
            self.log.remove(&key);
        }
        Ok(())
    }
}

impl RaftLogReader<AppTypeConfig> for InMemoryLogStore
where <AppTypeConfig as openraft::RaftTypeConfig>::Entry: Clone
{
    async fn try_get_log_entries<RB>(
        &mut self,
        range: RB,
    ) -> Result<Vec<<AppTypeConfig as openraft::RaftTypeConfig>::Entry>, io::Error>
    where
        RB: RangeBounds<u64> + Clone + Debug + OptionalSend,
    {
        let mut inner = self.inner.lock().await;
        inner.try_get_log_entries(range).await
    }

    async fn read_vote(&mut self) -> Result<Option<VoteOf<AppTypeConfig>>, io::Error> {
        let mut inner = self.inner.lock().await;
        inner.read_vote().await
    }
}

impl RaftLogStorage<AppTypeConfig> for InMemoryLogStore
where <AppTypeConfig as openraft::RaftTypeConfig>::Entry: Clone
{
    type LogReader = Self;

    async fn get_log_state(&mut self) -> Result<LogState<AppTypeConfig>, io::Error> {
        let mut inner = self.inner.lock().await;
        inner.get_log_state().await
    }

    async fn save_committed(&mut self, committed: Option<LogIdOf<AppTypeConfig>>) -> Result<(), io::Error> {
        let mut inner = self.inner.lock().await;
        inner.save_committed(committed).await
    }

    async fn read_committed(&mut self) -> Result<Option<LogIdOf<AppTypeConfig>>, io::Error> {
        let mut inner = self.inner.lock().await;
        inner.read_committed().await
    }

    async fn save_vote(&mut self, vote: &VoteOf<AppTypeConfig>) -> Result<(), io::Error> {
        let mut inner = self.inner.lock().await;
        inner.save_vote(vote).await
    }

    async fn append<I>(&mut self, entries: I, callback: IOFlushed<AppTypeConfig>) -> Result<(), io::Error>
    where
        I: IntoIterator<Item = <AppTypeConfig as openraft::RaftTypeConfig>::Entry> + OptionalSend,
        I::IntoIter: OptionalSend,
    {
        let mut inner = self.inner.lock().await;
        inner.append(entries, callback).await
    }

    async fn truncate(&mut self, log_id: LogIdOf<AppTypeConfig>) -> Result<(), io::Error> {
        let mut inner = self.inner.lock().await;
        inner.truncate(log_id).await
    }

    async fn purge(&mut self, log_id: LogIdOf<AppTypeConfig>) -> Result<(), io::Error> {
        let mut inner = self.inner.lock().await;
        inner.purge(log_id).await
    }

    async fn get_log_reader(&mut self) -> Self::LogReader {
        self.clone()
    }
}

// ====================================================================================
// Redb-backed Raft Log Store (Production Storage)
// ====================================================================================

/// Persistent Raft log backed by redb with chain hashing.
///
/// Stores log entries, vote state, committed index, and last purged log id on disk.
/// Provides ACID guarantees for all operations via redb transactions.
///
/// Each log entry has an associated chain hash computed as:
/// ```text
/// hash = blake3(prev_hash || log_index || term || entry_data)
/// ```
///
/// This creates an unbreakable chain where modifying any entry invalidates all
/// subsequent hashes, enabling detection of hardware corruption and tampering.
///
/// Tiger Style compliance:
/// - Explicitly sized types (u64 for log indices)
/// - Fixed database size limit (configurable at creation)
/// - Fail-fast on corruption (redb panics on invalid state)
/// - Bounded operations (no unbounded iteration)
/// - Chain hashing for integrity verification (32-byte Blake3)
///
/// # Cache Consistency
///
/// The `chain_tip` cache is an optimization to avoid database reads on every append.
/// It is safe because:
///
/// 1. **Raft serializes appends**: OpenRaft guarantees that log appends are serialized at the
///    consensus layer. Only the leader appends, and it does so sequentially. This eliminates
///    concurrent append races.
///
/// 2. **Database is source of truth**: On startup/recovery, `load_chain_tip()` reads from the
///    database, not the cache. A crash after database commit but before cache update is safe - the
///    next startup loads the correct state.
///
/// 3. **Truncate is also serialized**: Log truncation only happens during leader changes, which are
///    also serialized by Raft consensus.
#[derive(Clone, Debug)]
pub struct RedbLogStore {
    db: Arc<Database>,
    path: PathBuf,
    /// Cached chain tip state for efficient appends.
    /// Updated on each append, loaded on startup.
    ///
    /// Uses std::sync::RwLock because operations are fast and we need
    /// to access it from both sync (migration) and async (append) contexts.
    ///
    /// This cache is safe without versioning because Raft serializes all
    /// log operations. See struct-level documentation for details.
    chain_tip: Arc<StdRwLock<ChainTipState>>,
}

impl RedbLogStore {
    /// Create or open a redb-backed log store at the given path.
    ///
    /// Creates the database file and all required tables if they don't exist.
    /// Also initializes chain hashing tables and migrates existing databases.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path = path.as_ref().to_path_buf();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context(CreateDirectorySnafu { path: parent })?;
        }

        // Open existing database without truncating, create if missing.
        let db = if path.exists() {
            Database::open(&path).context(OpenDatabaseSnafu { path: &path })?
        } else {
            Database::create(&path).context(OpenDatabaseSnafu { path: &path })?
        };

        // Initialize tables (including chain hash tables)
        let write_txn = db.begin_write().context(BeginWriteSnafu)?;
        {
            write_txn.open_table(RAFT_LOG_TABLE).context(OpenTableSnafu)?;
            write_txn.open_table(RAFT_META_TABLE).context(OpenTableSnafu)?;
            write_txn.open_table(SNAPSHOT_TABLE).context(OpenTableSnafu)?;
            // Chain hashing tables
            write_txn.open_table(CHAIN_HASH_TABLE).context(OpenTableSnafu)?;
            write_txn.open_table(INTEGRITY_META_TABLE).context(OpenTableSnafu)?;
        }
        write_txn.commit().context(CommitSnafu)?;

        let db = Arc::new(db);

        // Load chain tip from database (or use defaults for empty/migrating database)
        let chain_tip = Self::load_chain_tip(&db)?;

        let store = Self {
            db,
            path,
            chain_tip: Arc::new(StdRwLock::new(chain_tip)),
        };

        // Migrate if needed (one-time operation for existing databases)
        store.migrate_if_needed()?;

        Ok(store)
    }

    /// Load chain tip state from database.
    ///
    /// Returns the cached chain tip if available, otherwise computes it
    /// by scanning the chain hash table (for migration scenarios).
    fn load_chain_tip(db: &Arc<Database>) -> Result<ChainTipState, StorageError> {
        let read_txn = db.begin_read().context(BeginReadSnafu)?;

        // Try to read cached chain tip
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
                // Chain tip not cached, check if we have any chain hashes
                let hash_table = read_txn.open_table(CHAIN_HASH_TABLE).context(OpenTableSnafu)?;

                // Get the last entry's hash
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

                // Empty database or no chain hashes yet
                Ok(ChainTipState::default())
            }
        }
    }

    /// Migrate existing database to chain hashing if needed.
    ///
    /// This is a one-time operation that computes chain hashes for all
    /// existing log entries.
    fn migrate_if_needed(&self) -> Result<(), StorageError> {
        let current_version = self.read_integrity_version()?;
        if current_version >= INTEGRITY_VERSION {
            return Ok(());
        }

        tracing::info!(current_version, target_version = INTEGRITY_VERSION, "migrating log storage to chain hashing");

        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        let mut prev_hash = GENESIS_HASH;
        let mut entry_count: u64 = 0;
        let mut last_index: u64 = 0;

        {
            let log_table = write_txn.open_table(RAFT_LOG_TABLE).context(OpenTableSnafu)?;
            let mut hash_table = write_txn.open_table(CHAIN_HASH_TABLE).context(OpenTableSnafu)?;

            // Iterate all existing entries and compute chain hashes
            for item in log_table.iter().context(RangeSnafu)? {
                let (key, value) = item.context(GetSnafu)?;
                let index = key.value();
                let entry_bytes = value.value();

                // Deserialize to get term
                let entry: <AppTypeConfig as openraft::RaftTypeConfig>::Entry =
                    bincode::deserialize(entry_bytes).context(DeserializeSnafu)?;
                let log_id = entry.log_id();
                let term = log_id.leader_id.term;

                // Compute chain hash
                let entry_hash = compute_entry_hash(&prev_hash, index, term, entry_bytes);

                hash_table.insert(index, entry_hash.as_slice()).context(InsertSnafu)?;

                prev_hash = entry_hash;
                last_index = index;
                entry_count += 1;

                if entry_count.is_multiple_of(10000) {
                    tracing::info!(entry_count, "migration progress");
                }
            }

            // Store migration version
            let mut meta_table = write_txn.open_table(INTEGRITY_META_TABLE).context(OpenTableSnafu)?;
            let version_bytes = bincode::serialize(&INTEGRITY_VERSION).context(SerializeSnafu)?;
            meta_table.insert("integrity_version", version_bytes.as_slice()).context(InsertSnafu)?;

            // Store chain tip
            meta_table.insert("chain_tip_hash", prev_hash.as_slice()).context(InsertSnafu)?;
            let index_bytes = bincode::serialize(&last_index).context(SerializeSnafu)?;
            meta_table.insert("chain_tip_index", index_bytes.as_slice()).context(InsertSnafu)?;
        }
        write_txn.commit().context(CommitSnafu)?;

        // Update in-memory chain tip
        {
            let mut chain_tip = self.chain_tip.write().map_err(|_| StorageError::LockPoisoned {
                context: "writing chain_tip during migration".into(),
            })?;
            chain_tip.hash = prev_hash;
            chain_tip.index = last_index;
        }

        tracing::info!(
            entry_count,
            chain_tip_hash = %hash_to_hex(&prev_hash),
            "migration complete"
        );

        Ok(())
    }

    /// Read the current integrity schema version.
    fn read_integrity_version(&self) -> Result<u32, StorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = match read_txn.open_table(INTEGRITY_META_TABLE) {
            Ok(t) => t,
            Err(_) => return Ok(0), // Table doesn't exist = version 0
        };

        match table.get("integrity_version").context(GetSnafu)? {
            Some(value) => {
                let version: u32 = bincode::deserialize(value.value()).context(DeserializeSnafu)?;
                Ok(version)
            }
            None => Ok(0),
        }
    }

    /// Get the path to the log store database file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Validates storage integrity (used by supervisor before restart).
    ///
    /// This method is called by the actor supervision system before allowing
    /// a RaftActor to restart after a crash. It ensures the storage is not
    /// corrupted and safe to use.
    ///
    /// # Returns
    /// - `Ok(ValidationReport)` if all validation checks pass
    /// - `Err(StorageValidationError)` if any corruption is detected
    ///
    /// # Note
    /// This validation is read-only and does not modify the database.
    /// The database is opened, validated, and closed within this function.
    pub fn validate(
        &self,
        node_id: u64,
    ) -> Result<crate::storage_validation::ValidationReport, crate::storage_validation::StorageValidationError> {
        crate::storage_validation::validate_raft_storage(node_id, &self.path)
    }

    // Internal helper: Read a value from the metadata table
    fn read_meta<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>, StorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(RAFT_META_TABLE).context(OpenTableSnafu)?;

        match table.get(key).context(GetSnafu)? {
            Some(value) => {
                let bytes = value.value();
                let data: T = bincode::deserialize(bytes).context(DeserializeSnafu)?;
                Ok(Some(data))
            }
            None => Ok(None),
        }
    }

    // Internal helper: Write a value to the metadata table
    fn write_meta<T: Serialize>(&self, key: &str, value: &T) -> Result<(), StorageError> {
        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut table = write_txn.open_table(RAFT_META_TABLE).context(OpenTableSnafu)?;
            let serialized = bincode::serialize(value).context(SerializeSnafu)?;
            table.insert(key, serialized.as_slice()).context(InsertSnafu)?;
        }
        write_txn.commit().context(CommitSnafu)?;
        Ok(())
    }

    // Internal helper: Delete a value from the metadata table
    fn delete_meta(&self, key: &str) -> Result<(), StorageError> {
        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut table = write_txn.open_table(RAFT_META_TABLE).context(OpenTableSnafu)?;
            table.remove(key).context(RemoveSnafu)?;
        }
        write_txn.commit().context(CommitSnafu)?;
        Ok(())
    }
}

impl RaftLogReader<AppTypeConfig> for RedbLogStore {
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
        Ok(self.read_meta("vote")?)
    }
}

impl RaftLogStorage<AppTypeConfig> for RedbLogStore {
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
                Ok::<_, StorageError>(entry.log_id())
            })
            .transpose()?;

        let last_purged: Option<LogIdOf<AppTypeConfig>> = self.read_meta("last_purged_log_id")?;
        let last = last_log_id.or(last_purged);

        Ok(LogState {
            last_purged_log_id: last_purged,
            last_log_id: last,
        })
    }

    async fn save_committed(&mut self, committed: Option<LogIdOf<AppTypeConfig>>) -> Result<(), io::Error> {
        if let Some(ref c) = committed {
            self.write_meta("committed", c)?;
        } else {
            self.delete_meta("committed")?;
        }
        Ok(())
    }

    async fn read_committed(&mut self) -> Result<Option<LogIdOf<AppTypeConfig>>, io::Error> {
        Ok(self.read_meta("committed")?)
    }

    async fn save_vote(&mut self, vote: &VoteOf<AppTypeConfig>) -> Result<(), io::Error> {
        // Tiger Style: Check disk space before write to prevent corruption on full disk
        ensure_disk_space_available(&self.path)?;

        self.write_meta("vote", vote)?;
        Ok(())
    }

    async fn append<I>(&mut self, entries: I, callback: IOFlushed<AppTypeConfig>) -> Result<(), io::Error>
    where
        I: IntoIterator<Item = <AppTypeConfig as openraft::RaftTypeConfig>::Entry> + OptionalSend,
        I::IntoIter: OptionalSend,
    {
        // Tiger Style: Check disk space before write to prevent corruption on full disk
        ensure_disk_space_available(&self.path)?;

        // Get current chain tip for hash computation
        let mut prev_hash = {
            let chain_tip = self.chain_tip.read().map_err(|_| StorageError::LockPoisoned {
                context: "reading chain_tip for append".into(),
            })?;
            chain_tip.hash
        };

        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        let mut new_tip_hash = prev_hash;
        let mut new_tip_index: u64 = 0;
        let mut has_entries = false;
        {
            let mut log_table = write_txn.open_table(RAFT_LOG_TABLE).context(OpenTableSnafu)?;
            let mut hash_table = write_txn.open_table(CHAIN_HASH_TABLE).context(OpenTableSnafu)?;

            // Performance optimization: Pre-serialize all entries before inserting
            // This reduces lock contention and allows redb to optimize bulk inserts
            // Tiger Style: Pre-allocate with MAX_BATCH_SIZE to avoid repeated reallocations
            let mut serialized_entries: Vec<(u64, u64, Vec<u8>, ChainHash)> =
                Vec::with_capacity(MAX_BATCH_SIZE as usize);

            for entry in entries {
                let log_id = entry.log_id();
                let index = log_id.index();
                let term = log_id.leader_id.term;
                let data = bincode::serialize(&entry).context(SerializeSnafu)?;

                // Compute chain hash
                let entry_hash = compute_entry_hash(&prev_hash, index, term, &data);

                serialized_entries.push((index, term, data, entry_hash));

                // Update prev_hash for next entry in batch
                prev_hash = entry_hash;
                has_entries = true;
            }

            // Bulk insert all serialized entries and their hashes
            for (index, _term, data, entry_hash) in &serialized_entries {
                log_table.insert(*index, data.as_slice()).context(InsertSnafu)?;
                hash_table.insert(*index, entry_hash.as_slice()).context(InsertSnafu)?;
            }

            // Update chain tip tracking
            if let Some((index, _, _, hash)) = serialized_entries.last() {
                new_tip_hash = *hash;
                new_tip_index = *index;

                // Persist chain tip to integrity metadata table for recovery across restarts
                let mut meta_table = write_txn.open_table(INTEGRITY_META_TABLE).context(OpenTableSnafu)?;
                meta_table.insert("chain_tip_hash", new_tip_hash.as_slice()).context(InsertSnafu)?;
                let index_bytes = bincode::serialize(&new_tip_index).context(SerializeSnafu)?;
                meta_table.insert("chain_tip_index", index_bytes.as_slice()).context(InsertSnafu)?;
            }
        }
        write_txn.commit().context(CommitSnafu)?;

        // Update cached chain tip after successful commit
        if has_entries {
            let mut chain_tip = self.chain_tip.write().map_err(|_| StorageError::LockPoisoned {
                context: "writing chain_tip after append".into(),
            })?;
            chain_tip.hash = new_tip_hash;
            chain_tip.index = new_tip_index;
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

            // Collect keys to remove (>= log_id.index())
            let keys: Vec<u64> = log_table
                .range(truncate_from..)
                .context(RangeSnafu)?
                .map(|item| {
                    let (key, _) = item.context(GetSnafu)?;
                    Ok::<_, StorageError>(key.value())
                })
                .collect::<Result<Vec<_>, _>>()?;

            for key in &keys {
                log_table.remove(*key).context(RemoveSnafu)?;
                hash_table.remove(*key).context(RemoveSnafu)?;
            }
        }
        write_txn.commit().context(CommitSnafu)?;

        // Repair chain tip: read hash from entry at (truncate_from - 1)
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

        // Persist chain tip to integrity metadata table for recovery across restarts
        {
            let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
            {
                let mut meta_table = write_txn.open_table(INTEGRITY_META_TABLE).context(OpenTableSnafu)?;
                meta_table.insert("chain_tip_hash", new_tip.hash.as_slice()).context(InsertSnafu)?;
                let index_bytes = bincode::serialize(&new_tip.index).context(SerializeSnafu)?;
                meta_table.insert("chain_tip_index", index_bytes.as_slice()).context(InsertSnafu)?;
            }
            write_txn.commit().context(CommitSnafu)?;
        }

        {
            let mut chain_tip = self.chain_tip.write().map_err(|_| StorageError::LockPoisoned {
                context: "writing chain_tip after truncate".into(),
            })?;
            *chain_tip = new_tip;
        }

        Ok(())
    }

    async fn purge(&mut self, log_id: LogIdOf<AppTypeConfig>) -> Result<(), io::Error> {
        // Verify purge is monotonic (Tiger Style: fail fast on programmer error)
        if let Some(prev) = self.read_meta::<LogIdOf<AppTypeConfig>>("last_purged_log_id")?
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

            // Collect keys to remove (<= log_id.index())
            let keys: Vec<u64> = log_table
                .range(..=log_id.index())
                .context(RangeSnafu)?
                .map(|item| {
                    let (key, _) = item.context(GetSnafu)?;
                    Ok::<_, StorageError>(key.value())
                })
                .collect::<Result<Vec<_>, _>>()?;

            for key in &keys {
                log_table.remove(*key).context(RemoveSnafu)?;
                hash_table.remove(*key).context(RemoveSnafu)?;
            }
        }
        write_txn.commit().context(CommitSnafu)?;

        self.write_meta("last_purged_log_id", &log_id)?;
        Ok(())
    }

    async fn get_log_reader(&mut self) -> Self::LogReader {
        self.clone()
    }
}

impl RedbLogStore {
    /// Reads the committed log index from metadata.
    ///
    /// Used for cross-storage validation to ensure state machine consistency.
    /// Returns the committed log index if it exists, None otherwise.
    pub async fn read_committed_sync(&self) -> Result<Option<u64>, io::Error> {
        let committed: Option<LogIdOf<AppTypeConfig>> = self.read_meta("committed").map_err(io::Error::other)?;
        Ok(committed.map(|log_id| log_id.index))
    }

    /// Read chain hash at a specific log index.
    ///
    /// Returns `None` if no hash exists at that index.
    fn read_chain_hash_at(&self, index: u64) -> Result<Option<ChainHash>, StorageError> {
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

    /// Get the current chain tip for cross-replica verification.
    ///
    /// Returns (tip_index, tip_hash) which can be compared across replicas.
    /// If chain tips match, logs are identical up to that point.
    pub fn chain_tip_for_verification(&self) -> Result<(u64, ChainHash), StorageError> {
        let chain_tip = self.chain_tip.read().map_err(|_| StorageError::LockPoisoned {
            context: "reading chain_tip for verification".into(),
        })?;
        Ok((chain_tip.index, chain_tip.hash))
    }

    /// Get chain tip hash as hex string for logging/display.
    pub fn chain_tip_hash_hex(&self) -> Result<String, StorageError> {
        let chain_tip = self.chain_tip.read().map_err(|_| StorageError::LockPoisoned {
            context: "reading chain_tip for hex display".into(),
        })?;
        Ok(hash_to_hex(&chain_tip.hash))
    }

    /// Verify chain integrity for a batch of log entries.
    ///
    /// Returns Ok(true) if the batch is valid, Ok(false) if verification
    /// cannot proceed (e.g., entries don't exist), or Err with corruption details.
    ///
    /// # Arguments
    ///
    /// * `start_index` - First log index to verify
    /// * `batch_size` - Number of entries to verify (bounded by CHAIN_VERIFY_BATCH_SIZE)
    ///
    /// # Tiger Style
    ///
    /// - Bounded batch size prevents unbounded verification
    /// - Fail-fast on corruption detection
    pub fn verify_chain_batch(&self, start_index: u64, batch_size: u32) -> Result<u64, StorageError> {
        use crate::integrity::verify_entry_hash;

        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let log_table = read_txn.open_table(RAFT_LOG_TABLE).context(OpenTableSnafu)?;
        let hash_table = read_txn.open_table(CHAIN_HASH_TABLE).context(OpenTableSnafu)?;

        // Get previous hash (for chain linkage)
        let mut prev_hash = if start_index == 0 || start_index == 1 {
            GENESIS_HASH
        } else {
            let prev_index = start_index - 1;
            match hash_table.get(prev_index).context(GetSnafu)? {
                Some(h) if h.value().len() == 32 => {
                    let mut hash = [0u8; 32];
                    hash.copy_from_slice(h.value());
                    hash
                }
                _ => return Ok(0), // Previous hash not found, cannot verify
            }
        };

        let end_index = start_index.saturating_add(batch_size as u64);
        let mut verified_count: u64 = 0;

        for index in start_index..end_index {
            // Get entry
            let entry_bytes = match log_table.get(index).context(GetSnafu)? {
                Some(v) => v.value().to_vec(),
                None => break, // No more entries
            };

            // Get stored hash
            let stored_hash = match hash_table.get(index).context(GetSnafu)? {
                Some(h) if h.value().len() == 32 => {
                    let mut hash = [0u8; 32];
                    hash.copy_from_slice(h.value());
                    hash
                }
                _ => {
                    return Err(StorageError::ChainHashMissing { index });
                }
            };

            // Deserialize to get term
            let entry: <AppTypeConfig as openraft::RaftTypeConfig>::Entry =
                bincode::deserialize(&entry_bytes).context(DeserializeSnafu)?;
            let log_id = entry.log_id();
            let term = log_id.leader_id.term;

            // Verify hash
            if !verify_entry_hash(&prev_hash, index, term, &entry_bytes, &stored_hash) {
                let computed = compute_entry_hash(&prev_hash, index, term, &entry_bytes);
                return Err(StorageError::ChainIntegrityViolation {
                    index,
                    expected: hash_to_hex(&stored_hash),
                    found: hash_to_hex(&computed),
                });
            }

            prev_hash = stored_hash;
            verified_count += 1;
        }

        Ok(verified_count)
    }

    /// Get the range of log indices available for verification.
    ///
    /// Returns (first_index, last_index) or None if no entries exist.
    pub fn verification_range(&self) -> Result<Option<(u64, u64)>, StorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let log_table = read_txn.open_table(RAFT_LOG_TABLE).context(OpenTableSnafu)?;

        let first = log_table.iter().context(RangeSnafu)?.next();
        let last = log_table.iter().context(RangeSnafu)?.last();

        match (first, last) {
            (Some(first_result), Some(last_result)) => {
                let (first_key, _) = first_result.context(GetSnafu)?;
                let (last_key, _) = last_result.context(GetSnafu)?;
                Ok(Some((first_key.value(), last_key.value())))
            }
            _ => Ok(None),
        }
    }
}

// ============================================================================
// Historical Log Reader Implementation
// ============================================================================

#[async_trait::async_trait]
impl aspen_transport::log_subscriber::HistoricalLogReader for RedbLogStore {
    async fn read_entries(
        &self,
        start_index: u64,
        end_index: u64,
    ) -> Result<Vec<aspen_transport::log_subscriber::LogEntryPayload>, std::io::Error> {
        {
            use aspen_transport::log_subscriber::KvOperation;
            use aspen_transport::log_subscriber::LogEntryPayload;
            use aspen_transport::log_subscriber::MAX_HISTORICAL_BATCH_SIZE;
            use openraft::EntryPayload;
            use snafu::ResultExt;

            // Tiger Style: Bound the batch size
            let actual_end = std::cmp::min(end_index, start_index.saturating_add(MAX_HISTORICAL_BATCH_SIZE as u64));

            let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
            let table = read_txn.open_table(RAFT_LOG_TABLE).context(OpenTableSnafu)?;

            let mut entries = Vec::new();
            let iter = table.range(start_index..=actual_end).context(RangeSnafu)?;

            // Create HLC for historical replay timestamps
            // Note: This is synthetic since we don't store the original HLC with entries
            let hlc = aspen_core::hlc::create_hlc("historical-reader");

            for item in iter {
                let (_key, value) = item.context(GetSnafu)?;
                let bytes = value.value();
                let entry: <AppTypeConfig as openraft::RaftTypeConfig>::Entry =
                    bincode::deserialize(bytes).context(DeserializeSnafu)?;

                let log_id = entry.log_id();
                let operation = match &entry.payload {
                    EntryPayload::Blank => KvOperation::Noop,
                    EntryPayload::Normal(app_request) => KvOperation::from(app_request.clone()),
                    EntryPayload::Membership(_) => KvOperation::MembershipChange {
                        description: "membership change".to_string(),
                    },
                };

                entries.push(LogEntryPayload {
                    index: log_id.index,
                    term: log_id.leader_id.term,
                    hlc_timestamp: SerializableTimestamp::from(hlc.new_timestamp()),
                    operation,
                });
            }

            Ok(entries)
        }
    }

    async fn earliest_available_index(&self) -> Result<Option<u64>, std::io::Error> {
        use snafu::ResultExt;

        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(RAFT_LOG_TABLE).context(OpenTableSnafu)?;

        let first = table.iter().context(RangeSnafu)?.next();
        match first {
            Some(result) => {
                let (key, _) = result.context(GetSnafu)?;
                Ok(Some(key.value()))
            }
            None => Ok(None),
        }
    }
}

/// Snapshot blob stored in memory for testing.
///
/// Contains both the snapshot metadata (last log ID, membership) and
/// the serialized state machine data, along with optional integrity hash
/// for corruption detection.
#[derive(Debug, Serialize, Deserialize)]
pub struct StoredSnapshot {
    /// Snapshot metadata (last log ID, membership, snapshot ID).
    pub meta: openraft::SnapshotMeta<AppTypeConfig>,
    /// Serialized state machine data (JSON-encoded KV map).
    pub data: Vec<u8>,
    /// Optional integrity hash for corruption detection (Tiger Style).
    #[serde(default)]
    pub integrity: Option<SnapshotIntegrity>,
}

/// Internal state machine data for InMemoryStateMachine.
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct StateMachineData {
    /// Last log ID that was applied to the state machine.
    pub last_applied_log: Option<openraft::LogId<AppTypeConfig>>,
    /// Last known membership configuration.
    pub last_membership: StoredMembership<AppTypeConfig>,
    /// Key-value data store.
    pub data: BTreeMap<String, String>,
}

/// Simple in-memory state machine that mirrors the openraft memstore example.
///
/// Provides a non-persistent KV store for testing and simulations. All data
/// is stored in a `BTreeMap` and lost when the state machine is dropped.
#[derive(Debug, Default)]
pub struct InMemoryStateMachine {
    /// State machine data (last applied log, membership, KV data).
    state_machine: RwLock<StateMachineData>,
    /// Counter for generating unique snapshot IDs.
    snapshot_idx: AtomicU64,
    /// Currently held snapshot.
    current_snapshot: RwLock<Option<StoredSnapshot>>,
}

impl InMemoryStateMachine {
    /// Create a new in-memory state machine wrapped in an Arc.
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Get a value from the state machine by key.
    ///
    /// Returns `None` if the key does not exist.
    pub async fn get(&self, key: &str) -> Option<String> {
        let sm = self.state_machine.read().await;
        sm.data.get(key).cloned()
    }

    /// Scan all keys that start with the given prefix (async version).
    ///
    /// Returns a list of full key names.
    ///
    /// # Arguments
    ///
    /// * `prefix` - Key prefix to match
    pub async fn scan_keys_with_prefix(&self, prefix: &str) -> Vec<String> {
        let sm = self.state_machine.read().await;
        sm.data.keys().filter(|k| k.starts_with(prefix)).cloned().collect()
    }

    /// Scan all key-value pairs that start with the given prefix.
    ///
    /// Returns a list of (key, value) pairs.
    ///
    /// # Arguments
    ///
    /// * `prefix` - Key prefix to match
    pub async fn scan_kv_with_prefix(&self, prefix: &str) -> Vec<(String, String)> {
        let sm = self.state_machine.read().await;
        sm.data.iter().filter(|(k, _)| k.starts_with(prefix)).map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    /// Async version of scan_kv_with_prefix for use in async contexts.
    ///
    /// Returns a list of (key, value) pairs matching the prefix.
    ///
    /// # Arguments
    ///
    /// * `prefix` - Key prefix to match
    pub async fn scan_kv_with_prefix_async(&self, prefix: &str) -> Vec<(String, String)> {
        let sm = self.state_machine.read().await;
        sm.data.iter().filter(|(k, _)| k.starts_with(prefix)).map(|(k, v)| (k.clone(), v.clone())).collect()
    }
}

impl RaftSnapshotBuilder<AppTypeConfig> for Arc<InMemoryStateMachine> {
    #[tracing::instrument(level = "trace", skip(self))]
    async fn build_snapshot(&mut self) -> Result<Snapshot<AppTypeConfig>, io::Error> {
        let state_machine = self.state_machine.read().await;
        let data =
            serde_json::to_vec(&state_machine.data).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        let last_applied_log = state_machine.last_applied_log;
        let last_membership = state_machine.last_membership.clone();
        let mut current_snapshot = self.current_snapshot.write().await;
        drop(state_machine);

        let snapshot_idx = self.snapshot_idx.fetch_add(1, Ordering::Relaxed) + 1;
        let snapshot_id = if let Some(last) = last_applied_log {
            format!("{}-{}-{snapshot_idx}", last.committed_leader_id(), last.index())
        } else {
            format!("--{snapshot_idx}")
        };

        let meta = openraft::SnapshotMeta {
            last_log_id: last_applied_log,
            last_membership,
            snapshot_id,
        };

        // Compute snapshot integrity hash (Tiger Style: verify data corruption)
        let meta_bytes = bincode::serialize(&meta).map_err(|err| io::Error::other(err.to_string()))?;
        let integrity = SnapshotIntegrity::compute(&meta_bytes, &data, GENESIS_HASH);

        let snapshot = StoredSnapshot {
            meta: meta.clone(),
            data: data.clone(),
            integrity: Some(integrity),
        };
        *current_snapshot = Some(snapshot);

        Ok(Snapshot {
            meta,
            snapshot: Cursor::new(data),
        })
    }
}

impl RaftStateMachine<AppTypeConfig> for Arc<InMemoryStateMachine> {
    type SnapshotBuilder = Self;

    async fn applied_state(
        &mut self,
    ) -> Result<(Option<openraft::LogId<AppTypeConfig>>, StoredMembership<AppTypeConfig>), io::Error> {
        let state_machine = self.state_machine.read().await;
        Ok((state_machine.last_applied_log, state_machine.last_membership.clone()))
    }

    #[tracing::instrument(level = "trace", skip(self, entries))]
    async fn apply<Strm>(&mut self, mut entries: Strm) -> Result<(), io::Error>
    where Strm: Stream<Item = Result<EntryResponder<AppTypeConfig>, io::Error>> + Unpin + OptionalSend {
        let mut sm = self.state_machine.write().await;
        while let Some((entry, responder)) = entries.try_next().await? {
            sm.last_applied_log = Some(entry.log_id);
            let response = match entry.payload {
                EntryPayload::Blank => AppResponse::default(),
                EntryPayload::Normal(ref req) => match req {
                    AppRequest::Set { key, value } => {
                        sm.data.insert(key.clone(), value.clone());
                        AppResponse {
                            value: Some(value.clone()),
                            ..Default::default()
                        }
                    }
                    // TTL operations in in-memory store: we store the value but don't
                    // track expiration (in-memory is for testing only, not production).
                    AppRequest::SetWithTTL { key, value, .. } => {
                        sm.data.insert(key.clone(), value.clone());
                        AppResponse {
                            value: Some(value.clone()),
                            ..Default::default()
                        }
                    }
                    AppRequest::SetMulti { pairs } => {
                        for (key, value) in pairs {
                            sm.data.insert(key.clone(), value.clone());
                        }
                        AppResponse::default()
                    }
                    AppRequest::SetMultiWithTTL { pairs, .. } => {
                        for (key, value) in pairs {
                            sm.data.insert(key.clone(), value.clone());
                        }
                        AppResponse::default()
                    }
                    AppRequest::Delete { key } => {
                        let existed = sm.data.remove(key).is_some();
                        AppResponse {
                            deleted: Some(existed),
                            ..Default::default()
                        }
                    }
                    AppRequest::DeleteMulti { keys } => {
                        let mut deleted_any = false;
                        for key in keys {
                            deleted_any |= sm.data.contains_key(key);
                            sm.data.remove(key);
                        }
                        AppResponse {
                            deleted: Some(deleted_any),
                            ..Default::default()
                        }
                    }
                    AppRequest::CompareAndSwap {
                        key,
                        expected,
                        new_value,
                    } => {
                        let current = sm.data.get(key).cloned();
                        let condition_matches = match (expected.as_ref(), &current) {
                            (None, None) => true,
                            (Some(exp), Some(cur)) => exp == cur,
                            _ => false,
                        };
                        if condition_matches {
                            sm.data.insert(key.clone(), new_value.clone());
                            AppResponse {
                                value: Some(new_value.clone()),
                                cas_succeeded: Some(true),
                                ..Default::default()
                            }
                        } else {
                            AppResponse {
                                value: current,
                                cas_succeeded: Some(false),
                                ..Default::default()
                            }
                        }
                    }
                    AppRequest::CompareAndDelete { key, expected } => {
                        let current = sm.data.get(key).cloned();
                        let condition_matches = matches!(&current, Some(cur) if cur == expected);
                        if condition_matches {
                            sm.data.remove(key);
                            AppResponse {
                                deleted: Some(true),
                                cas_succeeded: Some(true),
                                ..Default::default()
                            }
                        } else {
                            AppResponse {
                                value: current,
                                cas_succeeded: Some(false),
                                ..Default::default()
                            }
                        }
                    }
                    AppRequest::Batch { operations } => {
                        for (is_set, key, value) in operations {
                            if *is_set {
                                sm.data.insert(key.clone(), value.clone());
                            } else {
                                sm.data.remove(key);
                            }
                        }
                        AppResponse {
                            batch_applied: Some(operations.len() as u32),
                            ..Default::default()
                        }
                    }
                    AppRequest::ConditionalBatch { conditions, operations } => {
                        // Check all conditions first
                        // condition types: 0=ValueEquals, 1=KeyExists, 2=KeyNotExists
                        let mut conditions_met = true;
                        let mut failed_index = None;
                        for (i, (cond_type, key, expected)) in conditions.iter().enumerate() {
                            let current = sm.data.get(key);
                            let met = match cond_type {
                                0 => current.map(|v| v == expected).unwrap_or(false), // ValueEquals
                                1 => current.is_some(),                               // KeyExists
                                2 => current.is_none(),                               // KeyNotExists
                                _ => false,
                            };
                            if !met {
                                conditions_met = false;
                                failed_index = Some(i as u32);
                                break;
                            }
                        }

                        if conditions_met {
                            // Apply all operations
                            for (is_set, key, value) in operations {
                                if *is_set {
                                    sm.data.insert(key.clone(), value.clone());
                                } else {
                                    sm.data.remove(key);
                                }
                            }
                            AppResponse {
                                batch_applied: Some(operations.len() as u32),
                                conditions_met: Some(true),
                                ..Default::default()
                            }
                        } else {
                            AppResponse {
                                conditions_met: Some(false),
                                failed_condition_index: failed_index,
                                ..Default::default()
                            }
                        }
                    }
                    // Lease operations in in-memory store: store values but don't track leases.
                    // This is for testing only, not production.
                    AppRequest::SetWithLease { key, value, .. } => {
                        sm.data.insert(key.clone(), value.clone());
                        AppResponse {
                            value: Some(value.clone()),
                            ..Default::default()
                        }
                    }
                    AppRequest::SetMultiWithLease { pairs, .. } => {
                        for (key, value) in pairs {
                            sm.data.insert(key.clone(), value.clone());
                        }
                        AppResponse::default()
                    }
                    AppRequest::LeaseGrant { lease_id, ttl_seconds } => {
                        // In-memory doesn't track leases, just return success
                        AppResponse {
                            lease_id: Some(*lease_id),
                            ttl_seconds: Some(*ttl_seconds),
                            ..Default::default()
                        }
                    }
                    AppRequest::LeaseRevoke { lease_id } => {
                        // In-memory doesn't track leases, just return success
                        AppResponse {
                            lease_id: Some(*lease_id),
                            keys_deleted: Some(0),
                            ..Default::default()
                        }
                    }
                    AppRequest::LeaseKeepalive { lease_id } => {
                        // In-memory doesn't track leases, just return success
                        AppResponse {
                            lease_id: Some(*lease_id),
                            ttl_seconds: Some(60), // Dummy value
                            ..Default::default()
                        }
                    }
                    // Transaction: etcd-style conditional transactions
                    // Note: In-memory doesn't track versions, so version-based comparisons
                    // always compare against 0 (as if the key doesn't exist with version).
                    AppRequest::Transaction {
                        compare,
                        success,
                        failure,
                    } => {
                        // Evaluate all comparison conditions
                        let mut all_conditions_met = true;

                        for (target, op, key, value) in compare {
                            let current_value = sm.data.get(key);

                            let condition_met = match target {
                                0 => {
                                    // Value comparison
                                    match op {
                                        0 => current_value.map(|v| v.as_str()) == Some(value.as_str()), // Equal
                                        1 => current_value.map(|v| v.as_str()) != Some(value.as_str()), // NotEqual
                                        2 => current_value.map(|v| v.as_str() > value.as_str()).unwrap_or(false), /* Greater */
                                        3 => current_value.map(|v| v.as_str() < value.as_str()).unwrap_or(false), /* Less */
                                        _ => false,
                                    }
                                }
                                1..=3 => {
                                    // Version/CreateRevision/ModRevision comparison
                                    // In-memory doesn't track versions, treat as 0
                                    let current_version: i64 = 0;
                                    let expected_version: i64 = value.parse().unwrap_or(0);
                                    match op {
                                        0 => current_version == expected_version,
                                        1 => current_version != expected_version,
                                        2 => current_version > expected_version,
                                        3 => current_version < expected_version,
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
                                    sm.data.insert(key.clone(), value.clone());
                                    TxnOpResult::Put { revision: 0 }
                                }
                                1 => {
                                    // Delete operation
                                    let deleted = if sm.data.remove(key).is_some() { 1 } else { 0 };
                                    TxnOpResult::Delete { deleted }
                                }
                                2 => {
                                    // Get operation
                                    let kv = sm.data.get(key).map(|v| KeyValueWithRevision {
                                        key: key.clone(),
                                        value: v.clone(),
                                        version: 0,
                                        create_revision: 0,
                                        mod_revision: 0,
                                    });
                                    TxnOpResult::Get { kv }
                                }
                                3 => {
                                    // Range operation
                                    let limit: usize = value.parse().unwrap_or(10);
                                    let prefix = key;
                                    let kvs: Vec<_> = sm
                                        .data
                                        .iter()
                                        .filter(|(k, _)| k.starts_with(prefix))
                                        .take(limit)
                                        .map(|(k, v)| KeyValueWithRevision {
                                            key: k.clone(),
                                            value: v.clone(),
                                            version: 0,
                                            create_revision: 0,
                                            mod_revision: 0,
                                        })
                                        .collect();
                                    TxnOpResult::Range { kvs, more: false }
                                }
                                _ => continue,
                            };
                            results.push(result);
                        }

                        AppResponse {
                            succeeded: Some(all_conditions_met),
                            txn_results: Some(results),
                            ..Default::default()
                        }
                    }
                    // OptimisticTransaction: in-memory state machine doesn't track versions,
                    // so we can't do proper OCC validation. Just apply the writes.
                    AppRequest::OptimisticTransaction { write_set, .. } => {
                        for (is_set, key, value) in write_set {
                            if *is_set {
                                sm.data.insert(key.clone(), value.clone());
                            } else {
                                sm.data.remove(key);
                            }
                        }
                        AppResponse {
                            occ_conflict: Some(false),
                            batch_applied: Some(write_set.len() as u32),
                            ..Default::default()
                        }
                    }
                    // Shard topology operations: in-memory doesn't support sharding,
                    // just return success (for testing purposes only).
                    AppRequest::ShardSplit { .. }
                    | AppRequest::ShardMerge { .. }
                    | AppRequest::TopologyUpdate { .. } => AppResponse::default(),
                },
                EntryPayload::Membership(ref membership) => {
                    sm.last_membership = StoredMembership::new(Some(entry.log_id), membership.clone());
                    AppResponse::default()
                }
            };
            if let Some(responder) = responder {
                responder.send(response);
            }
        }
        Ok(())
    }

    async fn begin_receiving_snapshot(&mut self) -> Result<SnapshotDataOf<AppTypeConfig>, io::Error> {
        let mut current_snapshot = self.current_snapshot.write().await;
        Ok(match current_snapshot.take() {
            Some(snapshot) => Cursor::new(snapshot.data),
            None => Cursor::new(Vec::new()),
        })
    }

    async fn install_snapshot(
        &mut self,
        meta: &openraft::SnapshotMeta<AppTypeConfig>,
        mut snapshot: SnapshotDataOf<AppTypeConfig>,
    ) -> Result<(), io::Error> {
        // Read snapshot data
        let mut snapshot_data = Vec::new();
        std::io::copy(&mut snapshot, &mut snapshot_data)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

        let new_data: BTreeMap<String, String> =
            serde_json::from_slice(&snapshot_data).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

        // Update state machine
        let mut sm = self.state_machine.write().await;
        sm.data = new_data;
        sm.last_applied_log = meta.last_log_id;
        sm.last_membership = meta.last_membership.clone();
        drop(sm);

        // Compute integrity hash for the installed snapshot (Tiger Style)
        let meta_bytes = bincode::serialize(meta).map_err(|err| io::Error::other(err.to_string()))?;
        let integrity = SnapshotIntegrity::compute(&meta_bytes, &snapshot_data, GENESIS_HASH);

        // Store the installed snapshot so get_current_snapshot() returns it
        let mut current_snapshot = self.current_snapshot.write().await;
        *current_snapshot = Some(StoredSnapshot {
            meta: meta.clone(),
            data: snapshot_data,
            integrity: Some(integrity),
        });

        Ok(())
    }

    async fn get_current_snapshot(&mut self) -> Result<Option<Snapshot<AppTypeConfig>>, io::Error> {
        let snapshot = self.current_snapshot.read().await;
        Ok(snapshot.as_ref().map(|snap| Snapshot {
            meta: snap.meta.clone(),
            snapshot: Cursor::new(snap.data.clone()),
        }))
    }

    async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
        self.clone()
    }
}

// ====================================================================================
// Redb State Machine (REMOVED - Use SqliteStateMachine)
// ====================================================================================
//
// RedbStateMachine has been removed. Use SqliteStateMachine instead.
// The hybrid architecture now uses:
// - Redb for log storage (fast, append-only)
// - SQLite for state machine (ACID, queryable)
//
// See src/raft/storage_sqlite.rs for the SQLite state machine implementation.

#[cfg(test)]
mod tests {
    use openraft::Vote;
    use tempfile::TempDir;

    use super::*;
    use crate::types::NodeId;

    // =========================================================================
    // StorageBackend Enum Tests
    // =========================================================================

    #[test]
    fn test_storage_backend_default() {
        let backend = StorageBackend::default();
        assert_eq!(backend, StorageBackend::Redb);
    }

    #[test]
    fn test_storage_backend_from_str_inmemory() {
        assert_eq!("inmemory".parse::<StorageBackend>().unwrap(), StorageBackend::InMemory);
        assert_eq!("in-memory".parse::<StorageBackend>().unwrap(), StorageBackend::InMemory);
        assert_eq!("memory".parse::<StorageBackend>().unwrap(), StorageBackend::InMemory);
    }

    #[test]
    fn test_storage_backend_from_str_redb() {
        assert_eq!("redb".parse::<StorageBackend>().unwrap(), StorageBackend::Redb);
        assert_eq!("single-fsync".parse::<StorageBackend>().unwrap(), StorageBackend::Redb);
        assert_eq!("fast".parse::<StorageBackend>().unwrap(), StorageBackend::Redb);
        assert_eq!("persistent".parse::<StorageBackend>().unwrap(), StorageBackend::Redb);
        assert_eq!("disk".parse::<StorageBackend>().unwrap(), StorageBackend::Redb);
    }

    #[test]
    fn test_storage_backend_from_str_case_insensitive() {
        assert_eq!("INMEMORY".parse::<StorageBackend>().unwrap(), StorageBackend::InMemory);
        assert_eq!("REDB".parse::<StorageBackend>().unwrap(), StorageBackend::Redb);
        assert_eq!("InMemory".parse::<StorageBackend>().unwrap(), StorageBackend::InMemory);
    }

    #[test]
    fn test_storage_backend_from_str_invalid() {
        let result = "invalid".parse::<StorageBackend>();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Invalid storage backend"));
    }

    #[test]
    fn test_storage_backend_display_inmemory() {
        assert_eq!(format!("{}", StorageBackend::InMemory), "inmemory");
    }

    #[test]
    fn test_storage_backend_display_redb() {
        assert_eq!(format!("{}", StorageBackend::Redb), "redb");
    }

    #[test]
    fn test_storage_backend_roundtrip() {
        let original = StorageBackend::InMemory;
        let display = format!("{}", original);
        let parsed: StorageBackend = display.parse().unwrap();
        assert_eq!(original, parsed);

        let original = StorageBackend::Redb;
        let display = format!("{}", original);
        let parsed: StorageBackend = display.parse().unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_storage_backend_clone() {
        let backend = StorageBackend::Redb;
        let cloned = backend;
        assert_eq!(backend, cloned);
    }

    #[test]
    fn test_storage_backend_debug() {
        let debug_str = format!("{:?}", StorageBackend::InMemory);
        assert!(debug_str.contains("InMemory"));
    }

    #[test]
    fn test_storage_backend_serde_roundtrip() {
        let original = StorageBackend::Redb;
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: StorageBackend = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_storage_backend_serde_inmemory() {
        let original = StorageBackend::InMemory;
        let json = serde_json::to_string(&original).expect("serialize");
        assert_eq!(json, "\"inmemory\"");
    }

    // =========================================================================
    // StorageError Tests
    // =========================================================================

    #[test]
    fn test_storage_error_into_io_error() {
        let err = StorageError::ChainIntegrityViolation {
            index: 42,
            expected: "abc".to_string(),
            found: "def".to_string(),
        };
        let io_err: io::Error = err.into();
        let msg = io_err.to_string();
        assert!(msg.contains("chain integrity violation"));
        assert!(msg.contains("42"));
    }

    #[test]
    fn test_storage_error_snapshot_integrity_failed() {
        let err = StorageError::SnapshotIntegrityFailed {
            reason: "corrupted data".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("snapshot integrity"));
        assert!(msg.contains("corrupted data"));
    }

    #[test]
    fn test_storage_error_chain_hash_missing() {
        let err = StorageError::ChainHashMissing { index: 100 };
        let msg = format!("{}", err);
        assert!(msg.contains("chain hash missing"));
        assert!(msg.contains("100"));
    }

    // =========================================================================
    // InMemoryLogStore Tests
    // =========================================================================

    #[test]
    fn test_inmemory_log_store_default() {
        let store = InMemoryLogStore::default();
        // Should be able to clone
        let _cloned = store.clone();
    }

    #[tokio::test]
    async fn test_inmemory_log_store_vote_roundtrip() {
        let mut store = InMemoryLogStore::default();
        let vote = Vote::new(1, NodeId::new(1));

        // Initially no vote
        assert_eq!(store.read_vote().await.unwrap(), None);

        // Save vote
        store.save_vote(&vote).await.unwrap();

        // Read back
        assert_eq!(store.read_vote().await.unwrap(), Some(vote));
    }

    #[tokio::test]
    async fn test_inmemory_log_store_committed_roundtrip() {
        use openraft::testing::log_id;
        let mut store = InMemoryLogStore::default();

        // Initially no committed
        assert_eq!(store.read_committed().await.unwrap(), None);

        // Save committed
        let committed = log_id::<AppTypeConfig>(1, NodeId::new(1), 5);
        store.save_committed(Some(committed)).await.unwrap();

        // Read back
        assert_eq!(store.read_committed().await.unwrap(), Some(committed));

        // Clear committed
        store.save_committed(None).await.unwrap();
        assert_eq!(store.read_committed().await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_inmemory_log_store_get_log_state_empty() {
        let mut store = InMemoryLogStore::default();
        let state = store.get_log_state().await.unwrap();

        assert_eq!(state.last_purged_log_id, None);
        assert_eq!(state.last_log_id, None);
    }

    #[tokio::test]
    async fn test_inmemory_log_store_clone_shares_state() {
        let store = InMemoryLogStore::default();
        let mut store1 = store.clone();
        let mut store2 = store.clone();

        let vote = Vote::new(2, NodeId::new(2));
        store1.save_vote(&vote).await.unwrap();

        // Both clones should see the vote (shared state)
        assert_eq!(store2.read_vote().await.unwrap(), Some(vote));
    }

    // =========================================================================
    // RedbLogStore Tests
    // =========================================================================

    #[tokio::test]
    async fn redb_log_store_preserves_vote_on_reopen() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("raft-log.redb");

        let vote = Vote::new(1, NodeId::new(1));

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            store.save_vote(&vote).await.unwrap();
        }

        let mut reopened = RedbLogStore::new(&db_path).unwrap();
        let recovered = reopened.read_vote().await.unwrap();

        assert_eq!(recovered, Some(vote));
    }

    #[tokio::test]
    async fn test_redb_log_store_new_creates_file() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("new-log.redb");

        assert!(!db_path.exists());

        let _store = RedbLogStore::new(&db_path).unwrap();

        assert!(db_path.exists());
    }

    #[tokio::test]
    async fn test_redb_log_store_path() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test-log.redb");

        let store = RedbLogStore::new(&db_path).unwrap();
        assert_eq!(store.path(), db_path);
    }

    #[tokio::test]
    async fn test_redb_log_store_initial_state_empty() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("empty-log.redb");

        let mut store = RedbLogStore::new(&db_path).unwrap();
        let state = store.get_log_state().await.unwrap();

        assert_eq!(state.last_purged_log_id, None);
        assert_eq!(state.last_log_id, None);
    }

    #[tokio::test]
    async fn test_redb_log_store_committed_roundtrip() {
        use openraft::testing::log_id;
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("committed-log.redb");

        let mut store = RedbLogStore::new(&db_path).unwrap();

        // Initially no committed
        assert_eq!(store.read_committed().await.unwrap(), None);

        // Save committed
        let committed = log_id::<AppTypeConfig>(1, NodeId::new(1), 10);
        store.save_committed(Some(committed)).await.unwrap();

        // Read back
        assert_eq!(store.read_committed().await.unwrap(), Some(committed));
    }

    #[tokio::test]
    async fn test_redb_log_store_committed_clear() {
        use openraft::testing::log_id;
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("committed-clear-log.redb");

        let mut store = RedbLogStore::new(&db_path).unwrap();

        // Save then clear
        let committed = log_id::<AppTypeConfig>(1, NodeId::new(1), 5);
        store.save_committed(Some(committed)).await.unwrap();
        store.save_committed(None).await.unwrap();

        assert_eq!(store.read_committed().await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_redb_log_store_creates_parent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("nested").join("path").join("log.redb");

        // Parent doesn't exist yet
        assert!(!db_path.parent().unwrap().exists());

        let _store = RedbLogStore::new(&db_path).unwrap();

        assert!(db_path.exists());
    }

    #[tokio::test]
    async fn test_redb_log_store_chain_tip_initial() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("chain-tip-log.redb");

        let store = RedbLogStore::new(&db_path).unwrap();
        let (index, hash) = store.chain_tip_for_verification().unwrap();

        // Initial chain tip should be genesis
        assert_eq!(index, 0);
        assert_eq!(hash, GENESIS_HASH);
    }

    #[tokio::test]
    async fn test_redb_log_store_chain_tip_hash_hex() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("chain-hex-log.redb");

        let store = RedbLogStore::new(&db_path).unwrap();
        let hex = store.chain_tip_hash_hex().unwrap();

        // Should be a valid hex string (64 characters for 32 bytes)
        assert_eq!(hex.len(), 64);
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[tokio::test]
    async fn test_redb_log_store_verification_range_empty() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("range-empty-log.redb");

        let store = RedbLogStore::new(&db_path).unwrap();
        let range = store.verification_range().unwrap();

        assert_eq!(range, None);
    }

    #[tokio::test]
    async fn test_redb_log_store_clone() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("clone-log.redb");

        let store = RedbLogStore::new(&db_path).unwrap();
        let cloned = store.clone();

        // Both should point to same path
        assert_eq!(store.path(), cloned.path());
    }

    #[tokio::test]
    async fn test_redb_log_store_get_log_reader() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("reader-log.redb");

        let mut store = RedbLogStore::new(&db_path).unwrap();
        let mut reader = store.get_log_reader().await;

        // Reader should work
        let vote = reader.read_vote().await.unwrap();
        assert_eq!(vote, None);
    }

    // =========================================================================
    // InMemoryStateMachine Tests
    // =========================================================================

    #[test]
    fn test_inmemory_state_machine_new() {
        let sm = InMemoryStateMachine::new();
        // Should be wrapped in Arc
        let _cloned = Arc::clone(&sm);
    }

    #[tokio::test]
    async fn test_inmemory_state_machine_get_nonexistent() {
        let sm = InMemoryStateMachine::new();
        let value = sm.get("nonexistent").await;
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_inmemory_state_machine_scan_keys_empty() {
        let sm = InMemoryStateMachine::new();
        let keys = sm.scan_keys_with_prefix("test:").await;
        assert!(keys.is_empty());
    }

    #[tokio::test]
    async fn test_inmemory_state_machine_scan_kv_empty() {
        let sm = InMemoryStateMachine::new();
        let pairs = sm.scan_kv_with_prefix("test:").await;
        assert!(pairs.is_empty());
    }

    #[tokio::test]
    async fn test_inmemory_state_machine_scan_kv_async_empty() {
        let sm = InMemoryStateMachine::new();
        let pairs = sm.scan_kv_with_prefix_async("test:").await;
        assert!(pairs.is_empty());
    }

    #[tokio::test]
    async fn test_inmemory_state_machine_applied_state_initial() {
        let mut sm = InMemoryStateMachine::new();
        let (last_applied, membership) = sm.applied_state().await.unwrap();

        assert_eq!(last_applied, None);
        // Membership is an Option - check inner membership is empty
        assert!(membership.membership().nodes().next().is_none());
    }

    #[tokio::test]
    async fn test_inmemory_state_machine_get_snapshot_builder() {
        let mut sm = InMemoryStateMachine::new();
        let _builder = sm.get_snapshot_builder().await;
        // Builder should be a clone of self
    }

    #[tokio::test]
    async fn test_inmemory_state_machine_begin_receiving_snapshot() {
        let mut sm = InMemoryStateMachine::new();
        let cursor = sm.begin_receiving_snapshot().await.unwrap();
        // Should return empty cursor initially
        assert_eq!(cursor.get_ref().len(), 0);
    }

    #[tokio::test]
    async fn test_inmemory_state_machine_get_current_snapshot_none() {
        let mut sm = InMemoryStateMachine::new();
        let snapshot = sm.get_current_snapshot().await.unwrap();
        assert!(snapshot.is_none());
    }

    // =========================================================================
    // StoredSnapshot Tests
    // =========================================================================

    #[test]
    fn test_stored_snapshot_serde() {
        use openraft::Membership;
        use openraft::SnapshotMeta;
        use openraft::StoredMembership;

        let membership = Membership::<AppTypeConfig>::new_with_defaults(vec![], []);
        let meta = SnapshotMeta {
            last_log_id: None,
            last_membership: StoredMembership::new(None, membership),
            snapshot_id: "test-snap".to_string(),
        };
        let snapshot = StoredSnapshot {
            meta,
            data: vec![1, 2, 3, 4],
            integrity: None,
        };

        let serialized = bincode::serialize(&snapshot).expect("serialize");
        let deserialized: StoredSnapshot = bincode::deserialize(&serialized).expect("deserialize");

        assert_eq!(deserialized.data, vec![1, 2, 3, 4]);
        assert_eq!(deserialized.meta.snapshot_id, "test-snap");
    }

    // =========================================================================
    // StateMachineData Tests
    // =========================================================================

    #[test]
    fn test_state_machine_data_default() {
        let data = StateMachineData::default();
        assert_eq!(data.last_applied_log, None);
        assert!(data.data.is_empty());
    }

    #[test]
    fn test_state_machine_data_clone() {
        let mut data = StateMachineData::default();
        data.data.insert("key".to_string(), "value".to_string());

        let cloned = data.clone();
        assert_eq!(cloned.data.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_state_machine_data_serde() {
        let mut data = StateMachineData::default();
        data.data.insert("test".to_string(), "data".to_string());

        let serialized = bincode::serialize(&data).expect("serialize");
        let deserialized: StateMachineData = bincode::deserialize(&serialized).expect("deserialize");

        assert_eq!(deserialized.data.get("test"), Some(&"data".to_string()));
    }

    // =========================================================================
    // ChainTipState Tests
    // =========================================================================

    #[test]
    fn test_chain_tip_state_default() {
        let tip = ChainTipState::default();
        assert_eq!(tip.index, 0);
        assert_eq!(tip.hash, GENESIS_HASH);
    }

    // =========================================================================
    // RedbLogStore Log Persistence Tests (CRITICAL)
    // =========================================================================

    /// Type alias for Raft log entries.
    type Entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry;

    /// Helper to create test entries for log storage tests.
    fn create_test_entry(term: u64, node_id: u64, index: u64, key: &str, value: &str) -> Entry {
        use openraft::entry::RaftEntry;
        use openraft::testing::log_id;

        let log_id = log_id::<AppTypeConfig>(term, NodeId::from(node_id), index);
        Entry::new_normal(log_id, AppRequest::Set {
            key: key.to_string(),
            value: value.to_string(),
        })
    }

    /// Helper to create multiple test entries in sequence.
    fn create_test_entries(count: u64, term: u64, node_id: u64, start_index: u64) -> Vec<Entry> {
        (0..count)
            .map(|i| {
                let index = start_index + i;
                create_test_entry(term, node_id, index, &format!("key{}", index), &format!("value{}", index))
            })
            .collect()
    }

    #[tokio::test]
    async fn test_redb_log_append_persists_across_restart() {
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("append-persist.redb");

        // Phase 1: Create store, append entries, close
        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let entries = create_test_entries(5, 1, 1, 1);
            store.append(entries, IOFlushed::noop()).await.unwrap();
        }
        // Store dropped, database closed

        // Phase 2: Reopen and verify entries
        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let state = store.get_log_state().await.unwrap();
            assert_eq!(state.last_log_id.map(|id| id.index()), Some(5));

            let entries = store.try_get_log_entries(1..=5).await.unwrap();
            assert_eq!(entries.len(), 5);
        }
    }

    #[tokio::test]
    async fn test_redb_log_append_single_entry_persists() {
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("single-entry.redb");

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let entry = create_test_entry(1, 1, 1, "single_key", "single_value");
            store.append(vec![entry], IOFlushed::noop()).await.unwrap();
        }

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let entries = store.try_get_log_entries(1..=1).await.unwrap();
            assert_eq!(entries.len(), 1);
        }
    }

    #[tokio::test]
    async fn test_redb_log_append_batch_persists() {
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("batch-persist.redb");

        // Test with a large batch (but under MAX_BATCH_SIZE)
        let batch_size = 100;

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let entries = create_test_entries(batch_size, 1, 1, 1);
            store.append(entries, IOFlushed::noop()).await.unwrap();
        }

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let state = store.get_log_state().await.unwrap();
            assert_eq!(state.last_log_id.map(|id| id.index()), Some(batch_size));

            let entries = store.try_get_log_entries(1..=batch_size).await.unwrap();
            assert_eq!(entries.len(), batch_size as usize);
        }
    }

    #[tokio::test]
    async fn test_redb_log_get_entries_after_restart() {
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("get-entries-restart.redb");

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let entries = create_test_entries(10, 1, 1, 1);
            store.append(entries, IOFlushed::noop()).await.unwrap();
        }

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            // Test range query after restart
            let entries = store.try_get_log_entries(3..=7).await.unwrap();
            assert_eq!(entries.len(), 5);

            // Verify indices match
            for (i, entry) in entries.iter().enumerate() {
                assert_eq!(entry.log_id().index(), 3 + i as u64);
            }
        }
    }

    #[tokio::test]
    async fn test_redb_log_get_log_state_after_restart() {
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("state-restart.redb");

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let entries = create_test_entries(20, 2, 1, 1);
            store.append(entries, IOFlushed::noop()).await.unwrap();
        }

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let state = store.get_log_state().await.unwrap();
            assert_eq!(state.last_log_id.map(|id| id.index()), Some(20));
            assert_eq!(state.last_log_id.map(|id| id.leader_id.term), Some(2));
        }
    }

    #[tokio::test]
    async fn test_redb_log_multiple_restart_cycles() {
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("multi-restart.redb");

        // Cycle 1: Add entries 1-5
        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let entries = create_test_entries(5, 1, 1, 1);
            store.append(entries, IOFlushed::noop()).await.unwrap();
        }

        // Cycle 2: Add entries 6-10
        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let entries = create_test_entries(5, 1, 1, 6);
            store.append(entries, IOFlushed::noop()).await.unwrap();
        }

        // Cycle 3: Verify all entries present
        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let entries = store.try_get_log_entries(1..=10).await.unwrap();
            assert_eq!(entries.len(), 10);
        }
    }

    #[tokio::test]
    async fn test_redb_log_empty_restart() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("empty-restart.redb");

        {
            let _store = RedbLogStore::new(&db_path).unwrap();
            // Don't add any entries
        }

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let state = store.get_log_state().await.unwrap();
            assert_eq!(state.last_log_id, None);
            assert_eq!(state.last_purged_log_id, None);
        }
    }

    #[tokio::test]
    async fn test_redb_log_preserves_entry_payload() {
        use openraft::EntryPayload;
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("payload-persist.redb");

        let test_key = "my_unique_key";
        let test_value = "my_unique_value";

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let entry = create_test_entry(1, 1, 1, test_key, test_value);
            store.append(vec![entry], IOFlushed::noop()).await.unwrap();
        }

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let entries = store.try_get_log_entries(1..=1).await.unwrap();
            assert_eq!(entries.len(), 1);

            // Verify payload content
            match &entries[0].payload {
                EntryPayload::Normal(app_request) => match app_request {
                    AppRequest::Set { key, value } => {
                        assert_eq!(key, test_key);
                        assert_eq!(value, test_value);
                    }
                    _ => panic!("Expected Set request"),
                },
                _ => panic!("Expected Normal entry with payload"),
            }
        }
    }

    #[tokio::test]
    async fn test_redb_log_preserves_entry_log_id() {
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("log-id-persist.redb");

        let term = 5;
        let node_id = 42;
        let index = 100;

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let entry = create_test_entry(term, node_id, index, "key", "value");
            store.append(vec![entry], IOFlushed::noop()).await.unwrap();
        }

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let entries = store.try_get_log_entries(index..=index).await.unwrap();
            assert_eq!(entries.len(), 1);

            let log_id = entries[0].log_id();
            assert_eq!(log_id.index(), index);
            assert_eq!(log_id.leader_id.term, term);
            assert_eq!(log_id.leader_id.node_id.0, node_id);
        }
    }

    #[tokio::test]
    async fn test_redb_log_preserves_order_after_restart() {
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("order-persist.redb");

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let entries = create_test_entries(50, 1, 1, 1);
            store.append(entries, IOFlushed::noop()).await.unwrap();
        }

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let entries = store.try_get_log_entries(1..=50).await.unwrap();
            assert_eq!(entries.len(), 50);

            // Verify strict ordering
            for (i, entry) in entries.iter().enumerate() {
                assert_eq!(
                    entry.log_id().index(),
                    (i + 1) as u64,
                    "Entry at position {} should have index {}",
                    i,
                    i + 1
                );
            }
        }
    }

    // =========================================================================
    // RedbLogStore Vote Persistence Tests (CRITICAL)
    // =========================================================================

    #[tokio::test]
    async fn test_redb_vote_overwrites_previous() {
        use openraft::storage::RaftLogStorage;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("vote-overwrite.redb");

        let mut store = RedbLogStore::new(&db_path).unwrap();

        let vote1 = Vote::new(1, NodeId::new(1));
        store.save_vote(&vote1).await.unwrap();
        assert_eq!(store.read_vote().await.unwrap(), Some(vote1));

        let vote2 = Vote::new(2, NodeId::new(2));
        store.save_vote(&vote2).await.unwrap();
        assert_eq!(store.read_vote().await.unwrap(), Some(vote2));
    }

    #[tokio::test]
    async fn test_redb_vote_multiple_updates_persist() {
        use openraft::storage::RaftLogStorage;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("vote-multi-update.redb");

        // Update vote multiple times
        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            for term in 1..=10 {
                let vote = Vote::new(term, NodeId::new(1));
                store.save_vote(&vote).await.unwrap();
            }
        }

        // Only final vote should persist
        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let vote = store.read_vote().await.unwrap();
            assert_eq!(vote, Some(Vote::new(10, NodeId::new(1))));
        }
    }

    #[tokio::test]
    async fn test_redb_vote_with_different_terms() {
        use openraft::storage::RaftLogStorage;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("vote-terms.redb");

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let vote = Vote::new(42, NodeId::new(1));
            store.save_vote(&vote).await.unwrap();
        }

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let vote = store.read_vote().await.unwrap().unwrap();
            assert_eq!(vote.leader_id().term, 42);
        }
    }

    #[tokio::test]
    async fn test_redb_vote_with_different_node_ids() {
        use openraft::storage::RaftLogStorage;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("vote-nodes.redb");

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let vote = Vote::new(1, NodeId::new(999));
            store.save_vote(&vote).await.unwrap();
        }

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let vote = store.read_vote().await.unwrap().unwrap();
            assert_eq!(vote.leader_id().node_id.0, 999);
        }
    }

    #[tokio::test]
    async fn test_redb_vote_none_after_fresh_start() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("vote-fresh.redb");

        let mut store = RedbLogStore::new(&db_path).unwrap();
        let vote = store.read_vote().await.unwrap();
        assert_eq!(vote, None);
    }

    #[tokio::test]
    async fn test_redb_vote_committed_after_vote() {
        use openraft::storage::RaftLogStorage;
        use openraft::testing::log_id;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("vote-committed.redb");

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let vote = Vote::new(3, NodeId::new(1));
            store.save_vote(&vote).await.unwrap();

            let committed = log_id::<AppTypeConfig>(3, NodeId::new(1), 10);
            store.save_committed(Some(committed)).await.unwrap();
        }

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let vote = store.read_vote().await.unwrap();
            assert!(vote.is_some());

            let committed = store.read_committed().await.unwrap();
            assert!(committed.is_some());
            assert_eq!(committed.unwrap().index(), 10);
        }
    }

    // =========================================================================
    // RedbLogStore Committed Index Persistence Tests (CRITICAL)
    // =========================================================================

    #[tokio::test]
    async fn test_redb_committed_persists_across_restart() {
        use openraft::storage::RaftLogStorage;
        use openraft::testing::log_id;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("committed-restart.redb");

        let committed = log_id::<AppTypeConfig>(1, NodeId::new(1), 50);

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            store.save_committed(Some(committed)).await.unwrap();
        }

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let recovered = store.read_committed().await.unwrap();
            assert_eq!(recovered, Some(committed));
        }
    }

    #[tokio::test]
    async fn test_redb_committed_clear_persists() {
        use openraft::storage::RaftLogStorage;
        use openraft::testing::log_id;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("committed-clear-restart.redb");

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let committed = log_id::<AppTypeConfig>(1, NodeId::new(1), 10);
            store.save_committed(Some(committed)).await.unwrap();
            store.save_committed(None).await.unwrap();
        }

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let recovered = store.read_committed().await.unwrap();
            assert_eq!(recovered, None);
        }
    }

    #[tokio::test]
    async fn test_redb_committed_advances_monotonically() {
        use openraft::storage::RaftLogStorage;
        use openraft::testing::log_id;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("committed-mono.redb");

        let mut store = RedbLogStore::new(&db_path).unwrap();

        // Advance committed through multiple indices
        for index in [10, 20, 30, 40, 50] {
            let committed = log_id::<AppTypeConfig>(1, NodeId::new(1), index);
            store.save_committed(Some(committed)).await.unwrap();

            let current = store.read_committed().await.unwrap().unwrap();
            assert_eq!(current.index(), index);
        }
    }

    #[tokio::test]
    async fn test_redb_committed_with_log_entries() {
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;
        use openraft::testing::log_id;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("committed-with-log.redb");

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();

            // Append entries
            let entries = create_test_entries(20, 1, 1, 1);
            store.append(entries, IOFlushed::noop()).await.unwrap();

            // Set committed index
            let committed = log_id::<AppTypeConfig>(1, NodeId::new(1), 15);
            store.save_committed(Some(committed)).await.unwrap();
        }

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();

            // Verify both log and committed persist
            let state = store.get_log_state().await.unwrap();
            assert_eq!(state.last_log_id.map(|id| id.index()), Some(20));

            let committed = store.read_committed().await.unwrap();
            assert_eq!(committed.map(|id| id.index()), Some(15));
        }
    }

    #[tokio::test]
    async fn test_redb_committed_none_on_fresh_start() {
        use openraft::storage::RaftLogStorage;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("committed-fresh.redb");

        let mut store = RedbLogStore::new(&db_path).unwrap();
        let committed = store.read_committed().await.unwrap();
        assert_eq!(committed, None);
    }

    // =========================================================================
    // RedbLogStore Chain Integrity Tests (CRITICAL)
    // =========================================================================

    #[tokio::test]
    async fn test_redb_chain_hash_computed_on_append() {
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("chain-hash-compute.redb");

        let mut store = RedbLogStore::new(&db_path).unwrap();
        let entries = create_test_entries(1, 1, 1, 1);
        store.append(entries, IOFlushed::noop()).await.unwrap();

        // Chain tip should be updated from genesis
        let (index, hash) = store.chain_tip_for_verification().unwrap();
        assert_eq!(index, 1);
        assert_ne!(hash, GENESIS_HASH); // Hash should change from genesis
    }

    #[tokio::test]
    async fn test_redb_chain_hash_linked_to_previous() {
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("chain-hash-linked.redb");

        let mut store = RedbLogStore::new(&db_path).unwrap();

        // Add first entry
        let entries1 = create_test_entries(1, 1, 1, 1);
        store.append(entries1, IOFlushed::noop()).await.unwrap();
        let (index1, hash1) = store.chain_tip_for_verification().unwrap();

        // Add second entry
        let entries2 = create_test_entries(1, 1, 1, 2);
        store.append(entries2, IOFlushed::noop()).await.unwrap();
        let (index2, hash2) = store.chain_tip_for_verification().unwrap();

        assert_eq!(index1, 1);
        assert_eq!(index2, 2);
        assert_ne!(hash1, hash2); // Each entry has unique hash based on chain
    }

    #[tokio::test]
    async fn test_redb_chain_hash_batch_append_linked() {
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("chain-hash-batch.redb");

        let mut store = RedbLogStore::new(&db_path).unwrap();

        // Batch append 10 entries
        let entries = create_test_entries(10, 1, 1, 1);
        store.append(entries, IOFlushed::noop()).await.unwrap();

        // Chain tip should point to last entry
        let (index, _hash) = store.chain_tip_for_verification().unwrap();
        assert_eq!(index, 10);
    }

    #[tokio::test]
    async fn test_redb_chain_tip_updated_after_append() {
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("chain-tip-update.redb");

        let mut store = RedbLogStore::new(&db_path).unwrap();

        // Initial chain tip is genesis
        let (index0, hash0) = store.chain_tip_for_verification().unwrap();
        assert_eq!(index0, 0);
        assert_eq!(hash0, GENESIS_HASH);

        // After append, chain tip updated
        let entries = create_test_entries(5, 1, 1, 1);
        store.append(entries, IOFlushed::noop()).await.unwrap();

        let (index1, _hash1) = store.chain_tip_for_verification().unwrap();
        assert_eq!(index1, 5);
    }

    #[tokio::test]
    async fn test_redb_chain_hash_persists_across_restart() {
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("chain-hash-persist.redb");

        let original_hash: ChainHash;

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let entries = create_test_entries(5, 1, 1, 1);
            store.append(entries, IOFlushed::noop()).await.unwrap();
            (_, original_hash) = store.chain_tip_for_verification().unwrap();
        }

        {
            let store = RedbLogStore::new(&db_path).unwrap();
            let (index, hash) = store.chain_tip_for_verification().unwrap();
            assert_eq!(index, 5);
            assert_eq!(hash, original_hash);
        }
    }

    #[tokio::test]
    async fn test_redb_verify_chain_batch_succeeds() {
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("chain-verify-success.redb");

        let mut store = RedbLogStore::new(&db_path).unwrap();
        let entries = create_test_entries(20, 1, 1, 1);
        store.append(entries, IOFlushed::noop()).await.unwrap();

        // Verify batch should succeed
        let verified = store.verify_chain_batch(1, 100).unwrap();
        assert_eq!(verified, 20);
    }

    #[tokio::test]
    async fn test_redb_verify_chain_batch_bounds() {
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("chain-verify-bounds.redb");

        let mut store = RedbLogStore::new(&db_path).unwrap();
        let entries = create_test_entries(50, 1, 1, 1);
        store.append(entries, IOFlushed::noop()).await.unwrap();

        // Verify only 10 entries even though we requested 100
        let verified = store.verify_chain_batch(1, 10).unwrap();
        assert_eq!(verified, 10);
    }

    #[tokio::test]
    async fn test_redb_verification_range_with_entries() {
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("chain-verify-range.redb");

        let mut store = RedbLogStore::new(&db_path).unwrap();
        let entries = create_test_entries(100, 1, 1, 1);
        store.append(entries, IOFlushed::noop()).await.unwrap();

        let range = store.verification_range().unwrap();
        assert_eq!(range, Some((1, 100)));
    }

    #[tokio::test]
    async fn test_redb_verify_chain_full_log() {
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("chain-verify-full.redb");

        let mut store = RedbLogStore::new(&db_path).unwrap();
        let entries = create_test_entries(100, 1, 1, 1);
        store.append(entries, IOFlushed::noop()).await.unwrap();

        // Verify entire chain in batches
        let verified1 = store.verify_chain_batch(1, 50).unwrap();
        let verified2 = store.verify_chain_batch(51, 50).unwrap();
        assert_eq!(verified1 + verified2, 100);
    }

    // =========================================================================
    // RedbLogStore Truncation Tests (HIGH)
    // =========================================================================

    #[tokio::test]
    async fn test_redb_truncate_from_middle() {
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;
        use openraft::testing::log_id;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("truncate-middle.redb");

        let mut store = RedbLogStore::new(&db_path).unwrap();
        let entries = create_test_entries(20, 1, 1, 1);
        store.append(entries, IOFlushed::noop()).await.unwrap();

        // Truncate from index 10
        let truncate_id = log_id::<AppTypeConfig>(1, NodeId::new(1), 10);
        store.truncate(truncate_id).await.unwrap();

        // Entries 1-9 should remain
        let state = store.get_log_state().await.unwrap();
        assert_eq!(state.last_log_id.map(|id| id.index()), Some(9));

        let entries = store.try_get_log_entries(1..=9).await.unwrap();
        assert_eq!(entries.len(), 9);
    }

    #[tokio::test]
    async fn test_redb_truncate_all() {
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;
        use openraft::testing::log_id;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("truncate-all.redb");

        let mut store = RedbLogStore::new(&db_path).unwrap();
        let entries = create_test_entries(10, 1, 1, 1);
        store.append(entries, IOFlushed::noop()).await.unwrap();

        // Truncate from index 1 (remove all)
        let truncate_id = log_id::<AppTypeConfig>(1, NodeId::new(1), 1);
        store.truncate(truncate_id).await.unwrap();

        let state = store.get_log_state().await.unwrap();
        assert_eq!(state.last_log_id, None);
    }

    #[tokio::test]
    async fn test_redb_truncate_updates_chain_tip() {
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;
        use openraft::testing::log_id;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("truncate-chain-tip.redb");

        let mut store = RedbLogStore::new(&db_path).unwrap();
        let entries = create_test_entries(10, 1, 1, 1);
        store.append(entries, IOFlushed::noop()).await.unwrap();

        // Get chain tip before truncate
        let (_index_before, _hash_before) = store.chain_tip_for_verification().unwrap();

        // Truncate from index 6
        let truncate_id = log_id::<AppTypeConfig>(1, NodeId::new(1), 6);
        store.truncate(truncate_id).await.unwrap();

        // Chain tip should be updated to entry 5
        let (index_after, _hash_after) = store.chain_tip_for_verification().unwrap();
        assert_eq!(index_after, 5);
    }

    #[tokio::test]
    async fn test_redb_truncate_preserves_earlier_entries() {
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;
        use openraft::testing::log_id;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("truncate-preserve.redb");

        let mut store = RedbLogStore::new(&db_path).unwrap();
        let entries = create_test_entries(20, 1, 1, 1);
        store.append(entries, IOFlushed::noop()).await.unwrap();

        // Truncate from index 15
        let truncate_id = log_id::<AppTypeConfig>(1, NodeId::new(1), 15);
        store.truncate(truncate_id).await.unwrap();

        // Entries 1-14 should be intact
        let entries = store.try_get_log_entries(1..=14).await.unwrap();
        assert_eq!(entries.len(), 14);

        // Verify indices
        for (i, entry) in entries.iter().enumerate() {
            assert_eq!(entry.log_id().index(), (i + 1) as u64);
        }
    }

    #[tokio::test]
    async fn test_redb_truncate_persists_across_restart() {
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;
        use openraft::testing::log_id;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("truncate-persist.redb");

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let entries = create_test_entries(20, 1, 1, 1);
            store.append(entries, IOFlushed::noop()).await.unwrap();

            let truncate_id = log_id::<AppTypeConfig>(1, NodeId::new(1), 10);
            store.truncate(truncate_id).await.unwrap();
        }

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let state = store.get_log_state().await.unwrap();
            assert_eq!(state.last_log_id.map(|id| id.index()), Some(9));
        }
    }

    // =========================================================================
    // RedbLogStore Purge Tests (HIGH)
    // =========================================================================

    #[tokio::test]
    async fn test_redb_purge_up_to_index() {
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;
        use openraft::testing::log_id;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("purge-index.redb");

        let mut store = RedbLogStore::new(&db_path).unwrap();
        let entries = create_test_entries(20, 1, 1, 1);
        store.append(entries, IOFlushed::noop()).await.unwrap();

        // Purge up to index 10
        let purge_id = log_id::<AppTypeConfig>(1, NodeId::new(1), 10);
        store.purge(purge_id).await.unwrap();

        let state = store.get_log_state().await.unwrap();
        assert_eq!(state.last_purged_log_id.map(|id| id.index()), Some(10));

        // Entries 11-20 should remain
        let entries = store.try_get_log_entries(11..=20).await.unwrap();
        assert_eq!(entries.len(), 10);
    }

    #[tokio::test]
    async fn test_redb_purge_preserves_later_entries() {
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;
        use openraft::testing::log_id;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("purge-preserve.redb");

        let mut store = RedbLogStore::new(&db_path).unwrap();
        let entries = create_test_entries(30, 1, 1, 1);
        store.append(entries, IOFlushed::noop()).await.unwrap();

        // Purge first 15 entries
        let purge_id = log_id::<AppTypeConfig>(1, NodeId::new(1), 15);
        store.purge(purge_id).await.unwrap();

        // Last log should still be 30
        let state = store.get_log_state().await.unwrap();
        assert_eq!(state.last_log_id.map(|id| id.index()), Some(30));

        // Entries 16-30 accessible
        let entries = store.try_get_log_entries(16..=30).await.unwrap();
        assert_eq!(entries.len(), 15);
    }

    #[tokio::test]
    async fn test_redb_purge_updates_last_purged_log_id() {
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;
        use openraft::testing::log_id;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("purge-last-purged.redb");

        let mut store = RedbLogStore::new(&db_path).unwrap();
        let entries = create_test_entries(20, 1, 1, 1);
        store.append(entries, IOFlushed::noop()).await.unwrap();

        // Purge to 5
        let purge_id = log_id::<AppTypeConfig>(1, NodeId::new(1), 5);
        store.purge(purge_id).await.unwrap();

        let state = store.get_log_state().await.unwrap();
        assert_eq!(state.last_purged_log_id.map(|id| id.index()), Some(5));

        // Purge to 10
        let purge_id = log_id::<AppTypeConfig>(1, NodeId::new(1), 10);
        store.purge(purge_id).await.unwrap();

        let state = store.get_log_state().await.unwrap();
        assert_eq!(state.last_purged_log_id.map(|id| id.index()), Some(10));
    }

    #[tokio::test]
    async fn test_redb_purge_persists_across_restart() {
        use openraft::storage::IOFlushed;
        use openraft::storage::RaftLogStorage;
        use openraft::testing::log_id;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("purge-persist.redb");

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let entries = create_test_entries(20, 1, 1, 1);
            store.append(entries, IOFlushed::noop()).await.unwrap();

            let purge_id = log_id::<AppTypeConfig>(1, NodeId::new(1), 10);
            store.purge(purge_id).await.unwrap();
        }

        {
            let mut store = RedbLogStore::new(&db_path).unwrap();
            let state = store.get_log_state().await.unwrap();
            assert_eq!(state.last_purged_log_id.map(|id| id.index()), Some(10));
        }
    }
}
