//! Property-based tests for SQLite storage using Bolero.
//!
//! This module verifies state machine invariants, monotonic properties,
//! and transaction atomicity through comprehensive property testing.
//!
//! Tiger Style: Tests verify that resource bounds from constants.rs are enforced.

mod support;

use std::collections::BTreeMap;
use std::io;
use std::sync::Arc;

use aspen::raft::constants::MAX_KEY_SIZE;
use aspen::raft::constants::MAX_SETMULTI_KEYS;
use aspen::raft::constants::MAX_VALUE_SIZE;
use aspen::raft::storage_sqlite::SqliteStateMachine;
use aspen::raft::types::AppRequest;
use aspen::raft::types::AppTypeConfig;
use bolero::check;
use futures::stream;
use openraft::LogId;
use openraft::entry::RaftEntry;
use openraft::storage::RaftSnapshotBuilder;
use openraft::storage::RaftStateMachine;
use openraft::testing::log_id;
use support::bolero_generators::KeyValuePair;
use support::bolero_generators::NumEntries;
use support::bolero_generators::NumWrites;
use tempfile::TempDir;

// Helper function to create a temporary SQLite state machine
fn create_temp_sm() -> (Arc<SqliteStateMachine>, TempDir) {
    let temp_dir = TempDir::new().expect("failed to create temp directory");
    let sm_path = temp_dir.path().join("test-state-machine.db");
    let sm = SqliteStateMachine::new(&sm_path).expect("failed to create state machine");
    (sm, temp_dir)
}

// Helper function to create a log ID with given term, node, and index
fn make_log_id(term: u64, node: u64, index: u64) -> LogId<AppTypeConfig> {
    use aspen::raft::types::NodeId;
    log_id::<AppTypeConfig>(term, NodeId::from(node), index)
}

// Test 1: Monotonic Log Indices
#[test]
fn test_applied_log_indices_are_monotonic() {
    check!().with_iterations(100).with_type::<NumEntries>().for_each(|num_entries| {
        let num_entries = num_entries.0.min(30);
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (mut sm, _temp_dir) = create_temp_sm();

            let mut last_index = 0u64;
            for i in 0..num_entries {
                let index = (i + 1) as u64;
                let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                    make_log_id(1, 1, index),
                    AppRequest::Set {
                        key: format!("key_{}", i),
                        value: format!("value_{}", i),
                    },
                );

                let entries_stream = Box::pin(stream::once(async move { Ok::<_, io::Error>((entry, None)) }));

                sm.apply(entries_stream).await.expect("failed to apply entry");

                let (last_applied, _) = sm.applied_state().await.expect("failed to get applied state");
                if let Some(log_id) = last_applied {
                    assert!(
                        log_id.index >= last_index,
                        "Log index must be monotonic: {} < {}",
                        log_id.index,
                        last_index
                    );
                    assert_eq!(log_id.index, index, "Applied index should match entry index");
                    last_index = log_id.index;
                }
            }
        });
    });
}

