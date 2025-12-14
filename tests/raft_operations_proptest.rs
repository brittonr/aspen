/// Property-based tests for Raft cluster operations using proptest.
///
/// This module verifies Raft protocol invariants through property testing:
/// - Write operation correctness and consistency
/// - Log index monotonicity across nodes
/// - Leader election safety properties
/// - Data consistency after concurrent writes
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use aspen::raft::types::NodeId;
use aspen::testing::AspenRouter;
use aspen::testing::create_test_raft_member_info;
use openraft::{Config, ServerState};
use proptest::prelude::*;

// Helper to create key-value generators
fn arbitrary_key_value() -> impl Strategy<Value = (String, String)> {
    (
        "[a-z][a-z0-9_]{0,15}",                                    // Key: 1-16 chars
        prop::string::string_regex("[a-zA-Z0-9 ]{1,50}").unwrap(), // Value: 1-50 chars
    )
}

// Helper to initialize a single-node cluster
async fn init_single_node_cluster() -> anyhow::Result<AspenRouter> {
    let config = Arc::new(
        Config {
            enable_tick: false,
            ..Default::default()
        }
        .validate()?,
    );

    let mut router = AspenRouter::new(config);
    router.new_raft_node(NodeId(0)).await?;

    let node0 = router.get_raft_handle(NodeId(0))?;
    let mut nodes = BTreeMap::new();
    nodes.insert(NodeId(0), create_test_raft_member_info(NodeId(0)));
    node0.initialize(nodes).await?;

    router
        .wait(NodeId(0), Some(Duration::from_millis(2000)))
        .state(ServerState::Leader, "node 0 becomes leader")
        .await?;

    Ok(router)
}

// Test 1: Write Operations Preserve Order
proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]
    #[test]
    fn test_writes_preserve_insertion_order(
        writes in prop::collection::vec(arbitrary_key_value(), 1..10)
    ) {
        // Property: Sequential writes should be retrievable in the same order
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let router = init_single_node_cluster().await
                .map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;

            // Perform all writes
            for (key, value) in &writes {
                router
                    .write(NodeId(0), key.clone(), value.clone())
                    .await
                    .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("write failed: {}", e)))?;
            }

            // Wait for all writes to be applied
            let expected_index = (writes.len() + 1) as u64; // +1 for init
            router
                .wait(NodeId(0), Some(Duration::from_millis(2000)))
                .applied_index(Some(expected_index), "all writes applied")
                .await
                .map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;

            // Build expected state (last value wins for duplicate keys)
            let mut expected: std::collections::HashMap<String, String> =
                std::collections::HashMap::new();
            for (key, value) in &writes {
                expected.insert(key.clone(), value.clone());
            }

            // Verify all writes are present
            for (key, expected_value) in &expected {
                let stored = router.read(NodeId(0), key).await;
                prop_assert_eq!(
                    stored,
                    Some(expected_value.clone()),
                    "Key {} not found or wrong value",
                    key
                );
            }

            Ok::<(), proptest::test_runner::TestCaseError>(())
        })?;
    }
}

// Test 2: Log Index Monotonicity
proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]
    #[test]
    fn test_log_indices_monotonic(
        num_writes in 1usize..15usize
    ) {
        // Property: After N writes, the applied index should be N + 1 (including init)
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let router = init_single_node_cluster().await
                .map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;

            for i in 0..num_writes {
                let key = format!("key_{}", i);
                let value = format!("value_{}", i);

                router
                    .write(NodeId(0), key, value)
                    .await
                    .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("write failed: {}", e)))?;

                // Wait for this specific write to be applied
                let expected_index = (i + 2) as u64; // +1 for init, +1 for this write
                router
                    .wait(NodeId(0), Some(Duration::from_millis(1000)))
                    .applied_index(Some(expected_index), "write applied")
                    .await
                    .map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;
            }

            Ok::<(), proptest::test_runner::TestCaseError>(())
        })?;
    }
}

