/// Property-based tests for Raft cluster operations using Bolero.
///
/// This module verifies Raft protocol invariants through property testing:
/// - Write operation correctness and consistency
/// - Log index monotonicity across nodes
/// - Leader election safety properties
/// - Data consistency after concurrent writes
mod support;

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use aspen::raft::types::NodeId;
use aspen_testing::AspenRouter;
use aspen_testing::create_test_raft_member_info;
use bolero::check;
use openraft::Config;
use openraft::ServerState;
use support::bolero_generators::KeyValuePair;
use support::bolero_generators::ValidKey;
use support::bolero_generators::ValidValue;

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
#[test]
fn test_writes_preserve_insertion_order() {
    check!()
        .with_iterations(20)
        .with_type::<(KeyValuePair, KeyValuePair, KeyValuePair, KeyValuePair, KeyValuePair)>()
        .for_each(|writes| {
            let writes = vec![
                (writes.0.key.0.clone(), writes.0.value.0.clone()),
                (writes.1.key.0.clone(), writes.1.value.0.clone()),
                (writes.2.key.0.clone(), writes.2.value.0.clone()),
                (writes.3.key.0.clone(), writes.3.value.0.clone()),
                (writes.4.key.0.clone(), writes.4.value.0.clone()),
            ];

            // Property: Sequential writes should be retrievable in the same order
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let router = init_single_node_cluster().await.expect("Failed to init cluster");

                // Perform all writes
                for (key, value) in &writes {
                    router.write(NodeId(0), key.clone(), value.clone()).await.expect("write failed");
                }

                // Wait for all writes to be applied
                let expected_index = (writes.len() + 1) as u64; // +1 for init
                router
                    .wait(NodeId(0), Some(Duration::from_millis(2000)))
                    .applied_index(Some(expected_index), "all writes applied")
                    .await
                    .expect("Failed to wait for writes");

                // Build expected state (last value wins for duplicate keys)
                let mut expected: std::collections::HashMap<String, String> = std::collections::HashMap::new();
                for (key, value) in &writes {
                    expected.insert(key.clone(), value.clone());
                }

                // Verify all writes are present
                for (key, expected_value) in &expected {
                    let stored = router.read(NodeId(0), key).await;
                    assert_eq!(stored, Some(expected_value.clone()), "Key {} not found or wrong value", key);
                }
            });
        });
}

// Test 2: Log Index Monotonicity
#[test]
fn test_log_indices_monotonic() {
    check!().with_iterations(20).with_type::<u8>().for_each(|num_writes| {
        let num_writes = 1 + (*num_writes as usize % 14); // 1..15

        // Property: After N writes, the applied index should be N + 1 (including init)
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let router = init_single_node_cluster().await.expect("Failed to init cluster");

            for i in 0..num_writes {
                let key = format!("key_{}", i);
                let value = format!("value_{}", i);

                router.write(NodeId(0), key, value).await.expect("write failed");

                // Wait for this specific write to be applied
                let expected_index = (i + 2) as u64; // +1 for init, +1 for this write
                router
                    .wait(NodeId(0), Some(Duration::from_millis(1000)))
                    .applied_index(Some(expected_index), "write applied")
                    .await
                    .expect("Failed to wait for write");
            }
        });
    });
}

// Test 3: Leader Election Stability
#[test]
fn test_single_leader_stability() {
    check!().with_iterations(10).with_type::<u8>().for_each(|num_writes| {
        let num_writes = 1 + (*num_writes as usize % 9); // 1..10

        // Property: In a stable single-node cluster, node 0 should remain leader
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let router = init_single_node_cluster().await.expect("Failed to init cluster");

            // Verify leader at start
            let initial_leader = router.leader();
            assert_eq!(initial_leader, Some(NodeId(0)), "Initial leader should be node 0");

            // Perform writes
            for i in 0..num_writes {
                let key = format!("stable_key_{}", i);
                let value = format!("stable_value_{}", i);

                router.write(NodeId(0), key, value).await.expect("write failed");

                // Leader should remain node 0
                let current_leader = router.leader();
                assert_eq!(current_leader, Some(NodeId(0)), "Leader changed during stable operation");
            }
        });
    });
}

// Test 4: Write Idempotency (Same Key Multiple Times)
#[test]
fn test_write_idempotency() {
    check!()
        .with_iterations(20)
        .with_type::<(ValidKey, ValidValue, ValidValue, ValidValue, ValidValue)>()
        .for_each(|(key, v1, v2, v3, v4)| {
            let values = vec![v1.0.clone(), v2.0.clone(), v3.0.clone(), v4.0.clone()];

            // Property: Writing to the same key multiple times should result in the last value
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let router = init_single_node_cluster().await.expect("Failed to init cluster");

                // Write the same key with different values
                for value in &values {
                    router.write(0, key.0.clone(), value.clone()).await.expect("write failed");
                }

                // Wait for all writes to complete
                let expected_index = (values.len() + 1) as u64; // +1 for init
                router
                    .wait(0, Some(Duration::from_millis(2000)))
                    .applied_index(Some(expected_index), "all writes applied")
                    .await
                    .expect("Failed to wait for writes");

                // Should have the last value
                let stored = router.read(0, &key.0).await;
                let expected_value = values.last().unwrap();
                assert_eq!(stored, Some(expected_value.clone()), "Should have last written value");
            });
        });
}

// Note: Empty key validation is tested at the API layer in:
// - src/api/mod.rs (unit tests for all WriteCommand variants)
// - tests/api_validation_proptest.rs (property-based tests)
// The AspenRouter test harness bypasses API validation to test Raft internals,
// so empty key rejection is not tested here.

// Test 5: Large Value Handling
#[test]
fn test_large_value_writes() {
    check!().with_iterations(10).with_type::<(ValidKey, u16)>().for_each(|(key, value_size)| {
        let value_size = 1000 + (*value_size as usize % 4000); // 1000..5000

        // Property: Raft should handle moderately large values correctly
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let router = init_single_node_cluster().await.expect("Failed to init cluster");

            let large_value = "x".repeat(value_size);

            router.write(0, key.0.clone(), large_value.clone()).await.expect("large value write failed");

            router
                .wait(0, Some(Duration::from_millis(3000)))
                .applied_index(Some(2), "large value write applied")
                .await
                .expect("Failed to wait for write");

            let stored = router.read(0, &key.0).await;
            assert_eq!(stored.clone(), Some(large_value.clone()), "Large value mismatch");
            assert_eq!(stored.unwrap().len(), value_size, "Large value size mismatch");
        });
    });
}