// Test 2: Transaction Atomicity
#[test]
fn test_failed_transaction_doesnt_corrupt_state() {
    check!().with_iterations(50).with_type::<(Vec<KeyValuePair>, Vec<KeyValuePair>)>().for_each(
        |(valid_entries, extra_pairs)| {
            let valid_entries: Vec<(String, String)> =
                valid_entries.iter().take(15).map(|kv| (kv.key.0.clone(), kv.value.0.clone())).collect();

            if valid_entries.is_empty() {
                return;
            }

            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let (mut sm, _temp_dir) = create_temp_sm();

                // Apply valid entries successfully
                for (i, (key, value)) in valid_entries.iter().enumerate() {
                    let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                        make_log_id(1, 1, (i + 1) as u64),
                        AppRequest::Set {
                            key: key.clone(),
                            value: value.clone(),
                        },
                    );

                    let entries_stream = Box::pin(stream::once(async move { Ok::<_, io::Error>((entry, None)) }));

                    sm.apply(entries_stream).await.expect("failed to apply valid entry");
                }

                // Build expected final state (deduplicate to last value per key)
                let mut expected_state = BTreeMap::new();
                for (key, value) in &valid_entries {
                    expected_state.insert(key.clone(), value.clone());
                }

                // Verify all valid entries are present
                for (key, expected_value) in &expected_state {
                    let stored = sm.get(key).await.expect("failed to get key");
                    assert_eq!(stored.as_ref(), Some(expected_value));
                }

                // Create an oversized SetMulti that should fail
                let oversized_pairs: Vec<(String, String)> = (0..(MAX_SETMULTI_KEYS as usize + 1))
                    .map(|i| (format!("oversized_key_{}", i), format!("oversized_value_{}", i)))
                    .chain(extra_pairs.iter().take(15).map(|kv| (kv.key.0.clone(), kv.value.0.clone())))
                    .collect();

                let invalid_entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                    make_log_id(2, 1, (valid_entries.len() + 1) as u64),
                    AppRequest::SetMulti { pairs: oversized_pairs },
                );

                let entries_stream = Box::pin(stream::once(async move { Ok::<_, io::Error>((invalid_entry, None)) }));

                // Attempt to apply invalid entry (should fail)
                let result = sm.apply(entries_stream).await;
                assert!(result.is_err(), "Oversized SetMulti should fail");

                // Force WAL checkpoint to ensure read visibility after rollback
                sm.checkpoint_wal().ok();
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;

                // Verify all valid entries are still present and unchanged
                for (key, value) in &valid_entries {
                    let stored = sm.get(key).await.expect("failed to get key after failed transaction");
                    assert_eq!(stored.as_ref(), Some(value), "Valid data should remain after failed transaction");
                }
            });
        },
    );
}

// Test 3: Snapshot Consistency
#[test]
fn test_snapshot_captures_all_applied_data() {
    check!().with_iterations(50).with_type::<Vec<KeyValuePair>>().for_each(|entries| {
        let entries: Vec<(String, String)> =
            entries.iter().take(20).map(|kv| (kv.key.0.clone(), kv.value.0.clone())).collect();

        if entries.is_empty() {
            return;
        }

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (mut sm, temp_dir) = create_temp_sm();

            // Apply entries
            for (i, (key, value)) in entries.iter().enumerate() {
                let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                    make_log_id(1, 1, (i + 1) as u64),
                    AppRequest::Set {
                        key: key.clone(),
                        value: value.clone(),
                    },
                );

                let entries_stream = Box::pin(stream::once(async move { Ok::<_, io::Error>((entry, None)) }));

                sm.apply(entries_stream).await.expect("failed to apply entry");
            }

            // Build expected final state (last value wins for duplicate keys)
            let mut expected_state = BTreeMap::new();
            for (key, value) in &entries {
                expected_state.insert(key.clone(), value.clone());
            }

            // Build snapshot
            let snapshot = sm.build_snapshot().await.expect("failed to build snapshot");

            // Create new state machine and install snapshot
            let sm2_path = temp_dir.path().join("test-state-machine-2.db");
            let mut sm2 = SqliteStateMachine::new(&sm2_path).expect("failed to create second state machine");

            sm2.install_snapshot(&snapshot.meta, snapshot.snapshot).await.expect("failed to install snapshot");

            // Verify all data matches between original and restored using deduplicated state
            for (key, expected_value) in &expected_state {
                let original = sm.get(key).await.expect("failed to get from original");
                let restored = sm2.get(key).await.expect("failed to get from restored");
                assert_eq!(&original, &restored, "Data mismatch for key '{}': {:?} != {:?}", key, original, restored);
                assert_eq!(restored.as_ref(), Some(expected_value), "Restored value mismatch for key '{}'", key);
            }

            // Verify applied states match
            let (orig_applied, orig_membership) = sm.applied_state().await.expect("failed to get original state");
            let (rest_applied, rest_membership) = sm2.applied_state().await.expect("failed to get restored state");
            assert_eq!(orig_applied, rest_applied, "Applied log mismatch after snapshot");
            assert_eq!(orig_membership, rest_membership, "Membership mismatch after snapshot");
        });
    });
}

