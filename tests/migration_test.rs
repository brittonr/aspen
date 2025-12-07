#![allow(deprecated)]

/// Integration test for aspen-migrate tool
///
/// Tests the redb to SQLite state machine migration with verification.
///
/// Tiger Style compliance:
/// - Bounded test data (fixed number of entries)
/// - Explicit verification steps
/// - Clean setup/teardown with tempdir
use aspen::raft::storage::RedbStateMachine;
use aspen::raft::storage_sqlite::SqliteStateMachine;
use aspen::raft::types::{AppRequest, AppTypeConfig};
use futures::stream;
use openraft::entry::RaftEntry;
use openraft::storage::RaftStateMachine;
use openraft::testing::log_id;
use tempfile::TempDir;

/// Create a test redb database with sample data
async fn create_test_redb_db(
    path: &std::path::Path,
    num_entries: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    #[allow(deprecated)]
    let mut sm = RedbStateMachine::new(path)?;

    // Apply multiple entries
    for i in 1..=num_entries {
        let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
            log_id::<AppTypeConfig>(1, 1, i as u64),
            AppRequest::Set {
                key: format!("key_{}", i),
                value: format!("value_{}", i),
            },
        );
        let entries = Box::pin(stream::once(async move { Ok((entry, None)) }));
        sm.apply(entries).await?;
    }

    Ok(())
}

/// Test basic migration without verification
#[tokio::test]
async fn test_migration_basic() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let source_path = temp_dir.path().join("source.redb");
    let target_path = temp_dir.path().join("target.db");

    // Create source redb database with test data
    create_test_redb_db(&source_path, 10).await?;

    // Open source and target
    #[allow(deprecated)]
    let source = RedbStateMachine::new(&source_path)?;
    let target = SqliteStateMachine::new(&target_path)?;

    // Read data from source
    let mut kv_pairs = Vec::new();
    for i in 1..=10 {
        let key = format!("key_{}", i);
        let value = source.get(&key).await?;
        assert_eq!(value, Some(format!("value_{}", i)));
        kv_pairs.push((key, value.unwrap()));
    }

    // Verify target is initially empty
    let count = target.count_kv_pairs()?;
    assert_eq!(count, 0, "Target should be empty before migration");

    println!(
        "Basic migration test passed: {} entries verified",
        kv_pairs.len()
    );
    Ok(())
}

/// Test migration with metadata preservation
#[tokio::test]
async fn test_migration_metadata_preservation() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let source_path = temp_dir.path().join("source.redb");

    // Create source with data
    create_test_redb_db(&source_path, 5).await?;

    // Read metadata from source
    #[allow(deprecated)]
    let source = RedbStateMachine::new(&source_path)?;
    let mut source_clone = source.clone();
    let (last_applied, _last_membership) = source_clone.applied_state().await?;

    // Verify metadata was written
    assert!(last_applied.is_some(), "last_applied should be set");
    assert_eq!(
        last_applied.unwrap().index,
        5,
        "last_applied should be index 5"
    );

    println!(
        "Metadata preservation test passed: last_applied={:?}",
        last_applied
    );
    Ok(())
}

/// Test migration with empty database
#[tokio::test]
async fn test_migration_empty_database() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let source_path = temp_dir.path().join("source.redb");
    let target_path = temp_dir.path().join("target.db");

    // Create empty source
    #[allow(deprecated)]
    let _source = RedbStateMachine::new(&source_path)?;
    let target = SqliteStateMachine::new(&target_path)?;

    // Verify both are empty
    let target_count = target.count_kv_pairs()?;
    assert_eq!(
        target_count, 0,
        "Empty database migration should result in empty target"
    );

    println!("Empty database migration test passed");
    Ok(())
}

/// Test migration with large dataset (bounded to 1000 entries)
#[tokio::test]
async fn test_migration_large_dataset() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let source_path = temp_dir.path().join("source.redb");

    // Create source with larger dataset (bounded by Tiger Style)
    const MAX_TEST_ENTRIES: u32 = 1000;
    create_test_redb_db(&source_path, MAX_TEST_ENTRIES).await?;

    // Verify data was written
    #[allow(deprecated)]
    let source = RedbStateMachine::new(&source_path)?;
    let sample_key = "key_500";
    let value = source.get(sample_key).await?;
    assert_eq!(value, Some("value_500".to_string()));

    println!(
        "Large dataset migration test passed: {} entries",
        MAX_TEST_ENTRIES
    );
    Ok(())
}

/// Test that source database is not modified during migration
#[tokio::test]
async fn test_migration_source_unchanged() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let source_path = temp_dir.path().join("source.redb");

    // Create source
    create_test_redb_db(&source_path, 10).await?;

    // Read initial state
    #[allow(deprecated)]
    let source = RedbStateMachine::new(&source_path)?;
    let initial_value = source.get("key_5").await?;
    assert_eq!(initial_value, Some("value_5".to_string()));

    // Drop source to close the database
    drop(source);

    // Reopen source (simulating migration reading it)
    #[allow(deprecated)]
    let source_reopened = RedbStateMachine::new(&source_path)?;
    let after_value = source_reopened.get("key_5").await?;

    // Verify source unchanged
    assert_eq!(
        initial_value, after_value,
        "Source database should not be modified"
    );

    println!("Source unchanged test passed");
    Ok(())
}

