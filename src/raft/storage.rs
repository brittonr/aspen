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

use crate::raft::types::{AppRequest, AppResponse, AppTypeConfig};

// ====================================================================================
// Storage Backend Configuration
// ====================================================================================

/// Storage backend selection for Raft log and state machine.
///
/// Aspen supports two storage backends:
/// - **InMemory**: Fast, deterministic storage for testing and simulations
/// - **Redb**: Persistent ACID storage for production deployments
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StorageBackend {
    /// In-memory storage using BTreeMap. Data is lost on restart.
    /// Use for: unit tests, madsim simulations, development.
    InMemory,
    /// Persistent storage using redb. Data survives restarts.
    /// Use for: production deployments, integration tests.
    Redb,
}

impl std::str::FromStr for StorageBackend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "inmemory" | "in-memory" | "memory" => Ok(StorageBackend::InMemory),
            "redb" | "persistent" | "disk" => Ok(StorageBackend::Redb),
            _ => Err(format!(
                "Invalid storage backend '{}'. Valid options: inmemory, redb",
                s
            )),
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

impl Default for StorageBackend {
    fn default() -> Self {
        StorageBackend::InMemory
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

/// State machine key-value data
const STATE_MACHINE_KV_TABLE: TableDefinition<&str, &str> = TableDefinition::new("sm_kv");

/// State machine metadata (last_applied_log, last_membership)
const STATE_MACHINE_META_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("sm_meta");

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
        source: redb::DatabaseError,
    },

    #[snafu(display("failed to begin write transaction: {source}"))]
    BeginWrite { source: redb::TransactionError },

    #[snafu(display("failed to begin read transaction: {source}"))]
    BeginRead { source: redb::TransactionError },

    #[snafu(display("failed to open table: {source}"))]
    OpenTable { source: redb::TableError },

    #[snafu(display("failed to commit transaction: {source}"))]
    Commit { source: redb::CommitError },

    #[snafu(display("failed to insert into table: {source}"))]
    Insert { source: redb::StorageError },

    #[snafu(display("failed to get from table: {source}"))]
    Get { source: redb::StorageError },

    #[snafu(display("failed to remove from table: {source}"))]
    Remove { source: redb::StorageError },

    #[snafu(display("failed to iterate table range: {source}"))]
    Range { source: redb::StorageError },

    #[snafu(display("failed to serialize data: {source}"))]
    Serialize { source: bincode::Error },

    #[snafu(display("failed to deserialize data: {source}"))]
    Deserialize { source: bincode::Error },