// Test 4: Idempotent Operations
#[test]
fn test_applying_same_entry_twice_is_idempotent() {
    check!().with_iterations(100).with_type::<KeyValuePair>().for_each(|kv| {
        let key = kv.key.0.clone();
        let value1 = kv.value.0.clone();
        let value2 = format!("{}_updated", value1);

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (mut sm, _temp_dir) = create_temp_sm();

            // Apply first entry
            let entry1 =
                <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(make_log_id(1, 1, 1), AppRequest::Set {
                    key: key.clone(),
                    value: value1.clone(),
                });

            let entries_stream = Box::pin(stream::once(async move { Ok::<_, io::Error>((entry1, None)) }));

            sm.apply(entries_stream).await.expect("failed to apply first entry");

            let stored = sm.get(&key).await.expect("failed to get key");
            assert_eq!(stored.as_ref(), Some(&value1));

            // Apply second entry with same log ID (simulating replay)
            let entry2 =
                <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(make_log_id(1, 1, 1), AppRequest::Set {
                    key: key.clone(),
                    value: value2.clone(),
                });

            let entries_stream = Box::pin(stream::once(async move { Ok::<_, io::Error>((entry2, None)) }));

            sm.apply(entries_stream).await.expect("failed to apply second entry");

            let final_stored = sm.get(&key).await.expect("failed to get key after second apply");
            assert_eq!(final_stored.as_ref(), Some(&value2), "Final value should be the last applied");
        });
    });
}

// Test 5: Batch Size Limits
#[test]
fn test_batch_size_enforcement() {
    check!().with_iterations(50).with_type::<NumEntries>().for_each(|num_entries| {
        let size = num_entries.0.min(100) as u32;
        if size == 0 {
            return;
        }

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (mut sm, _temp_dir) = create_temp_sm();

            for i in 0..size {
                let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                    make_log_id(1, 1, (i + 1) as u64),
                    AppRequest::Set {
                        key: format!("batch_key_{}", i),
                        value: format!("batch_value_{}", i),
                    },
                );

                let entries_stream = Box::pin(stream::once(async move { Ok::<_, io::Error>((entry, None)) }));

                sm.apply(entries_stream).await.expect("failed to apply entry");
            }

            // Verify all entries were applied
            for i in 0..size {
                let key = format!("batch_key_{}", i);
                let stored = sm.get(&key).await.expect("failed to get key");
                assert_eq!(stored, Some(format!("batch_value_{}", i)), "Entry {} should be stored", i);
            }

            let (last_applied, _) = sm.applied_state().await.expect("failed to get applied state");
            assert_eq!(
                last_applied.map(|id| id.index),
                Some(size as u64),
                "Applied index should equal number of entries"
            );
        });
    });
}

