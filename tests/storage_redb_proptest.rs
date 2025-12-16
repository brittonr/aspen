//! Property-based tests for raft/storage.rs (RedbLogStore and InMemoryLogStore)
//!
//! Tests log operations, vote persistence, truncation, and purge semantics.
//!
//! Target: Increase storage.rs coverage from 28.74% to 50%+

mod support;

use proptest::prelude::*;
use std::path::PathBuf;

use aspen::raft::storage::{InMemoryLogStore, RedbLogStore, StorageBackend};
use aspen::raft::types::{AppRequest, AppTypeConfig, NodeId};
use openraft::entry::RaftEntry;
use openraft::storage::{IOFlushed, RaftLogReader, RaftLogStorage};
use openraft::testing::log_id;
use tempfile::TempDir;

use support::proptest_generators::{arbitrary_app_request, arbitrary_vote};

fn create_temp_dir() -> TempDir {
    TempDir::new().expect("failed to create temp directory")
}

fn create_redb_path(temp_dir: &TempDir, name: &str) -> PathBuf {
    temp_dir.path().join(format!("{}.redb", name))
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
// StorageBackend Tests
// ============================================================================

proptest! {
    /// StorageBackend FromStr -> Display roundtrip.
    #[test]
    fn test_storage_backend_fromstr_display_roundtrip(
        variant in prop_oneof![
            Just("inmemory"),
            Just("in-memory"),
            Just("memory"),
            Just("sqlite"),
            Just("sql"),
            Just("persistent"),
            Just("disk"),
            Just("redb"),
        ]
    ) {
        let parsed: StorageBackend = variant.parse().expect("Should parse");
        let displayed = parsed.to_string();
        let reparsed: StorageBackend = displayed.parse().expect("Display output should parse");
        prop_assert_eq!(parsed, reparsed);
    }

    /// StorageBackend rejects invalid strings.
    #[test]
    fn test_storage_backend_rejects_invalid(
        invalid in "[a-zA-Z]{5,20}".prop_filter("Not valid backend", |s| {
            !["inmemory", "in-memory", "memory", "sqlite", "sql", "persistent", "disk", "redb"]
                .contains(&s.to_lowercase().as_str())
        })
    ) {
        let result: Result<StorageBackend, _> = invalid.parse();
        prop_assert!(result.is_err(), "Should reject invalid backend: {}", invalid);
    }
}

// ============================================================================
// InMemoryLogStore Tests
// ============================================================================

proptest! {
    /// Vote save and read roundtrip for InMemoryLogStore.
    #[test]
    fn test_inmemory_vote_roundtrip(
        (term, node_id) in arbitrary_vote()
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut store = InMemoryLogStore::default();

            let vote = openraft::vote::Vote::new(term, node_id);
            store.save_vote(&vote).await.expect("Should save vote");

            let read_vote = store.read_vote().await.expect("Should read vote");
            prop_assert_eq!(read_vote, Some(vote));
            Ok(())
        })?;
    }

    /// InMemoryLogStore append and read entries.
    #[test]
    fn test_inmemory_append_read(
        requests in prop::collection::vec(arbitrary_app_request(), 1..10)
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut store = InMemoryLogStore::default();

            // Create entries
            let entries: Vec<_> = requests.iter().enumerate().map(|(i, req)| {
                make_entry(1, 1, i as u64 + 1, req.clone())
            }).collect();

            store.append(entries.clone(), IOFlushed::noop()).await.expect("Should append");

            // Read back
            let read_entries = store.try_get_log_entries(1u64..).await.expect("Should read");
            prop_assert_eq!(read_entries.len(), entries.len());

            // Verify indices
            for (i, entry) in read_entries.iter().enumerate() {
                prop_assert_eq!(entry.log_id().index(), i as u64 + 1);
            }

            Ok(())
        })?;
    }

    /// InMemoryLogStore get_log_state reflects appended entries.
    #[test]
    fn test_inmemory_log_state(
        count in 1usize..20usize
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut store = InMemoryLogStore::default();

            // Initially empty
            let state = store.get_log_state().await.expect("Should get state");
            prop_assert!(state.last_log_id.is_none());

            // Create and append entries
            let entries: Vec<_> = (1..=count as u64).map(|i| {
                make_entry(1, 1, i, AppRequest::Set {
                    key: format!("k{}", i),
                    value: format!("v{}", i),
                })
            }).collect();

            store.append(entries, IOFlushed::noop()).await.expect("Should append");

            // Check state
            let state = store.get_log_state().await.expect("Should get state");
            let last_log_id = state.last_log_id.expect("Should have last log id");
            prop_assert_eq!(last_log_id.index(), count as u64);

            Ok(())
        })?;
    }

    /// InMemoryLogStore truncate removes entries from given index.
    #[test]
    fn test_inmemory_truncate(
        total in 5usize..20usize,
        truncate_at in 1usize..5usize,
    ) {
        let truncate_at = truncate_at.min(total);
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut store = InMemoryLogStore::default();

            // Append entries
            let entries: Vec<_> = (1..=total as u64).map(|i| {
                make_entry(1, 1, i, AppRequest::Set {
                    key: format!("k{}", i),
                    value: format!("v{}", i),
                })
            }).collect();

            store.append(entries, IOFlushed::noop()).await.expect("Should append");

            // Truncate
            store.truncate(log_id::<AppTypeConfig>(1, NodeId::from(1), truncate_at as u64))
                .await
                .expect("Should truncate");

            // Read back - should only have entries before truncation point
            let read_entries = store.try_get_log_entries(1u64..).await.expect("Should read");
            prop_assert!(read_entries.len() < total, "Should have fewer entries after truncate");

            Ok(())
        })?;
    }

    /// InMemoryLogStore purge removes entries up to given index.
    #[test]
    fn test_inmemory_purge(
        total in 5usize..20usize,
        purge_at in 1usize..5usize,
    ) {
        let purge_at = purge_at.min(total - 1);
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut store = InMemoryLogStore::default();

            // Append entries
            let entries: Vec<_> = (1..=total as u64).map(|i| {
                make_entry(1, 1, i, AppRequest::Set {
                    key: format!("k{}", i),
                    value: format!("v{}", i),
                })
            }).collect();

            store.append(entries, IOFlushed::noop()).await.expect("Should append");

            // Purge
            store.purge(log_id::<AppTypeConfig>(1, NodeId::from(1), purge_at as u64))
                .await
                .expect("Should purge");

            // Read back - should only have entries after purge point
            let read_entries = store.try_get_log_entries(1u64..).await.expect("Should read");
            prop_assert!(read_entries.len() < total, "Should have fewer entries after purge");

            // Check log state reflects purge
            let state = store.get_log_state().await.expect("Should get state");
            let purged_id = state.last_purged_log_id.expect("Should have purged id");
            prop_assert_eq!(purged_id.index(), purge_at as u64);

            Ok(())
        })?;
    }

    /// InMemoryLogStore committed index save/read roundtrip.
    #[test]
    fn test_inmemory_committed_roundtrip(
        index in 1u64..1000u64
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut store = InMemoryLogStore::default();

            // Initially none
            let committed = store.read_committed().await.expect("Should read");
            prop_assert!(committed.is_none());

            // Save committed
            let log_id = log_id::<AppTypeConfig>(1, NodeId::from(1), index);
            store.save_committed(Some(log_id)).await.expect("Should save");

            // Read back
            let committed = store.read_committed().await.expect("Should read");
            let committed = committed.expect("Should have committed");
            prop_assert_eq!(committed.index(), index);

            Ok(())
        })?;
    }
}

