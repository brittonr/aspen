#![allow(deprecated)]

use std::path::PathBuf;

use aspen::raft::storage::{RedbLogStore, RedbStateMachine};
use aspen::raft::storage_validation::{StorageValidationError, validate_raft_storage};
use aspen::raft::types::{AppRequest, AppTypeConfig};
use futures::stream;
use openraft::entry::RaftEntry;
use openraft::storage::{RaftLogStorage, RaftStateMachine};
use openraft::testing::log_id;
use redb::{Database, TableDefinition};
use tempfile::TempDir;

/// Redb table definitions (must match storage.rs)
const RAFT_LOG_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("raft_log");
const RAFT_META_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("raft_meta");
const SNAPSHOT_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("snapshots");

/// Helper: Creates a temporary directory for tests
fn create_temp_dir() -> TempDir {
    TempDir::new().expect("failed to create temp directory")
}

/// Helper: Creates a valid redb database path
fn create_db_path(temp_dir: &TempDir, name: &str) -> PathBuf {
    temp_dir.path().join(format!("{}.redb", name))
}

/// Helper: Creates a database with sequential log entries
fn create_db_with_log_entries(db_path: &PathBuf, num_entries: u32) -> Database {
    let db = Database::create(db_path).expect("failed to create database");
    let write_txn = db.begin_write().expect("failed to begin write transaction");
    {
        // Create all required tables that validation expects
        let _ = write_txn.open_table(SNAPSHOT_TABLE);
        let _ = write_txn.open_table(RAFT_META_TABLE);

        let mut table = write_txn
            .open_table(RAFT_LOG_TABLE)
            .expect("failed to open log table");

        for i in 0..num_entries {
            let log_id = log_id::<AppTypeConfig>(1, 1, u64::from(i));
            let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                log_id,
                AppRequest::Set {
                    key: format!("key{}", i),
                    value: format!("value{}", i),
                },
            );
            let serialized = bincode::serialize(&entry).expect("failed to serialize entry");
            table
                .insert(u64::from(i), serialized.as_slice())
                .expect("failed to insert entry");
        }
    }
    write_txn.commit().expect("failed to commit transaction");
    db
}

#[test]
fn test_validation_passes_on_healthy_storage() {
    let temp_dir = create_temp_dir();
    let db_path = create_db_path(&temp_dir, "healthy");
    {
        let _db = create_db_with_log_entries(&db_path, 10);
    } // Database dropped here

    let result = validate_raft_storage(1, &db_path);
    assert!(result.is_ok(), "validation should pass on healthy storage");

    let report = result.unwrap();
    assert_eq!(report.node_id, 1);
    assert_eq!(
        report.checks_passed, 5,
        "all 5 validation checks should pass"
    );
    assert_eq!(report.last_log_index, Some(9));
    assert!(
        report.validation_duration.as_millis() < 100,
        "validation should complete in <100ms"
    );
}

#[test]
fn test_validation_fails_on_missing_database() {
    let temp_dir = create_temp_dir();
    let db_path = create_db_path(&temp_dir, "nonexistent");
    // Don't create the database file

    let result = validate_raft_storage(1, &db_path);
    assert!(
        result.is_err(),
        "validation should fail on missing database"
    );

    match result.unwrap_err() {
        StorageValidationError::DatabaseNotFound { path } => {
            assert_eq!(path, db_path);
        }
        other => panic!("expected DatabaseNotFound, got {:?}", other),
    }
}

#[test]
fn test_validation_fails_on_log_gap() {
    let temp_dir = create_temp_dir();
    let db_path = create_db_path(&temp_dir, "gap");
    let db = create_db_with_log_entries(&db_path, 10);

    // Create a gap by deleting entry at index 5
    let write_txn = db.begin_write().expect("failed to begin write");
    {
        let mut table = write_txn
            .open_table(RAFT_LOG_TABLE)
            .expect("failed to open table");
        table.remove(5u64).expect("failed to remove entry");
    }
    write_txn.commit().expect("failed to commit");
    drop(db);

    let result = validate_raft_storage(1, &db_path);
    assert!(result.is_err(), "validation should fail on log gap");

    match result.unwrap_err() {
        StorageValidationError::LogNotMonotonic { prev, current } => {
            assert_eq!(prev, 4, "gap should be detected after index 4");
            assert_eq!(current, 6, "gap should skip to index 6");
        }
        other => panic!("expected LogNotMonotonic, got {:?}", other),
    }
}