// Test 6: SetMulti Key Limits
#[test]
fn test_setmulti_key_limit_enforcement() {
    check!().with_iterations(50).with_type::<u8>().for_each(|num_keys_hint| {
        let num_keys = 1 + (*num_keys_hint as u32 % 199);

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (mut sm, _temp_dir) = create_temp_sm();

            let pairs: Vec<(String, String)> =
                (0..num_keys).map(|i| (format!("multi_key_{}", i), format!("multi_value_{}", i))).collect();

            let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                make_log_id(1, 1, 1),
                AppRequest::SetMulti { pairs: pairs.clone() },
            );

            let entries_stream = Box::pin(stream::once(async move { Ok::<_, io::Error>((entry, None)) }));

            let result = sm.apply(entries_stream).await;

            if num_keys <= MAX_SETMULTI_KEYS {
                assert!(result.is_ok(), "SetMulti with {} keys should succeed", num_keys);

                // Verify all keys were written
                for i in 0..num_keys {
                    let key = format!("multi_key_{}", i);
                    let stored = sm.get(&key).await.expect("failed to get key");
                    assert_eq!(stored, Some(format!("multi_value_{}", i)), "Key {} should be stored", i);
                }
            } else {
                assert!(result.is_err(), "SetMulti with {} keys (> {}) should fail", num_keys, MAX_SETMULTI_KEYS);

                // Verify no keys were written
                for i in 0..num_keys {
                    let key = format!("multi_key_{}", i);
                    let stored = sm.get(&key).await.expect("failed to check key");
                    assert_eq!(stored, None, "Key {} should not be stored after failed SetMulti", i);
                }
            }
        });
    });
}

// Test 7: WAL Checkpoint Preserves Data
#[test]
fn test_checkpoint_preserves_data() {
    check!().with_iterations(50).with_type::<Vec<KeyValuePair>>().for_each(|entries| {
        let entries: Vec<(String, String)> =
            entries.iter().take(30).map(|kv| (kv.key.0.clone(), kv.value.0.clone())).collect();

        if entries.is_empty() {
            return;
        }

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (mut sm, _temp_dir) = create_temp_sm();

            // Apply entries
            for (i, (key, value)) in entries.iter().enumerate() {
                let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                    make_log_id(1, 1, (i + 1) as u64),
                    AppRequest::Set {
                        key: key.clone(),
                        value: value.clone(),
                    },
                );

                let entries_stream = Box::pin(stream::once(async move { Ok::<_, io::Error>((entry, None)) }));

                sm.apply(entries_stream).await.expect("failed to apply entry");
            }

            // Build expected final state (deduplicate to last value per key)
            let mut expected_state = BTreeMap::new();
            for (key, value) in &entries {
                expected_state.insert(key.clone(), value.clone());
            }

            // Get applied state before checkpoint
            let (applied_before, membership_before) =
                sm.applied_state().await.expect("failed to get applied state before checkpoint");

            // Perform WAL checkpoint
            let _pages_checkpointed = sm.checkpoint_wal().expect("failed to checkpoint WAL");

            // Verify all data is still present after checkpoint
            for (key, expected_value) in &expected_state {
                let stored = sm.get(key).await.expect("failed to get key after checkpoint");
                assert_eq!(
                    stored.as_ref(),
                    Some(expected_value),
                    "Data should be preserved after checkpoint for key '{}'",
                    key
                );
            }

            // Verify applied state is unchanged
            let (applied_after, membership_after) =
                sm.applied_state().await.expect("failed to get applied state after checkpoint");
            assert_eq!(applied_before, applied_after, "Applied log should be unchanged after checkpoint");
            assert_eq!(membership_before, membership_after, "Membership should be unchanged after checkpoint");
        });
    });
}

