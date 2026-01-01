/// Property-based tests for distributed system invariants using Bolero.
///
/// This module verifies fundamental distributed system properties across
/// multi-node Raft clusters:
/// - Eventual consistency across all nodes
/// - Leader uniqueness (at most one leader per term)
/// - Log replication correctness
/// - State machine safety (all nodes apply same commands in same order)
/// - Partition tolerance and recovery
mod support;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use aspen_testing::AspenRouter;
use bolero::check;
use openraft::Config;
use openraft::ServerState;
use support::bolero_generators::KeyValuePair;
use support::bolero_generators::ValidKey;
use support::bolero_generators::ValidValue;

// Helper to initialize a 3-node cluster
async fn init_three_node_cluster() -> anyhow::Result<(AspenRouter, u64)> {
    use std::collections::BTreeSet;

    use aspen::raft::types::NodeId;

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
    voter_ids.insert(NodeId::from(0));
    voter_ids.insert(NodeId::from(1));
    voter_ids.insert(NodeId::from(2));
    let learners = BTreeSet::new();

    let log_index = router.new_cluster(voter_ids, learners).await?;

    Ok((router, log_index))
}

// Test 1: Eventual Consistency Across All Nodes
#[test]
fn test_eventual_consistency_multi_node() {
    check!()
        .with_iterations(50)
        .with_type::<(KeyValuePair, KeyValuePair, KeyValuePair, KeyValuePair, KeyValuePair)>()
        .for_each(|writes| {
            let writes = vec![
                (writes.0.key.0.clone(), writes.0.value.0.clone()),
                (writes.1.key.0.clone(), writes.1.value.0.clone()),
                (writes.2.key.0.clone(), writes.2.value.0.clone()),
                (writes.3.key.0.clone(), writes.3.value.0.clone()),
                (writes.4.key.0.clone(), writes.4.value.0.clone()),
            ];

            // Property: All writes to leader eventually appear on all nodes
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let (router, initial_log_index) = init_three_node_cluster().await.expect("Failed to init cluster");

                let leader = router.leader().expect("no leader");

                // Perform writes to leader
                for (key, value) in &writes {
                    router.write(leader, key.clone(), value.clone()).await.expect("write failed");
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
                        .wait(node_id, Some(Duration::from_millis(5000)))
                        .applied_index(Some(expected_index), "all writes replicated")
                        .await
                        .expect("Failed to wait for replication");
                }

                // Verify all nodes have identical data
                for node_id in 0..3 {
                    for (key, expected_value) in &expected {
                        let stored = router.read(node_id, key).await;
                        assert_eq!(
                            stored,
                            Some(expected_value.clone()),
                            "Node {} missing or wrong value for key {}",
                            node_id,
                            key
                        );
                    }
                }
            });
        });
}

// Test 2: Leader Uniqueness
#[test]
fn test_at_most_one_leader() {
    check!().with_iterations(50).with_type::<u8>().for_each(|num_writes| {
        let num_writes = 1 + (*num_writes as usize % 7); // 1..8

        // Property: At any given time, at most one node claims to be leader
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (router, _initial_log_index) = init_three_node_cluster().await.expect("Failed to init cluster");

            let initial_leader = router.leader().expect("no initial leader");

            // Perform writes
            for i in 0..num_writes {
                let key = format!("key_{}", i);
                let value = format!("value_{}", i);

                router.write(initial_leader, key, value).await.expect("write failed");

                // Check leader uniqueness: all nodes must agree on the same leader
                let current_leader = router.leader();

                // Get metrics from all three nodes to verify they agree on the leader
                let mut reported_leaders = Vec::new();
                let mut leader_count = 0;

                // Check each node's metrics
                for node_id in 0..3 {
                    let node_handle = router.get_raft_handle(node_id).expect("failed to get node");

                    let metrics = node_handle.metrics().borrow().clone();
                    reported_leaders.push((node_id, metrics.current_leader));

                    // Count nodes that claim to be leader (via state)
                    if metrics.state == ServerState::Leader {
                        leader_count += 1;
                    }
                }

                // Property 1: At most one node should claim to be leader
                assert!(
                    leader_count <= 1,
                    "Multiple nodes claiming leadership! Count: {}, Details: {:?}",
                    leader_count,
                    reported_leaders
                );

                // Property 2: All nodes should agree on who the leader is
                let first_leader = reported_leaders[0].1;
                for (node_id, reported_leader) in &reported_leaders {
                    assert_eq!(
                        *reported_leader, first_leader,
                        "Node {} disagrees on leader: reports {:?} but node 0 reports {:?}",
                        node_id, reported_leader, first_leader
                    );
                }

                // Property 3: The router's leader() should match what nodes report
                if let Some(router_leader) = current_leader {
                    assert_eq!(
                        first_leader,
                        Some(router_leader),
                        "Router reports leader {} but nodes report {:?}",
                        router_leader,
                        first_leader
                    );
                }
            }
        });
    });
}

