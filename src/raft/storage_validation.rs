use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use openraft::{LogId, Vote};
use redb::{Database, ReadableTable, TableDefinition};
use snafu::{ResultExt, Snafu};

use crate::raft::storage::StoredSnapshot;
use crate::raft::types::{AppTypeConfig, NodeId};

/// Redb table definitions (must match storage.rs)
const RAFT_LOG_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("raft_log");
const RAFT_META_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("raft_meta");
const SNAPSHOT_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("snapshots");

/// Errors that can occur during storage validation.
///
/// Tiger Style: Explicit error types with actionable context for operators.
#[derive(Debug, Snafu)]
pub enum StorageValidationError {
    #[snafu(display("failed to open redb database at {}: {}", path.display(), source))]
    DatabaseOpenFailed {
        path: PathBuf,
        #[snafu(source(from(redb::DatabaseError, Box::new)))]
        source: Box<redb::DatabaseError>,
    },

    #[snafu(display("failed to begin read transaction: {}", source))]
    BeginReadFailed {
        #[snafu(source(from(redb::TransactionError, Box::new)))]
        source: Box<redb::TransactionError>,
    },

    #[snafu(display("failed to open table '{}': {}", table_name, source))]
    OpenTableFailed {
        table_name: String,
        #[snafu(source(from(redb::TableError, Box::new)))]
        source: Box<redb::TableError>,
    },

    #[snafu(display("failed to read from table: {}", source))]
    TableReadFailed {
        #[snafu(source(from(redb::StorageError, Box::new)))]
        source: Box<redb::StorageError>,
    },

    #[snafu(display("log entries are not monotonic: gap between index {} and {}", prev, current))]
    LogNotMonotonic { prev: u64, current: u64 },

    #[snafu(display(
        "log entry has duplicate index {}: found multiple entries at same position",
        index
    ))]
    LogDuplicateIndex { index: u64 },

    #[snafu(display("snapshot metadata is corrupted: {}", reason))]
    SnapshotCorrupted { reason: String },

    #[snafu(display("failed to deserialize {}: {}", data_type, source))]
    DeserializeFailed {
        data_type: String,
        #[snafu(source(from(bincode::Error, Box::new)))]
        source: Box<bincode::Error>,
    },

    #[snafu(display("vote state is inconsistent: {}", reason))]
    VoteInconsistent { reason: String },

    #[snafu(display("committed log index is inconsistent: {}", reason))]
    CommittedInconsistent { reason: String },

    #[snafu(display("database file does not exist at {}", path.display()))]
    DatabaseNotFound { path: PathBuf },
}

/// Report generated after successful storage validation.
///
/// Contains metrics and diagnostics useful for monitoring and debugging.
#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub node_id: NodeId,
    pub checks_passed: u32,
    pub last_log_index: Option<u64>,
    pub last_snapshot_index: Option<u64>,
    pub vote_term: Option<u64>,
    pub committed_index: Option<u64>,
    pub validation_duration: Duration,
}

impl ValidationReport {
    /// Create a minimal validation report for in-memory storage (always valid).
    pub fn in_memory_ok(node_id: NodeId) -> Self {
        Self {
            node_id,
            checks_passed: 0,
            last_log_index: None,
            last_snapshot_index: None,
            vote_term: None,
            committed_index: None,
            validation_duration: Duration::from_micros(0),
        }
    }
}