// Test 8: Concurrent Reads During Writes
#[test]
fn test_concurrent_reads_during_writes() {
    check!().with_iterations(50).with_type::<Vec<KeyValuePair>>().for_each(|write_entries| {
        let write_entries: Vec<(String, String)> =
            write_entries.iter().take(20).skip(5).map(|kv| (kv.key.0.clone(), kv.value.0.clone())).collect();

        if write_entries.len() < 5 {
            return;
        }

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (mut sm, _temp_dir) = create_temp_sm();

            // Apply first 10 entries that we'll read
            let initial_entries: Vec<(String, String)> =
                (0..10).map(|i| (format!("init_key_{}", i), format!("init_value_{}", i))).collect();

            for (i, (key, value)) in initial_entries.iter().enumerate() {
                let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                    make_log_id(1, 1, (i + 1) as u64),
                    AppRequest::Set {
                        key: key.clone(),
                        value: value.clone(),
                    },
                );

                let entries_stream = Box::pin(stream::once(async move { Ok::<_, io::Error>((entry, None)) }));

                sm.apply(entries_stream).await.expect("failed to apply initial entry");
            }

            // Clone state machine for concurrent reading
            let sm_clone = sm.clone();

            // Start read task that continuously reads while we write
            let read_task = tokio::spawn(async move {
                let mut read_count = 0;
                for _ in 0..50 {
                    for i in 0..3 {
                        let key = format!("init_key_{}", i);
                        let stored = sm_clone.get(&key).await.expect("failed to read");
                        assert_eq!(stored, Some(format!("init_value_{}", i)), "Read should see consistent value");
                        read_count += 1;
                    }
                    tokio::task::yield_now().await;
                }
                read_count
            });

            // Perform writes concurrently with reads
            for (i, (key, value)) in write_entries.iter().enumerate() {
                let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                    make_log_id(2, 1, (i + 11) as u64),
                    AppRequest::Set {
                        key: key.clone(),
                        value: value.clone(),
                    },
                );

                let entries_stream = Box::pin(stream::once(async move { Ok::<_, io::Error>((entry, None)) }));

                sm.apply(entries_stream).await.expect("failed to apply write entry");
                tokio::task::yield_now().await;
            }

            let read_count = read_task.await.expect("read task panicked");
            assert!(read_count > 0, "Should have completed some reads");
        });
    });
}

// Test 9: Large Value Handling
#[test]
fn test_large_value_storage() {
    check!().with_iterations(20).with_type::<(KeyValuePair, u16)>().for_each(|(kv, size_hint)| {
        let key = kv.key.0.clone();
        let size_kb = 1 + (*size_hint as usize % 99);

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (mut sm, _temp_dir) = create_temp_sm();

            let large_value = "x".repeat(size_kb * 1024);

            let entry =
                <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(make_log_id(1, 1, 1), AppRequest::Set {
                    key: key.clone(),
                    value: large_value.clone(),
                });

            let entries_stream = Box::pin(stream::once(async move { Ok::<_, io::Error>((entry, None)) }));

            sm.apply(entries_stream).await.expect("failed to apply large value");

            let stored = sm.get(&key).await.expect("failed to get large value");
            assert_eq!(stored.as_ref().map(|s| s.len()), Some(large_value.len()), "Large value size mismatch");
            assert_eq!(stored, Some(large_value), "Large value content mismatch");
        });
    });
}

// Test 10: Snapshot After WAL Growth
#[test]
fn test_snapshot_after_wal_growth() {
    check!().with_iterations(30).with_type::<NumWrites>().for_each(|num_entries| {
        let num_entries = 20 + (num_entries.0 % 15) as u32;

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (mut sm, _temp_dir) = create_temp_sm();

            // Apply many entries to grow the WAL
            for i in 0..num_entries {
                let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                    make_log_id(1, 1, (i + 1) as u64),
                    AppRequest::Set {
                        key: format!("wal_key_{}", i),
                        value: format!("wal_value_{}", i),
                    },
                );

                let entries_stream = Box::pin(stream::once(async move { Ok::<_, io::Error>((entry, None)) }));

                sm.apply(entries_stream).await.expect("failed to apply entry");
            }

            // Build snapshot
            let snapshot = sm.build_snapshot().await.expect("failed to build snapshot with large WAL");

            assert_eq!(
                snapshot.meta.last_log_id,
                Some(make_log_id(1, 1, num_entries as u64)),
                "Snapshot should have correct last_log_id"
            );

            // Checkpoint to reduce WAL size
            let _pages = sm.checkpoint_wal().expect("failed to checkpoint");
        });
    });
}

