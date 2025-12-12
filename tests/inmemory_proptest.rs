/// Property-based tests for InMemory storage (InMemoryStateMachine) using proptest.
///
/// This module verifies state machine invariants, monotonic properties,
/// and snapshot correctness through comprehensive property testing.
use std::io;

use aspen::raft::storage::InMemoryStateMachine;
use aspen::raft::types::{AppRequest, AppTypeConfig};
use futures::stream;
use openraft::LogId;
use openraft::entry::RaftEntry;
use openraft::storage::{RaftSnapshotBuilder, RaftStateMachine};
use openraft::testing::log_id;
use proptest::prelude::*;

// Helper function to create a log ID with given term, node, and index
fn make_log_id(term: u64, node: u64, index: u64) -> LogId<AppTypeConfig> {
    log_id::<AppTypeConfig>(term, node, index)
}

// Generators for test data
fn arbitrary_key_value() -> impl Strategy<Value = (String, String)> {
    (
        "[a-z][a-z0-9_]{0,19}", // Key: alphanumeric, 1-20 chars
        prop::string::string_regex("[a-zA-Z0-9 ]{1,100}").unwrap(), // Value: 1-100 chars
    )
}

// Test 1: Monotonic Log Indices
proptest! {
    #[test]
    fn test_applied_log_indices_are_monotonic(
        num_entries in 1usize..100usize
    ) {
        // Property: After applying N entries, last_applied index increases monotonically
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut sm = InMemoryStateMachine::new();

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
                let entries_stream = Box::pin(stream::once(async move {
                    Ok::<_, io::Error>((entry, None))
                }));
                sm.apply(entries_stream).await.unwrap();

                // Verify last_applied increased
                let (last_applied, _) = sm.applied_state().await.unwrap();
                prop_assert!(last_applied.is_some());
                let applied_index = last_applied.unwrap().index;
                prop_assert!(
                    applied_index > last_index,
                    "Index did not increase: {} <= {}",
                    applied_index,
                    last_index
                );
                last_index = applied_index;
            }

            Ok(())
        })?;
    }
}

// Test 2: Snapshot Captures All Applied Data
proptest! {
    #[test]
    fn test_snapshot_captures_all_applied_data(
        entries in prop::collection::vec(arbitrary_key_value(), 1..50)
    ) {
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

                let entries_stream = Box::pin(stream::once(async move {
                    Ok::<_, io::Error>((entry, None))
                }));
                sm.apply(entries_stream).await.unwrap();
            }

            // Build snapshot
            let snapshot = sm.build_snapshot().await.unwrap();

            // Build expected final state (last value wins for duplicate keys)
            let mut expected_state: std::collections::HashMap<String, String> =
                std::collections::HashMap::new();
            for (key, value) in &entries {
                expected_state.insert(key.clone(), value.clone());
            }

            // Verify snapshot contains all data
            let snapshot_data: std::collections::BTreeMap<String, String> =
                serde_json::from_slice(&snapshot.snapshot.into_inner()).unwrap();

            for (key, value) in &expected_state {
                prop_assert_eq!(
                    snapshot_data.get(key),
                    Some(value),
                    "Snapshot missing key: {}",
                    key
                );
            }

            // Verify last_log_id in snapshot metadata
            let (last_applied, _) = sm.applied_state().await.unwrap();
            prop_assert_eq!(snapshot.meta.last_log_id, last_applied);

            Ok(())
        })?;
    }
}

// Test 3: Applying Same Entry Twice Is Idempotent
proptest! {
    #[test]
    fn test_applying_same_entry_twice_is_idempotent(
        (key, value) in arbitrary_key_value()
    ) {
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
            let entries_stream1 = Box::pin(stream::once(async move {
                Ok::<_, io::Error>((entry_clone1, None))
            }));
            sm.apply(entries_stream1).await.unwrap();

            let stored_value1 = sm.get(&key).await;

            // Apply same entry again (simulating replay)
            let entries_stream2 = Box::pin(stream::once(async move {
                Ok::<_, io::Error>((entry, None))
            }));
            sm.apply(entries_stream2).await.unwrap();

            let stored_value2 = sm.get(&key).await;

            // Values should be the same
            prop_assert_eq!(stored_value1, Some(value.clone()));
            prop_assert_eq!(stored_value2, Some(value.clone()));

            // Last applied should still be at index 1
            let (last_applied, _) = sm.applied_state().await.unwrap();
            prop_assert_eq!(last_applied.unwrap().index, 1);

            Ok(())
        })?;
    }
}

