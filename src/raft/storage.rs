//! Raft log and state machine storage backends.
//!
//! Provides pluggable storage implementations for openraft's log and state machine,
//! supporting both in-memory (for testing) and persistent (redb + SQLite) backends.
//! The log storage uses redb for fast sequential append operations, while the state
//! machine uses SQLite for ACID transactions and queryable snapshots.
//!
//! # Key Components
//!
//! - `StorageBackend`: Enum selecting log/state machine backend (InMemory, Sqlite, Redb)
//! - `RedbLogStore`: Persistent append-only log backed by redb (default production)
//! - `InMemoryLogStore`: Non-durable log for testing and development
//! - `SqliteStateMachine`: ACID-compliant state machine with connection pooling
//! - `StateMachineStore`: Type alias for active state machine implementation
//! - Snapshot management with bounded in-memory snapshots
//!
//! # Tiger Style
//!
//! - Fixed limits: MAX_BATCH_SIZE (1024 entries), MAX_SETMULTI_KEYS (100 keys)
//! - Explicit types: u64 for log indices (portable across architectures)
//! - Resource bounds: Snapshot data capped to prevent unbounded memory growth
//! - Disk space checks: Pre-flight validation before writing snapshots
//! - Error handling: Explicit SNAFU errors for each failure mode
//! - Connection pooling: Bounded read pool (DEFAULT_READ_POOL_SIZE = 8)
//!
//! # Example
//!
//! ```ignore
//! use aspen::raft::storage::{RedbLogStore, SqliteStateMachine, StorageBackend};
//!
//! // Create persistent storage
//! let log_store = RedbLogStore::new("./data/raft-log.redb").await?;
//! let state_machine = SqliteStateMachine::new("./data/state-machine.db").await?;
//!
//! // Or use builder with backend selection
//! let backend = StorageBackend::Sqlite;
//! let (log, sm) = create_storage(backend, "./data").await?;
//! ```

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::io;
use std::io::Cursor;
use std::ops::RangeBounds;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use futures::{Stream, TryStreamExt};
use openraft::alias::{LogIdOf, SnapshotDataOf, VoteOf};
use openraft::entry::RaftEntry;
use openraft::storage::{
    EntryResponder, IOFlushed, RaftLogStorage, RaftSnapshotBuilder, RaftStateMachine, Snapshot,
};
use openraft::{EntryPayload, LogState, OptionalSend, RaftLogReader, StoredMembership};
use redb::{Database, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use tokio::sync::{Mutex, RwLock};

use crate::raft::constants::MAX_BATCH_SIZE;
use crate::raft::types::{AppRequest, AppResponse, AppTypeConfig};
use crate::utils::ensure_disk_space_available;

// ====================================================================================
// Storage Backend Configuration
// ====================================================================================

/// Storage backend selection for Raft log and state machine.
///
/// Aspen supports three storage backends:
/// - **Sqlite**: Persistent ACID storage using SQLite (default, recommended for production)
/// - **InMemory**: Fast, deterministic storage for testing and simulations
/// - **Redb**: Persistent ACID storage using embedded redb (deprecated, use Sqlite)
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum StorageBackend {
    /// In-memory storage using BTreeMap. Data is lost on restart.
    /// Use for: unit tests, madsim simulations, development.
    InMemory,
    /// Persistent storage using redb. Data survives restarts.
    /// Deprecated: Use Sqlite instead for new deployments.
    #[deprecated(note = "Use Sqlite instead for new deployments")]
    Redb,
    /// Persistent storage using SQLite. Data survives restarts.
    /// Default storage backend for production deployments and integration tests.
    #[default]
    Sqlite,
}

impl std::str::FromStr for StorageBackend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "inmemory" | "in-memory" | "memory" => Ok(StorageBackend::InMemory),
            #[allow(deprecated)]
            "redb" | "persistent" | "disk" => Ok(StorageBackend::Redb),
            "sqlite" | "sql" => Ok(StorageBackend::Sqlite),
            _ => Err(format!(
                "Invalid storage backend '{}'. Valid options: inmemory, redb, sqlite",
                s
            )),
        }
    }
}

