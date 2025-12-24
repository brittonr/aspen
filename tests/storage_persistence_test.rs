//! Persistence tests for RedbLogStore and SqliteStateMachine.
//!
//! Tests that data survives process restarts by closing and reopening storage.
//!
//! Target: Increase storage coverage by testing persistence edge cases.

use std::path::PathBuf;

use aspen::raft::storage::RedbLogStore;
use aspen::raft::storage_sqlite::SqliteStateMachine;
use aspen::raft::types::AppRequest;
use aspen::raft::types::AppTypeConfig;
use aspen::raft::types::NodeId;
use futures::stream;
use openraft::RaftLogReader;
use openraft::entry::RaftEntry;
use openraft::storage::IOFlushed;
use openraft::storage::RaftLogStorage;
use openraft::storage::RaftStateMachine;
use openraft::testing::log_id;
use tempfile::TempDir;

fn create_temp_dir() -> TempDir {
    TempDir::new().expect("failed to create temp directory")
}

fn create_redb_path(temp_dir: &TempDir, name: &str) -> PathBuf {
    temp_dir.path().join(format!("{}.redb", name))
}

fn create_sqlite_path(temp_dir: &TempDir, name: &str) -> PathBuf {
    temp_dir.path().join(format!("{}.db", name))
}

fn make_entry(
    term: u64,
    node: u64,
    index: u64,
    request: AppRequest,
) -> <AppTypeConfig as openraft::RaftTypeConfig>::Entry {
    <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
        log_id::<AppTypeConfig>(term, NodeId::from(node), index),
        request,
    )
}

// ============================================================================
// RedbLogStore Persistence Tests
// ============================================================================

#[tokio::test]
async fn test_redb_log_entries_persist_across_close_reopen() {
    let temp_dir = create_temp_dir();
    let path = create_redb_path(&temp_dir, "log_persist");

    // Write entries in first session
    {
        let mut store = RedbLogStore::new(&path).expect("Should create store");

        let entries: Vec<_> = (1..=10u64)
            .map(|i| {
                make_entry(
                    1,
                    1,
                    i,
                    AppRequest::Set {
                        key: format!("key{}", i),
                        value: format!("value{}", i),
                    },
                )
            })
            .collect();

        store.append(entries, IOFlushed::noop()).await.expect("Should append");
    }

    // Reopen and verify
    {
        let mut store = RedbLogStore::new(&path).expect("Should reopen store");

        let entries = store.try_get_log_entries(1u64..).await.expect("Should read");
        assert_eq!(entries.len(), 10, "All 10 entries should persist");

        for (i, entry) in entries.iter().enumerate() {
            assert_eq!(entry.log_id().index(), i as u64 + 1);
        }
    }
}

#[tokio::test]
async fn test_redb_vote_persists_across_close_reopen() {
    let temp_dir = create_temp_dir();
    let path = create_redb_path(&temp_dir, "vote_persist");

    // Save vote in first session
    {
        let mut store = RedbLogStore::new(&path).expect("Should create store");
        let vote = openraft::vote::Vote::new(5, NodeId::from(3));
        store.save_vote(&vote).await.expect("Should save vote");
    }

    // Reopen and verify
    {
        let mut store = RedbLogStore::new(&path).expect("Should reopen store");
        let vote = store.read_vote().await.expect("Should read vote");
        assert!(vote.is_some(), "Vote should persist");
    }
}

#[tokio::test]
async fn test_redb_committed_persists_across_close_reopen() {
    let temp_dir = create_temp_dir();
    let path = create_redb_path(&temp_dir, "committed_persist");

    // Save committed in first session
    {
        let mut store = RedbLogStore::new(&path).expect("Should create store");
        let log_id = log_id::<AppTypeConfig>(1, NodeId::from(1), 42);
        store.save_committed(Some(log_id)).await.expect("Should save committed");
    }

    // Reopen and verify
    {
        let mut store = RedbLogStore::new(&path).expect("Should reopen store");
        let committed = store.read_committed().await.expect("Should read committed");
        let committed = committed.expect("Should have committed");
        assert_eq!(committed.index(), 42);
    }
}

