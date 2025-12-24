/// Redb log storage corruption scenario tests (non-madsim).
///
/// This test suite validates storage corruption detection and fail-fast behavior:
/// - Log index gap detection after corruption
/// - Vote metadata corruption detection
/// - Validation module catches storage inconsistencies
/// - Clear operator error messages for recovery
///
/// These tests use regular tokio runtime (not madsim) since we're testing
/// storage layer validation, not distributed behavior.
use std::path::Path;
/// Redb log storage corruption scenario tests (non-madsim).
///
/// This test suite validates storage corruption detection and fail-fast behavior:
/// - Log index gap detection after corruption
/// - Vote metadata corruption detection
/// - Validation module catches storage inconsistencies
/// - Clear operator error messages for recovery
///
/// These tests use regular tokio runtime (not madsim) since we're testing
/// storage layer validation, not distributed behavior.
use std::path::PathBuf;

use aspen::raft::storage::RedbLogStore;
use aspen::raft::storage_validation::StorageValidationError;
use aspen::raft::storage_validation::validate_raft_storage;
use aspen::raft::types::AppTypeConfig;
use aspen::raft::types::NodeId;
use openraft::RaftLogReader;
use openraft::entry::RaftEntry;
use openraft::storage::IOFlushed;
use openraft::storage::RaftLogStorage;
use openraft::testing::log_id;
use redb::Database;
use redb::TableDefinition;
use tempfile::TempDir;

/// Redb table definitions (must match storage.rs)
const RAFT_LOG_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("raft_log");

/// Helper: Create a temporary directory for tests
fn create_temp_dir() -> TempDir {
    TempDir::new().expect("failed to create temp directory")
}

/// Helper: Create a redb log path
fn create_log_path(temp_dir: &TempDir, name: &str) -> PathBuf {
    temp_dir.path().join(format!("{}-log.redb", name))
}

/// Corrupt redb database by deleting log entries to create index gap.
///
/// This simulates partial write corruption or disk failure scenarios where
/// log entries are lost but the database remains openable.
fn corrupt_redb_delete_entries(
    log_path: &Path,
    start_index: u64,
    end_index: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::open(log_path)?;
    let write_txn = db.begin_write()?;

    {
        let mut table = write_txn.open_table(RAFT_LOG_TABLE)?;

        // Delete entries in range [start_index, end_index)
        for index in start_index..end_index {
            table.remove(index)?;
        }
    }

    write_txn.commit()?;
    Ok(())
}

// ====================================================================================
// Test 1: Redb Log Index Gap Detection
// ====================================================================================

#[tokio::test]
async fn test_redb_log_gap_detection() {
    let temp_dir = create_temp_dir();
    let log_path = create_log_path(&temp_dir, "gap_detection");

    // Phase 1: Create log store and write 50 entries
    {
        let mut log_store = RedbLogStore::new(&log_path).expect("failed to create log store");

        // Write 50 log entries
        use aspen::raft::types::NodeId;
        for i in 0..50_u64 {
            let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                log_id::<AppTypeConfig>(1, NodeId::from(1), i),
                aspen::raft::types::AppRequest::Set {
                    key: format!("key-{}", i),
                    value: format!("value-{}", i),
                },
            );

            log_store.append([entry], IOFlushed::noop()).await.expect("failed to append entry");
        }

        // Verify 50 entries written
        let log_state = log_store.get_log_state().await.expect("failed to get log state");
        assert_eq!(log_state.last_log_id.unwrap().index, 49, "last log index should be 49");

        // Drop log store to close database
        drop(log_store);
    }

    // Phase 2: Corrupt database by deleting entries 20-25
    corrupt_redb_delete_entries(&log_path, 20, 25).expect("failed to corrupt redb database");

    // Phase 3: Run validation - should detect log gap
    let validation_result = validate_raft_storage(1, &log_path);

    match validation_result {
        Err(StorageValidationError::LogNotMonotonic { prev, current }) => {
            // The gap should be between index 19 and 25 (we deleted 20-24)
            assert!(prev < current, "prev index {} should be less than current index {}", prev, current);
            assert!(
                current - prev > 1,
                "gap should be detected (indices not consecutive): prev={}, current={}",
                prev,
                current
            );
            println!("SUCCESS: Detected log gap between index {} and {}", prev, current);
        }
        Err(e) => {
            panic!("expected LogNotMonotonic error, got: {:?}", e);
        }
        Ok(report) => {
            panic!("validation should have failed with LogNotMonotonic error, got success: {:?}", report);
        }
    }
}

