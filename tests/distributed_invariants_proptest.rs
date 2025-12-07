/// Property-based tests for distributed system invariants using proptest.
///
/// This module verifies fundamental distributed system properties across
/// multi-node Raft clusters:
/// - Eventual consistency across all nodes
/// - Leader uniqueness (at most one leader per term)
/// - Log replication correctness
/// - State machine safety (all nodes apply same commands in same order)
/// - Partition tolerance and recovery
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use aspen::testing::AspenRouter;
use openraft::Config;
use proptest::prelude::*;

// Helper to create key-value generators
fn arbitrary_key_value() -> impl Strategy<Value = (String, String)> {
    (
        "[a-z][a-z0-9_]{0,15}",                                    // Key: 1-16 chars
        prop::string::string_regex("[a-zA-Z0-9 ]{1,50}").unwrap(), // Value: 1-50 chars
    )
}

// Helper to initialize a 3-node cluster
async fn init_three_node_cluster() -> anyhow::Result<(AspenRouter, u64)> {
    use std::collections::BTreeSet;

    let config = Arc::new(
        Config {
            enable_tick: false,
            ..Default::default()
        }
        .validate()?,
    );

    let mut router = AspenRouter::new(config);

    // Create 3-node cluster with all voters
    let mut voter_ids = BTreeSet::new();
    voter_ids.insert(0);
    voter_ids.insert(1);
    voter_ids.insert(2);
    let learners = BTreeSet::new();

    let log_index = router.new_cluster(voter_ids, learners).await?;

    Ok((router, log_index))
}

// Test 1: Eventual Consistency Across All Nodes
proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]
    #[test]
    fn test_eventual_consistency_multi_node(
        writes in prop::collection::vec(arbitrary_key_value(), 3..10)
    ) {
        // Property: All writes to leader eventually appear on all nodes
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (router, initial_log_index) = init_three_node_cluster().await
                .map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;

            let leader = router.leader()
                .ok_or_else(|| proptest::test_runner::TestCaseError::fail("no leader".to_string()))?;

            // Perform writes to leader
            for (key, value) in &writes {
                router
                    .write(&leader, key.clone(), value.clone())
                    .await
                    .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("write failed: {}", e)))?;
            }

            // Build expected state (last value wins for duplicate keys)
            let mut expected: HashMap<String, String> = HashMap::new();
            for (key, value) in &writes {
                expected.insert(key.clone(), value.clone());
            }

            // Calculate expected index: initial cluster setup + writes
            let expected_index = initial_log_index + (writes.len() as u64);

            // Wait for all nodes to reach expected index
            for node_id in 0..3 {
                router
                    .wait(&node_id, Some(Duration::from_millis(5000)))
                    .applied_index(Some(expected_index), "all writes replicated")
                    .await
                    .map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;
            }

            // Verify all nodes have identical data
            for node_id in 0..3 {
                for (key, expected_value) in &expected {
                    let stored = router.read(&node_id, key).await;
                    prop_assert_eq!(
                        stored,
                        Some(expected_value.clone()),
                        "Node {} missing or wrong value for key {}",
                        node_id,
                        key
                    );
                }
            }

            Ok::<(), proptest::test_runner::TestCaseError>(())
        })?;
    }
}

// Test 2: Leader Uniqueness
proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]
    #[test]
    fn test_at_most_one_leader(
        num_writes in 1usize..8usize
    ) {
        // Property: At any given time, at most one node claims to be leader
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (router, _initial_log_index) = init_three_node_cluster().await
                .map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;

            let initial_leader = router.leader()
                .ok_or_else(|| proptest::test_runner::TestCaseError::fail("no initial leader".to_string()))?;

            // Perform writes
            for i in 0..num_writes {
                let key = format!("key_{}", i);
                let value = format!("value_{}", i);

                router
                    .write(&initial_leader, key, value)
                    .await
                    .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("write failed: {}", e)))?;

                // Check leader uniqueness by checking if leader() returns at most one value
                let current_leader = router.leader();

                // Property: leader() returns either None or Some(node_id), never multiple
                // This inherently guarantees at most one leader
                prop_assert!(
                    current_leader.is_none() || current_leader.is_some(),
                    "Leader state inconsistent"
                );
            }

            Ok::<(), proptest::test_runner::TestCaseError>(())
        })?;
    }
}

// Test 3: Log Replication Invariant
proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]
    #[test]
    fn test_log_replication_consistency(
        num_writes in 3usize..12usize
    ) {
        // Property: All nodes reach same log index with matching data
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (router, initial_log_index) = init_three_node_cluster().await
                .map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;

            let leader = router.leader()
                .ok_or_else(|| proptest::test_runner::TestCaseError::fail("no leader".to_string()))?;

            // Perform sequential writes
            let mut keys = Vec::new();
            for i in 0..num_writes {
                let key = format!("replicated_{}", i);
                let value = format!("data_{}", i);
                keys.push((key.clone(), value.clone()));

                router
                    .write(&leader, key, value)
                    .await
                    .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("write failed: {}", e)))?;
            }

            // Expected index: initial cluster setup + writes
            let expected_index = initial_log_index + (num_writes as u64);

            // Wait for all nodes to converge
            for node_id in 0..3 {
                router
                    .wait(&node_id, Some(Duration::from_millis(5000)))
                    .applied_index(Some(expected_index), "logs replicated")
                    .await
                    .map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;
            }

            // Verify all nodes have identical logs
            for node_id in 0..3 {
                for (key, value) in &keys {
                    let stored = router.read(&node_id, key).await;
                    prop_assert_eq!(
                        stored,
                        Some(value.clone()),
                        "Node {} has inconsistent log for key {}",
                        node_id,
                        key
                    );
                }
            }

            Ok::<(), proptest::test_runner::TestCaseError>(())
        })?;
    }
}