#[tokio::test]
async fn test_redb_purge_persists_across_close_reopen() {
    let temp_dir = create_temp_dir();
    let path = create_redb_path(&temp_dir, "purge_persist");

    // Write entries and purge in first session
    {
        let mut store = RedbLogStore::new(&path).expect("Should create store");

        // Append entries 1-10
        let entries: Vec<_> = (1..=10u64)
            .map(|i| {
                make_entry(
                    1,
                    1,
                    i,
                    AppRequest::Set {
                        key: format!("k{}", i),
                        value: "v".to_string(),
                    },
                )
            })
            .collect();

        store.append(entries, IOFlushed::noop()).await.expect("Should append");

        // Purge entries up to index 5
        store.purge(log_id::<AppTypeConfig>(1, NodeId::from(1), 5)).await.expect("Should purge");
    }

    // Reopen and verify
    {
        let mut store = RedbLogStore::new(&path).expect("Should reopen store");

        let state = store.get_log_state().await.expect("Should get state");
        let purged = state.last_purged_log_id.expect("Should have purged id");
        assert_eq!(purged.index(), 5, "Purge point should persist");

        // Entries 6-10 should still exist
        let entries = store.try_get_log_entries(6u64..).await.expect("Should read");
        assert_eq!(entries.len(), 5, "Entries after purge should persist");
    }
}

#[tokio::test]
async fn test_redb_truncate_then_append_persists() {
    let temp_dir = create_temp_dir();
    let path = create_redb_path(&temp_dir, "truncate_append");

    // Write, truncate, and append more in first session
    {
        let mut store = RedbLogStore::new(&path).expect("Should create store");

        // Append entries 1-10
        let entries: Vec<_> = (1..=10u64)
            .map(|i| {
                make_entry(
                    1,
                    1,
                    i,
                    AppRequest::Set {
                        key: format!("k{}", i),
                        value: "original".to_string(),
                    },
                )
            })
            .collect();

        store.append(entries, IOFlushed::noop()).await.expect("Should append");

        // Truncate from index 6
        store.truncate(log_id::<AppTypeConfig>(1, NodeId::from(1), 6)).await.expect("Should truncate");

        // Append new entries 6-8 with different values
        let new_entries: Vec<_> = (6..=8u64)
            .map(|i| {
                make_entry(
                    2, // Different term
                    1,
                    i,
                    AppRequest::Set {
                        key: format!("k{}", i),
                        value: "replacement".to_string(),
                    },
                )
            })
            .collect();

        store.append(new_entries, IOFlushed::noop()).await.expect("Should append new");
    }

    // Reopen and verify
    {
        let mut store = RedbLogStore::new(&path).expect("Should reopen store");

        let state = store.get_log_state().await.expect("Should get state");
        let last_id = state.last_log_id.expect("Should have last log id");
        assert_eq!(last_id.index(), 8, "Should have entries 1-5 + 6-8");

        // Should have 8 entries total
        let entries = store.try_get_log_entries(1u64..).await.expect("Should read");
        assert_eq!(entries.len(), 8);
    }
}

#[tokio::test]
async fn test_redb_multiple_sessions_incremental_append() {
    let temp_dir = create_temp_dir();
    let path = create_redb_path(&temp_dir, "incremental");

    // Session 1: append entries 1-3
    {
        let mut store = RedbLogStore::new(&path).expect("Should create store");
        let entries: Vec<_> = (1..=3u64)
            .map(|i| {
                make_entry(
                    1,
                    1,
                    i,
                    AppRequest::Set {
                        key: format!("k{}", i),
                        value: "s1".to_string(),
                    },
                )
            })
            .collect();
        store.append(entries, IOFlushed::noop()).await.expect("Should append");
    }

    // Session 2: append entries 4-6
    {
        let mut store = RedbLogStore::new(&path).expect("Should reopen store");

        // Verify existing
        let existing = store.try_get_log_entries(1u64..).await.expect("Should read");
        assert_eq!(existing.len(), 3);

        let entries: Vec<_> = (4..=6u64)
            .map(|i| {
                make_entry(
                    1,
                    1,
                    i,
                    AppRequest::Set {
                        key: format!("k{}", i),
                        value: "s2".to_string(),
                    },
                )
            })
            .collect();
        store.append(entries, IOFlushed::noop()).await.expect("Should append");
    }

    // Session 3: verify all entries
    {
        let mut store = RedbLogStore::new(&path).expect("Should reopen store");
        let entries = store.try_get_log_entries(1u64..).await.expect("Should read");
        assert_eq!(entries.len(), 6, "Should have entries from both sessions");
    }
}