#[test]
fn test_validation_report_contains_metrics() {
    let temp_dir = create_temp_dir();
    let db_path = create_db_path(&temp_dir, "metrics");
    {
        let _db = create_db_with_log_entries(&db_path, 100);
    } // Database dropped here

    let result = validate_raft_storage(42, &db_path);
    assert!(result.is_ok());

    let report = result.unwrap();
    assert_eq!(report.node_id, 42);
    assert_eq!(report.last_log_index, Some(99));
    assert!(report.validation_duration.as_micros() > 0);
}

#[test]
fn test_empty_log_is_valid() {
    let temp_dir = create_temp_dir();
    let db_path = create_db_path(&temp_dir, "empty");
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
    assert!(result.is_ok(), "empty log should be valid");

    let report = result.unwrap();
    assert_eq!(report.last_log_index, None);
    assert_eq!(report.committed_index, None);
}

#[test]
fn test_validation_with_vote_state() {
    let temp_dir = create_temp_dir();
    let db_path = create_db_path(&temp_dir, "vote");
    {
        let db = create_db_with_log_entries(&db_path, 5);

        // Add vote state
        let write_txn = db.begin_write().expect("failed to begin write");
        {
            let mut table = write_txn
                .open_table(RAFT_META_TABLE)
                .expect("failed to open table");
            let vote = openraft::Vote::<AppTypeConfig>::new(3, 1);
            let serialized = bincode::serialize(&vote).expect("failed to serialize vote");
            table
                .insert("vote", serialized.as_slice())
                .expect("failed to insert vote");
        }
        write_txn.commit().expect("failed to commit");
    } // Database dropped here

    let result = validate_raft_storage(1, &db_path);
    assert!(result.is_ok());

    let report = result.unwrap();
    assert_eq!(report.vote_term, Some(3));
}

#[test]
fn test_validation_with_committed_index() {
    let temp_dir = create_temp_dir();
    let db_path = create_db_path(&temp_dir, "committed");
    {
        let db = create_db_with_log_entries(&db_path, 10);

        // Add committed state
        let write_txn = db.begin_write().expect("failed to begin write");
        {
            let mut table = write_txn
                .open_table(RAFT_META_TABLE)
                .expect("failed to open table");
            let committed = log_id::<AppTypeConfig>(1, 1, 5);
            let serialized = bincode::serialize(&committed).expect("failed to serialize committed");
            table
                .insert("committed", serialized.as_slice())
                .expect("failed to insert committed");
        }
        write_txn.commit().expect("failed to commit");
    } // Database dropped here

    let result = validate_raft_storage(1, &db_path);
    assert!(result.is_ok());

    let report = result.unwrap();
    assert_eq!(report.committed_index, Some(5));
}

#[test]
fn test_validation_fails_on_committed_beyond_log() {
    let temp_dir = create_temp_dir();
    let db_path = create_db_path(&temp_dir, "committed_beyond");
    {
        let db = create_db_with_log_entries(&db_path, 5);

        // Set committed index beyond last log index
        let write_txn = db.begin_write().expect("failed to begin write");
        {
            let mut table = write_txn
                .open_table(RAFT_META_TABLE)
                .expect("failed to open table");
            let committed = log_id::<AppTypeConfig>(1, 1, 100); // Beyond log index 4
            let serialized = bincode::serialize(&committed).expect("failed to serialize committed");
            table
                .insert("committed", serialized.as_slice())
                .expect("failed to insert committed");
        }
        write_txn.commit().expect("failed to commit");
    } // Database dropped here

    let result = validate_raft_storage(1, &db_path);
    assert!(result.is_err());

    match result.unwrap_err() {
        StorageValidationError::CommittedInconsistent { reason } => {
            assert!(reason.contains("100 > last log index 4"));
        }
        other => panic!("expected CommittedInconsistent, got {:?}", other),
    }
}

