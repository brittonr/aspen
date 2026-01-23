//! Property-based tests for InMemory storage (InMemoryStateMachine) using Bolero.
//!
//! This module verifies state machine invariants, monotonic properties,
//! and snapshot correctness through comprehensive property testing.

mod support;

use std::io;

use aspen::raft::storage::InMemoryStateMachine;
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

// Helper function to create a log ID with given term, node, and index
fn make_log_id(term: u64, node: u64, index: u64) -> LogId<AppTypeConfig> {
    use aspen::raft::types::NodeId;
    log_id::<AppTypeConfig>(term, NodeId::from(node), index)
}

// Test 1: Monotonic Log Indices
#[test]
fn test_applied_log_indices_are_monotonic() {
    check!().with_iterations(100).with_type::<NumEntries>().for_each(|num_entries| {
        // Property: After applying N entries, last_applied index increases monotonically
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut sm = InMemoryStateMachine::new();
            let num_entries = num_entries.0;

            let mut last_index = 0u64;
            for i in 0..num_entries {
                // Create entries with strictly increasing indices
                let index = (i + 1) as u64;
                let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                    make_log_id(1, 1, index),
                    AppRequest::Set {
                        key: format!("key_{}", i),
                        value: format!("value_{}", i),
                    },
                );

                // Apply entry
                let entries_stream = Box::pin(stream::once(async move { Ok::<_, io::Error>((entry, None)) }));
                sm.apply(entries_stream).await.unwrap();

                // Verify last_applied increased
                let (last_applied, _) = sm.applied_state().await.unwrap();
                assert!(last_applied.is_some());
                let applied_index = last_applied.unwrap().index;
                assert!(applied_index > last_index, "Index did not increase: {} <= {}", applied_index, last_index);
                last_index = applied_index;
            }
        });
    });
}

// Test 2: Snapshot Captures All Applied Data
#[test]
fn test_snapshot_captures_all_applied_data() {
    check!().with_iterations(50).with_type::<Vec<KeyValuePair>>().for_each(|entries| {
        let entries: Vec<(String, String)> =
            entries.iter().take(50).map(|kv| (kv.key.0.clone(), kv.value.0.clone())).collect();

        if entries.is_empty() {
            return;
        }

        // Property: Snapshot must contain all key-value pairs that were applied
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut sm = InMemoryStateMachine::new();

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
                sm.apply(entries_stream).await.unwrap();
            }

            // Build snapshot
            let snapshot = sm.build_snapshot().await.unwrap();

            // Build expected final state (last value wins for duplicate keys)
            let mut expected_state: std::collections::HashMap<String, String> = std::collections::HashMap::new();
            for (key, value) in &entries {
                expected_state.insert(key.clone(), value.clone());
            }

            // Verify snapshot contains all data
            let snapshot_data: std::collections::BTreeMap<String, String> =
                serde_json::from_slice(&snapshot.snapshot.into_inner()).unwrap();

            for (key, value) in &expected_state {
                assert_eq!(snapshot_data.get(key), Some(value), "Snapshot missing key: {}", key);
            }

            // Verify last_log_id in snapshot metadata
            let (last_applied, _) = sm.applied_state().await.unwrap();
            assert_eq!(snapshot.meta.last_log_id, last_applied);
        });
    });
}

// Test 3: Applying Same Entry Twice Is Idempotent
#[test]
fn test_applying_same_entry_twice_is_idempotent() {
    check!().with_iterations(100).with_type::<KeyValuePair>().for_each(|kv| {
        let key = kv.key.0.clone();
        let value = kv.value.0.clone();

        // Property: Applying same log entry twice should not corrupt state
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut sm = InMemoryStateMachine::new();

            let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                make_log_id(1, 1, 1),
                AppRequest::Set {
                    key: key.clone(),
                    value: value.clone(),
                },
            );

            // Apply first time
            let entry_clone1 = entry.clone();
            let entries_stream1 = Box::pin(stream::once(async move { Ok::<_, io::Error>((entry_clone1, None)) }));
            sm.apply(entries_stream1).await.unwrap();

            let stored_value1 = sm.get(&key).await;

            // Apply same entry again (simulating replay)
            let entries_stream2 = Box::pin(stream::once(async move { Ok::<_, io::Error>((entry, None)) }));
            sm.apply(entries_stream2).await.unwrap();

            let stored_value2 = sm.get(&key).await;

            // Values should be the same
            assert_eq!(stored_value1, Some(value.clone()));
            assert_eq!(stored_value2, Some(value.clone()));

            // Last applied should still be at index 1
            let (last_applied, _) = sm.applied_state().await.unwrap();
            assert_eq!(last_applied.unwrap().index, 1);
        });
    });
}