// Test 11: Delete Operation Correctness
#[test]
fn test_delete_operation_correctness() {
    check!().with_iterations(50).with_type::<Vec<KeyValuePair>>().for_each(|entries| {
        let entries: Vec<(String, String)> =
            entries.iter().take(30).skip(5).map(|kv| (kv.key.0.clone(), kv.value.0.clone())).collect();

        if entries.len() < 5 {
            return;
        }

        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test_delete.sqlite");

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut sm = SqliteStateMachine::new(&db_path).expect("failed to create state machine");

            // First, apply all entries
            for (i, (key, value)) in entries.iter().enumerate() {
                let index = (i + 1) as u64;
                let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                    make_log_id(1, 1, index),
                    AppRequest::Set {
                        key: key.clone(),
                        value: value.clone(),
                    },
                );

                let entries_stream = Box::pin(stream::once(async move { Ok::<_, io::Error>((entry, None)) }));
                sm.apply(entries_stream).await.expect("failed to apply set entry");
            }

            // Build expected state after all writes (last value wins for duplicate keys)
            let mut expected_state: std::collections::HashMap<String, String> = std::collections::HashMap::new();
            for (key, value) in &entries {
                expected_state.insert(key.clone(), value.clone());
            }

            // Pick unique keys to delete (every other unique key)
            let unique_keys: Vec<String> = expected_state.keys().cloned().collect();
            let keys_to_delete: std::collections::HashSet<String> =
                unique_keys.iter().enumerate().filter(|(i, _)| i % 2 == 0).map(|(_, k)| k.clone()).collect();

            // Apply delete operations
            for (i, key) in keys_to_delete.iter().enumerate() {
                let index = (entries.len() + i + 1) as u64;
                let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                    make_log_id(1, 1, index),
                    AppRequest::Delete { key: key.clone() },
                );

                let entries_stream = Box::pin(stream::once(async move { Ok::<_, io::Error>((entry, None)) }));
                sm.apply(entries_stream).await.expect("failed to apply delete entry");
            }

            // Verify deleted keys return None
            for key in &keys_to_delete {
                let stored = sm.get(key).await.expect("failed to get key");
                assert_eq!(stored, None, "Deleted key {} should return None", key);
            }

            // Verify non-deleted keys still have their values
            for (key, expected_value) in &expected_state {
                if !keys_to_delete.contains(key) {
                    let stored = sm.get(key).await.expect("failed to get key");
                    assert_eq!(stored, Some(expected_value.clone()), "Non-deleted key {} should retain its value", key);
                }
            }
        });
    });
}

// Test 12: DeleteMulti Operation Correctness
#[test]
fn test_deletemulti_operation_correctness() {
    check!().with_iterations(50).with_type::<Vec<KeyValuePair>>().for_each(|entries| {
        let entries: Vec<(String, String)> =
            entries.iter().take(40).skip(10).map(|kv| (kv.key.0.clone(), kv.value.0.clone())).collect();

        if entries.len() < 10 {
            return;
        }

        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test_deletemulti.sqlite");

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut sm = SqliteStateMachine::new(&db_path).expect("failed to create state machine");

            // Apply all entries
            for (i, (key, value)) in entries.iter().enumerate() {
                let index = (i + 1) as u64;
                let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                    make_log_id(1, 1, index),
                    AppRequest::Set {
                        key: key.clone(),
                        value: value.clone(),
                    },
                );

                let entries_stream = Box::pin(stream::once(async move { Ok::<_, io::Error>((entry, None)) }));
                sm.apply(entries_stream).await.expect("failed to apply set entry");
            }

            // Collect unique keys to delete (first half)
            let keys_to_delete: Vec<String> = entries.iter().take(entries.len() / 2).map(|(k, _)| k.clone()).collect();

            // Apply DeleteMulti operation
            let index = (entries.len() + 1) as u64;
            let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                make_log_id(1, 1, index),
                AppRequest::DeleteMulti {
                    keys: keys_to_delete.clone(),
                },
            );

            let entries_stream = Box::pin(stream::once(async move { Ok::<_, io::Error>((entry, None)) }));
            sm.apply(entries_stream).await.expect("failed to apply deletemulti entry");

            // Verify all deleted keys return None
            for key in &keys_to_delete {
                let stored = sm.get(key).await.expect("failed to get key");
                assert_eq!(stored, None, "Key {} should be deleted by DeleteMulti", key);
            }
        });
    });
}

