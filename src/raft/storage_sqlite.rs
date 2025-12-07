use std::collections::BTreeMap;
use std::io::{self, Cursor};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use futures::{Stream, TryStreamExt};
use openraft::storage::{EntryResponder, RaftSnapshotBuilder, RaftStateMachine, Snapshot};
use openraft::{EntryPayload, OptionalSend, StoredMembership};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};

use crate::raft::types::{AppRequest, AppResponse, AppTypeConfig};

/// Maximum number of entries to apply in a single batch.
/// Tiger Style: Fixed limits prevent unbounded resource usage.
const MAX_BATCH_SIZE: u32 = 1000;

/// Maximum number of key-value pairs in a single SetMulti operation.
/// Tiger Style: Fixed limits prevent unbounded resource usage.
const MAX_SETMULTI_KEYS: u32 = 100;

/// Default size of read connection pool.
/// Tiger Style: Fixed limit on concurrent readers.
const DEFAULT_READ_POOL_SIZE: u32 = 10;

/// Errors that can occur when using SQLite storage
#[derive(Debug, Snafu)]
pub enum SqliteStorageError {
    #[snafu(display("failed to open sqlite database at {}: {source}", path.display()))]
    OpenDatabase {
        path: PathBuf,
        source: rusqlite::Error,
    },

    #[snafu(display("failed to execute SQL statement: {source}"))]
    Execute { source: rusqlite::Error },

    #[snafu(display("failed to query database: {source}"))]
    Query { source: rusqlite::Error },

    #[snafu(display("failed to serialize data: {source}"))]
    Serialize { source: bincode::Error },

    #[snafu(display("failed to deserialize data: {source}"))]
    Deserialize { source: bincode::Error },

    #[snafu(display("failed to serialize JSON: {source}"))]
    JsonSerialize { source: serde_json::Error },

    #[snafu(display("failed to deserialize JSON: {source}"))]
    JsonDeserialize { source: serde_json::Error },

    #[snafu(display("failed to create directory {}: {source}", path.display()))]
    CreateDirectory {
        path: PathBuf,
        source: std::io::Error,
    },

    #[snafu(display("I/O error on path {}: {source}", path.display()))]
    IoError {
        path: PathBuf,
        source: std::io::Error,
    },

    #[snafu(display("connection pool error: {source}"))]
    PoolError { source: r2d2::Error },

    #[snafu(display("failed to build connection pool: {source}"))]
    PoolBuild { source: r2d2::Error },

    #[snafu(display(
        "storage lock poisoned during {operation} - database may be in inconsistent state, restart required"
    ))]
    MutexPoisoned { operation: &'static str },
}

impl From<SqliteStorageError> for io::Error {
    fn from(err: SqliteStorageError) -> Self {
        io::Error::other(err.to_string())
    }
}

/// RAII guard that ensures SQLite transactions are properly rolled back on error/panic.
/// Automatically rolls back on drop unless explicitly committed.
///
/// Tiger Style compliance:
/// - Fail-fast: Transaction begins immediately or returns error
/// - RAII: Automatic cleanup via Drop trait
/// - Explicit commit: Caller must explicitly commit to persist changes
struct TransactionGuard<'a> {
    conn: &'a Connection,
    committed: bool,
}

impl<'a> TransactionGuard<'a> {
    /// Begin a new IMMEDIATE transaction.
    ///
    /// Returns error if transaction cannot be started.
    /// Transaction will automatically roll back if not committed.
    fn new(conn: &'a Connection) -> Result<Self, SqliteStorageError> {
        // End any lingering transaction first (Tiger Style: fail-fast on unexpected state)
        // In SQLite, BEGIN when already in a transaction is a no-op, which can cause
        // commits to commit the WRONG transaction. We must ensure we start fresh.
        let _ = conn.execute("ROLLBACK", []);

        conn.execute("BEGIN IMMEDIATE", []).context(ExecuteSnafu)?;
        Ok(Self {
            conn,
            committed: false,
        })
    }

    /// Commit the transaction, marking it as complete.
    ///
    /// Consumes the guard to prevent further use.
    /// If commit fails, the guard is dropped and transaction rolls back.
    fn commit(mut self) -> Result<(), SqliteStorageError> {
        self.conn.execute("COMMIT", []).context(ExecuteSnafu)?;
        self.committed = true;
        Ok(())
    }
}