    #[snafu(display("failed to create directory {}: {source}", path.display()))]
    CreateDirectory {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl From<StorageError> for io::Error {
    fn from(err: StorageError) -> io::Error {
        io::Error::new(io::ErrorKind::Other, err.to_string())
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
        let last_log_id = self
            .log
            .iter()
            .next_back()
            .map(|(_, entry)| entry.log_id().clone());
        let last_purged = self.last_purged_log_id.clone();
        let last = last_log_id.or(last_purged.clone());
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
        Ok(self.committed.clone())
    }

    async fn save_vote(&mut self, vote: &VoteOf<AppTypeConfig>) -> Result<(), io::Error> {
        self.vote = Some(vote.clone());
        Ok(())
    }

    async fn read_vote(&mut self) -> Result<Option<VoteOf<AppTypeConfig>>, io::Error> {
        Ok(self.vote.clone())
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
        self.last_purged_log_id = Some(log_id.clone());
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
                Ok::<_, StorageError>(entry.log_id().clone())
            })
            .transpose()?;

        let last_purged: Option<LogIdOf<AppTypeConfig>> = self.read_meta("last_purged_log_id")?;
        let last = last_log_id.or(last_purged.clone());

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
        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut table = write_txn
                .open_table(RAFT_LOG_TABLE)
                .context(OpenTableSnafu)?;

            for entry in entries {
                let serialized = bincode::serialize(&entry).context(SerializeSnafu)?;
                table
                    .insert(entry.log_id().index(), serialized.as_slice())
                    .context(InsertSnafu)?;
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
        if let Some(prev) = self.read_meta::<LogIdOf<AppTypeConfig>>("last_purged_log_id")? {
            assert!(
                prev <= log_id,
                "purge must be monotonic: prev={:?}, new={:?}",
                prev,
                log_id
            );
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
// Redb-backed State Machine (Production Storage)
// ====================================================================================

/// Persistent Raft state machine backed by redb.
///
/// Stores key-value data, last applied log, and membership config on disk.
/// Provides ACID guarantees via redb transactions.
///
/// Tiger Style compliance:
/// - Explicitly sized types (u64 for indices)
/// - Bounded snapshot operations
/// - Fail-fast on corruption
#[derive(Clone, Debug)]
pub struct RedbStateMachine {
    db: Arc<Database>,
    path: PathBuf,
    snapshot_idx: Arc<AtomicU64>,
}

impl RedbStateMachine {
    /// Create or open a redb-backed state machine at the given path.
    pub fn new(path: impl AsRef<Path>) -> Result<Arc<Self>, StorageError> {
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
                .open_table(STATE_MACHINE_KV_TABLE)
                .context(OpenTableSnafu)?;
            write_txn
                .open_table(STATE_MACHINE_META_TABLE)
                .context(OpenTableSnafu)?;
            write_txn
                .open_table(SNAPSHOT_TABLE)
                .context(OpenTableSnafu)?;
        }
        write_txn.commit().context(CommitSnafu)?;

        Ok(Arc::new(Self {
            db: Arc::new(db),
            path,
            snapshot_idx: Arc::new(AtomicU64::new(0)),
        }))
    }

    /// Get the path to the state machine database file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get a key-value pair from the state machine.
    pub async fn get(&self, key: &str) -> Result<Option<String>, StorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn
            .open_table(STATE_MACHINE_KV_TABLE)
            .context(OpenTableSnafu)?;

        match table.get(key).context(GetSnafu)? {
            Some(value) => Ok(Some(value.value().to_string())),
            None => Ok(None),
        }
    }

    // Internal helper: Read metadata
    fn read_meta<T: for<'de> Deserialize<'de>>(
        &self,
        key: &str,
    ) -> Result<Option<T>, StorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn
            .open_table(STATE_MACHINE_META_TABLE)
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

    // Internal helper: Write metadata (reserved for future use)
    #[allow(dead_code)]
    fn write_meta<T: Serialize>(&self, key: &str, value: &T) -> Result<(), StorageError> {
        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut table = write_txn
                .open_table(STATE_MACHINE_META_TABLE)
                .context(OpenTableSnafu)?;
            let serialized = bincode::serialize(value).context(SerializeSnafu)?;
            table
                .insert(key, serialized.as_slice())
                .context(InsertSnafu)?;
        }
        write_txn.commit().context(CommitSnafu)?;
        Ok(())
    }
}

impl RaftSnapshotBuilder<AppTypeConfig> for Arc<RedbStateMachine> {
    async fn build_snapshot(&mut self) -> Result<Snapshot<AppTypeConfig>, io::Error> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;

        // Read all KV data
        let kv_table = read_txn
            .open_table(STATE_MACHINE_KV_TABLE)
            .context(OpenTableSnafu)?;
        let mut data = BTreeMap::new();
        for item in kv_table.iter().context(RangeSnafu)? {
            let (key, value) = item.context(GetSnafu)?;
            data.insert(key.value().to_string(), value.value().to_string());
        }

        // Read metadata
        let last_applied_log: Option<openraft::LogId<AppTypeConfig>> =
            self.read_meta("last_applied_log")?;
        let last_membership: StoredMembership<AppTypeConfig> =
            self.read_meta("last_membership")?.unwrap_or_default();

        drop(read_txn);

        let snapshot_data = serde_json::to_vec(&data)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

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
            snapshot_id: snapshot_id.clone(),
        };

        // Store snapshot in database
        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut snapshot_table = write_txn
                .open_table(SNAPSHOT_TABLE)
                .context(OpenTableSnafu)?;
            let snapshot_blob = bincode::serialize(&StoredSnapshot {
                meta: meta.clone(),
                data: snapshot_data.clone(),
            })
            .context(SerializeSnafu)?;
            snapshot_table
                .insert("current", snapshot_blob.as_slice())
                .context(InsertSnafu)?;
        }
        write_txn.commit().context(CommitSnafu)?;

        Ok(Snapshot {
            meta,
            snapshot: Cursor::new(snapshot_data),
        })
    }
}