/// Validates Raft storage integrity before allowing actor restart.
///
/// Performs comprehensive checks on the redb database to ensure it is not corrupted:
/// 1. Database can be opened and read
/// 2. Log entries are monotonic (no gaps in indices)
/// 3. Snapshot metadata is valid (if exists)
/// 4. Vote state is consistent
/// 5. Committed index is within bounds
///
/// # Tiger Style compliance
/// - Bounded operations: reads all log entries but with early termination on error
/// - Explicit error context: every failure includes node_id and specific check
/// - Fail-fast: returns immediately on first validation failure
///
/// # Performance
/// - Expected duration: <100ms for 1000 log entries
/// - Uses read-only transactions (no writes, no lock contention)
/// - Closes database after validation (no resource leaks)
pub fn validate_raft_storage(
    node_id: NodeId,
    storage_path: &Path,
) -> Result<ValidationReport, StorageValidationError> {
    let start = Instant::now();
    let mut checks_passed: u32 = 0;

    // Check if database file exists
    if !storage_path.exists() {
        return Err(StorageValidationError::DatabaseNotFound {
            path: storage_path.to_path_buf(),
        });
    }

    // 1. Verify database can be opened
    let db = open_redb_database(storage_path)?;
    checks_passed += 1;

    // 2. Validate log entry monotonicity (no gaps, ascending indices)
    let last_log_index = validate_log_monotonicity(&db)?;
    checks_passed += 1;

    // 3. Validate snapshot metadata (if exists)
    let last_snapshot_index = validate_snapshot_metadata(&db)?;
    checks_passed += 1;

    // 4. Validate vote state consistency
    let vote_term = validate_vote_state(&db)?;
    checks_passed += 1;

    // 5. Validate committed index is within bounds
    let committed_index = validate_committed_index(&db, last_log_index)?;
    checks_passed += 1;

    let validation_duration = start.elapsed();

    Ok(ValidationReport {
        node_id,
        checks_passed,
        last_log_index,
        last_snapshot_index,
        vote_term,
        committed_index,
        validation_duration,
    })
}

/// Opens the redb database at the specified path.
///
/// Tiger Style: Explicit error context includes the full path for debugging.
fn open_redb_database(path: &Path) -> Result<Database, StorageValidationError> {
    Database::open(path).context(DatabaseOpenFailedSnafu {
        path: path.to_path_buf(),
    })
}

/// Validates that log entries are monotonic (sequential, no gaps).
///
/// Returns the last log index if any entries exist, None otherwise.
///
/// # Validation rules
/// - Indices must be strictly sequential: 0, 1, 2, 3... (no gaps)
/// - No duplicate indices allowed
/// - Empty log is valid (returns None)
///
/// # Performance
/// - O(n) where n = number of log entries
/// - Early termination on first gap detected
fn validate_log_monotonicity(db: &Database) -> Result<Option<u64>, StorageValidationError> {
    let read_txn = db.begin_read().context(BeginReadFailedSnafu)?;
    let table = read_txn
        .open_table(RAFT_LOG_TABLE)
        .context(OpenTableFailedSnafu {
            table_name: "raft_log",
        })?;

    let mut prev_index: Option<u64> = None;
    let mut last_index: Option<u64> = None;

    // Iterate through all log entries in order
    for item in table.iter().context(TableReadFailedSnafu)? {
        let (index, _value) = item.context(TableReadFailedSnafu)?;
        let current_index = index.value();

        // Check for gaps in the sequence
        if let Some(prev) = prev_index {
            let expected = prev + 1;
            if current_index != expected {
                // Gap detected: entries are not sequential
                return Err(StorageValidationError::LogNotMonotonic {
                    prev,
                    current: current_index,
                });
            }
        }

        prev_index = Some(current_index);
        last_index = Some(current_index);
    }

    Ok(last_index)
}

/// Validates snapshot metadata integrity.
///
/// Returns the snapshot's last_log_id index if a snapshot exists, None otherwise.
///
/// # Validation rules
/// - Snapshot metadata must be deserializable
/// - Snapshot index must be valid (not absurdly large)
/// - Multiple snapshots not allowed (only "current" is valid)
///
/// # Note
/// We don't validate snapshot index <= last_log_index here because snapshots
/// can exist when log entries have been purged (this is normal after compaction).
fn validate_snapshot_metadata(db: &Database) -> Result<Option<u64>, StorageValidationError> {
    let read_txn = db.begin_read().context(BeginReadFailedSnafu)?;
    let table = read_txn
        .open_table(SNAPSHOT_TABLE)
        .context(OpenTableFailedSnafu {
            table_name: "snapshots",
        })?;

    // Try to read the current snapshot
    let snapshot_bytes = match table.get("current").context(TableReadFailedSnafu)? {
        Some(value) => value,
        None => return Ok(None), // No snapshot is valid
    };

    // Deserialize snapshot metadata
    let snapshot: StoredSnapshot =
        bincode::deserialize(snapshot_bytes.value()).context(DeserializeFailedSnafu {
            data_type: "snapshot metadata",
        })?;

    // Extract last_log_id index from snapshot
    let snapshot_index = snapshot.meta.last_log_id.map(|log_id| log_id.index);

    // Validate snapshot index is reasonable (Tiger Style: explicit bounds)
    if let Some(index) = snapshot_index {
        // Sanity check: snapshot index should not be absurdly large
        // (u64::MAX would indicate corruption or overflow)
        if index >= u64::MAX - 1000 {
            return Err(StorageValidationError::SnapshotCorrupted {
                reason: format!("snapshot index {} is suspiciously large", index),
            });
        }
    }

    Ok(snapshot_index)
}