impl Drop for TransactionGuard<'_> {
    fn drop(&mut self) {
        if !self.committed {
            // Best-effort rollback - ignore errors since we're already unwinding
            // Logging is not done here to avoid allocation during panic
            let _ = self.conn.execute("ROLLBACK", []);
        }
    }
}

/// Stored snapshot format (matches redb implementation)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredSnapshot {
    pub meta: openraft::SnapshotMeta<AppTypeConfig>,
    pub data: Vec<u8>,
}

/// SQLite-backed Raft state machine with connection pooling.
///
/// Stores key-value data, last applied log, and membership config in SQLite.
/// Provides ACID guarantees via SQLite transactions.
///
/// Tiger Style compliance:
/// - WAL mode for performance and durability
/// - FULL synchronous mode for safety
/// - Explicitly sized types (u64 for indices)
/// - Bounded operations (fixed pool size)
/// - Fail-fast on corruption
/// - Connection pooling for concurrent reads
///
/// Schema:
/// - state_machine_kv: (key TEXT PRIMARY KEY, value TEXT)
/// - state_machine_meta: (key TEXT PRIMARY KEY, value BLOB)
/// - snapshots: (id TEXT PRIMARY KEY, data BLOB)
///
/// WAL Checkpoint Management:
/// - WAL file size monitoring via wal_file_size()
/// - Manual checkpoint via checkpoint_wal()
/// - Auto-checkpoint at threshold via auto_checkpoint_if_needed()
#[derive(Clone, Debug)]
pub struct SqliteStateMachine {
    /// Connection pool for read operations (allows concurrent reads via WAL mode)
    read_pool: Pool<SqliteConnectionManager>,
    /// Single connection for write operations (SQLite single-writer constraint)
    /// Public for migration tool access
    pub write_conn: Arc<Mutex<Connection>>,
    /// Path to the database file
    path: PathBuf,
    /// Snapshot index counter (for generating unique snapshot IDs)
    snapshot_idx: Arc<AtomicU64>,
}

impl SqliteStateMachine {
    /// Create or open a SQLite-backed state machine at the given path.
    ///
    /// Initializes the database schema if it doesn't exist.
    /// Configures WAL mode and FULL synchronous for durability.
    /// Creates a connection pool with DEFAULT_READ_POOL_SIZE connections.
    pub fn new(path: impl AsRef<Path>) -> Result<Arc<Self>, SqliteStorageError> {
        Self::with_pool_size(path, DEFAULT_READ_POOL_SIZE)
    }

    /// Create or open a SQLite-backed state machine with a custom pool size.
    ///
    /// Allows configuring the number of read connections in the pool.
    /// Tiger Style: Explicit pool size parameter, bounded by caller.
    pub fn with_pool_size(
        path: impl AsRef<Path>,
        pool_size: u32,
    ) -> Result<Arc<Self>, SqliteStorageError> {
        let path = path.as_ref().to_path_buf();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context(CreateDirectorySnafu { path: parent })?;
        }

        // Create write connection
        let write_conn = Connection::open(&path).context(OpenDatabaseSnafu { path: &path })?;