// ============================================================================
// SqliteStateMachine Persistence Tests
// ============================================================================

#[tokio::test]
async fn test_sqlite_data_persists_across_close_reopen() {
    let temp_dir = create_temp_dir();
    let path = create_sqlite_path(&temp_dir, "data_persist");

    // Write data in first session
    {
        let mut sm = SqliteStateMachine::new(&path).expect("Should create");

        let entry = make_entry(
            1,
            1,
            1,
            AppRequest::Set {
                key: "persist_key".to_string(),
                value: "persist_value".to_string(),
            },
        );

        let entries = Box::pin(stream::once(async move { Ok((entry, None)) }));
        sm.apply(entries).await.expect("Should apply");
    }

    // Reopen and verify
    {
        let sm = SqliteStateMachine::new(&path).expect("Should reopen");

        let value = sm.get("persist_key").await.expect("Should get");
        assert_eq!(value, Some("persist_value".to_string()));
    }
}

#[tokio::test]
async fn test_sqlite_multiple_operations_persist() {
    let temp_dir = create_temp_dir();
    let path = create_sqlite_path(&temp_dir, "multi_ops");

    // Apply multiple operations
    {
        let mut sm = SqliteStateMachine::new(&path).expect("Should create");

        // Set multiple keys
        for i in 0..10 {
            let entry = make_entry(
                1,
                1,
                i,
                AppRequest::Set {
                    key: format!("key{}", i),
                    value: format!("value{}", i),
                },
            );
            let entries = Box::pin(stream::once(async move { Ok((entry, None)) }));
            sm.apply(entries).await.expect("Should apply");
        }

        // Delete some keys
        let delete_entry = make_entry(
            1,
            1,
            10,
            AppRequest::DeleteMulti {
                keys: vec!["key2".to_string(), "key4".to_string()],
            },
        );
        let entries = Box::pin(stream::once(async move { Ok((delete_entry, None)) }));
        sm.apply(entries).await.expect("Should apply delete");
    }

    // Reopen and verify
    {
        let sm = SqliteStateMachine::new(&path).expect("Should reopen");

        // Existing keys should be present
        assert_eq!(sm.get("key0").await.unwrap(), Some("value0".to_string()));
        assert_eq!(sm.get("key1").await.unwrap(), Some("value1".to_string()));
        assert_eq!(sm.get("key3").await.unwrap(), Some("value3".to_string()));

        // Deleted keys should be absent
        assert_eq!(sm.get("key2").await.unwrap(), None);
        assert_eq!(sm.get("key4").await.unwrap(), None);
    }
}

#[tokio::test]
async fn test_sqlite_setmulti_persists() {
    let temp_dir = create_temp_dir();
    let path = create_sqlite_path(&temp_dir, "setmulti");

    // Apply SetMulti
    {
        let mut sm = SqliteStateMachine::new(&path).expect("Should create");

        let entry = make_entry(
            1,
            1,
            1,
            AppRequest::SetMulti {
                pairs: vec![
                    ("batch_a".to_string(), "val_a".to_string()),
                    ("batch_b".to_string(), "val_b".to_string()),
                    ("batch_c".to_string(), "val_c".to_string()),
                ],
            },
        );
        let entries = Box::pin(stream::once(async move { Ok((entry, None)) }));
        sm.apply(entries).await.expect("Should apply");
    }

    // Reopen and verify all keys exist
    {
        let sm = SqliteStateMachine::new(&path).expect("Should reopen");

        assert_eq!(sm.get("batch_a").await.unwrap(), Some("val_a".to_string()));
        assert_eq!(sm.get("batch_b").await.unwrap(), Some("val_b".to_string()));
        assert_eq!(sm.get("batch_c").await.unwrap(), Some("val_c".to_string()));
    }
}