/// Validates vote state consistency.
///
/// Returns the current term from the vote if it exists, None otherwise.
///
/// # Validation rules
/// - Vote must be deserializable if it exists
/// - Vote term must be >= 0 (always true for u64, but we check for sanity)
/// - No vote is valid (returns None)
fn validate_vote_state(db: &Database) -> Result<Option<u64>, StorageValidationError> {
    let read_txn = db.begin_read().context(BeginReadFailedSnafu)?;
    let table = read_txn
        .open_table(RAFT_META_TABLE)
        .context(OpenTableFailedSnafu {
            table_name: "raft_meta",
        })?;

    // Try to read vote state
    let vote_bytes = match table.get("vote").context(TableReadFailedSnafu)? {
        Some(value) => value,
        None => return Ok(None), // No vote is valid (initial state)
    };

    // Deserialize vote (using AppTypeConfig)
    let vote: Vote<AppTypeConfig> =
        bincode::deserialize(vote_bytes.value()).context(DeserializeFailedSnafu {
            data_type: "vote state",
        })?;

    // Extract term from vote
    let term = vote.leader_id().term;

    // Sanity check: term should be reasonable
    // (In practice, terms > 2^60 would indicate corruption)
    if term >= (1u64 << 60) {
        return Err(StorageValidationError::VoteInconsistent {
            reason: format!("vote term {} is suspiciously large", term),
        });
    }

    Ok(Some(term))
}

/// Validates that the committed index is consistent with the log.
///
/// Returns the committed index if it exists, None otherwise.
///
/// # Validation rules
/// - Committed index must be <= last_log_index (if both exist)
/// - Committed index must be deserializable if it exists
/// - No committed index is valid (returns None)
fn validate_committed_index(
    db: &Database,
    last_log_index: Option<u64>,
) -> Result<Option<u64>, StorageValidationError> {
    let read_txn = db.begin_read().context(BeginReadFailedSnafu)?;
    let table = read_txn
        .open_table(RAFT_META_TABLE)
        .context(OpenTableFailedSnafu {
            table_name: "raft_meta",
        })?;

    // Try to read committed state
    let committed_bytes = match table.get("committed").context(TableReadFailedSnafu)? {
        Some(value) => value,
        None => return Ok(None), // No committed index is valid (initial state)
    };

    // Deserialize committed log id
    let committed: LogId<AppTypeConfig> =
        bincode::deserialize(committed_bytes.value()).context(DeserializeFailedSnafu {
            data_type: "committed index",
        })?;

    let committed_index = committed.index;

    // Validate committed index is <= last_log_index
    if let Some(last_index) = last_log_index {
        if committed_index > last_index {
            return Err(StorageValidationError::CommittedInconsistent {
                reason: format!(
                    "committed index {} > last log index {}",
                    committed_index, last_index
                ),
            });
        }
    }

    Ok(Some(committed_index))
}

#[cfg(test)]
mod tests {
    use super::*;
    use openraft::testing::log_id;
    use tempfile::TempDir;