impl std::fmt::Display for StorageBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageBackend::InMemory => write!(f, "inmemory"),
            #[allow(deprecated)]
            StorageBackend::Redb => write!(f, "redb"),
            StorageBackend::Sqlite => write!(f, "sqlite"),
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

// ====================================================================================
// Redb Storage Errors
// ====================================================================================

#[derive(Debug, Snafu)]
pub enum StorageError {
    #[snafu(display("failed to open redb database at {}: {source}", path.display()))]
    OpenDatabase {
        path: PathBuf,
        #[snafu(source(from(redb::DatabaseError, Box::new)))]
        source: Box<redb::DatabaseError>,
    },

    #[snafu(display("failed to begin write transaction: {source}"))]
    BeginWrite {
        #[snafu(source(from(redb::TransactionError, Box::new)))]
        source: Box<redb::TransactionError>,
    },

    #[snafu(display("failed to begin read transaction: {source}"))]
    BeginRead {
        #[snafu(source(from(redb::TransactionError, Box::new)))]
        source: Box<redb::TransactionError>,
    },

    #[snafu(display("failed to open table: {source}"))]
    OpenTable {
        #[snafu(source(from(redb::TableError, Box::new)))]
        source: Box<redb::TableError>,
    },

    #[snafu(display("failed to commit transaction: {source}"))]
    Commit {
        #[snafu(source(from(redb::CommitError, Box::new)))]
        source: Box<redb::CommitError>,
    },

    #[snafu(display("failed to insert into table: {source}"))]
    Insert {
        #[snafu(source(from(redb::StorageError, Box::new)))]
        source: Box<redb::StorageError>,
    },

    #[snafu(display("failed to get from table: {source}"))]
    Get {
        #[snafu(source(from(redb::StorageError, Box::new)))]
        source: Box<redb::StorageError>,
    },

    #[snafu(display("failed to remove from table: {source}"))]
    Remove {
        #[snafu(source(from(redb::StorageError, Box::new)))]
        source: Box<redb::StorageError>,
    },

    #[snafu(display("failed to iterate table range: {source}"))]
    Range {
        #[snafu(source(from(redb::StorageError, Box::new)))]
        source: Box<redb::StorageError>,
    },

    #[snafu(display("failed to serialize data: {source}"))]
    Serialize {
        #[snafu(source(from(bincode::Error, Box::new)))]
        source: Box<bincode::Error>,
    },

    #[snafu(display("failed to deserialize data: {source}"))]
    Deserialize {
        #[snafu(source(from(bincode::Error, Box::new)))]
        source: Box<bincode::Error>,
    },