#[tokio::test]
async fn test_integration_with_redb_log_store() {
    let temp_dir = create_temp_dir();
    let log_path = create_db_path(&temp_dir, "log_store");

    // Create a log store and append entries
    let mut log_store = RedbLogStore::new(&log_path).expect("failed to create log store");

    // Append some entries
    let entries = vec![
        <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
            log_id::<AppTypeConfig>(1, 1, 0),
            AppRequest::Set {
                key: "key1".into(),
                value: "value1".into(),
            },
        ),
        <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
            log_id::<AppTypeConfig>(1, 1, 1),
            AppRequest::Set {
                key: "key2".into(),
                value: "value2".into(),
            },
        ),
    ];

    use openraft::storage::IOFlushed;
    let (tx, rx) = tokio::sync::oneshot::channel();
    let callback = IOFlushed::signal(tx);

    log_store
        .append(entries, callback)
        .await
        .expect("failed to append entries");

    // Wait for IO to complete
    rx.await
        .expect("failed to receive callback")
        .expect("IO failed");

    // Drop log store before validation
    let log_path = log_store.path().to_path_buf();
    drop(log_store);

    // Validate storage directly
    let result = validate_raft_storage(1, &log_path);
    assert!(result.is_ok());

    let report = result.unwrap();
    assert_eq!(report.last_log_index, Some(1));
}

#[tokio::test]
async fn test_integration_with_redb_state_machine() {
    let temp_dir = create_temp_dir();
    let sm_path = create_db_path(&temp_dir, "state_machine");

    // Create a state machine and apply entries
    let mut sm = RedbStateMachine::new(&sm_path).expect("failed to create state machine");

    let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
        log_id::<AppTypeConfig>(1, 1, 0),
        AppRequest::Set {
            key: "test_key".into(),
            value: "test_value".into(),
        },
    );

    let entries = Box::pin(stream::once(async move { Ok((entry, None)) }));
    sm.apply(entries).await.expect("failed to apply entry");

    // Drop state machine before validation
    let sm_path = sm.path().to_path_buf();
    drop(sm);

    // Validate storage directly
    let result = validate_raft_storage(1, &sm_path);
    assert!(result.is_ok());
}

#[test]
fn test_validation_performance() {
    let temp_dir = create_temp_dir();
    let db_path = create_db_path(&temp_dir, "performance");

    // Create a large log (1000 entries)
    {
        let _db = create_db_with_log_entries(&db_path, 1000);
    } // Database dropped here

    // Measure validation time
    let start = std::time::Instant::now();
    let result = validate_raft_storage(1, &db_path);
    let duration = start.elapsed();

    assert!(result.is_ok());
    assert!(
        duration.as_millis() < 100,
        "validation should complete in <100ms, took {}ms",
        duration.as_millis()
    );

    let report = result.unwrap();
    assert_eq!(report.last_log_index, Some(999));
}

#[test]
fn test_validation_with_large_log() {
    let temp_dir = create_temp_dir();
    let db_path = create_db_path(&temp_dir, "large");

    // Create a very large log (10000 entries)
    {
        let _db = create_db_with_log_entries(&db_path, 10000);
    } // Database dropped here

    let result = validate_raft_storage(1, &db_path);
    assert!(result.is_ok());

    let report = result.unwrap();
    assert_eq!(report.last_log_index, Some(9999));
    println!("Validation took: {:?}", report.validation_duration);
}

#[test]
fn test_validation_multiple_times() {
    let temp_dir = create_temp_dir();
    let db_path = create_db_path(&temp_dir, "multiple");
    {
        let _db = create_db_with_log_entries(&db_path, 10);
    } // Database dropped here

    // Run validation multiple times (should be idempotent)
    for i in 0..5 {
        let result = validate_raft_storage(1, &db_path);
        assert!(result.is_ok(), "validation #{} should pass", i + 1);
    }
}