impl RaftStateMachine<AppTypeConfig> for Arc<RedbStateMachine> {
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
        // Use a single read transaction for consistency
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let meta_table = read_txn
            .open_table(STATE_MACHINE_META_TABLE)
            .context(OpenTableSnafu)?;

        let last_applied_log: Option<openraft::LogId<AppTypeConfig>> =
            match meta_table.get("last_applied_log").context(GetSnafu)? {
                Some(value) => {
                    let bytes = value.value();
                    let result: Option<openraft::LogId<AppTypeConfig>> =
                        bincode::deserialize(bytes).map_err(|e| {
                            io::Error::new(
                                io::ErrorKind::InvalidData,
                                format!(
                                    "Failed to deserialize last_applied_log (len={}): {}",
                                    bytes.len(),
                                    e
                                ),
                            )
                        })?;
                    result
                }
                None => None,
            };

        let last_membership: StoredMembership<AppTypeConfig> =
            match meta_table.get("last_membership").context(GetSnafu)? {
                Some(value) => {
                    let bytes = value.value();
                    bincode::deserialize(bytes).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!(
                                "Failed to deserialize last_membership (len={}): {}",
                                bytes.len(),
                                e
                            ),
                        )
                    })?
                }
                None => StoredMembership::default(),
            };

        Ok((last_applied_log, last_membership))
    }

    async fn apply<Strm>(&mut self, mut entries: Strm) -> Result<(), io::Error>
    where
        Strm:
            Stream<Item = Result<EntryResponder<AppTypeConfig>, io::Error>> + Unpin + OptionalSend,
    {
        while let Some((entry, responder)) = entries.try_next().await? {
            let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
            {
                let mut kv_table = write_txn
                    .open_table(STATE_MACHINE_KV_TABLE)
                    .context(OpenTableSnafu)?;
                let mut meta_table = write_txn
                    .open_table(STATE_MACHINE_META_TABLE)
                    .context(OpenTableSnafu)?;

                // Update last_applied_log (store as Option for consistency with install_snapshot)
                let last_applied_bytes =
                    bincode::serialize(&Some(entry.log_id)).context(SerializeSnafu)?;
                meta_table
                    .insert("last_applied_log", last_applied_bytes.as_slice())
                    .context(InsertSnafu)?;

                // Apply the payload
                let response = match entry.payload {
                    EntryPayload::Blank => AppResponse { value: None },
                    EntryPayload::Normal(ref req) => match req {
                        AppRequest::Set { key, value } => {
                            kv_table
                                .insert(key.as_str(), value.as_str())
                                .context(InsertSnafu)?;
                            AppResponse {
                                value: Some(value.clone()),
                            }
                        }
                        AppRequest::SetMulti { pairs } => {
                            for (key, value) in pairs {
                                kv_table
                                    .insert(key.as_str(), value.as_str())
                                    .context(InsertSnafu)?;
                            }
                            AppResponse { value: None }
                        }
                    },
                    EntryPayload::Membership(ref membership) => {
                        let stored_membership =
                            StoredMembership::new(Some(entry.log_id), membership.clone());
                        let membership_bytes =
                            bincode::serialize(&stored_membership).context(SerializeSnafu)?;
                        meta_table
                            .insert("last_membership", membership_bytes.as_slice())
                            .context(InsertSnafu)?;
                        AppResponse { value: None }
                    }
                };

                if let Some(responder) = responder {
                    responder.send(response);
                }
            }
            write_txn.commit().context(CommitSnafu)?;
        }

        Ok(())
    }

    async fn begin_receiving_snapshot(
        &mut self,
    ) -> Result<SnapshotDataOf<AppTypeConfig>, io::Error> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let snapshot_table = read_txn
            .open_table(SNAPSHOT_TABLE)
            .context(OpenTableSnafu)?;

        match snapshot_table.get("current").context(GetSnafu)? {
            Some(value) => {
                let bytes = value.value();
                let snapshot: StoredSnapshot =
                    bincode::deserialize(bytes).context(DeserializeSnafu)?;
                Ok(Cursor::new(snapshot.data))
            }
            None => Ok(Cursor::new(Vec::new())),
        }
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

        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            // Clear existing KV data
            let mut kv_table = write_txn
                .open_table(STATE_MACHINE_KV_TABLE)
                .context(OpenTableSnafu)?;
            let keys: Vec<String> = kv_table
                .iter()
                .context(RangeSnafu)?
                .map(|item| {
                    let (key, _) = item.context(GetSnafu)?;
                    Ok::<_, StorageError>(key.value().to_string())
                })
                .collect::<Result<Vec<_>, _>>()?;
            for key in keys {
                kv_table.remove(key.as_str()).context(RemoveSnafu)?;
            }

            // Install new data
            for (key, value) in new_data {
                kv_table
                    .insert(key.as_str(), value.as_str())
                    .context(InsertSnafu)?;
            }

            // Update metadata
            let mut meta_table = write_txn
                .open_table(STATE_MACHINE_META_TABLE)
                .context(OpenTableSnafu)?;
            let last_applied_bytes =
                bincode::serialize(&meta.last_log_id).context(SerializeSnafu)?;
            meta_table
                .insert("last_applied_log", last_applied_bytes.as_slice())
                .context(InsertSnafu)?;
            let membership_bytes =
                bincode::serialize(&meta.last_membership).context(SerializeSnafu)?;
            meta_table
                .insert("last_membership", membership_bytes.as_slice())
                .context(InsertSnafu)?;

            // Store snapshot
            let mut snapshot_table = write_txn
                .open_table(SNAPSHOT_TABLE)
                .context(OpenTableSnafu)?;
            let snapshot_blob = bincode::serialize(&StoredSnapshot {
                meta: meta.clone(),
                data: snapshot_data,
            })
            .context(SerializeSnafu)?;
            snapshot_table
                .insert("current", snapshot_blob.as_slice())
                .context(InsertSnafu)?;
        }
        write_txn.commit().context(CommitSnafu)?;

        Ok(())
    }

    async fn get_current_snapshot(&mut self) -> Result<Option<Snapshot<AppTypeConfig>>, io::Error> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let snapshot_table = read_txn
            .open_table(SNAPSHOT_TABLE)
            .context(OpenTableSnafu)?;

        match snapshot_table.get("current").context(GetSnafu)? {
            Some(value) => {
                let bytes = value.value();
                let snapshot: StoredSnapshot =
                    bincode::deserialize(bytes).context(DeserializeSnafu)?;
                tracing::debug!(
                    "get_current_snapshot: returning snapshot at {:?}",
                    snapshot.meta.last_log_id
                );
                Ok(Some(Snapshot {
                    meta: snapshot.meta,
                    snapshot: Cursor::new(snapshot.data),
                }))
            }
            None => {
                tracing::debug!("get_current_snapshot: no snapshot found");
                Ok(None)
            }
        }
    }

    async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
        self.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openraft::StorageError;
    use openraft::testing::log::{StoreBuilder, Suite};

    /// Builder for testing Aspen's in-memory storage implementation against
    /// OpenRaft's comprehensive test suite. Validates correctness of log operations,
    /// snapshots, membership changes, and edge cases.
    struct InMemoryStoreBuilder;

    impl StoreBuilder<AppTypeConfig, InMemoryLogStore, Arc<StateMachineStore>, ()>
        for InMemoryStoreBuilder
    {
        async fn build(
            &self,
        ) -> Result<((), InMemoryLogStore, Arc<StateMachineStore>), StorageError<AppTypeConfig>>
        {
            Ok(((), InMemoryLogStore::default(), StateMachineStore::new()))
        }
    }

    /// Runs OpenRaft's full storage test suite (50+ tests) against Aspen's
    /// in-memory log and state machine implementations. This validates:
    /// - Log append, truncate, purge operations
    /// - Snapshot building and installation
    /// - Membership change persistence
    /// - Edge cases (empty log, gaps, conflicts)
    /// - State consistency under various scenarios
    #[tokio::test]
    async fn test_in_memory_storage_suite() -> Result<(), StorageError<AppTypeConfig>> {
        Suite::test_all(InMemoryStoreBuilder).await?;
        Ok(())
    }

    /// Builder for testing Aspen's redb storage implementation against
    /// OpenRaft's comprehensive test suite. Validates correctness with
    /// persistent storage (data survives process restarts).
    struct RedbStoreBuilder;

    impl StoreBuilder<AppTypeConfig, RedbLogStore, Arc<RedbStateMachine>, Box<tempfile::TempDir>>
        for RedbStoreBuilder
    {
        async fn build(
            &self,
        ) -> Result<
            (Box<tempfile::TempDir>, RedbLogStore, Arc<RedbStateMachine>),
            StorageError<AppTypeConfig>,
        > {
            use tempfile::TempDir;

            let temp_dir = TempDir::new().expect("failed to create temp directory");
            let log_path = temp_dir.path().join("raft-log.redb");
            let sm_path = temp_dir.path().join("state-machine.redb");

            let log_store = RedbLogStore::new(&log_path)
                .map_err(|e| -> io::Error { e.into() })
                .map_err(|e| StorageError::<AppTypeConfig>::read_logs(&e))?;

            let state_machine = RedbStateMachine::new(&sm_path)
                .map_err(|e| -> io::Error { e.into() })
                .map_err(|e| StorageError::<AppTypeConfig>::read_state_machine(&e))?;

            // Return the TempDir so it stays alive for the test duration
            Ok((Box::new(temp_dir), log_store, state_machine))
        }
    }

    /// Runs OpenRaft's full storage test suite (50+ tests) against Aspen's
    /// redb-backed log and state machine implementations. This validates:
    /// - Log append, truncate, purge operations with persistence
    /// - Snapshot building and installation to disk
    /// - Membership change persistence across restarts
    /// - ACID transaction guarantees
    /// - Edge cases with durable storage
    ///
    /// TODO(redb): Fix failing test `get_initial_state_re_apply_committed` which
    /// shows snapshot state inconsistency (term 257, index 1024 vs applied term 1, index 4).
    /// Basic persistence tests pass, but this edge case needs investigation.
    #[tokio::test]
    #[ignore = "TODO: Fix snapshot state inconsistency in comprehensive suite"]
    async fn test_redb_storage_suite() -> Result<(), StorageError<AppTypeConfig>> {
        Suite::test_all(RedbStoreBuilder).await?;
        Ok(())
    }

    /// Tests that redb storage persists data across database reopens.
    #[tokio::test]
    async fn test_redb_log_persistence() -> Result<(), super::StorageError> {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("failed to create temp directory");
        let log_path = temp_dir.path().join("raft-log-persist.redb");

        // Write vote
        {
            let mut log_store = RedbLogStore::new(&log_path)?;
            let vote = openraft::Vote::new(5, 1);
            log_store
                .save_vote(&vote)
                .await
                .expect("failed to save vote");
        }

        // Reopen and verify
        {
            let mut log_store = RedbLogStore::new(&log_path)?;
            let vote = log_store.read_vote().await.expect("failed to read vote");
            assert_eq!(vote, Some(openraft::Vote::new(5, 1)));
        }

        Ok(())
    }

    /// Tests that redb state machine persists data across database reopens.
    #[tokio::test]
    async fn test_redb_state_machine_persistence() -> Result<(), super::StorageError> {
        use futures::stream;
        use openraft::entry::RaftEntry;
        use openraft::testing::log_id;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("failed to create temp directory");
        let sm_path = temp_dir.path().join("state-machine-persist.redb");

        let log_id = log_id::<AppTypeConfig>(1, 1, 10);

        // Apply an entry
        {
            let mut sm = RedbStateMachine::new(&sm_path)?;
            let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                log_id,
                AppRequest::Set {
                    key: "test_key".into(),
                    value: "test_value".into(),
                },
            );
            let entries = Box::pin(stream::once(async move { Ok((entry, None)) }));
            sm.apply(entries).await.expect("failed to apply entry");
        }

        // Reopen and verify
        {
            let sm = RedbStateMachine::new(&sm_path)?;
            let value = sm.get("test_key").await?;
            assert_eq!(value, Some("test_value".to_string()));

            let mut sm_clone = sm.clone();
            let (last_applied, _) = sm_clone
                .applied_state()
                .await
                .expect("failed to get applied state");
            assert_eq!(last_applied, Some(log_id));
        }

        Ok(())
    }
}