#[tokio::test]
async fn test_sqlite_scan_after_reopen() {
    let temp_dir = create_temp_dir();
    let path = create_sqlite_path(&temp_dir, "scan");

    // Create data with common prefix
    {
        let mut sm = SqliteStateMachine::new(&path).expect("Should create");

        for i in 0..5 {
            let entry = make_entry(
                1,
                1,
                i,
                AppRequest::Set {
                    key: format!("prefix_{}", i),
                    value: format!("value_{}", i),
                },
            );
            let entries = Box::pin(stream::once(async move { Ok((entry, None)) }));
            sm.apply(entries).await.expect("Should apply");
        }

        // Add some keys without the prefix
        let entry = make_entry(
            1,
            1,
            5,
            AppRequest::Set {
                key: "other_key".to_string(),
                value: "other_value".to_string(),
            },
        );
        let entries = Box::pin(stream::once(async move { Ok((entry, None)) }));
        sm.apply(entries).await.expect("Should apply");
    }

    // Reopen and scan
    {
        let sm = SqliteStateMachine::new(&path).expect("Should reopen");

        let results = sm.scan("prefix_", None, Some(100)).await.expect("Should scan");
        assert_eq!(results.len(), 5, "Should find 5 keys with prefix");

        for (key, _) in &results {
            assert!(key.starts_with("prefix_"));
        }
    }
}

#[tokio::test]
async fn test_sqlite_overwrite_value_persists() {
    let temp_dir = create_temp_dir();
    let path = create_sqlite_path(&temp_dir, "overwrite");

    // Set initial value
    {
        let mut sm = SqliteStateMachine::new(&path).expect("Should create");

        let entry = make_entry(
            1,
            1,
            1,
            AppRequest::Set {
                key: "overwrite_key".to_string(),
                value: "initial".to_string(),
            },
        );
        let entries = Box::pin(stream::once(async move { Ok((entry, None)) }));
        sm.apply(entries).await.expect("Should apply");
    }

    // Overwrite value
    {
        let mut sm = SqliteStateMachine::new(&path).expect("Should reopen");

        // Verify initial value
        assert_eq!(sm.get("overwrite_key").await.unwrap(), Some("initial".to_string()));

        let entry = make_entry(
            1,
            1,
            2,
            AppRequest::Set {
                key: "overwrite_key".to_string(),
                value: "updated".to_string(),
            },
        );
        let entries = Box::pin(stream::once(async move { Ok((entry, None)) }));
        sm.apply(entries).await.expect("Should apply");
    }

    // Verify updated value persists
    {
        let sm = SqliteStateMachine::new(&path).expect("Should reopen again");
        assert_eq!(sm.get("overwrite_key").await.unwrap(), Some("updated".to_string()));
    }
}

#[tokio::test]
async fn test_sqlite_empty_database_get_returns_none() {
    let temp_dir = create_temp_dir();
    let path = create_sqlite_path(&temp_dir, "empty_get");

    let sm = SqliteStateMachine::new(&path).expect("Should create");
    let value = sm.get("nonexistent").await.expect("Should get without error");
    assert!(value.is_none());
}

#[tokio::test]
async fn test_sqlite_validation_after_reopen() {
    let temp_dir = create_temp_dir();
    let path = create_sqlite_path(&temp_dir, "validate");

    // Create and populate
    {
        let mut sm = SqliteStateMachine::new(&path).expect("Should create");

        for i in 0..5 {
            let entry = make_entry(
                1,
                1,
                i,
                AppRequest::Set {
                    key: format!("k{}", i),
                    value: format!("v{}", i),
                },
            );
            let entries = Box::pin(stream::once(async move { Ok((entry, None)) }));
            sm.apply(entries).await.expect("Should apply");
        }
    }

    // Reopen and validate
    {
        let sm = SqliteStateMachine::new(&path).expect("Should reopen");
        let result = sm.validate(1);
        assert!(result.is_ok(), "Validation should pass");
    }
}
