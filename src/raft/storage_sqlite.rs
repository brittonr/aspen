use std::collections::BTreeMap;
use std::io::{self, Cursor};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use futures::{Stream, TryStreamExt};
use openraft::storage::{EntryResponder, RaftSnapshotBuilder, RaftStateMachine, Snapshot};
use openraft::{EntryPayload, OptionalSend, StoredMembership};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};

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
}

impl From<SqliteStorageError> for io::Error {
    fn from(err: SqliteStorageError) -> Self {
        io::Error::new(io::ErrorKind::Other, err.to_string())
    }
}

/// Stored snapshot format (matches redb implementation)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredSnapshot {
    pub meta: openraft::SnapshotMeta<AppTypeConfig>,
    pub data: Vec<u8>,
}

/// SQLite-backed Raft state machine.
///
/// Stores key-value data, last applied log, and membership config in SQLite.
/// Provides ACID guarantees via SQLite transactions.
///
/// Tiger Style compliance:
/// - WAL mode for performance and durability
/// - FULL synchronous mode for safety
/// - Explicitly sized types (u64 for indices)
/// - Bounded operations
/// - Fail-fast on corruption
///
/// Schema:
/// - state_machine_kv: (key TEXT PRIMARY KEY, value TEXT)
/// - state_machine_meta: (key TEXT PRIMARY KEY, value BLOB)
/// - snapshots: (id TEXT PRIMARY KEY, data BLOB)
#[derive(Clone, Debug)]
pub struct SqliteStateMachine {
    /// SQLite connection (wrapped in Arc<Mutex> for interior mutability)
    /// Note: SQLite has internal locking, but we use Mutex for Rust's type system
    conn: Arc<Mutex<Connection>>,
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
    pub fn new(path: impl AsRef<Path>) -> Result<Arc<Self>, SqliteStorageError> {
        let path = path.as_ref().to_path_buf();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context(CreateDirectorySnafu { path: parent })?;
        }

        let conn = Connection::open(&path).context(OpenDatabaseSnafu { path: &path })?;

        // Configure SQLite for durability and performance (Tiger Style)
        // WAL mode: Better concurrency, crash-safe
        // FULL synchronous: Ensure data is on disk before commit returns
        conn.pragma_update(None, "journal_mode", "WAL")
            .context(ExecuteSnafu)?;
        conn.pragma_update(None, "synchronous", "FULL")
            .context(ExecuteSnafu)?;

        // Create tables if they don't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS state_machine_kv (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )
        .context(ExecuteSnafu)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS state_machine_meta (
                key TEXT PRIMARY KEY,
                value BLOB NOT NULL
            )",
            [],
        )
        .context(ExecuteSnafu)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS snapshots (
                id TEXT PRIMARY KEY,
                data BLOB NOT NULL
            )",
            [],
        )
        .context(ExecuteSnafu)?;

        Ok(Arc::new(Self {
            conn: Arc::new(Mutex::new(conn)),
            path,
            snapshot_idx: Arc::new(AtomicU64::new(0)),
        }))
    }

    /// Get the path to the state machine database file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Validates storage integrity (used by supervisor before restart).
    ///
    /// Performs basic SQLite database health checks.
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
        let conn = self.conn.lock().unwrap();

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

        drop(conn); // Release lock before duration calculation

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
    pub async fn get(&self, key: &str) -> Result<Option<String>, SqliteStorageError> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT value FROM state_machine_kv WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()
        .context(QuerySnafu)
    }

    /// Read metadata from the database
    fn read_meta<T: for<'de> Deserialize<'de>>(
        &self,
        key: &str,
    ) -> Result<Option<T>, SqliteStorageError> {
        let conn = self.conn.lock().unwrap();
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
}

impl RaftSnapshotBuilder<AppTypeConfig> for Arc<SqliteStateMachine> {
    async fn build_snapshot(
        &mut self,
    ) -> Result<Snapshot<AppTypeConfig>, io::Error> {
        let conn = self.conn.lock().unwrap();

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

        // Read metadata (using the already-locked connection to avoid deadlock)
        drop(stmt); // Release statement before reading meta

        // Read last_applied_log directly from conn (avoid calling read_meta which would deadlock)
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

        // Read last_membership directly from conn (avoid calling read_meta which would deadlock)
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

        // Serialize snapshot data
        let snapshot_data =
            serde_json::to_vec(&data).context(JsonSerializeSnafu)?;

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

        // Store snapshot in database
        let snapshot_blob = bincode::serialize(&StoredSnapshot {
            meta: meta.clone(),
            data: snapshot_data.clone(),
        })
        .context(SerializeSnafu)?;

        conn.execute(
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
        // Note: We serialize Option<LogId> (see apply() method line 362),
        // so we must deserialize as Option<Option<LogId>> then flatten
        let last_applied_log: Option<openraft::LogId<AppTypeConfig>> =
            self.read_meta::<Option<openraft::LogId<AppTypeConfig>>>("last_applied_log")?
                .flatten();
        let last_membership: StoredMembership<AppTypeConfig> =
            self.read_meta("last_membership")?.unwrap_or_default();

        Ok((last_applied_log, last_membership))
    }

    async fn apply<Strm>(&mut self, mut entries: Strm) -> Result<(), io::Error>
    where
        Strm: Stream<Item = Result<EntryResponder<AppTypeConfig>, io::Error>>
            + Unpin
            + OptionalSend,
    {
        while let Some((entry, responder)) = entries.try_next().await? {
            let conn = self.conn.lock().unwrap();

            // Start transaction
            conn.execute("BEGIN IMMEDIATE", []).context(ExecuteSnafu)?;

            // Update last_applied_log
            let last_applied_bytes = bincode::serialize(&Some(entry.log_id))
                .context(SerializeSnafu)?;
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
                        for (key, value) in pairs {
                            conn.execute(
                                "INSERT OR REPLACE INTO state_machine_kv (key, value) VALUES (?1, ?2)",
                                params![key, value],
                            )
                            .context(ExecuteSnafu)?;
                        }
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

            // Commit transaction
            conn.execute("COMMIT", []).context(ExecuteSnafu)?;

            if let Some(responder) = responder {
                responder.send(response);
            }
        }

        Ok(())
    }

    async fn begin_receiving_snapshot(
        &mut self,
    ) -> Result<Cursor<Vec<u8>>, io::Error> {
        let conn = self.conn.lock().unwrap();
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

        let conn = self.conn.lock().unwrap();

        // Start transaction
        conn.execute("BEGIN IMMEDIATE", []).context(ExecuteSnafu)?;

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
        let last_applied_bytes =
            bincode::serialize(&meta.last_log_id).context(SerializeSnafu)?;
        conn.execute(
            "INSERT OR REPLACE INTO state_machine_meta (key, value) VALUES ('last_applied_log', ?1)",
            params![last_applied_bytes],
        )
        .context(ExecuteSnafu)?;

        let membership_bytes =
            bincode::serialize(&meta.last_membership).context(SerializeSnafu)?;
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

        // Commit transaction
        conn.execute("COMMIT", []).context(ExecuteSnafu)?;

        Ok(())
    }

    async fn get_current_snapshot(
        &mut self,
    ) -> Result<Option<Snapshot<AppTypeConfig>>, io::Error> {
        let conn = self.conn.lock().unwrap();
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