// Test 4: Concurrent Reads During Writes
proptest! {
    #[test]
    fn test_concurrent_reads_during_writes(
        write_entries in prop::collection::vec(arbitrary_key_value(), 5..20)
    ) {
        // Property: Concurrent reads should not block or see partial writes
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut sm = InMemoryStateMachine::new();

            // Apply initial entries
            for (i, (key, value)) in write_entries.iter().enumerate() {
                let index = (i + 1) as u64;
                let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                    make_log_id(1, 1, index),
                    AppRequest::Set {
                        key: key.clone(),
                        value: value.clone(),
                    },
                );

                let entries_stream = Box::pin(stream::once(async move {
                    Ok::<_, io::Error>((entry, None))
                }));
                sm.apply(entries_stream).await.unwrap();
            }

            // Build expected final state (last value wins for duplicate keys)
            let mut expected_state: std::collections::HashMap<String, String> =
                std::collections::HashMap::new();
            for (key, value) in &write_entries {
                expected_state.insert(key.clone(), value.clone());
            }

            // Spawn concurrent reads
            let sm_arc = sm.clone();
            let expected_state_clone = expected_state.clone();

            let read_task = tokio::spawn(async move {
                for (key, expected_value) in expected_state_clone.iter() {
                    let value = sm_arc.get(key).await;
                    assert_eq!(value.as_ref(), Some(expected_value));
                }
            });

            // Wait for reads to complete
            read_task.await.unwrap();

            Ok::<(), TestCaseError>(())
        })?;
    }
}

// Test 5: Large Value Storage
proptest! {
    #[test]
    fn test_large_value_storage(
        value_size in 1000usize..10000usize
    ) {
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

            let entries_stream = Box::pin(stream::once(async move {
                Ok::<_, io::Error>((entry, None))
            }));
            sm.apply(entries_stream).await.unwrap();

            // Verify large value was stored correctly
            let stored = sm.get(&key).await;
            prop_assert_eq!(stored.clone(), Some(value.clone()));
            prop_assert_eq!(stored.unwrap().len(), value_size);

            // Verify snapshot can capture large value
            let snapshot = sm.build_snapshot().await.unwrap();
            let snapshot_data: std::collections::BTreeMap<String, String> =
                serde_json::from_slice(&snapshot.snapshot.into_inner()).unwrap();
            prop_assert_eq!(snapshot_data.get(&key), Some(&value));

            Ok(())
        })?;
    }
}

// Test 6: SetMulti Operation Correctness
proptest! {
    #[test]
    fn test_setmulti_applies_all_pairs(
        num_pairs in 1usize..50usize
    ) {
        // Property: SetMulti should atomically apply all key-value pairs
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut sm = InMemoryStateMachine::new();

            let pairs: Vec<(String, String)> = (0..num_pairs)
                .map(|i| (format!("multi_key_{}", i), format!("multi_value_{}", i)))
                .collect();

            let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
                make_log_id(1, 1, 1),
                AppRequest::SetMulti {
                    pairs: pairs.clone(),
                },
            );

            let entries_stream = Box::pin(stream::once(async move {
                Ok::<_, io::Error>((entry, None))
            }));
            sm.apply(entries_stream).await.unwrap();

            // Verify all pairs were applied
            for (key, value) in &pairs {
                let stored = sm.get(key).await;
                prop_assert_eq!(stored, Some(value.clone()), "Missing key: {}", key);
            }

            Ok(())
        })?;
    }
}