// Test 4: Concurrent Reads During Writes
#[test]
fn test_concurrent_reads_during_writes() {
    check!().with_iterations(20).with_type::<Vec<KeyValuePair>>().for_each(|write_entries| {
        let write_entries: Vec<(String, String)> =
            write_entries.iter().take(20).skip(5).map(|kv| (kv.key.0.clone(), kv.value.0.clone())).collect();

        if write_entries.len() < 5 {
            return;
        }

        // Property: Concurrent reads should not block or see partial writes
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut sm = InMemoryStateMachine::new();

            // Apply initial entries that we'll read during concurrent writes
            let initial_entries: Vec<(String, String)> =
                (0..10).map(|i| (format!("init_key_{}", i), format!("init_value_{}", i))).collect();

            for (i, (key, value)) in initial_entries.iter().enumerate() {
                let index = (i + 1) as u64;
                let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                    make_log_id(1, 1, index),
                    AppRequest::Set {
                        key: key.clone(),
                        value: value.clone(),
                    },
                );

                let entries_stream = Box::pin(stream::once(async move { Ok::<_, io::Error>((entry, None)) }));
                sm.apply(entries_stream).await.unwrap();
            }

            // Clone state machine for concurrent reading
            let sm_clone = sm.clone();

            // Start a read task that runs concurrently with writes
            let read_task = tokio::spawn(async move {
                let mut read_count = 0;
                for _ in 0..50 {
                    for i in 0..10 {
                        let key = format!("init_key_{}", i);
                        let value = sm_clone.get(&key).await;
                        // Initial values should always be consistent
                        assert_eq!(
                            value,
                            Some(format!("init_value_{}", i)),
                            "Read should see consistent value for key {}",
                            key
                        );
                        read_count += 1;
                    }
                    // Yield to allow write task to progress
                    tokio::task::yield_now().await;
                }
                read_count
            });

            // Apply write_entries concurrently with reads
            for (i, (key, value)) in write_entries.iter().enumerate() {
                let index = (i + 11) as u64;
                let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                    make_log_id(2, 1, index),
                    AppRequest::Set {
                        key: key.clone(),
                        value: value.clone(),
                    },
                );

                let entries_stream = Box::pin(stream::once(async move { Ok::<_, io::Error>((entry, None)) }));
                sm.apply(entries_stream).await.unwrap();

                // Yield to allow read task to progress
                tokio::task::yield_now().await;
            }

            // Wait for reads to complete and verify some reads happened
            let read_count = read_task.await.unwrap();
            assert!(read_count > 0, "Should have completed concurrent reads");
        });
    });
}

// Test 5: Large Value Storage
#[test]
fn test_large_value_storage() {
    check!().with_iterations(20).with_type::<u16>().for_each(|size_hint| {
        let value_size = 1000 + (*size_hint as usize % 9000);

        // Property: InMemory storage should handle large values correctly
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut sm = InMemoryStateMachine::new();

            let key = "large_key".to_string();
            let value = "x".repeat(value_size);

            let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                make_log_id(1, 1, 1),
                AppRequest::Set {
                    key: key.clone(),
                    value: value.clone(),
                },
            );

            let entries_stream = Box::pin(stream::once(async move { Ok::<_, io::Error>((entry, None)) }));
            sm.apply(entries_stream).await.unwrap();

            // Verify large value was stored correctly
            let stored = sm.get(&key).await;
            assert_eq!(stored.clone(), Some(value.clone()));
            assert_eq!(stored.unwrap().len(), value_size);

            // Verify snapshot can capture large value
            let snapshot = sm.build_snapshot().await.unwrap();
            let snapshot_data: std::collections::BTreeMap<String, String> =
                serde_json::from_slice(&snapshot.snapshot.into_inner()).unwrap();
            assert_eq!(snapshot_data.get(&key), Some(&value));
        });
    });
}

// Test 6: SetMulti Operation Correctness
#[test]
fn test_setmulti_applies_all_pairs() {
    check!().with_iterations(50).with_type::<NumWrites>().for_each(|num_pairs| {
        let num_pairs = num_pairs.0;

        // Property: SetMulti should atomically apply all key-value pairs
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut sm = InMemoryStateMachine::new();

            let pairs: Vec<(String, String)> =
                (0..num_pairs).map(|i| (format!("multi_key_{}", i), format!("multi_value_{}", i))).collect();

            let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                make_log_id(1, 1, 1),
                AppRequest::SetMulti { pairs: pairs.clone() },
            );

            let entries_stream = Box::pin(stream::once(async move { Ok::<_, io::Error>((entry, None)) }));
            sm.apply(entries_stream).await.unwrap();

            // Verify all pairs were applied
            for (key, value) in &pairs {
                let stored = sm.get(key).await;
                assert_eq!(stored, Some(value.clone()), "Missing key: {}", key);
            }
        });
    });
}