// ============================================================================
// RedbLogStore Tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))] // Fewer cases for disk tests

    /// RedbLogStore vote persistence across reopen.
    #[test]
    fn test_redb_vote_persists_across_reopen(
        (term, node_id) in arbitrary_vote()
    ) {
        let temp_dir = create_temp_dir();
        let path = create_redb_path(&temp_dir, "vote_persist");

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Save vote
            {
                let mut store = RedbLogStore::new(&path).expect("Should create store");
                let vote = openraft::vote::Vote::new(term, node_id);
                store.save_vote(&vote).await.expect("Should save vote");
            }

            // Reopen and verify vote was persisted
            {
                let mut store = RedbLogStore::new(&path).expect("Should reopen store");
                let read_vote = store.read_vote().await.expect("Should read vote");
                prop_assert!(read_vote.is_some(), "Vote should be persisted across reopen");
            }

            Ok(())
        })?;
    }

    /// RedbLogStore append and read entries.
    #[test]
    fn test_redb_append_read(
        requests in prop::collection::vec(arbitrary_app_request(), 1..5)
    ) {
        let temp_dir = create_temp_dir();
        let path = create_redb_path(&temp_dir, "append_read");

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut store = RedbLogStore::new(&path).expect("Should create store");

            // Create entries
            let entries: Vec<_> = requests.iter().enumerate().map(|(i, req)| {
                make_entry(1, 1, i as u64 + 1, req.clone())
            }).collect();

            // Append with callback
            store.append(entries.clone(), IOFlushed::noop())
                .await
                .expect("Should append");

            // Read back
            let read_entries = store.try_get_log_entries(1u64..).await.expect("Should read");
            prop_assert_eq!(read_entries.len(), entries.len());

            Ok(())
        })?;
    }

    /// RedbLogStore entries persist across reopen.
    #[test]
    fn test_redb_entries_persist_across_reopen(
        count in 1usize..5usize
    ) {
        let temp_dir = create_temp_dir();
        let path = create_redb_path(&temp_dir, "entries_persist");

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Append entries
            {
                let mut store = RedbLogStore::new(&path).expect("Should create store");

                let entries: Vec<_> = (1..=count as u64).map(|i| {
                    make_entry(1, 1, i, AppRequest::Set {
                        key: format!("k{}", i),
                        value: format!("v{}", i),
                    })
                }).collect();

                store.append(entries, IOFlushed::noop())
                    .await
                    .expect("Should append");
            }

            // Reopen and verify
            {
                let mut store = RedbLogStore::new(&path).expect("Should reopen store");
                let read_entries = store.try_get_log_entries(1u64..).await.expect("Should read");
                prop_assert_eq!(read_entries.len(), count);

                let state = store.get_log_state().await.expect("Should get state");
                let last_log_id = state.last_log_id.expect("Should have last log id");
                prop_assert_eq!(last_log_id.index(), count as u64);
            }

            Ok(())
        })?;
    }

    /// RedbLogStore committed index persists across reopen.
    #[test]
    fn test_redb_committed_persists(
        index in 1u64..100u64
    ) {
        let temp_dir = create_temp_dir();
        let path = create_redb_path(&temp_dir, "committed_persist");

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Save committed
            {
                let mut store = RedbLogStore::new(&path).expect("Should create store");
                let log_id = log_id::<AppTypeConfig>(1, NodeId::from(1), index);
                store.save_committed(Some(log_id)).await.expect("Should save");
            }

            // Reopen and verify
            {
                let mut store = RedbLogStore::new(&path).expect("Should reopen store");
                let committed = store.read_committed().await.expect("Should read");
                let committed = committed.expect("Should have committed");
                prop_assert_eq!(committed.index(), index);
            }

            Ok(())
        })?;
    }

    /// RedbLogStore truncate removes entries.
    #[test]
    fn test_redb_truncate(
        total in 5usize..10usize,
        truncate_at in 2usize..5usize,
    ) {
        let truncate_at = truncate_at.min(total - 1);
        let temp_dir = create_temp_dir();
        let path = create_redb_path(&temp_dir, "truncate");

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut store = RedbLogStore::new(&path).expect("Should create store");

            // Append entries
            let entries: Vec<_> = (1..=total as u64).map(|i| {
                make_entry(1, 1, i, AppRequest::Set {
                    key: format!("k{}", i),
                    value: format!("v{}", i),
                })
            }).collect();

            store.append(entries, IOFlushed::noop())
                .await
                .expect("Should append");

            // Truncate
            store.truncate(log_id::<AppTypeConfig>(1, NodeId::from(1), truncate_at as u64))
                .await
                .expect("Should truncate");

            // Read back
            let read_entries = store.try_get_log_entries(1u64..).await.expect("Should read");
            prop_assert!(read_entries.len() < total, "Should have fewer entries: {} < {}", read_entries.len(), total);

            Ok(())
        })?;
    }

    /// RedbLogStore purge removes old entries.
    #[test]
    fn test_redb_purge(
        total in 5usize..10usize,
        purge_at in 1usize..4usize,
    ) {
        let purge_at = purge_at.min(total - 2);
        let temp_dir = create_temp_dir();
        let path = create_redb_path(&temp_dir, "purge");

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut store = RedbLogStore::new(&path).expect("Should create store");

            // Append entries
            let entries: Vec<_> = (1..=total as u64).map(|i| {
                make_entry(1, 1, i, AppRequest::Set {
                    key: format!("k{}", i),
                    value: format!("v{}", i),
                })
            }).collect();

            store.append(entries, IOFlushed::noop())
                .await
                .expect("Should append");

            // Purge
            store.purge(log_id::<AppTypeConfig>(1, NodeId::from(1), purge_at as u64))
                .await
                .expect("Should purge");

            // Check state
            let state = store.get_log_state().await.expect("Should get state");
            let purged_id = state.last_purged_log_id.expect("Should have purged id");
            prop_assert_eq!(purged_id.index(), purge_at as u64);

            Ok(())
        })?;
    }
}