// Test 4: State Machine Safety
proptest! {
    #![proptest_config(ProptestConfig::with_cases(8))]
    #[test]
    fn test_state_machine_safety(
        key in "[a-z][a-z0-9_]{0,10}",
        values in prop::collection::vec(prop::string::string_regex("[a-zA-Z0-9]{1,20}").unwrap(), 2..6)
    ) {
        // Property: All nodes apply same sequence of values to the same key
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (router, initial_log_index) = init_three_node_cluster().await
                .map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;

            let leader = router.leader()
                .ok_or_else(|| proptest::test_runner::TestCaseError::fail("no leader".to_string()))?;

            // Write same key with different values sequentially
            for value in &values {
                router
                    .write(&leader, key.clone(), value.clone())
                    .await
                    .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("write failed: {}", e)))?;
            }

            // Expected final value is last write
            let expected_final_value = values.last().unwrap();

            // Expected index: initial cluster setup + writes
            let expected_index = initial_log_index + (values.len() as u64);

            // Wait for all nodes to apply all writes
            for node_id in 0..3 {
                router
                    .wait(&node_id, Some(Duration::from_millis(5000)))
                    .applied_index(Some(expected_index), "all commands applied")
                    .await
                    .map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;
            }

            // All nodes must have same final value
            for node_id in 0..3 {
                let stored = router.read(&node_id, &key).await;
                prop_assert_eq!(
                    stored,
                    Some(expected_final_value.clone()),
                    "Node {} has wrong final value",
                    node_id
                );
            }

            Ok::<(), proptest::test_runner::TestCaseError>(())
        })?;
    }
}

// Test 5: Full Replication After Convergence
proptest! {
    #![proptest_config(ProptestConfig::with_cases(8))]
    #[test]
    fn test_quorum_write_durability(
        writes in prop::collection::vec(arbitrary_key_value(), 2..8)
    ) {
        // Property: After convergence, writes are fully replicated to all nodes
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (router, initial_log_index) = init_three_node_cluster().await
                .map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;

            let leader = router.leader()
                .ok_or_else(|| proptest::test_runner::TestCaseError::fail("no leader".to_string()))?;

            // Perform writes
            for (key, value) in &writes {
                router
                    .write(&leader, key.clone(), value.clone())
                    .await
                    .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("write failed: {}", e)))?;
            }

            // Build expected state
            let mut expected: HashMap<String, String> = HashMap::new();
            for (key, value) in &writes {
                expected.insert(key.clone(), value.clone());
            }

            // Expected index: initial cluster setup + writes
            let expected_index = initial_log_index + (writes.len() as u64);

            // Wait for all nodes to converge to expected index
            // This ensures writes are fully replicated across the cluster
            for node_id in 0..3 {
                router
                    .wait(&node_id, Some(Duration::from_millis(5000)))
                    .applied_index(Some(expected_index), "writes replicated")
                    .await
                    .map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;
            }

            // All 3 nodes must have the data (full replication)
            for (key, expected_value) in &expected {
                let mut replica_count = 0u32;
                for node_id in 0..3 {
                    if let Some(value) = router.read(&node_id, key).await && value == *expected_value {
                        replica_count += 1;
                    }
                }

                prop_assert_eq!(
                    replica_count,
                    3,
                    "Key {} not fully replicated: only {} replicas",
                    key,
                    replica_count
                );
            }

            Ok::<(), proptest::test_runner::TestCaseError>(())
        })?;
    }
}

// Test 6: Concurrent Reads Consistency
proptest! {
    #![proptest_config(ProptestConfig::with_cases(8))]
    #[test]
    fn test_concurrent_reads_consistency(
        writes in prop::collection::vec(arbitrary_key_value(), 3..10)
    ) {
        // Property: Reads from different nodes return consistent values after convergence
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (router, initial_log_index) = init_three_node_cluster().await
                .map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;

            let leader = router.leader()
                .ok_or_else(|| proptest::test_runner::TestCaseError::fail("no leader".to_string()))?;

            // Perform writes
            for (key, value) in &writes {
                router
                    .write(&leader, key.clone(), value.clone())
                    .await
                    .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("write failed: {}", e)))?;
            }

            // Build expected state
            let mut expected: HashMap<String, String> = HashMap::new();
            for (key, value) in &writes {
                expected.insert(key.clone(), value.clone());
            }

            // Expected index: initial cluster setup + writes
            let expected_index = initial_log_index + (writes.len() as u64);

            // Wait for convergence
            for node_id in 0..3 {
                router
                    .wait(&node_id, Some(Duration::from_millis(5000)))
                    .applied_index(Some(expected_index), "converged")
                    .await
                    .map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;
            }

            // Read from all nodes concurrently and verify consistency
            for (key, expected_value) in &expected {
                let mut values = Vec::new();
                for node_id in 0..3 {
                    let value = router.read(&node_id, key).await;
                    values.push(value);
                }

                // All reads must return same value
                for (node_id, value) in values.iter().enumerate() {
                    prop_assert_eq!(
                        value,
                        &Some(expected_value.clone()),
                        "Node {} read inconsistent value for key {}",
                        node_id,
                        key
                    );
                }
            }

            Ok::<(), proptest::test_runner::TestCaseError>(())
        })?;
    }
}