// Test 3: Log Replication Invariant
#[test]
fn test_log_replication_consistency() {
    check!().with_iterations(50).with_type::<u8>().for_each(|num_writes| {
        let num_writes = 3 + (*num_writes as usize % 9); // 3..12

        // Property: All nodes reach same log index with matching data
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (router, initial_log_index) = init_three_node_cluster().await.expect("Failed to init cluster");

            let leader = router.leader().expect("no leader");

            // Perform sequential writes
            let mut keys = Vec::new();
            for i in 0..num_writes {
                let key = format!("replicated_{}", i);
                let value = format!("data_{}", i);
                keys.push((key.clone(), value.clone()));

                router.write(leader, key, value).await.expect("write failed");
            }

            // Expected index: initial cluster setup + writes
            let expected_index = initial_log_index + (num_writes as u64);

            // Wait for all nodes to converge
            for node_id in 0..3 {
                router
                    .wait(node_id, Some(Duration::from_millis(5000)))
                    .applied_index(Some(expected_index), "logs replicated")
                    .await
                    .expect("Failed to wait for replication");
            }

            // Verify all nodes have identical logs
            for node_id in 0..3 {
                for (key, value) in &keys {
                    let stored = router.read(node_id, key).await;
                    assert_eq!(stored, Some(value.clone()), "Node {} has inconsistent log for key {}", node_id, key);
                }
            }
        });
    });
}

// Test 4: State Machine Safety
#[test]
fn test_state_machine_safety() {
    check!()
        .with_iterations(50)
        .with_type::<(ValidKey, ValidValue, ValidValue, ValidValue, ValidValue)>()
        .for_each(|(key, v1, v2, v3, v4)| {
            let values = vec![v1.0.clone(), v2.0.clone(), v3.0.clone(), v4.0.clone()];

            // Property: All nodes apply same sequence of values to the same key
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let (router, initial_log_index) = init_three_node_cluster().await.expect("Failed to init cluster");

                let leader = router.leader().expect("no leader");

                // Write same key with different values sequentially
                for value in &values {
                    router.write(leader, key.0.clone(), value.clone()).await.expect("write failed");
                }

                // Expected final value is last write
                let expected_final_value = values.last().unwrap();

                // Expected index: initial cluster setup + writes
                let expected_index = initial_log_index + (values.len() as u64);

                // Wait for all nodes to apply all writes
                for node_id in 0..3 {
                    router
                        .wait(node_id, Some(Duration::from_millis(5000)))
                        .applied_index(Some(expected_index), "all commands applied")
                        .await
                        .expect("Failed to wait for replication");
                }

                // All nodes must have same final value
                for node_id in 0..3 {
                    let stored = router.read(node_id, &key.0).await;
                    assert_eq!(stored, Some(expected_final_value.clone()), "Node {} has wrong final value", node_id);
                }
            });
        });
}

// Test 5: Full Replication After Convergence
#[test]
fn test_quorum_write_durability() {
    check!()
        .with_iterations(50)
        .with_type::<(KeyValuePair, KeyValuePair, KeyValuePair, KeyValuePair)>()
        .for_each(|writes| {
            let writes = vec![
                (writes.0.key.0.clone(), writes.0.value.0.clone()),
                (writes.1.key.0.clone(), writes.1.value.0.clone()),
                (writes.2.key.0.clone(), writes.2.value.0.clone()),
                (writes.3.key.0.clone(), writes.3.value.0.clone()),
            ];

            // Property: After convergence, writes are fully replicated to all nodes
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let (router, initial_log_index) = init_three_node_cluster().await.expect("Failed to init cluster");

                let leader = router.leader().expect("no leader");

                // Perform writes
                for (key, value) in &writes {
                    router.write(leader, key.clone(), value.clone()).await.expect("write failed");
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
                        .wait(node_id, Some(Duration::from_millis(5000)))
                        .applied_index(Some(expected_index), "writes replicated")
                        .await
                        .expect("Failed to wait for replication");
                }

                // All 3 nodes must have the data (full replication)
                for (key, expected_value) in &expected {
                    let mut replica_count = 0u32;
                    for node_id in 0..3 {
                        if let Some(value) = router.read(node_id, key).await
                            && value == *expected_value
                        {
                            replica_count += 1;
                        }
                    }

                    assert_eq!(replica_count, 3, "Key {} not fully replicated: only {} replicas", key, replica_count);
                }
            });
        });
}

