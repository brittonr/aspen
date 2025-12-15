//! SQLite-based state machine implementation for Raft.
//!
//! Provides an ACID-compliant key-value state machine backed by SQLite with connection
//! pooling for concurrent reads. This is the default production state machine, offering
//! durability, queryability, and snapshot support. Write operations are serialized through
//! a single connection, while reads use a bounded connection pool (r2d2) for parallelism.
//!
//! # Key Components
//!
//! - `SqliteStateMachine`: RaftStateMachine implementation with pooled reads
//! - `SnapshotBuilder`: Incremental snapshot creation from SQLite state
//! - Connection pooling: r2d2 manages bounded read connections (DEFAULT_READ_POOL_SIZE = 8)
//! - Batch operations: SetMulti supports up to MAX_SETMULTI_KEYS (100) keys
//! - Snapshot metadata: Tracks last applied log and membership in separate table
//!
//! # Tiger Style
//!
//! - Fixed limits: MAX_BATCH_SIZE (1024), MAX_SETMULTI_KEYS (100), read pool size (8)
//! - Explicit types: u64 for log indices, i64 for SQLite integers (cross-platform)
//! - Resource bounds: Connection pool prevents unbounded connection growth
//! - Error handling: SNAFU errors with actionable context for operators
//! - Transactions: All writes use BEGIN IMMEDIATE for write serialization
//! - WAL mode: Write-ahead logging enabled for better concurrency
//!
//! # Schema
//!
//! - `kv_data`: (key TEXT PRIMARY KEY, value BLOB) - Main key-value store
//! - `snapshot_meta`: Stores last_applied_log and membership JSON
//!
//! # Example
//!
//! ```ignore
//! use aspen::raft::storage_sqlite::SqliteStateMachine;
//!
//! let state_machine = SqliteStateMachine::new("./data/state.db").await?;
//!
//! // Apply operations through Raft
//! let request = AppRequest::Set { key: "foo".into(), value: b"bar".to_vec() };
//! let response = state_machine.apply(log_id, request).await?;
//! ```

use std::collections::BTreeMap;
use std::io::{self, Cursor};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use futures::{Stream, TryStreamExt};
use openraft::storage::{
    ApplyResponder, EntryResponder, RaftSnapshotBuilder, RaftStateMachine, Snapshot,
};
use openraft::{EntryPayload, OptionalSend, StoredMembership};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};

use crate::raft::constants::{
    DEFAULT_READ_POOL_SIZE, MAX_BATCH_SIZE, MAX_SETMULTI_KEYS, MAX_SNAPSHOT_ENTRIES,
    MAX_SNAPSHOT_SIZE,
};
use crate::raft::types::{AppRequest, AppResponse, AppTypeConfig};

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