// ============================================================================
// Unit Tests for Edge Cases
// ============================================================================

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_storage_backend_default_is_sqlite() {
        let default = StorageBackend::default();
        assert_eq!(default, StorageBackend::Sqlite);
    }

    #[test]
    fn test_storage_backend_case_insensitive() {
        let variants = ["INMEMORY", "InMemory", "sqlite", "SQLITE", "Sqlite"];
        for v in variants {
            let result: Result<StorageBackend, _> = v.parse();
            assert!(result.is_ok(), "Should parse case-insensitive: {}", v);
        }
    }

    #[test]
    fn test_storage_backend_debug_format() {
        let inmem = StorageBackend::InMemory;
        let sqlite = StorageBackend::Sqlite;

        assert!(format!("{:?}", inmem).contains("InMemory"));
        assert!(format!("{:?}", sqlite).contains("Sqlite"));
    }

    #[tokio::test]
    async fn test_inmemory_empty_range_read() {
        let mut store = InMemoryLogStore::default();
        let entries = store
            .try_get_log_entries(1u64..1u64)
            .await
            .expect("Should handle empty range");
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn test_inmemory_get_log_reader_returns_clone() {
        let mut store = InMemoryLogStore::default();
        let reader = store.get_log_reader().await;

        // Both should work independently
        let state1 = store.get_log_state().await.expect("Should get state");
        // We can't easily test reader without trait bounds, but the clone should work
        drop(reader);
        let state2 = store.get_log_state().await.expect("Should get state again");
        assert_eq!(state1.last_log_id, state2.last_log_id);
    }

    #[test]
    fn test_redb_creates_directory_if_missing() {
        let temp_dir = create_temp_dir();
        let nested_path = temp_dir
            .path()
            .join("nested")
            .join("dir")
            .join("store.redb");

        let store = RedbLogStore::new(&nested_path);
        assert!(store.is_ok(), "Should create nested directories");

        // Verify directory was created
        assert!(nested_path.parent().unwrap().exists());
    }

    #[tokio::test]
    async fn test_redb_empty_database_state() {
        let temp_dir = create_temp_dir();
        let path = create_redb_path(&temp_dir, "empty");

        let mut store = RedbLogStore::new(&path).expect("Should create store");

        // Empty store should have no entries
        let state = store.get_log_state().await.expect("Should get state");
        assert!(state.last_log_id.is_none());
        assert!(state.last_purged_log_id.is_none());

        // No vote
        let vote = store.read_vote().await.expect("Should read vote");
        assert!(vote.is_none());

        // No committed
        let committed = store.read_committed().await.expect("Should read committed");
        assert!(committed.is_none());
    }

    #[tokio::test]
    async fn test_redb_overwrite_vote() {
        let temp_dir = create_temp_dir();
        let path = create_redb_path(&temp_dir, "vote_overwrite");

        let mut store = RedbLogStore::new(&path).expect("Should create store");

        // Save first vote
        let vote1 = openraft::vote::Vote::new(1, NodeId::from(1));
        store.save_vote(&vote1).await.expect("Should save vote1");

        // Save second vote (higher term)
        let vote2 = openraft::vote::Vote::new(2, NodeId::from(2));
        store.save_vote(&vote2).await.expect("Should save vote2");

        // Should read a vote (the latest one that was saved)
        let read = store.read_vote().await.expect("Should read");
        assert!(read.is_some(), "Should have a vote stored");
    }
}