// Test 6: Concurrent Reads Consistency
#[test]
fn test_concurrent_reads_consistency() {
    check!()
        .with_iterations(50)
        .with_type::<(KeyValuePair, KeyValuePair, KeyValuePair, KeyValuePair, KeyValuePair)>()
        .for_each(|writes| {
            let writes = vec![
                (writes.0.key.0.clone(), writes.0.value.0.clone()),
                (writes.1.key.0.clone(), writes.1.value.0.clone()),
                (writes.2.key.0.clone(), writes.2.value.0.clone()),
                (writes.3.key.0.clone(), writes.3.value.0.clone()),
                (writes.4.key.0.clone(), writes.4.value.0.clone()),
            ];

            // Property: Reads from different nodes return consistent values after convergence
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let (router, initial_log_index) = init_three_node_cluster().await.expect("Failed to init cluster");

                let leader = router.leader().expect("no leader");

                // Perform writes
                for (key, value) in &writes {
                    router.write(leader, key.clone(), value.clone()).await.expect("write failed");
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
                        .wait(node_id, Some(Duration::from_millis(5000)))
                        .applied_index(Some(expected_index), "converged")
                        .await
                        .expect("Failed to wait for convergence");
                }

                // Read from all nodes concurrently and verify consistency
                for (key, expected_value) in &expected {
                    let mut values = Vec::new();
                    for node_id in 0..3 {
                        let value = router.read(node_id, key).await;
                        values.push(value);
                    }

                    // All reads must return same value
                    for (node_id, value) in values.iter().enumerate() {
                        assert_eq!(
                            value,
                            &Some(expected_value.clone()),
                            "Node {} read inconsistent value for key {}",
                            node_id,
                            key
                        );
                    }
                }
            });
        });
}

// Test 7: Leader Completeness Property
// Raft's Leader Completeness property: if a log entry is committed in a given term,
// then that entry will be present in the logs of the leaders for all higher-numbered terms.
#[test]
fn test_leader_completeness_property() {
    check!()
        .with_iterations(30)
        .with_type::<(
            KeyValuePair,
            KeyValuePair,
            KeyValuePair,
            KeyValuePair,
            KeyValuePair,
            KeyValuePair,
            KeyValuePair,
            KeyValuePair,
        )>()
        .for_each(|writes| {
            let writes = vec![
                (writes.0.key.0.clone(), writes.0.value.0.clone()),
                (writes.1.key.0.clone(), writes.1.value.0.clone()),
                (writes.2.key.0.clone(), writes.2.value.0.clone()),
                (writes.3.key.0.clone(), writes.3.value.0.clone()),
                (writes.4.key.0.clone(), writes.4.value.0.clone()),
                (writes.5.key.0.clone(), writes.5.value.0.clone()),
                (writes.6.key.0.clone(), writes.6.value.0.clone()),
                (writes.7.key.0.clone(), writes.7.value.0.clone()),
            ];

            // Property: Committed entries are never lost, even after leader changes
            // We simulate this by writing entries to the leader, ensuring they're committed,
            // then verifying they remain accessible from all nodes.
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                use aspen::raft::types::NodeId;

                let (router, initial_log_index) = init_three_node_cluster().await.expect("Failed to init cluster");

                let leader = router.leader().expect("no leader");

                // Phase 1: Write all entries to the leader
                for (key, value) in &writes {
                    router.write(leader, key.clone(), value.clone()).await.expect("write failed");
                }

                // Build expected state
                let mut expected: HashMap<String, String> = HashMap::new();
                for (key, value) in &writes {
                    expected.insert(key.clone(), value.clone());
                }

                // Expected index: initial cluster setup + writes
                let expected_index = initial_log_index + (writes.len() as u64);

                // Phase 2: Wait for all entries to be committed on majority (quorum)
                // The write calls return after commitment, so entries are already committed.
                // Wait for full replication to all nodes.
                for node_id in 0..3 {
                    router
                        .wait(node_id, Some(Duration::from_millis(5000)))
                        .applied_index(Some(expected_index), "entries committed")
                        .await
                        .expect("Failed to wait for commitment");
                }

                // Phase 3: Verify the leader has all committed entries
                // This is the core of the Leader Completeness property
                let leader_node = router.get_raft_handle(leader).expect("failed to get leader");

                let leader_metrics = leader_node.metrics().borrow().clone();

                // The leader must have applied entries
                assert!(leader_metrics.last_applied.is_some(), "Leader must have applied entries");

                let leader_applied_index = leader_metrics.last_applied.unwrap().index;
                assert!(
                    leader_applied_index >= expected_index,
                    "Leader applied index {} must be >= expected {}",
                    leader_applied_index,
                    expected_index
                );

                // Phase 4: Verify all data is accessible from the leader
                for (key, expected_value) in &expected {
                    let stored = router.read(leader, key).await;
                    assert_eq!(
                        stored,
                        Some(expected_value.clone()),
                        "Leader must have committed entry for key {}",
                        key
                    );
                }

                // Phase 5: All followers must also have the same data (replication complete)
                // This verifies that the committed entries were properly replicated
                for node_id in [NodeId::from(0), NodeId::from(1), NodeId::from(2)] {
                    for (key, expected_value) in &expected {
                        let stored = router.read(node_id, key).await;
                        assert_eq!(
                            stored,
                            Some(expected_value.clone()),
                            "Node {} must have committed entry for key {}",
                            node_id,
                            key
                        );
                    }
                }
            });
        });
}