/// Apply a batch of buffered entries in a single SQLite transaction.
///
/// This is the core optimization: instead of one transaction per entry,
/// we batch up to BATCH_BUFFER_SIZE entries into a single atomic transaction.
/// This reduces:
/// - fsync operations from N to 1
/// - Transaction overhead from N to 1
/// - Lock acquisitions from N to 1
/// - WAL checkpoint operations from N to 1
///
/// # Tiger Style
/// - Fixed batch size limits prevent unbounded resource usage
/// - RAII TransactionGuard ensures atomic commit/rollback
/// - All responses sent after transaction commits for durability
fn apply_buffered_entries_impl(
    write_conn: &Arc<Mutex<Connection>>,
    buffer: &mut Vec<EntryResponder<AppTypeConfig>>,
) -> Result<(), io::Error> {
    if buffer.is_empty() {
        return Ok(());
    }

    let conn = write_conn.lock().map_err(|_| {
        io::Error::other(SqliteStorageError::MutexPoisoned {
            operation: "apply_batch",
        })
    })?;

    // Start single transaction for entire batch
    let guard = TransactionGuard::new(&conn)?;

    // Collect responses to send after transaction commits
    // ApplyResponder is an enum with send() method for the response
    type ResponsePair = (Option<ApplyResponder<AppTypeConfig>>, AppResponse);
    let mut responses: Vec<ResponsePair> = Vec::with_capacity(buffer.len());

    // Apply all entries within the transaction
    for (entry, responder) in buffer.drain(..) {
        // Update last_applied_log for each entry (idempotent, only final value matters)
        SqliteStateMachine::update_last_applied_log(&conn, &entry.log_id)?;

        // Apply the payload
        let response =
            SqliteStateMachine::apply_entry_payload(&conn, &entry.payload, &entry.log_id)?;

        responses.push((responder, response));
    }

    // Single COMMIT for entire batch - this is where durability is guaranteed
    guard.commit()?;

    // Force a single WAL checkpoint for the entire batch.
    // This amortizes the checkpoint cost across all entries in the batch.
    // Use PASSIVE mode which doesn't block writers.
    let _ = conn.pragma_update(None, "wal_checkpoint", "PASSIVE");

    // Send all responses after transaction is durably committed
    // This ensures linearizability: responses only sent after data is durable
    for (responder, response) in responses {
        if let Some(r) = responder {
            r.send(response);
        }
    }

    Ok(())
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

        // Create indexes for efficient vault operations
        write_conn
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_kv_prefix ON state_machine_kv(key)",
                [],
            )
            .context(ExecuteSnafu)?;

        // Create vault metadata table for tracking vault-level information
        write_conn
            .execute(
                "CREATE TABLE IF NOT EXISTS vault_metadata (
                vault_name TEXT PRIMARY KEY,
                created_at INTEGER NOT NULL DEFAULT (unixepoch()),
                updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
                key_count INTEGER NOT NULL DEFAULT 0,
                total_bytes INTEGER NOT NULL DEFAULT 0,
                description TEXT,
                owner TEXT,
                tags TEXT
            )",
                [],
            )
            .context(ExecuteSnafu)?;

        // Create vault access log for monitoring and optimization
        write_conn
            .execute(
                "CREATE TABLE IF NOT EXISTS vault_access_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                vault_name TEXT NOT NULL,
                operation TEXT NOT NULL,
                key TEXT,
                timestamp INTEGER NOT NULL DEFAULT (unixepoch()),
                duration_us INTEGER
            )",
                [],
            )
            .context(ExecuteSnafu)?;

        // Index for time-based access queries
        write_conn
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_access_time ON vault_access_log(timestamp)",
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
            node_id: node_id.into(),
            checks_passed: 2, // Integrity check + schema check
            last_log_index: None,
            last_snapshot_index: None,
            vote_term: None,
            committed_index: None,
            validation_duration: start.elapsed(),
        })
    }

    /// Compute a Blake3 checksum of the entire state machine for replica verification.
    ///
    /// This allows comparing state machines across replicas to detect divergence.
    /// The checksum includes:
    /// - All key-value pairs (sorted by key for determinism)
    /// - The last_applied_log metadata
    ///
    /// # TigerBeetle-Style Consistency Verification
    ///
    /// TigerBeetle uses checksums to verify replica consistency after operations.
    /// This method enables similar verification in Aspen:
    ///
    /// ```ignore
    /// // After replication settles, verify all replicas have identical state
    /// let checksums: Vec<_> = nodes.iter().map(|n| n.state_machine_checksum()).collect();
    /// assert!(checksums.windows(2).all(|w| w[0] == w[1]), "Replica divergence detected!");
    /// ```
    ///
    /// # Performance
    ///
    /// This operation reads all key-value pairs, so it should be used judiciously:
    /// - Good for periodic consistency checks in tests
    /// - Good for debugging suspected divergence
    /// - Avoid in hot paths or high-frequency operations
    ///
    /// # Returns
    ///
    /// A 32-byte Blake3 hash as a hex string, or error if database access fails.
    pub fn state_machine_checksum(&self) -> Result<String, SqliteStorageError> {
        let conn = self.read_pool.get().context(PoolSnafu)?;
        Self::reset_read_connection(&conn)?;

        let mut hasher = blake3::Hasher::new();

        // Hash all key-value pairs in deterministic (sorted) order
        // SQLite ORDER BY ensures consistent ordering across replicas
        let mut stmt = conn
            .prepare("SELECT key, value FROM state_machine_kv ORDER BY key")
            .context(QuerySnafu)?;

        let rows = stmt
            .query_map([], |row| {
                let key: String = row.get(0)?;
                let value: String = row.get(1)?;
                Ok((key, value))
            })
            .context(QuerySnafu)?;

        for row in rows {
            let (key, value) = row.context(QuerySnafu)?;
            // Hash key length + key + value length + value for unambiguous parsing
            hasher.update(&(key.len() as u64).to_le_bytes());
            hasher.update(key.as_bytes());
            hasher.update(&(value.len() as u64).to_le_bytes());
            hasher.update(value.as_bytes());
        }

        // Include last_applied_log in checksum for complete state verification
        let meta_bytes: Option<Vec<u8>> = conn
            .query_row(
                "SELECT value FROM state_machine_meta WHERE key = 'last_applied_log'",
                [],
                |row| row.get(0),
            )
            .optional()
            .context(QuerySnafu)?;

        if let Some(bytes) = meta_bytes {
            hasher.update(b"last_applied_log:");
            hasher.update(&bytes);
        }

        let hash = hasher.finalize();
        Ok(hash.to_hex().to_string())
    }

    /// Compute a partial checksum for a range of keys.
    ///
    /// Useful for debugging divergence by narrowing down which keys differ.
    ///
    /// # Arguments
    /// * `prefix` - Only include keys starting with this prefix
    ///
    /// # Returns
    /// A Blake3 hash of matching key-value pairs as a hex string.
    pub fn state_machine_checksum_prefix(
        &self,
        prefix: &str,
    ) -> Result<String, SqliteStorageError> {
        let conn = self.read_pool.get().context(PoolSnafu)?;
        Self::reset_read_connection(&conn)?;

        let mut hasher = blake3::Hasher::new();

        let mut stmt = conn
            .prepare("SELECT key, value FROM state_machine_kv WHERE key LIKE ?1 ORDER BY key")
            .context(QuerySnafu)?;

        let pattern = format!("{}%", prefix);
        let rows = stmt
            .query_map([pattern], |row| {
                let key: String = row.get(0)?;
                let value: String = row.get(1)?;
                Ok((key, value))
            })
            .context(QuerySnafu)?;

        for row in rows {
            let (key, value) = row.context(QuerySnafu)?;
            hasher.update(&(key.len() as u64).to_le_bytes());
            hasher.update(key.as_bytes());
            hasher.update(&(value.len() as u64).to_le_bytes());
            hasher.update(value.as_bytes());
        }

        let hash = hasher.finalize();
        Ok(hash.to_hex().to_string())
    }

    /// Get a key-value pair from the state machine.
    /// Uses read pool for concurrent access. Calls `reset_read_connection` to ensure
    /// we see the latest committed data.
    ///
    /// Note: In WAL mode, pooled connections can hold onto stale snapshots.
    /// We reset the connection before each read to force SQLite to update to the
    /// latest WAL snapshot.
    pub async fn get(&self, key: &str) -> Result<Option<String>, SqliteStorageError> {
        let conn = self.read_pool.get().context(PoolSnafu)?;
        Self::reset_read_connection(&conn)?;

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

    /// Scan all keys that start with the given prefix.
    ///
    /// Returns a list of full key names.
    pub fn scan_keys_with_prefix(&self, prefix: &str) -> Result<Vec<String>, SqliteStorageError> {
        let conn = self.read_pool.get().context(PoolSnafu)?;
        Self::reset_read_connection(&conn)?;

        // Use range query for better performance with indexes
        // The end bound is the prefix with the next character incremented
        let end_prefix = format!("{}\u{10000}", prefix); // Use a high Unicode character as boundary

        let mut stmt = conn
            .prepare_cached("SELECT key FROM state_machine_kv WHERE key >= ?1 AND key < ?2 ORDER BY key LIMIT ?3")
            .context(QuerySnafu)?;

        let keys = stmt
            .query_map(params![prefix, end_prefix, MAX_BATCH_SIZE as i64], |row| {
                row.get(0)
            })
            .context(QuerySnafu)?
            .filter_map(|r| r.ok())
            .filter(|k: &String| k.starts_with(prefix)) // Double-check the prefix match
            .collect();

        Ok(keys)
    }

    /// Scan all key-value pairs that start with the given prefix.
    ///
    /// Returns a list of (key, value) pairs.
    pub fn scan_kv_with_prefix(
        &self,
        prefix: &str,
    ) -> Result<Vec<(String, String)>, SqliteStorageError> {
        let conn = self.read_pool.get().context(PoolSnafu)?;
        Self::reset_read_connection(&conn)?;

        // Use range query for better performance with indexes
        let end_prefix = format!("{}\u{10000}", prefix);

        let mut stmt = conn
            .prepare_cached("SELECT key, value FROM state_machine_kv WHERE key >= ?1 AND key < ?2 ORDER BY key LIMIT ?3")
            .context(QuerySnafu)?;

        let kv_pairs = stmt
            .query_map(params![prefix, end_prefix, MAX_BATCH_SIZE as i64], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .context(QuerySnafu)?
            .filter_map(|r| r.ok())
            .filter(|(k, _)| k.starts_with(prefix)) // Double-check the prefix match
            .collect();

        Ok(kv_pairs)
    }

    /// Async version of scan_kv_with_prefix for use in async contexts.
    ///
    /// This is a thin async wrapper around the sync version since SQLite
    /// operations are synchronous but may need to be called from async code.
    pub async fn scan_kv_with_prefix_async(
        &self,
        prefix: &str,
    ) -> Result<Vec<(String, String)>, SqliteStorageError> {
        // SQLite operations are sync but fast enough for inline execution
        // For very large datasets, consider tokio::task::spawn_blocking
        self.scan_kv_with_prefix(prefix)
    }

    /// Scan keys matching a prefix with pagination support.
    ///
    /// # Arguments
    /// * `prefix` - Key prefix to match (empty string matches all)
    /// * `after_key` - Optional continuation token (exclusive start key)
    /// * `limit` - Optional maximum number of results (defaults to MAX_BATCH_SIZE)
    ///
    /// # Tiger Style
    /// - Fixed limits prevent unbounded memory usage
    /// - Cursor-based pagination for stable iteration
    pub async fn scan(
        &self,
        prefix: &str,
        after_key: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<(String, String)>, SqliteStorageError> {
        let conn = self.read_pool.get().context(PoolSnafu)?;
        Self::reset_read_connection(&conn)?;

        let limit = limit
            .unwrap_or(MAX_BATCH_SIZE as usize)
            .min(MAX_BATCH_SIZE as usize);
        let end_prefix = format!("{}\u{10000}", prefix);

        // Different query based on whether we have a continuation token
        let kv_pairs = if let Some(start) = after_key {
            let mut stmt = conn
                .prepare_cached(
                    "SELECT key, value FROM state_machine_kv \
                     WHERE key > ?1 AND key >= ?2 AND key < ?3 \
                     ORDER BY key LIMIT ?4",
                )
                .context(QuerySnafu)?;

            stmt.query_map(params![start, prefix, end_prefix, limit as i64], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .context(QuerySnafu)?
            .filter_map(|r| r.ok())
            .filter(|(k, _)| k.starts_with(prefix))
            .collect()
        } else {
            let mut stmt = conn
                .prepare_cached(
                    "SELECT key, value FROM state_machine_kv \
                     WHERE key >= ?1 AND key < ?2 \
                     ORDER BY key LIMIT ?3",
                )
                .context(QuerySnafu)?;

            stmt.query_map(params![prefix, end_prefix, limit as i64], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .context(QuerySnafu)?
            .filter_map(|r| r.ok())
            .filter(|(k, _)| k.starts_with(prefix))
            .collect()
        };

        Ok(kv_pairs)
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

    /// Update the last_applied_log metadata in the state machine.
    ///
    /// Serializes and stores the log ID to track which entries have been applied.
    /// Tiger Style: Single-purpose function, explicit error handling.
    fn update_last_applied_log(
        conn: &Connection,
        log_id: &openraft::LogId<AppTypeConfig>,
    ) -> Result<(), io::Error> {
        let last_applied_bytes = bincode::serialize(&Some(log_id)).context(SerializeSnafu)?;
        conn.execute(
            "INSERT OR REPLACE INTO state_machine_meta (key, value) VALUES ('last_applied_log', ?1)",
            params![last_applied_bytes],
        )
        .context(ExecuteSnafu)?;
        Ok(())
    }

    /// Apply a single key-value Set operation.
    ///
    /// Inserts or replaces a key-value pair in the state machine.
    /// Returns response with the value that was set.
    ///
    /// Tiger Style: Single-purpose, explicit return type.
    fn apply_set(conn: &Connection, key: &str, value: &str) -> Result<AppResponse, io::Error> {
        conn.execute(
            "INSERT OR REPLACE INTO state_machine_kv (key, value) VALUES (?1, ?2)",
            params![key, value],
        )
        .context(ExecuteSnafu)?;
        Ok(AppResponse {
            value: Some(value.to_string()),
        })
    }

    /// Apply a batched SetMulti operation.
    ///
    /// Inserts or replaces multiple key-value pairs in a single operation.
    /// Enforces MAX_SETMULTI_KEYS limit to prevent unbounded resource usage.
    /// Uses prepared statement for 20-30% performance improvement.
    ///
    /// Tiger Style: Fixed limit, batched operation, explicit cleanup.
    fn apply_set_multi(
        conn: &Connection,
        pairs: &[(String, String)],
    ) -> Result<AppResponse, io::Error> {
        if pairs.len() > MAX_SETMULTI_KEYS as usize {
            return Err(io::Error::other(format!(
                "SetMulti operation with {} keys exceeds maximum limit of {}",
                pairs.len(),
                MAX_SETMULTI_KEYS
            )));
        }

        // Prepare statement once and reuse for all inserts (20-30% performance improvement)
        let mut stmt = conn
            .prepare("INSERT OR REPLACE INTO state_machine_kv (key, value) VALUES (?1, ?2)")
            .context(QuerySnafu)?;

        for (key, value) in pairs {
            stmt.execute(params![key, value]).context(ExecuteSnafu)?;
        }

        drop(stmt); // Explicit drop (Tiger Style)
        Ok(AppResponse { value: None })
    }

    /// Apply a single key Delete operation.
    ///
    /// Deletes a key from the state machine.
    /// Returns response indicating the operation completed.
    ///
    /// Tiger Style: Single-purpose, idempotent (no error if key doesn't exist).
    fn apply_delete(conn: &Connection, key: &str) -> Result<AppResponse, io::Error> {
        conn.execute("DELETE FROM state_machine_kv WHERE key = ?1", params![key])
            .context(ExecuteSnafu)?;
        Ok(AppResponse { value: None })
    }

    /// Apply a batched DeleteMulti operation.
    ///
    /// Deletes multiple keys in a single operation.
    /// Enforces MAX_SETMULTI_KEYS limit to prevent unbounded resource usage.
    /// Uses prepared statement for consistent performance.
    ///
    /// Tiger Style: Fixed limit, batched operation, idempotent.
    fn apply_delete_multi(conn: &Connection, keys: &[String]) -> Result<AppResponse, io::Error> {
        if keys.len() > MAX_SETMULTI_KEYS as usize {
            return Err(io::Error::other(format!(
                "DeleteMulti operation with {} keys exceeds maximum limit of {}",
                keys.len(),
                MAX_SETMULTI_KEYS
            )));
        }

        // Prepare statement once and reuse for all deletes
        let mut stmt = conn
            .prepare("DELETE FROM state_machine_kv WHERE key = ?1")
            .context(QuerySnafu)?;

        for key in keys {
            stmt.execute(params![key]).context(ExecuteSnafu)?;
        }

        drop(stmt); // Explicit drop (Tiger Style)
        Ok(AppResponse { value: None })
    }

    /// Apply a membership change operation.
    ///
    /// Stores the new membership configuration in the state machine metadata.
    /// Tiger Style: Single-purpose, explicit serialization.
    fn apply_membership(
        conn: &Connection,
        log_id: &openraft::LogId<AppTypeConfig>,
        membership: &openraft::Membership<AppTypeConfig>,
    ) -> Result<AppResponse, io::Error> {
        let stored_membership = StoredMembership::new(Some(*log_id), membership.clone());
        let membership_bytes = bincode::serialize(&stored_membership).context(SerializeSnafu)?;
        conn.execute(
            "INSERT OR REPLACE INTO state_machine_meta (key, value) VALUES ('last_membership', ?1)",
            params![membership_bytes],
        )
        .context(ExecuteSnafu)?;
        Ok(AppResponse { value: None })
    }

    /// Apply an entry payload to the state machine.
    ///
    /// Dispatches to the appropriate handler based on payload type.
    /// Tiger Style: Centralized control flow, delegates to single-purpose helpers.
    fn apply_entry_payload(
        conn: &Connection,
        payload: &EntryPayload<AppTypeConfig>,
        log_id: &openraft::LogId<AppTypeConfig>,
    ) -> Result<AppResponse, io::Error> {
        match payload {
            EntryPayload::Blank => Ok(AppResponse { value: None }),
            EntryPayload::Normal(req) => match req {
                AppRequest::Set { key, value } => Self::apply_set(conn, key, value),
                AppRequest::SetMulti { pairs } => Self::apply_set_multi(conn, pairs),
                AppRequest::Delete { key } => Self::apply_delete(conn, key),
                AppRequest::DeleteMulti { keys } => Self::apply_delete_multi(conn, keys),
            },
            EntryPayload::Membership(membership) => {
                Self::apply_membership(conn, log_id, membership)
            }
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
        let mut entry_count: u32 = 0;
        for row in rows {
            let (key, value) = row.context(QuerySnafu)?;

            // Tiger Style: Check entry count BEFORE inserting to prevent unbounded growth
            entry_count += 1;
            if entry_count > MAX_SNAPSHOT_ENTRIES {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "snapshot exceeds maximum entry count of {} (current: {})",
                        MAX_SNAPSHOT_ENTRIES, entry_count
                    ),
                ));
            }

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

        // Tiger Style: Validate size BEFORE storing to database
        if snapshot_data.len() as u64 > MAX_SNAPSHOT_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "serialized snapshot size {} bytes exceeds maximum of {} bytes",
                    snapshot_data.len(),
                    MAX_SNAPSHOT_SIZE
                ),
            ));
        }

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

        // Start transaction with RAII guard for automatic rollback on error
        // Tiger Style: Atomic snapshot write, fail-fast with explicit error handling
        let guard = TransactionGuard::new(&write_conn)?;

        write_conn
            .execute(
                "INSERT OR REPLACE INTO snapshots (id, data) VALUES ('current', ?1)",
                params![snapshot_blob],
            )
            .context(ExecuteSnafu)?;

        // Commit transaction - guard is consumed and dropped after commit
        guard.commit()?;

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
        // Tiger Style: Fixed batch size limit for memory and transaction bounds
        const BATCH_BUFFER_SIZE: usize = 100;

        // Buffer entries for batch application within a single transaction.
        // This significantly reduces fsync operations and lock contention.
        // Per TigerBeetle design patterns: batch multiple operations into single
        // atomic transaction for 30-90% write throughput improvement.
        //
        // EntryResponder<C> is a type alias for (Entry<C>, Option<ApplyResponder<C>>)
        let mut buffer: Vec<EntryResponder<AppTypeConfig>> = Vec::with_capacity(BATCH_BUFFER_SIZE);
        let mut total_count: u32 = 0;

        // Collect entries from stream into buffer
        while let Some(entry_responder) = entries.try_next().await? {
            total_count += 1;
            if total_count > MAX_BATCH_SIZE {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "Batch size {} exceeds maximum limit of {}",
                        total_count, MAX_BATCH_SIZE
                    ),
                ));
            }

            buffer.push(entry_responder);

            // Apply buffer when full to bound memory usage
            if buffer.len() >= BATCH_BUFFER_SIZE {
                apply_buffered_entries_impl(&self.write_conn, &mut buffer)?;
            }
        }

        // Apply remaining entries in buffer
        if !buffer.is_empty() {
            apply_buffered_entries_impl(&self.write_conn, &mut buffer)?;
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