// Test 3: Leader Election Stability
proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]
    #[test]
    fn test_single_leader_stability(
        num_writes in 1usize..10usize
    ) {
        // Property: In a stable single-node cluster, node 0 should remain leader
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let router = init_single_node_cluster().await
                .map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;

            // Verify leader at start
            let initial_leader = router.leader();
            prop_assert_eq!(initial_leader, Some(NodeId(0)), "Initial leader should be node 0");

            // Perform writes
            for i in 0..num_writes {
                let key = format!("stable_key_{}", i);
                let value = format!("stable_value_{}", i);

                router
                    .write(NodeId(0), key, value)
                    .await
                    .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("write failed: {}", e)))?;

                // Leader should remain node 0
                let current_leader = router.leader();
                prop_assert_eq!(
                    current_leader,
                    Some(NodeId(0)),
                    "Leader changed during stable operation"
                );
            }

            Ok::<(), proptest::test_runner::TestCaseError>(())
        })?;
    }
}

// Test 4: Write Idempotency (Same Key Multiple Times)
proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]
    #[test]
    fn test_write_idempotency(
        key in "[a-z][a-z0-9_]{0,10}",
        values in prop::collection::vec(prop::string::string_regex("[a-zA-Z0-9]{1,20}").unwrap(), 2..8)
    ) {
        // Property: Writing to the same key multiple times should result in the last value
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let router = init_single_node_cluster().await
                .map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;

            // Write the same key with different values
            for value in &values {
                router
                    .write(0, key.clone(), value.clone())
                    .await
                    .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("write failed: {}", e)))?;
            }

            // Wait for all writes to complete
            let expected_index = (values.len() + 1) as u64; // +1 for init
            router
                .wait(0, Some(Duration::from_millis(2000)))
                .applied_index(Some(expected_index), "all writes applied")
                .await
                .map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;

            // Should have the last value
            let stored = router.read(0, &key).await;
            let expected_value = values.last().unwrap();
            prop_assert_eq!(
                stored,
                Some(expected_value.clone()),
                "Should have last written value"
            );

            Ok::<(), proptest::test_runner::TestCaseError>(())
        })?;
    }
}

// Test 5: Empty Key Behavior
// Note: This test verifies the system's consistent handling of empty keys.
// The current implementation allows empty keys (they are valid strings),
// so we test that empty key writes round-trip correctly.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]
    #[test]
    fn test_empty_key_behavior(
        value in prop::string::string_regex("[a-zA-Z0-9]{1,20}").unwrap()
    ) {
        // Property: Empty key writes should round-trip consistently
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let router = init_single_node_cluster().await
                .map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;

            // Write with empty key
            let result = router.write(0, String::new(), value.clone()).await;

            // Since the AspenRouter/testing infrastructure allows empty keys,
            // verify the write succeeds and the value can be read back
            result.map_err(|e| {
                proptest::test_runner::TestCaseError::fail(format!("empty key write failed: {}", e))
            })?;

            // Wait for the write to be applied
            router
                .wait(NodeId(0), Some(std::time::Duration::from_millis(2000)))
                .applied_index(Some(2), "empty key write applied")
                .await
                .map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;

            // Verify we can read back the empty key
            let stored = router.read(0, "").await;
            prop_assert_eq!(
                stored,
                Some(value),
                "Empty key should be readable after write"
            );

            // Verify the system is still functional for normal keys
            router
                .write(0, "normal_key".to_string(), "normal_value".to_string())
                .await
                .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("normal write failed: {}", e)))?;

            let normal_stored = router.read(0, "normal_key").await;
            prop_assert_eq!(
                normal_stored,
                Some("normal_value".to_string()),
                "Normal key should work alongside empty key"
            );

            Ok::<(), proptest::test_runner::TestCaseError>(())
        })?;
    }
}

// Test 6: Large Value Handling
proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]
    #[test]
    fn test_large_value_writes(
        key in "[a-z][a-z0-9_]{0,10}",
        value_size in 1000usize..5000usize
    ) {
        // Property: Raft should handle moderately large values correctly
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let router = init_single_node_cluster().await
                .map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;

            let large_value = "x".repeat(value_size);

            router
                .write(0, key.clone(), large_value.clone())
                .await
                .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("large value write failed: {}", e)))?;

            router
                .wait(0, Some(Duration::from_millis(3000)))
                .applied_index(Some(2), "large value write applied")
                .await
                .map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;

            let stored = router.read(0, &key).await;
            prop_assert_eq!(stored.clone(), Some(large_value.clone()), "Large value mismatch");
            prop_assert_eq!(
                stored.unwrap().len(),
                value_size,
                "Large value size mismatch"
            );

            Ok::<(), proptest::test_runner::TestCaseError>(())
        })?;
    }
}