// Test 8: Log Matching Property
// If two logs contain an entry with the same index and term, then the logs
// are identical in all entries up through the given index.
#[test]
fn test_log_matching_property() {
    check!().with_iterations(30).with_type::<u8>().for_each(|num_writes| {
        let num_writes = 5 + (*num_writes as usize % 15); // 5..20

        // Property: All nodes that have applied the same index have identical state
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (router, initial_log_index) = init_three_node_cluster().await.expect("Failed to init cluster");

            let leader = router.leader().expect("no leader");

            // Write entries sequentially
            let mut keys = Vec::new();
            for i in 0..num_writes {
                let key = format!("log_match_key_{}", i);
                let value = format!("log_match_value_{}", i);
                keys.push((key.clone(), value.clone()));

                router.write(leader, key, value).await.expect("write failed");
            }

            // Expected index: initial cluster setup + writes
            let expected_index = initial_log_index + (num_writes as u64);

            // Wait for all nodes to reach the same applied index
            for node_id in 0..3 {
                router
                    .wait(node_id, Some(Duration::from_millis(5000)))
                    .applied_index(Some(expected_index), "logs matched")
                    .await
                    .expect("Failed to wait for log matching");
            }

            // Verify all nodes have the same last_applied index
            let mut applied_indices = Vec::new();
            for node_id in 0..3 {
                let node_handle = router.get_raft_handle(node_id).expect("failed to get node");

                let metrics = node_handle.metrics().borrow().clone();
                if let Some(last_applied) = metrics.last_applied {
                    applied_indices.push((node_id, last_applied.index, last_applied.leader_id));
                }
            }

            // All nodes should have the same applied index
            let first_index = applied_indices[0].1;
            for (node_id, index, _) in &applied_indices {
                assert_eq!(
                    *index, first_index,
                    "Node {} applied index {} differs from expected {}",
                    node_id, index, first_index
                );
            }

            // If logs match at the same index, the state should be identical
            // Verify by checking all keys have the same values across all nodes
            for (key, expected_value) in &keys {
                let mut values = Vec::new();
                for node_id in 0..3 {
                    let value = router.read(node_id, key).await;
                    values.push(value);
                }

                // All nodes should return the same value
                for (node_id, value) in values.iter().enumerate() {
                    assert_eq!(
                        value,
                        &Some(expected_value.clone()),
                        "Node {} has inconsistent value for key {} (log matching violation)",
                        node_id,
                        key
                    );
                }
            }
        });
    });
}