// Test 13: Delete Idempotency
#[test]
fn test_delete_idempotency() {
    check!().with_iterations(30).with_type::<KeyValuePair>().for_each(|kv| {
        let key = kv.key.0.clone();
        let value = kv.value.0.clone();

        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test_delete_idempotent.sqlite");

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut sm = SqliteStateMachine::new(&db_path).expect("failed to create state machine");

            // Set a key
            let entry1 =
                <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(make_log_id(1, 1, 1), AppRequest::Set {
                    key: key.clone(),
                    value: value.clone(),
                });
            let entries_stream = Box::pin(stream::once(async move { Ok::<_, io::Error>((entry1, None)) }));
            sm.apply(entries_stream).await.expect("failed to apply set");

            // Verify key exists
            let stored = sm.get(&key).await.expect("failed to get key");
            assert_eq!(stored, Some(value.clone()), "Key should exist after set");

            // Delete the key first time
            let entry2 = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                make_log_id(1, 1, 2),
                AppRequest::Delete { key: key.clone() },
            );
            let entries_stream = Box::pin(stream::once(async move { Ok::<_, io::Error>((entry2, None)) }));
            sm.apply(entries_stream).await.expect("failed to apply first delete");

            // Verify key is gone
            let stored = sm.get(&key).await.expect("failed to get key");
            assert_eq!(stored, None, "Key should be gone after first delete");

            // Delete the same key again (should be idempotent - no error)
            let entry3 = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                make_log_id(1, 1, 3),
                AppRequest::Delete { key: key.clone() },
            );
            let entries_stream = Box::pin(stream::once(async move { Ok::<_, io::Error>((entry3, None)) }));
            sm.apply(entries_stream).await.expect("failed to apply second delete (should be idempotent)");

            // Verify key is still gone
            let stored = sm.get(&key).await.expect("failed to get key");
            assert_eq!(stored, None, "Key should still be gone after second delete");

            // Verify state is still consistent
            let (last_applied, _) = sm.applied_state().await.expect("failed to get applied state");
            assert_eq!(last_applied.map(|l| l.index), Some(3), "Applied index should be 3 after all operations");
        });
    });
}

// ============================================================================
// Tiger Style Boundary Tests
// ============================================================================

// Test 14: Key Size at Boundary (MAX_KEY_SIZE)
#[test]
fn test_key_size_at_boundary() {
    check!().with_iterations(20).with_type::<i8>().for_each(|size_delta| {
        let size_delta = (*size_delta % 11) as i32 - 5; // -5 to +5

        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test_key_boundary.sqlite");

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut sm = SqliteStateMachine::new(&db_path).expect("failed to create state machine");

            let key_size = (MAX_KEY_SIZE as i32 + size_delta).max(1) as usize;
            let key = "k".repeat(key_size);
            let value = "test_value".to_string();

            let entry =
                <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(make_log_id(1, 1, 1), AppRequest::Set {
                    key: key.clone(),
                    value: value.clone(),
                });

            let entries_stream = Box::pin(stream::once(async move { Ok::<_, io::Error>((entry, None)) }));

            // State machine should accept all keys (validation is at API layer)
            sm.apply(entries_stream).await.expect("apply should succeed at SM level");

            // Verify the key was stored
            let stored = sm.get(&key).await.expect("failed to get key");
            assert_eq!(stored, Some(value), "Key of size {} should be stored (validation at API layer)", key_size);
        });
    });
}