// ====================================================================================
// Test 2: Redb Vote Metadata Corruption
// ====================================================================================

#[tokio::test]
async fn test_redb_vote_corruption_detection() {
    let temp_dir = create_temp_dir();
    let log_path = create_log_path(&temp_dir, "vote_corruption");

    // Phase 1: Create log store and persist vote
    {
        let mut log_store = RedbLogStore::new(&log_path).expect("failed to create log store");

        // Save a vote
        let vote = openraft::Vote::new(5, NodeId::from(1));
        log_store.save_vote(&vote).await.expect("failed to save vote");

        // Verify vote saved
        let read_vote = log_store.read_vote().await.expect("failed to read vote");
        assert_eq!(read_vote, Some(vote), "vote should be persisted");

        // Drop log store to close database
        drop(log_store);
    }

    // Phase 2: Corrupt vote metadata in database
    {
        let db = Database::open(&log_path).expect("failed to open database");
        let write_txn = db.begin_write().expect("failed to begin write");

        {
            let mut meta_table = write_txn
                .open_table(TableDefinition::<&str, &[u8]>::new("raft_meta"))
                .expect("failed to open meta table");

            // Write random garbage to vote key
            let random_bytes: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
            meta_table.insert("vote", random_bytes.as_slice()).expect("failed to insert corrupted vote");
        }

        write_txn.commit().expect("failed to commit");
    }

    // Phase 3: Run validation - should detect vote corruption
    let validation_result = validate_raft_storage(1, &log_path);

    match validation_result {
        Err(StorageValidationError::DeserializeFailed { .. }) => {
            println!("SUCCESS: Detected vote metadata corruption");
        }
        Err(StorageValidationError::VoteInconsistent { reason }) => {
            println!("SUCCESS: Detected vote inconsistency: {}", reason);
        }
        Err(e) => {
            panic!("expected Deserialize or VoteInconsistent error, got: {:?}", e);
        }
        Ok(report) => {
            panic!("validation should have failed with corruption error, got success: {:?}", report);
        }
    }
}

// ====================================================================================
// Test 3: Redb Database Recovery After Corruption
// ====================================================================================

#[tokio::test]
async fn test_redb_prevents_restart_with_corrupted_log() {
    let temp_dir = create_temp_dir();
    let log_path = create_log_path(&temp_dir, "prevent_restart");

    // Phase 1: Create log store and write entries
    {
        let mut log_store = RedbLogStore::new(&log_path).expect("failed to create log store");

        // Write 30 log entries
        for i in 0..30_u64 {
            let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                log_id::<AppTypeConfig>(1, NodeId(1), i),
                aspen::raft::types::AppRequest::Set {
                    key: format!("key-{}", i),
                    value: format!("value-{}", i),
                },
            );

            log_store.append([entry], IOFlushed::noop()).await.expect("failed to append entry");
        }

        drop(log_store);
    }

    // Phase 2: Corrupt database (delete middle entries)
    corrupt_redb_delete_entries(&log_path, 10, 20).expect("failed to corrupt database");

    // Phase 3: Validation should catch corruption before allowing restart
    let validation_result = validate_raft_storage(1, &log_path);

    assert!(validation_result.is_err(), "validation should fail on corrupted database");

    match validation_result {
        Err(StorageValidationError::LogNotMonotonic { prev, current }) => {
            println!("SUCCESS: Prevented restart with corrupted log (gap: {} -> {})", prev, current);
        }
        Err(e) => {
            println!("SUCCESS: Prevented restart with error: {:?}", e);
        }
        Ok(_) => {
            panic!("validation should have detected corruption");
        }
    }
}