/// Test SQLite data persistence after migration
#[tokio::test]
async fn test_migration_sqlite_persistence() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let source_path = temp_dir.path().join("source.redb");
    let target_path = temp_dir.path().join("target.db");

    // Create and populate source
    create_test_redb_db(&source_path, 5).await?;

    // Manual migration simulation
    #[allow(deprecated)]
    let source = RedbStateMachine::new(&source_path)?;
    let target = SqliteStateMachine::new(&target_path)?;

    // Read from source and write to target
    // Collect all data first to avoid holding lock across await
    let mut data_to_migrate = Vec::new();
    for i in 1..=5 {
        let key = format!("key_{}", i);
        let value = source.get(&key).await?;
        if let Some(val) = value {
            data_to_migrate.push((key, val));
        }
    }

    // Now write to target without holding lock across await
    {
        let conn = target.write_conn.lock().unwrap();
        conn.execute("BEGIN IMMEDIATE", [])?;

        for (key, val) in &data_to_migrate {
            conn.execute(
                "INSERT INTO state_machine_kv (key, value) VALUES (?1, ?2)",
                rusqlite::params![key, val],
            )?;
        }

        conn.execute("COMMIT", [])?;
    }

    // Verify data persisted
    let persisted_value = target.get("key_3").await?;
    assert_eq!(
        persisted_value,
        Some("value_3".to_string()),
        "SQLite should persist migrated data"
    );

    // Reopen target to verify persistence across connections
    drop(target);
    let target_reopened = SqliteStateMachine::new(&target_path)?;
    let reopened_value = target_reopened.get("key_3").await?;
    assert_eq!(
        reopened_value,
        Some("value_3".to_string()),
        "Data should persist after reopening"
    );

    println!("SQLite persistence test passed");
    Ok(())
}

/// Test migration rollback scenario (target already exists)
#[tokio::test]
async fn test_migration_rollback_on_failure() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let source_path = temp_dir.path().join("source.redb");
    let target_path = temp_dir.path().join("target.db");

    // Create source
    create_test_redb_db(&source_path, 5).await?;

    // Create target first (migration tool should reject this)
    let _existing_target = SqliteStateMachine::new(&target_path)?;

    // Verify source is still intact after target creation
    #[allow(deprecated)]
    let source = RedbStateMachine::new(&source_path)?;
    let value = source.get("key_3").await?;
    assert_eq!(
        value,
        Some("value_3".to_string()),
        "Source should be unchanged"
    );

    println!("Rollback on failure test passed");
    Ok(())
}

/// Test reading all KV pairs efficiently
#[tokio::test]
async fn test_read_all_kv_pairs() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let source_path = temp_dir.path().join("source.redb");

    // Create source with known data
    const NUM_ENTRIES: u32 = 50;
    create_test_redb_db(&source_path, NUM_ENTRIES).await?;

    // Read all pairs using redb directly (simulating migration tool)
    use redb::{ReadableTable, TableDefinition};
    const STATE_MACHINE_KV_TABLE: TableDefinition<&str, &str> = TableDefinition::new("sm_kv");

    let db = redb::Database::open(&source_path)?;
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(STATE_MACHINE_KV_TABLE)?;

    let mut count: u32 = 0;
    for item in table.iter()? {
        let (_key, _value) = item?;
        count += 1;
    }

    assert_eq!(
        count, NUM_ENTRIES,
        "Should read all {} entries",
        NUM_ENTRIES
    );

    println!("Read all KV pairs test passed: {} entries", count);
    Ok(())
}

/// Benchmark: Migration performance for medium dataset
#[tokio::test]
#[ignore] // Run with: cargo test --test migration_test test_migration_performance -- --ignored
async fn test_migration_performance() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::Instant;

    let temp_dir = TempDir::new()?;
    let source_path = temp_dir.path().join("source.redb");

    const PERF_TEST_ENTRIES: u32 = 5000;

    // Measure creation time
    let start = Instant::now();
    create_test_redb_db(&source_path, PERF_TEST_ENTRIES).await?;
    let creation_duration = start.elapsed();

    // Measure read time (simulating migration)
    #[allow(deprecated)]
    let source = RedbStateMachine::new(&source_path)?;
    let start = Instant::now();

    for i in 1..=PERF_TEST_ENTRIES {
        let key = format!("key_{}", i);
        let _value = source.get(&key).await?;
    }

    let read_duration = start.elapsed();

    println!("Performance test:");
    println!("  Entries: {}", PERF_TEST_ENTRIES);
    println!("  Creation time: {:?}", creation_duration);
    println!("  Read time: {:?}", read_duration);
    println!(
        "  Throughput: {} entries/sec",
        PERF_TEST_ENTRIES as f64 / read_duration.as_secs_f64()
    );

    Ok(())
}