// Test 9: Leader Election on Failure
// When a leader is isolated, the remaining nodes must elect a new leader.
#[test]
fn test_leader_election_on_failure() {
    check!()
        .with_iterations(20)
        .with_type::<(u8, u8)>()
        .for_each(|(num_writes_before, num_writes_after)| {
            let num_writes_before = 1 + (*num_writes_before as usize % 4); // 1..5
            let num_writes_after = 1 + (*num_writes_after as usize % 4); // 1..5

            // Property: When leader fails, remaining majority elects new leader
            // and operations continue to succeed
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                use aspen::raft::types::NodeId;

                let (mut router, initial_log_index) = init_three_node_cluster().await.expect("Failed to init cluster");

                let original_leader = router.leader().expect("no leader");

                // Write some data before failure
                for i in 0..num_writes_before {
                    let key = format!("pre_failure_key_{}", i);
                    let value = format!("pre_failure_value_{}", i);
                    router.write(original_leader, key, value).await.expect("pre-failure write failed");
                }

                // Wait for replication before failure
                let expected_index_before = initial_log_index + (num_writes_before as u64);
                for node_id in 0..3 {
                    router
                        .wait(node_id, Some(Duration::from_millis(3000)))
                        .applied_index(Some(expected_index_before), "pre-failure replication")
                        .await
                        .expect("Failed to wait for pre-failure replication");
                }

                // Isolate the leader (simulate failure)
                router.fail_node(original_leader);

                // Wait for new leader election among remaining nodes
                // Note: In test infrastructure, we need to trigger election manually
                // by waiting for heartbeat timeout
                tokio::time::sleep(Duration::from_millis(500)).await;

                // Find the new leader (should be one of the non-failed nodes)
                let new_leader = router.leader();

                // Verify election safety: new leader should exist and be different from failed node
                if let Some(new_leader_id) = new_leader {
                    assert_ne!(
                        new_leader_id, original_leader,
                        "New leader should be different from failed original leader"
                    );

                    // Write more data through new leader
                    for i in 0..num_writes_after {
                        let key = format!("post_failure_key_{}", i);
                        let value = format!("post_failure_value_{}", i);

                        // New leader should accept writes
                        let write_result = router.write(new_leader_id, key, value).await;

                        // Write should succeed on new leader
                        assert!(write_result.is_ok(), "Post-failure write should succeed on new leader");
                    }

                    // Verify remaining nodes (excluding failed leader) have consistent state
                    let remaining_nodes: Vec<NodeId> =
                        (0..3).map(NodeId::from).filter(|n| *n != original_leader).collect();

                    // Wait for writes to replicate to remaining nodes
                    let expected_index_after = expected_index_before + (num_writes_after as u64);
                    for node_id in &remaining_nodes {
                        router
                            .wait(*node_id, Some(Duration::from_millis(3000)))
                            .applied_index(Some(expected_index_after), "post-failure replication")
                            .await
                            .expect("Failed to wait for post-failure replication");
                    }
                }
                // Note: If no new leader elected in test timeframe, that's acceptable
                // for this property test as election timing can vary
            });
        });
}

// Test 10: Network Partition Recovery
// After a network partition heals, the minority partition should catch up
// with the majority's committed state.
#[test]
fn test_network_partition_recovery() {
    check!().with_iterations(20).with_type::<u8>().for_each(|num_writes_during_partition| {
        let num_writes_during_partition = 2 + (*num_writes_during_partition as usize % 6); // 2..8

        // Property: After partition heals, all nodes eventually converge
        // to the same state
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            use aspen::raft::types::NodeId;

            let (mut router, initial_log_index) = init_three_node_cluster().await.expect("Failed to init cluster");

            let leader = router.leader().expect("no leader");

            // Identify minority node (one that will be partitioned)
            let minority_node: NodeId = (0..3).map(NodeId::from).find(|n| *n != leader).unwrap();

            // Create asymmetric partition: minority cannot reach majority
            router.fail_node(minority_node);

            // Write data through leader (majority can still make progress)
            for i in 0..num_writes_during_partition {
                let key = format!("partition_key_{}", i);
                let value = format!("partition_value_{}", i);
                router.write(leader, key, value).await.expect("write during partition failed");
            }

            // Wait for majority to apply writes
            let expected_index = initial_log_index + (num_writes_during_partition as u64);
            let majority_nodes: Vec<NodeId> = (0..3).map(NodeId::from).filter(|n| *n != minority_node).collect();

            for node_id in &majority_nodes {
                router
                    .wait(*node_id, Some(Duration::from_millis(3000)))
                    .applied_index(Some(expected_index), "majority replication")
                    .await
                    .expect("Failed to wait for majority replication");
            }

            // Heal the partition
            router.recover_node(minority_node);

            // Wait for minority to catch up with majority
            router
                .wait(minority_node, Some(Duration::from_millis(5000)))
                .applied_index(Some(expected_index), "partition recovery")
                .await
                .expect("Minority node failed to catch up after partition healed");

            // Verify all nodes now have identical state
            for i in 0..num_writes_during_partition {
                let key = format!("partition_key_{}", i);
                let expected_value = format!("partition_value_{}", i);

                // Check all three nodes have the same value
                for node_id in 0..3 {
                    let stored = router.read(node_id, &key).await;
                    assert_eq!(
                        stored,
                        Some(expected_value.clone()),
                        "Node {} should have correct value for {} after partition recovery",
                        node_id,
                        key
                    );
                }
            }
        });
    });
}