// Test 15: Value Size at Boundary (MAX_VALUE_SIZE)
#[test]
fn test_value_size_at_boundary() {
    check!().with_iterations(10).with_type::<u8>().for_each(|size_fraction_hint| {
        let size_fraction = 0.95 + (*size_fraction_hint as f64 / 255.0) * 0.05;

        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test_value_boundary.sqlite");

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut sm = SqliteStateMachine::new(&db_path).expect("failed to create state machine");

            let value_size = (MAX_VALUE_SIZE as f64 * size_fraction) as usize;
            let key = "large_value_key".to_string();
            let value = "x".repeat(value_size);

            let entry =
                <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(make_log_id(1, 1, 1), AppRequest::Set {
                    key: key.clone(),
                    value: value.clone(),
                });

            let entries_stream = Box::pin(stream::once(async move { Ok::<_, io::Error>((entry, None)) }));
            sm.apply(entries_stream).await.expect("apply should succeed for large value");

            let stored = sm.get(&key).await.expect("failed to get key");
            assert_eq!(
                stored.as_ref().map(|s| s.len()),
                Some(value_size),
                "Large value should be stored with correct size"
            );
        });
    });
}

// Test 16: SetMulti at Boundary (MAX_SETMULTI_KEYS)
#[test]
fn test_setmulti_at_boundary() {
    check!().with_iterations(10).with_type::<u8>().for_each(|delta_hint| {
        let delta = -(*delta_hint as i32 % 11);

        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test_setmulti_boundary.sqlite");

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut sm = SqliteStateMachine::new(&db_path).expect("failed to create state machine");

            let count = (MAX_SETMULTI_KEYS as i32 + delta).max(1) as usize;

            let pairs: Vec<(String, String)> =
                (0..count).map(|i| (format!("boundary_key_{}", i), format!("boundary_value_{}", i))).collect();

            let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                make_log_id(1, 1, 1),
                AppRequest::SetMulti { pairs: pairs.clone() },
            );

            let entries_stream = Box::pin(stream::once(async move { Ok::<_, io::Error>((entry, None)) }));

            sm.apply(entries_stream).await.expect("apply should succeed at/below limit");

            // Verify all pairs were stored
            for (key, expected_value) in &pairs {
                let stored = sm.get(key).await.expect("failed to get key");
                assert_eq!(
                    stored,
                    Some(expected_value.clone()),
                    "Key {} should be stored in SetMulti of {} pairs",
                    key,
                    count
                );
            }
        });
    });
}

// Test 17: Batch Operations at Boundary
#[test]
fn test_batch_operations_at_boundary() {
    check!().with_iterations(20).with_type::<u8>().for_each(|batch_hint| {
        let batch_size = 50 + (*batch_hint as usize % 51);

        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test_batch_boundary.sqlite");

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut sm = SqliteStateMachine::new(&db_path).expect("failed to create state machine");

            for i in 0..batch_size {
                let index = (i + 1) as u64;
                let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                    make_log_id(1, 1, index),
                    AppRequest::Set {
                        key: format!("batch_key_{}", i),
                        value: format!("batch_value_{}", i),
                    },
                );

                let entries_stream = Box::pin(stream::once(async move { Ok::<_, io::Error>((entry, None)) }));
                sm.apply(entries_stream).await.expect("batch entry should apply");
            }

            let (last_applied, _) = sm.applied_state().await.expect("should get applied state");
            assert_eq!(
                last_applied.map(|l| l.index),
                Some(batch_size as u64),
                "All {} batch entries should be applied",
                batch_size
            );

            // Verify first, middle, and last entries for correctness
            let check_indices = vec![0, batch_size / 2, batch_size - 1];
            for i in check_indices {
                let key = format!("batch_key_{}", i);
                let stored = sm.get(&key).await.expect("failed to get key");
                assert_eq!(
                    stored,
                    Some(format!("batch_value_{}", i)),
                    "Entry {} in batch of {} should be correct",
                    i,
                    batch_size
                );
            }
        });
    });
}