        // Set restrictive file permissions (owner-only read/write)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&path)
                .context(IoSnafu { path: &path })?
                .permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(&path, perms).context(IoSnafu { path: &path })?;
        }

        // Configure SQLite for durability and performance (Tiger Style)
        // WAL mode: Better concurrency, crash-safe
        // FULL synchronous: Ensure data is on disk before commit returns
        write_conn
            .pragma_update(None, "journal_mode", "WAL")
            .context(ExecuteSnafu)?;
        write_conn
            .pragma_update(None, "synchronous", "FULL")
            .context(ExecuteSnafu)?;

        // Performance optimizations
        // cache_size: 64MB cache for better read performance (10-20% improvement)
        // Negative value = size in KB (Tiger Style: explicit units)
        write_conn
            .pragma_update(None, "cache_size", "-64000")
            .context(ExecuteSnafu)?;

        // mmap_size: 256MB memory-mapped I/O for faster reads (5-15% improvement)
        write_conn
            .pragma_update(None, "mmap_size", "268435456")
            .context(ExecuteSnafu)?;

        // Create tables if they don't exist
        write_conn
            .execute(
                "CREATE TABLE IF NOT EXISTS state_machine_kv (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
                [],
            )
            .context(ExecuteSnafu)?;

        write_conn
            .execute(
                "CREATE TABLE IF NOT EXISTS state_machine_meta (
                key TEXT PRIMARY KEY,
                value BLOB NOT NULL
            )",
                [],
            )
            .context(ExecuteSnafu)?;

        write_conn
            .execute(
                "CREATE TABLE IF NOT EXISTS snapshots (
                id TEXT PRIMARY KEY,
                data BLOB NOT NULL
            )",
                [],
            )
            .context(ExecuteSnafu)?;

        // Create read connection pool
        let manager = SqliteConnectionManager::file(&path);
        let read_pool = Pool::builder()
            .max_size(pool_size)
            .build(manager)
            .context(PoolBuildSnafu)?;

        Ok(Arc::new(Self {
            read_pool,
            write_conn: Arc::new(Mutex::new(write_conn)),
            path,
            snapshot_idx: Arc::new(AtomicU64::new(0)),
        }))
    }

    /// Get the path to the state machine database file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Reset read connection to ensure we see latest committed data.
    ///
    /// In SQLite WAL mode, pooled connections may reuse read snapshots from previous queries,
    /// causing reads to return stale data even after writes have committed.
    /// This function forces a fresh read snapshot by executing a dummy query in auto-commit mode.
    ///
    /// # Tiger Style compliance
    /// - Fail-fast: Returns error if transaction commands fail (indicates DB corruption)
    /// - Explicit: Function name clearly states intent
    /// - Safe: Only affects read snapshot, never modifies data
    fn reset_read_connection(conn: &Connection) -> Result<(), SqliteStorageError> {
        // End any lingering transaction (even if none exists, this is safe)
        let _ = conn.execute("ROLLBACK", []);

        // Execute a dummy query to force SQLite to update to the latest WAL snapshot
        // This is necessary because WAL readers can hold onto old snapshots
        let _: i32 = conn
            .query_row("SELECT 1", [], |row| row.get(0))
            .context(QuerySnafu)?;

        Ok(())
    }

    /// Count the number of key-value pairs in the state machine.
    /// Uses read pool for concurrent reads.
    pub fn count_kv_pairs(&self) -> Result<i64, SqliteStorageError> {
        let conn = self.read_pool.get().context(PoolSnafu)?;
        Self::reset_read_connection(&conn)?;
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM state_machine_kv", [], |row| {
                row.get(0)
            })
            .context(QuerySnafu)?;
        Ok(count)
    }

    /// Count key-value pairs matching a LIKE pattern.
    /// Uses read pool for concurrent reads.
    pub fn count_kv_pairs_like(&self, pattern: &str) -> Result<i64, SqliteStorageError> {
        let conn = self.read_pool.get().context(PoolSnafu)?;
        Self::reset_read_connection(&conn)?;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM state_machine_kv WHERE key LIKE ?1",
                params![pattern],
                |row| row.get(0),
            )
            .context(QuerySnafu)?;
        Ok(count)
    }

    /// Validates storage integrity (used by supervisor before restart).
    ///
    /// Performs basic SQLite database health checks.
    /// Uses read pool for concurrent reads.
    pub fn validate(
        &self,
        node_id: u64,
    ) -> Result<
        crate::raft::storage_validation::ValidationReport,
        crate::raft::storage_validation::StorageValidationError,
    > {
        use std::time::Instant;

        let start = Instant::now();

        // Basic validation: Check that we can access the database
        let conn = self.read_pool.get().map_err(|_e| {
            crate::raft::storage_validation::StorageValidationError::DatabaseNotFound {
                path: self.path.clone(),
            }
        })?;

        // Run SQLite integrity check
        let integrity: String = conn
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .map_err(|_e| {
                crate::raft::storage_validation::StorageValidationError::DatabaseNotFound {
                    path: self.path.clone(),
                }
            })?;

        if integrity != "ok" {
            return Err(
                crate::raft::storage_validation::StorageValidationError::DatabaseNotFound {
                    path: self.path.clone(),
                },
            );
        }

        // Verify required tables exist
        let table_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('state_machine_kv', 'state_machine_meta', 'snapshots')",
                [],
                |row| row.get(0),
            )
            .map_err(|_| {
                crate::raft::storage_validation::StorageValidationError::DatabaseNotFound {
                    path: self.path.clone(),
                }
            })?;

        if table_count != 3 {
            return Err(
                crate::raft::storage_validation::StorageValidationError::DatabaseNotFound {
                    path: self.path.clone(),
                },
            );
        }

        drop(conn); // Release connection to pool

        Ok(crate::raft::storage_validation::ValidationReport {
            node_id,
            checks_passed: 2, // Integrity check + schema check
            last_log_index: None,
            last_snapshot_index: None,
            vote_term: None,
            committed_index: None,
            validation_duration: start.elapsed(),
        })
    }

    /// Get a key-value pair from the state machine.
    /// Uses write connection to ensure we see latest committed data.
    ///
    /// Note: In WAL mode, read pooled connections can see stale snapshots even after
    /// writes commit. To guarantee consistency, we use the write connection for reads
    /// which always sees its own writes and the latest committed state.
    pub async fn get(&self, key: &str) -> Result<Option<String>, SqliteStorageError> {
        let conn = self
            .write_conn
            .lock()
            .map_err(|_| SqliteStorageError::MutexPoisoned { operation: "get" })?;

        let result = conn
            .query_row(
                "SELECT value FROM state_machine_kv WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()
            .context(QuerySnafu)?;

        Ok(result)
    }

    /// Validates that the SQLite state machine is consistent with the redb log.
    ///
    /// Returns error if last_applied exceeds the log's committed index, which
    /// violates Raft invariants and indicates corruption.
    ///
    /// # Tiger Style compliance
    /// - Fail-fast: Returns immediately on inconsistency detection
    /// - Explicit bounds: Checks last_applied <= committed
    /// - Clear error messages: Operator knows exactly what's wrong
    pub async fn validate_consistency_with_log(
        &self,
        log_store: &crate::raft::storage::RedbLogStore,
    ) -> Result<(), String> {
        // Read last_applied directly (cannot use applied_state() which requires &mut self)
        let last_applied: Option<openraft::LogId<AppTypeConfig>> = self
            .read_meta::<Option<openraft::LogId<AppTypeConfig>>>("last_applied_log")
            .map_err(|e| format!("Failed to read applied state: {}", e))?
            .flatten();

        let committed = log_store
            .read_committed_sync()
            .await
            .map_err(|e| format!("Failed to read committed index: {}", e))?;

        if let (Some(applied), Some(committed_idx)) = (last_applied, committed)
            && applied.index > committed_idx
        {
            return Err(format!(
                "State machine corruption detected: last_applied ({}) exceeds committed ({})",
                applied.index, committed_idx
            ));
        }

        Ok(())
    }

    /// Read metadata from the database.
    /// Uses read pool for concurrent reads.
    fn read_meta<T: for<'de> Deserialize<'de>>(
        &self,
        key: &str,
    ) -> Result<Option<T>, SqliteStorageError> {
        let conn = self.read_pool.get().context(PoolSnafu)?;
        Self::reset_read_connection(&conn)?;
        let bytes: Option<Vec<u8>> = conn
            .query_row(
                "SELECT value FROM state_machine_meta WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()
            .context(QuerySnafu)?;

        match bytes {
            Some(bytes) => {
                let data: T = bincode::deserialize(&bytes).context(DeserializeSnafu)?;
                Ok(Some(data))
            }
            None => Ok(None),
        }
    }

    /// Returns the size of the WAL file in bytes, or None if WAL file doesn't exist.
    ///
    /// Tiger Style: Fail-fast on I/O errors accessing WAL file.
    pub fn wal_file_size(&self) -> Result<Option<u64>, SqliteStorageError> {
        let wal_path = self.path.with_extension("db-wal");

        match std::fs::metadata(&wal_path) {
            Ok(metadata) => Ok(Some(metadata.len())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(SqliteStorageError::IoError {
                path: wal_path,
                source: e,
            }),
        }
    }

    /// Performs a WAL checkpoint to reclaim space.
    ///
    /// Uses TRUNCATE mode to checkpoint and truncate the WAL file.
    /// Returns the number of pages checkpointed.
    ///
    /// Tiger Style: Explicit return type, fail-fast on checkpoint errors.
    pub fn checkpoint_wal(&self) -> Result<u32, SqliteStorageError> {
        let conn = self
            .write_conn
            .lock()
            .map_err(|_| SqliteStorageError::MutexPoisoned {
                operation: "checkpoint",
            })?;

        // TRUNCATE mode: checkpoint and truncate WAL file
        let mut checkpointed: i32 = 0;
        conn.pragma_update_and_check(None, "wal_checkpoint", "TRUNCATE", |row| {
            checkpointed = row.get(0)?;
            Ok(())
        })
        .context(ExecuteSnafu)?;

        Ok(checkpointed as u32)
    }

    /// Checks if WAL file exceeds threshold and auto-checkpoints if needed.
    ///
    /// Returns Some(pages_checkpointed) if checkpoint was performed.
    /// Returns None if checkpoint was not needed.
    ///
    /// Tiger Style: Explicit threshold parameter, bounded operation.
    pub fn auto_checkpoint_if_needed(
        &self,
        threshold_bytes: u64,
    ) -> Result<Option<u32>, SqliteStorageError> {
        match self.wal_file_size()? {
            Some(size) if size > threshold_bytes => {
                let pages = self.checkpoint_wal()?;
                Ok(Some(pages))
            }
            _ => Ok(None),
        }
    }
}

impl RaftSnapshotBuilder<AppTypeConfig> for Arc<SqliteStateMachine> {
    async fn build_snapshot(&mut self) -> Result<Snapshot<AppTypeConfig>, io::Error> {
        // Use read pool connection for snapshot build (non-blocking for writes)
        let conn = self.read_pool.get().context(PoolSnafu)?;
        SqliteStateMachine::reset_read_connection(&conn)?;

        // Read all KV data
        let mut stmt = conn
            .prepare("SELECT key, value FROM state_machine_kv")
            .context(QuerySnafu)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .context(QuerySnafu)?;

        let mut data = BTreeMap::new();
        for row in rows {
            let (key, value) = row.context(QuerySnafu)?;
            data.insert(key, value);
        }

        drop(stmt); // Release statement

        // Read metadata
        let last_applied_log: Option<openraft::LogId<AppTypeConfig>> = {
            let bytes: Option<Vec<u8>> = conn
                .query_row(
                    "SELECT value FROM state_machine_meta WHERE key = ?1",
                    params!["last_applied_log"],
                    |row| row.get(0),
                )
                .optional()
                .context(QuerySnafu)?;

            match bytes {
                Some(bytes) => {
                    let data: Option<openraft::LogId<AppTypeConfig>> =
                        bincode::deserialize(&bytes).context(DeserializeSnafu)?;
                    data
                }
                None => None,
            }
        };

        let last_membership: StoredMembership<AppTypeConfig> = {
            let bytes: Option<Vec<u8>> = conn
                .query_row(
                    "SELECT value FROM state_machine_meta WHERE key = ?1",
                    params!["last_membership"],
                    |row| row.get(0),
                )
                .optional()
                .context(QuerySnafu)?;

            match bytes {
                Some(bytes) => bincode::deserialize(&bytes).context(DeserializeSnafu)?,
                None => StoredMembership::default(),
            }
        };

        drop(conn); // Release connection to pool

        // Serialize snapshot data
        let snapshot_data = serde_json::to_vec(&data).context(JsonSerializeSnafu)?;

        // Generate snapshot ID
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

        // Store snapshot in database (write operation)
        let snapshot_blob = bincode::serialize(&StoredSnapshot {
            meta: meta.clone(),
            data: snapshot_data.clone(),
        })
        .context(SerializeSnafu)?;

        let write_conn = self
            .write_conn
            .lock()
            .map_err(|_| SqliteStorageError::MutexPoisoned {
                operation: "snapshot_build",
            })?;

        write_conn
            .execute(
                "INSERT OR REPLACE INTO snapshots (id, data) VALUES ('current', ?1)",
                params![snapshot_blob],
            )
            .context(ExecuteSnafu)?;

        Ok(Snapshot {
            meta,
            snapshot: Cursor::new(snapshot_data),
        })
    }
}

impl RaftStateMachine<AppTypeConfig> for Arc<SqliteStateMachine> {
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
        // Note: We serialize Option<LogId> (see apply() method),
        // so we must deserialize as Option<Option<LogId>> then flatten
        let last_applied_log: Option<openraft::LogId<AppTypeConfig>> = self
            .read_meta::<Option<openraft::LogId<AppTypeConfig>>>("last_applied_log")?
            .flatten();
        let last_membership: StoredMembership<AppTypeConfig> =
            self.read_meta("last_membership")?.unwrap_or_default();

        Ok((last_applied_log, last_membership))
    }

    async fn apply<Strm>(&mut self, mut entries: Strm) -> Result<(), io::Error>
    where
        Strm:
            Stream<Item = Result<EntryResponder<AppTypeConfig>, io::Error>> + Unpin + OptionalSend,
    {
        let mut batch_count: u32 = 0;

        while let Some((entry, responder)) = entries.try_next().await? {
            batch_count += 1;
            if batch_count > MAX_BATCH_SIZE {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "Batch size {} exceeds maximum limit of {}",
                        batch_count, MAX_BATCH_SIZE
                    ),
                ));
            }
            let conn = self.write_conn.lock().map_err(|_| {
                io::Error::other(SqliteStorageError::MutexPoisoned { operation: "apply" })
            })?;

            // Start transaction with RAII guard for automatic rollback on error
            let guard = TransactionGuard::new(&conn)?;

            // Update last_applied_log
            let last_applied_bytes =
                bincode::serialize(&Some(entry.log_id)).context(SerializeSnafu)?;
            conn.execute(
                "INSERT OR REPLACE INTO state_machine_meta (key, value) VALUES ('last_applied_log', ?1)",
                params![last_applied_bytes],
            )
            .context(ExecuteSnafu)?;

            // Apply the payload
            let response = match entry.payload {
                EntryPayload::Blank => AppResponse { value: None },
                EntryPayload::Normal(ref req) => match req {
                    AppRequest::Set { key, value } => {
                        conn.execute(
                            "INSERT OR REPLACE INTO state_machine_kv (key, value) VALUES (?1, ?2)",
                            params![key, value],
                        )
                        .context(ExecuteSnafu)?;
                        AppResponse {
                            value: Some(value.clone()),
                        }
                    }
                    AppRequest::SetMulti { pairs } => {
                        if pairs.len() > MAX_SETMULTI_KEYS as usize {
                            return Err(io::Error::other(format!(
                                "SetMulti operation with {} keys exceeds maximum limit of {}",
                                pairs.len(),
                                MAX_SETMULTI_KEYS
                            )));
                        }

                        // Prepare statement once and reuse for all inserts (20-30% performance improvement)
                        let mut stmt = conn.prepare(
                            "INSERT OR REPLACE INTO state_machine_kv (key, value) VALUES (?1, ?2)"
                        ).context(QuerySnafu)?;

                        for (key, value) in pairs {
                            stmt.execute(params![key, value]).context(ExecuteSnafu)?;
                        }

                        drop(stmt); // Explicit drop (Tiger Style)
                        AppResponse { value: None }
                    }
                },
                EntryPayload::Membership(ref membership) => {
                    let stored_membership =
                        StoredMembership::new(Some(entry.log_id), membership.clone());
                    let membership_bytes =
                        bincode::serialize(&stored_membership).context(SerializeSnafu)?;
                    conn.execute(
                        "INSERT OR REPLACE INTO state_machine_meta (key, value) VALUES ('last_membership', ?1)",
                        params![membership_bytes],
                    )
                    .context(ExecuteSnafu)?;
                    AppResponse { value: None }
                }
            };

            // Commit transaction - guard is consumed and dropped after commit
            guard.commit()?;

            // Force a WAL checkpoint to ensure data is visible to other connections
            // This is necessary because in WAL mode, readers can see stale snapshots
            // until the WAL is checkpointed or they explicitly refresh.
            // Use PASSIVE mode which doesn't block writers.
            let _ = conn.pragma_update(None, "wal_checkpoint", "PASSIVE");

            if let Some(responder) = responder {
                responder.send(response);
            }
        }

        Ok(())
    }

    async fn begin_receiving_snapshot(&mut self) -> Result<Cursor<Vec<u8>>, io::Error> {
        // Use read pool for non-blocking read
        let conn = self.read_pool.get().context(PoolSnafu)?;
        SqliteStateMachine::reset_read_connection(&conn)?;
        let bytes: Option<Vec<u8>> = conn
            .query_row(
                "SELECT data FROM snapshots WHERE id = 'current'",
                [],
                |row| row.get(0),
            )
            .optional()
            .context(QuerySnafu)?;

        match bytes {
            Some(bytes) => {
                let snapshot: StoredSnapshot =
                    bincode::deserialize(&bytes).context(DeserializeSnafu)?;
                Ok(Cursor::new(snapshot.data))
            }
            None => Ok(Cursor::new(Vec::new())),
        }
    }

    async fn install_snapshot(
        &mut self,
        meta: &openraft::SnapshotMeta<AppTypeConfig>,
        mut snapshot: Cursor<Vec<u8>>,
    ) -> Result<(), io::Error> {
        // Read snapshot data
        let mut snapshot_data = Vec::new();
        std::io::copy(&mut snapshot, &mut snapshot_data)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

        let new_data: BTreeMap<String, String> =
            serde_json::from_slice(&snapshot_data).context(JsonDeserializeSnafu)?;

        let conn = self.write_conn.lock().map_err(|_| {
            io::Error::other(SqliteStorageError::MutexPoisoned {
                operation: "snapshot_install",
            })
        })?;

        // Start transaction with RAII guard for automatic rollback on error
        let guard = TransactionGuard::new(&conn)?;

        // Clear existing KV data
        conn.execute("DELETE FROM state_machine_kv", [])
            .context(ExecuteSnafu)?;

        // Install new data
        for (key, value) in new_data {
            conn.execute(
                "INSERT INTO state_machine_kv (key, value) VALUES (?1, ?2)",
                params![key, value],
            )
            .context(ExecuteSnafu)?;
        }

        // Update metadata
        let last_applied_bytes = bincode::serialize(&meta.last_log_id).context(SerializeSnafu)?;
        conn.execute(
            "INSERT OR REPLACE INTO state_machine_meta (key, value) VALUES ('last_applied_log', ?1)",
            params![last_applied_bytes],
        )
        .context(ExecuteSnafu)?;

        let membership_bytes = bincode::serialize(&meta.last_membership).context(SerializeSnafu)?;
        conn.execute(
            "INSERT OR REPLACE INTO state_machine_meta (key, value) VALUES ('last_membership', ?1)",
            params![membership_bytes],
        )
        .context(ExecuteSnafu)?;

        // Store snapshot
        let snapshot_blob = bincode::serialize(&StoredSnapshot {
            meta: meta.clone(),
            data: snapshot_data,
        })
        .context(SerializeSnafu)?;
        conn.execute(
            "INSERT OR REPLACE INTO snapshots (id, data) VALUES ('current', ?1)",
            params![snapshot_blob],
        )
        .context(ExecuteSnafu)?;

        // Commit transaction - guard is consumed and dropped after commit
        guard.commit()?;

        Ok(())
    }

    async fn get_current_snapshot(&mut self) -> Result<Option<Snapshot<AppTypeConfig>>, io::Error> {
        // Use read pool for non-blocking read
        let conn = self.read_pool.get().context(PoolSnafu)?;
        SqliteStateMachine::reset_read_connection(&conn)?;
        let bytes: Option<Vec<u8>> = conn
            .query_row(
                "SELECT data FROM snapshots WHERE id = 'current'",
                [],
                |row| row.get(0),
            )
            .optional()
            .context(QuerySnafu)?;

        match bytes {
            Some(bytes) => {
                let snapshot: StoredSnapshot =
                    bincode::deserialize(&bytes).context(DeserializeSnafu)?;
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