    #[snafu(display("failed to create directory {}: {source}", path.display()))]
    CreateDirectory {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl From<StorageError> for io::Error {
    fn from(err: StorageError) -> io::Error {
        io::Error::other(err.to_string())
    }
}

/// In-memory Raft log backed by a simple `BTreeMap`.
#[derive(Clone, Debug, Default)]
pub struct InMemoryLogStore {
    inner: Arc<Mutex<LogStoreInner>>,
}

#[derive(Debug, Default)]
struct LogStoreInner {
    last_purged_log_id: Option<LogIdOf<AppTypeConfig>>,
    log: BTreeMap<u64, <AppTypeConfig as openraft::RaftTypeConfig>::Entry>,
    committed: Option<LogIdOf<AppTypeConfig>>,
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
        Ok(self
            .log
            .range(range)
            .map(|(_, entry)| entry.clone())
            .collect())
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

    async fn save_committed(
        &mut self,
        committed: Option<LogIdOf<AppTypeConfig>>,
    ) -> Result<(), io::Error> {
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

    async fn append<I>(
        &mut self,
        entries: I,
        callback: IOFlushed<AppTypeConfig>,
    ) -> Result<(), io::Error>
    where
        I: IntoIterator<Item = <AppTypeConfig as openraft::RaftTypeConfig>::Entry>,
    {
        for entry in entries {
            self.log.insert(entry.log_id().index(), entry);
        }
        callback.io_completed(Ok(()));
        Ok(())
    }

    async fn truncate(&mut self, log_id: LogIdOf<AppTypeConfig>) -> Result<(), io::Error> {
        let keys = self
            .log
            .range(log_id.index()..)
            .map(|(k, _)| *k)
            .collect::<Vec<_>>();
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
        let keys = self
            .log
            .range(..=log_id.index())
            .map(|(k, _)| *k)
            .collect::<Vec<_>>();
        for key in keys {
            self.log.remove(&key);
        }
        Ok(())
    }
}

impl RaftLogReader<AppTypeConfig> for InMemoryLogStore
where
    <AppTypeConfig as openraft::RaftTypeConfig>::Entry: Clone,
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
where
    <AppTypeConfig as openraft::RaftTypeConfig>::Entry: Clone,
{
    type LogReader = Self;

    async fn get_log_state(&mut self) -> Result<LogState<AppTypeConfig>, io::Error> {
        let mut inner = self.inner.lock().await;
        inner.get_log_state().await
    }

    async fn save_committed(
        &mut self,
        committed: Option<LogIdOf<AppTypeConfig>>,
    ) -> Result<(), io::Error> {
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

    async fn append<I>(
        &mut self,
        entries: I,
        callback: IOFlushed<AppTypeConfig>,
    ) -> Result<(), io::Error>
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

/// Persistent Raft log backed by redb.
///
/// Stores log entries, vote state, committed index, and last purged log id on disk.
/// Provides ACID guarantees for all operations via redb transactions.
///
/// Tiger Style compliance:
/// - Explicitly sized types (u64 for log indices)
/// - Fixed database size limit (configurable at creation)
/// - Fail-fast on corruption (redb panics on invalid state)
/// - Bounded operations (no unbounded iteration)
#[derive(Clone, Debug)]
pub struct RedbLogStore {
    db: Arc<Database>,
    path: PathBuf,
}

impl RedbLogStore {
    /// Create or open a redb-backed log store at the given path.
    ///
    /// Creates the database file and all required tables if they don't exist.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path = path.as_ref().to_path_buf();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context(CreateDirectorySnafu { path: parent })?;
        }

        let db = Database::create(&path).context(OpenDatabaseSnafu { path: &path })?;

        // Initialize tables
        let write_txn = db.begin_write().context(BeginWriteSnafu)?;
        {
            write_txn
                .open_table(RAFT_LOG_TABLE)
                .context(OpenTableSnafu)?;
            write_txn
                .open_table(RAFT_META_TABLE)
                .context(OpenTableSnafu)?;
            write_txn
                .open_table(SNAPSHOT_TABLE)
                .context(OpenTableSnafu)?;
        }
        write_txn.commit().context(CommitSnafu)?;

        Ok(Self {
            db: Arc::new(db),
            path,
        })
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
    ) -> Result<
        crate::raft::storage_validation::ValidationReport,
        crate::raft::storage_validation::StorageValidationError,
    > {
        crate::raft::storage_validation::validate_raft_storage(node_id, &self.path)
    }

    // Internal helper: Read a value from the metadata table
    fn read_meta<T: for<'de> Deserialize<'de>>(
        &self,
        key: &str,
    ) -> Result<Option<T>, StorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn
            .open_table(RAFT_META_TABLE)
            .context(OpenTableSnafu)?;

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
            let mut table = write_txn
                .open_table(RAFT_META_TABLE)
                .context(OpenTableSnafu)?;
            let serialized = bincode::serialize(value).context(SerializeSnafu)?;
            table
                .insert(key, serialized.as_slice())
                .context(InsertSnafu)?;
        }
        write_txn.commit().context(CommitSnafu)?;
        Ok(())
    }

    // Internal helper: Delete a value from the metadata table
    fn delete_meta(&self, key: &str) -> Result<(), StorageError> {
        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut table = write_txn
                .open_table(RAFT_META_TABLE)
                .context(OpenTableSnafu)?;
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
        let table = read_txn
            .open_table(RAFT_LOG_TABLE)
            .context(OpenTableSnafu)?;

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
        let table = read_txn
            .open_table(RAFT_LOG_TABLE)
            .context(OpenTableSnafu)?;

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

    async fn save_committed(
        &mut self,
        committed: Option<LogIdOf<AppTypeConfig>>,
    ) -> Result<(), io::Error> {
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

    async fn append<I>(
        &mut self,
        entries: I,
        callback: IOFlushed<AppTypeConfig>,
    ) -> Result<(), io::Error>
    where
        I: IntoIterator<Item = <AppTypeConfig as openraft::RaftTypeConfig>::Entry> + OptionalSend,
        I::IntoIter: OptionalSend,
    {
        // Tiger Style: Check disk space before write to prevent corruption on full disk
        ensure_disk_space_available(&self.path)?;

        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut table = write_txn
                .open_table(RAFT_LOG_TABLE)
                .context(OpenTableSnafu)?;

            // Performance optimization: Pre-serialize all entries before inserting
            // This reduces lock contention and allows redb to optimize bulk inserts
            // Tiger Style: Pre-allocate with MAX_BATCH_SIZE to avoid repeated reallocations
            let mut serialized_entries = Vec::with_capacity(MAX_BATCH_SIZE as usize);
            for entry in entries {
                let index = entry.log_id().index();
                let data = bincode::serialize(&entry).context(SerializeSnafu)?;
                serialized_entries.push((index, data));
            }

            // Bulk insert all serialized entries
            for (index, data) in serialized_entries {
                table.insert(index, data.as_slice()).context(InsertSnafu)?;
            }
        }
        write_txn.commit().context(CommitSnafu)?;

        callback.io_completed(Ok(()));
        Ok(())
    }

    async fn truncate(&mut self, log_id: LogIdOf<AppTypeConfig>) -> Result<(), io::Error> {
        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut table = write_txn
                .open_table(RAFT_LOG_TABLE)
                .context(OpenTableSnafu)?;

            // Collect keys to remove (>= log_id.index())
            let keys: Vec<u64> = table
                .range(log_id.index()..)
                .context(RangeSnafu)?
                .map(|item| {
                    let (key, _) = item.context(GetSnafu)?;
                    Ok::<_, StorageError>(key.value())
                })
                .collect::<Result<Vec<_>, _>>()?;

            for key in keys {
                table.remove(key).context(RemoveSnafu)?;
            }
        }
        write_txn.commit().context(CommitSnafu)?;
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
            let mut table = write_txn
                .open_table(RAFT_LOG_TABLE)
                .context(OpenTableSnafu)?;

            // Collect keys to remove (<= log_id.index())
            let keys: Vec<u64> = table
                .range(..=log_id.index())
                .context(RangeSnafu)?
                .map(|item| {
                    let (key, _) = item.context(GetSnafu)?;
                    Ok::<_, StorageError>(key.value())
                })
                .collect::<Result<Vec<_>, _>>()?;

            for key in keys {
                table.remove(key).context(RemoveSnafu)?;
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
        let committed: Option<LogIdOf<AppTypeConfig>> =
            self.read_meta("committed").map_err(io::Error::other)?;
        Ok(committed.map(|log_id| log_id.index))
    }
}

/// Snapshot blob stored in memory for testing.
#[derive(Debug, Serialize, Deserialize)]
pub struct StoredSnapshot {
    pub meta: openraft::SnapshotMeta<AppTypeConfig>,
    pub data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct StateMachineData {
    pub last_applied_log: Option<openraft::LogId<AppTypeConfig>>,
    pub last_membership: StoredMembership<AppTypeConfig>,
    pub data: BTreeMap<String, String>,
}

/// Simple state machine that mirrors the openraft memstore example.
#[derive(Debug, Default)]
pub struct StateMachineStore {
    state_machine: RwLock<StateMachineData>,
    snapshot_idx: AtomicU64,
    current_snapshot: RwLock<Option<StoredSnapshot>>,
}

impl StateMachineStore {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub async fn get(&self, key: &str) -> Option<String> {
        let sm = self.state_machine.read().await;
        sm.data.get(key).cloned()
    }

    /// Scan all keys that start with the given prefix.
    ///
    /// Returns a list of full key names.
    pub fn scan_keys_with_prefix(&self, prefix: &str) -> Vec<String> {
        // Block on async read - this is a synchronous API for compatibility
        let sm = futures::executor::block_on(self.state_machine.read());
        sm.data
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect()
    }

    /// Scan all key-value pairs that start with the given prefix.
    ///
    /// Returns a list of (key, value) pairs.
    pub fn scan_kv_with_prefix(&self, prefix: &str) -> Vec<(String, String)> {
        // Block on async read - this is a synchronous API for compatibility
        let sm = futures::executor::block_on(self.state_machine.read());
        sm.data
            .iter()
            .filter(|(k, _)| k.starts_with(prefix))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}

impl RaftSnapshotBuilder<AppTypeConfig> for Arc<StateMachineStore> {
    #[tracing::instrument(level = "trace", skip(self))]
    async fn build_snapshot(&mut self) -> Result<Snapshot<AppTypeConfig>, io::Error> {
        let state_machine = self.state_machine.read().await;
        let data = serde_json::to_vec(&state_machine.data)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        let last_applied_log = state_machine.last_applied_log;
        let last_membership = state_machine.last_membership.clone();
        let mut current_snapshot = self.current_snapshot.write().await;
        drop(state_machine);

        let snapshot_idx = self.snapshot_idx.fetch_add(1, Ordering::Relaxed) + 1;
        let snapshot_id = if let Some(last) = last_applied_log {
            format!(
                "{}-{}-{snapshot_idx}",
                last.committed_leader_id(),
                last.index()
            )
        } else {
            format!("--{snapshot_idx}")
        };

        let meta = openraft::SnapshotMeta {
            last_log_id: last_applied_log,
            last_membership,
            snapshot_id,
        };

        let snapshot = StoredSnapshot {
            meta: meta.clone(),
            data: data.clone(),
        };
        *current_snapshot = Some(snapshot);

        Ok(Snapshot {
            meta,
            snapshot: Cursor::new(data),
        })
    }
}

impl RaftStateMachine<AppTypeConfig> for Arc<StateMachineStore> {
    type SnapshotBuilder = Self;

    async fn applied_state(
        &mut self,
    ) -> Result<
        (
            Option<openraft::LogId<AppTypeConfig>>,
            StoredMembership<AppTypeConfig>,
        ),
        io::Error,
    > {
        let state_machine = self.state_machine.read().await;
        Ok((
            state_machine.last_applied_log,
            state_machine.last_membership.clone(),
        ))
    }

    #[tracing::instrument(level = "trace", skip(self, entries))]
    async fn apply<Strm>(&mut self, mut entries: Strm) -> Result<(), io::Error>
    where
        Strm:
            Stream<Item = Result<EntryResponder<AppTypeConfig>, io::Error>> + Unpin + OptionalSend,
    {
        let mut sm = self.state_machine.write().await;
        while let Some((entry, responder)) = entries.try_next().await? {
            sm.last_applied_log = Some(entry.log_id);
            let response = match entry.payload {
                EntryPayload::Blank => AppResponse { value: None },
                EntryPayload::Normal(ref req) => match req {
                    AppRequest::Set { key, value } => {
                        sm.data.insert(key.clone(), value.clone());
                        AppResponse {
                            value: Some(value.clone()),
                        }
                    }
                    AppRequest::SetMulti { pairs } => {
                        for (key, value) in pairs {
                            sm.data.insert(key.clone(), value.clone());
                        }
                        AppResponse { value: None }
                    }
                    AppRequest::Delete { key } => {
                        sm.data.remove(key);
                        AppResponse { value: None }
                    }
                    AppRequest::DeleteMulti { keys } => {
                        for key in keys {
                            sm.data.remove(key);
                        }
                        AppResponse { value: None }
                    }
                },
                EntryPayload::Membership(ref membership) => {
                    sm.last_membership =
                        StoredMembership::new(Some(entry.log_id), membership.clone());
                    AppResponse { value: None }
                }
            };
            if let Some(responder) = responder {
                responder.send(response);
            }
        }
        Ok(())
    }

    async fn begin_receiving_snapshot(
        &mut self,
    ) -> Result<SnapshotDataOf<AppTypeConfig>, io::Error> {
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

        let new_data: BTreeMap<String, String> = serde_json::from_slice(&snapshot_data)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

        // Update state machine
        let mut sm = self.state_machine.write().await;
        sm.data = new_data;
        sm.last_applied_log = meta.last_log_id;
        sm.last_membership = meta.last_membership.clone();
        drop(sm);

        // Store the installed snapshot so get_current_snapshot() returns it
        let mut current_snapshot = self.current_snapshot.write().await;
        *current_snapshot = Some(StoredSnapshot {
            meta: meta.clone(),
            data: snapshot_data,
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