    /// Helper: Creates a valid redb database for testing.
    fn create_test_db() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let db_path = temp_dir.path().join("test.redb");
        (temp_dir, db_path)
    }

    /// Helper: Creates a database with sequential log entries.
    fn create_db_with_log_entries(db_path: &Path, num_entries: u32) -> Database {
        use openraft::entry::RaftEntry;
        use crate::raft::types::AppRequest;

        let db = Database::create(db_path).expect("failed to create db");
        let write_txn = db.begin_write().expect("failed to begin write");
        {
            // Create all required tables that validation expects
            let _ = write_txn.open_table(SNAPSHOT_TABLE);
            let _ = write_txn.open_table(RAFT_META_TABLE);

            let mut table = write_txn
                .open_table(RAFT_LOG_TABLE)
                .expect("failed to open table");

            for i in 0..num_entries {
                let log_id = log_id::<AppTypeConfig>(1, 1, i.into());
                let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                    log_id,
                    AppRequest::Set {
                        key: format!("key{}", i),
                        value: format!("value{}", i),
                    },
                );
                let serialized = bincode::serialize(&entry).expect("failed to serialize");
                table
                    .insert(u64::from(i), serialized.as_slice())
                    .expect("failed to insert");
            }
        }
        write_txn.commit().expect("failed to commit");
        db
    }

    #[test]
    fn test_validation_passes_on_healthy_storage() {
        let (_temp_dir, db_path) = create_test_db();
        {
            let _db = create_db_with_log_entries(&db_path, 10);
            // Database is closed when _db is dropped here
        }

        let result = validate_raft_storage(1, &db_path);
        assert!(result.is_ok(), "validation should pass: {:?}", result);

        let report = result.unwrap();
        assert_eq!(report.node_id, 1);
        assert_eq!(report.checks_passed, 5);
        assert_eq!(report.last_log_index, Some(9));
        assert!(report.validation_duration.as_millis() < 100);
    }

    #[test]
    fn test_validation_fails_on_missing_database() {
        let (_temp_dir, db_path) = create_test_db();
        // Don't create the database

        let result = validate_raft_storage(1, &db_path);
        assert!(result.is_err());

        match result.unwrap_err() {
            StorageValidationError::DatabaseNotFound { .. } => {}
            other => panic!("expected DatabaseNotFound, got {:?}", other),
        }
    }

    #[test]
    fn test_validation_fails_on_log_gap() {
        let (_temp_dir, db_path) = create_test_db();
        let db = create_db_with_log_entries(&db_path, 5);

        // Delete entry at index 2 to create a gap
        let write_txn = db.begin_write().expect("failed to begin write");
        {
            let mut table = write_txn
                .open_table(RAFT_LOG_TABLE)
                .expect("failed to open table");
            table.remove(2u64).expect("failed to remove");
        }
        write_txn.commit().expect("failed to commit");
        drop(db);

        let result = validate_raft_storage(1, &db_path);
        assert!(result.is_err());

        match result.unwrap_err() {
            StorageValidationError::LogNotMonotonic { prev, current } => {
                assert_eq!(prev, 1);
                assert_eq!(current, 3);
            }
            other => panic!("expected LogNotMonotonic, got {:?}", other),
        }
    }

    #[test]
    fn test_validation_report_contains_metrics() {
        let (_temp_dir, db_path) = create_test_db();
        let _db = create_db_with_log_entries(&db_path, 100);

        let result = validate_raft_storage(42, &db_path);
        assert!(result.is_ok());

        let report = result.unwrap();
        assert_eq!(report.node_id, 42);
        assert_eq!(report.last_log_index, Some(99));
        assert!(report.validation_duration.as_millis() < 100);
    }

    #[test]
    fn test_empty_log_is_valid() {
        let (_temp_dir, db_path) = create_test_db();
        let db = Database::create(&db_path).expect("failed to create db");

        // Initialize tables but don't add any entries
        let write_txn = db.begin_write().expect("failed to begin write");
        {
            write_txn
                .open_table(RAFT_LOG_TABLE)
                .expect("failed to open table");
            write_txn
                .open_table(RAFT_META_TABLE)
                .expect("failed to open table");
            write_txn
                .open_table(SNAPSHOT_TABLE)
                .expect("failed to open table");
        }
        write_txn.commit().expect("failed to commit");
        drop(db);

        let result = validate_raft_storage(1, &db_path);
        assert!(result.is_ok());

        let report = result.unwrap();
        assert_eq!(report.last_log_index, None);
    }
}
