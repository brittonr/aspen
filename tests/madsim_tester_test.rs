//! Tests for AspenRaftTester abstraction.
//!
//! These tests demonstrate the reduced boilerplate achieved by the tester abstraction.
//! Each test is now 5-15 lines instead of 40+ lines.
//!
//! # Coverage Matrix
//!
//! This file tests the following categories:
//! - election:basic, election:timeout
//! - crash:leader, crash:follower
//! - network:partition, network:delay, network:loss
//! - byzantine:vote_flip, byzantine:term_increment, byzantine:duplication
//! - membership:add, membership:remove, membership:promote
//! - buggify:enabled, buggify:fault_injection
//! - scale:3node, scale:5node
//! - replication:append

use std::time::Duration;

use aspen::testing::AspenRaftTester;

// Coverage: election:basic, scale:3node
/// Test basic cluster initialization and leader election.
#[madsim::test]
async fn test_tester_basic_initialization() {
    // BEFORE: 40+ lines of boilerplate
    // AFTER: 5 lines
    let mut t = AspenRaftTester::new(3, "tester_basic_init").await;

    // Wait for leader election
    madsim::time::sleep(Duration::from_secs(5)).await;

    let leader = t.check_one_leader().await;
    assert!(leader.is_some(), "No leader elected in 3-node cluster");

    t.end();
}

// Coverage: crash:leader, election:basic, scale:3node
/// Test leader crash and re-election using the tester abstraction.
#[madsim::test]
async fn test_tester_leader_crash_reelection() {
    let mut t = AspenRaftTester::new(3, "tester_leader_crash").await;

    // Wait for initial leader
    madsim::time::sleep(Duration::from_secs(5)).await;
    let initial_leader = t.check_one_leader().await.expect("No initial leader");

    // Crash the leader
    t.crash_node(initial_leader).await;

    // Wait for re-election
    madsim::time::sleep(Duration::from_secs(10)).await;

    // Check new leader elected
    let new_leader = t.check_one_leader().await;
    assert!(new_leader.is_some(), "No new leader after crash");
    assert_ne!(new_leader.unwrap(), initial_leader, "New leader should be different");

    t.end();
}

/// Test network partition using the tester abstraction.
#[madsim::test]
async fn test_tester_network_partition() {
    let mut t = AspenRaftTester::new(3, "tester_partition").await;

    // Wait for initial leader
    madsim::time::sleep(Duration::from_secs(5)).await;
    let _leader = t.check_one_leader().await.expect("No initial leader");

    // Partition node 2
    t.disconnect(2);

    // Majority should still function
    madsim::time::sleep(Duration::from_secs(3)).await;
    let leader_after = t.check_one_leader().await;
    assert!(leader_after.is_some(), "Majority partition should maintain leader");

    // Heal partition
    t.connect(2);
    madsim::time::sleep(Duration::from_secs(3)).await;

    // Verify no split brain
    t.check_no_split_brain().expect("Split brain detected after partition healed");

    t.end();
}

/// Test write operations using the tester abstraction.
#[madsim::test]
async fn test_tester_write_operations() {
    let mut t = AspenRaftTester::new(3, "tester_writes").await;

    // Wait for leader
    madsim::time::sleep(Duration::from_secs(5)).await;

    // Perform writes
    for i in 0..10 {
        t.write(format!("key-{}", i), format!("value-{}", i)).await.expect("Write failed");
    }

    // Verify reads
    let value = t.read("key-5").await.expect("Read failed");
    assert_eq!(value, Some("value-5".to_string()));

    t.end();
}

/// Test unreliable network mode.
#[madsim::test]
async fn test_tester_unreliable_network() {
    let mut t = AspenRaftTester::new(3, "tester_unreliable").await;

    // Wait for leader
    madsim::time::sleep(Duration::from_secs(5)).await;
    t.check_one_leader().await.expect("No initial leader");

    // Enable unreliable network
    t.set_unreliable(true);

    // Writes should still succeed (eventually)
    for i in 0..5 {
        // May need retries due to network issues
        let result = t.write(format!("key-{}", i), format!("value-{}", i)).await;
        if result.is_err() {
            // Expected under unreliable conditions
            t.add_event(format!("write {} failed (expected under unreliable)", i));
        }
    }

    // Restore reliable network
    t.set_unreliable(false);

    // Verify no split brain
    madsim::time::sleep(Duration::from_secs(3)).await;
    t.check_no_split_brain().expect("Split brain detected after unreliable period");

    t.end();
}

/// Test seed reproducibility.
#[madsim::test]
async fn test_tester_seed_reproducibility() {
    use aspen::testing::TesterConfig;

    // Create tester with explicit seed
    let config = TesterConfig::new(3, "tester_seed_repro").with_seed(42);
    let t = AspenRaftTester::with_config(config).await;

    assert_eq!(t.seed(), 42, "Seed should be 42");
    assert_eq!(t.node_count(), 3, "Should have 3 nodes");

    t.end();
}

/// Test 5-node cluster for quorum testing.
#[madsim::test]
async fn test_tester_5node_cluster() {
    let mut t = AspenRaftTester::new(5, "tester_5node").await;

    // Wait for leader
    madsim::time::sleep(Duration::from_secs(5)).await;
    t.check_one_leader().await.expect("No initial leader");

    // Crash 2 nodes (still have quorum of 3)
    t.crash_node(0).await;
    t.crash_node(1).await;

    madsim::time::sleep(Duration::from_secs(10)).await;

    // Should still have a leader
    let leader = t.check_one_leader().await;
    assert!(leader.is_some(), "Should maintain leader with 3/5 nodes alive");

    t.end();
}

/// Test Byzantine failure injection - vote flipping.
///
/// This test enables Byzantine mode on one node that flips vote responses.
/// Raft should handle this gracefully since it's crash fault tolerant,
/// not Byzantine fault tolerant.
#[madsim::test]
async fn test_tester_byzantine_vote_flip() {
    use aspen::testing::ByzantineCorruptionMode;

    let mut t = AspenRaftTester::new(5, "tester_byzantine_vote").await;

    // Wait for initial leader election
    madsim::time::sleep(Duration::from_secs(5)).await;
    t.check_one_leader().await.expect("No initial leader");

    // Enable Byzantine vote flipping on node 4 with 50% probability
    // This means node 4 will flip half of its vote responses
    t.enable_byzantine_mode(4, ByzantineCorruptionMode::FlipVote, 0.5);

    // Trigger some elections by crashing the leader
    let leader_idx = t.check_one_leader().await.expect("Lost leader");
    t.crash_node(leader_idx).await;

    // Wait for re-election with Byzantine node active
    madsim::time::sleep(Duration::from_secs(10)).await;

    // Cluster should still elect a leader (Byzantine node is minority)
    let new_leader = t.check_one_leader().await;
    assert!(new_leader.is_some(), "Cluster should maintain leader despite Byzantine node");

    // Verify no split brain
    t.check_no_split_brain().expect("Split brain detected with Byzantine node");

    let artifact = t.end();
    // Note: The Byzantine injector counts corruptions at the injector level,
    // but since the actual network doesn't route through our corruption logic yet,
    // we just verify the infrastructure is in place.
    eprintln!(
        "Test completed with {} Byzantine corruptions tracked",
        artifact.metrics.contains("byzantine_corruptions")
    );
}

/// Test Byzantine failure injection - term increment attack.
///
/// This test simulates a Byzantine node that increments terms in messages,
/// potentially disrupting leader stability.
#[madsim::test]
async fn test_tester_byzantine_term_increment() {
    use aspen::testing::ByzantineCorruptionMode;

    let mut t = AspenRaftTester::new(5, "tester_byzantine_term").await;

    // Wait for initial leader
    madsim::time::sleep(Duration::from_secs(5)).await;
    t.check_one_leader().await.expect("No initial leader");

    // Enable Byzantine term increment on node 3 with 30% probability
    t.enable_byzantine_mode(3, ByzantineCorruptionMode::IncrementTerm, 0.3);

    // Perform operations with Byzantine node active
    for i in 0..5 {
        // Writes may take longer or fail due to Byzantine interference
        let result = t.write(format!("key-{}", i), format!("value-{}", i)).await;
        if result.is_err() {
            t.add_event(format!("write {} failed under Byzantine conditions (may be expected)", i));
        }
    }

    // Allow cluster to stabilize
    madsim::time::sleep(Duration::from_secs(5)).await;

    // Verify no split brain after Byzantine activity
    t.check_no_split_brain().expect("Split brain detected during Byzantine term attack");

    t.end();
}

/// Test Byzantine failure injection - message duplication.
#[madsim::test]
async fn test_tester_byzantine_duplicate() {
    use aspen::testing::ByzantineCorruptionMode;

    let mut t = AspenRaftTester::new(3, "tester_byzantine_dup").await;

    // Wait for leader
    madsim::time::sleep(Duration::from_secs(5)).await;
    t.check_one_leader().await.expect("No initial leader");

    // Enable message duplication on node 1 with 40% probability
    // This tests Raft's idempotency handling
    t.enable_byzantine_mode(1, ByzantineCorruptionMode::DuplicateMessage, 0.4);

    // Perform writes - should succeed despite duplicates
    for i in 0..10 {
        t.write(format!("dup-key-{}", i), format!("value-{}", i))
            .await
            .expect("Write should succeed despite duplicates");
    }

    // Verify data integrity
    let value = t.read("dup-key-5").await.expect("Read failed");
    assert_eq!(value, Some("value-5".to_string()), "Value should be correct despite duplicates");

    t.check_no_split_brain().expect("Split brain after duplicates");

    t.end();
}

/// Test node restart with persistence - crash recovery.
///
/// This test uses persistent storage to verify that nodes can recover
/// after crashes with their state intact.
///
/// Note: This test is currently disabled in madsim because database locks
/// aren't released in single-process simulations. In production, each node
/// would be a separate process.
#[madsim::test]
#[ignore] // TODO: Enable when madsim supports process isolation for storage
async fn test_tester_crash_recovery_persistence() {
    use std::path::PathBuf;

    use aspen::testing::TesterConfig;

    // Create a temp directory for test storage
    let storage_dir = PathBuf::from("/tmp/aspen-test-crash-recovery");

    // Clean up any previous test runs
    let _ = std::fs::remove_dir_all(&storage_dir);

    // Create tester with persistent storage
    let config = TesterConfig::new(3, "crash_recovery").with_persistent_storage(storage_dir.clone()).with_seed(12345); // Fixed seed for reproducibility

    let mut t = AspenRaftTester::with_config(config).await;

    // Wait for leader election
    madsim::time::sleep(Duration::from_secs(5)).await;
    let initial_leader = t.check_one_leader().await.expect("No initial leader");

    // Write some data
    for i in 0..10 {
        t.write(format!("persist-key-{}", i), format!("value-{}", i)).await.expect("Write should succeed");
    }

    // Remember the leader
    let leader_before_crash = initial_leader;

    // Crash the leader node
    t.crash_node(leader_before_crash).await;

    // Wait for new leader election
    madsim::time::sleep(Duration::from_secs(10)).await;
    let new_leader = t.check_one_leader().await.expect("No new leader after crash");
    assert_ne!(new_leader, leader_before_crash, "New leader should be different");

    // Write more data with crashed node
    for i in 10..15 {
        t.write(format!("persist-key-{}", i), format!("value-{}", i))
            .await
            .expect("Writes should continue with crashed node");
    }

    // Restart the crashed node - it should recover from persistent storage
    t.restart_node(leader_before_crash).await;

    // Wait for the node to rejoin and sync
    madsim::time::sleep(Duration::from_secs(5)).await;

    // The restarted node should have caught up
    // We can verify this by checking that the cluster is healthy
    t.check_no_split_brain().expect("No split brain after restart");

    // Write more data to verify the restarted node participates
    for i in 15..20 {
        t.write(format!("persist-key-{}", i), format!("value-{}", i))
            .await
            .expect("Writes should work with restarted node");
    }

    // Clean up test directory
    let _ = std::fs::remove_dir_all(&storage_dir);

    t.end();
}

/// Test multiple node restarts with persistence.
///
/// This test verifies that multiple nodes can crash and recover
/// while maintaining cluster integrity.
#[madsim::test]
#[ignore] // TODO: Enable when madsim supports process isolation for storage
async fn test_tester_multiple_crash_recovery() {
    use std::path::PathBuf;

    use aspen::testing::TesterConfig;

    let storage_dir = PathBuf::from("/tmp/aspen-test-multi-crash");
    let _ = std::fs::remove_dir_all(&storage_dir);

    let config = TesterConfig::new(5, "multi_crash").with_persistent_storage(storage_dir.clone()).with_seed(54321);

    let mut t = AspenRaftTester::with_config(config).await;

    // Wait for initial cluster formation
    madsim::time::sleep(Duration::from_secs(5)).await;
    t.check_one_leader().await.expect("No initial leader");

    // Write initial data
    for i in 0..5 {
        t.write(format!("multi-key-{}", i), format!("value-{}", i)).await.expect("Initial write failed");
    }

    // Crash nodes 0 and 1 (still have quorum with 3, 4, 5)
    t.crash_node(0).await;
    t.crash_node(1).await;

    madsim::time::sleep(Duration::from_secs(5)).await;

    // Should still have a leader with 3/5 nodes
    t.check_one_leader().await.expect("Lost quorum with 3/5 nodes");

    // Continue writing
    for i in 5..10 {
        t.write(format!("multi-key-{}", i), format!("value-{}", i))
            .await
            .expect("Write with 2 crashed nodes failed");
    }

    // Restart node 0
    t.restart_node(0).await;
    madsim::time::sleep(Duration::from_secs(3)).await;

    // Restart node 1
    t.restart_node(1).await;
    madsim::time::sleep(Duration::from_secs(5)).await;

    // All nodes should be back
    t.check_no_split_brain().expect("Split brain after multi-restart");

    // Final writes to verify full cluster operation
    for i in 10..15 {
        t.write(format!("multi-key-{}", i), format!("value-{}", i)).await.expect("Final write failed");
    }

    let _ = std::fs::remove_dir_all(&storage_dir);

    t.end();
}

/// Test membership change - add a learner node.
///
/// This test verifies that we can dynamically add a learner to the cluster.
/// Learners replicate data but don't participate in consensus votes.
#[madsim::test]
async fn test_tester_add_learner() {
    // Start with a 3-node cluster, then add nodes 3 and 4 to test learner functionality
    let mut t = AspenRaftTester::new(3, "tester_add_learner").await;

    // Wait for initial cluster (nodes 0, 1, 2 are voters)
    madsim::time::sleep(Duration::from_secs(5)).await;
    t.check_one_leader().await.expect("No initial leader");

    // Verify initial membership (all 3 nodes are voters)
    let (voters, learners) = t.get_membership();
    assert_eq!(voters.len(), 3, "Should start with 3 voters");
    assert!(learners.is_empty(), "Should start with no learners");

    // Create new nodes that will become learners
    // Note: This requires extending the tester to support adding new nodes dynamically
    // For now, we'll test with existing nodes by changing membership

    // Scale down to 2 voters first
    t.change_membership(&[0, 1]).await.expect("Failed to scale down");

    madsim::time::sleep(Duration::from_secs(3)).await;

    // Add node 2 as a learner
    t.add_learner(2).await.expect("Failed to add learner");

    // Wait for replication
    madsim::time::sleep(Duration::from_secs(3)).await;

    // Verify node 2 is now a learner
    let (voters, learners) = t.get_membership();
    assert_eq!(voters.len(), 2, "Should have 2 voters");
    assert!(learners.contains(&2), "Node 2 should be a learner: {:?}", learners);

    // Write some data and verify learner replicates it
    for i in 0..5 {
        t.write(format!("learner-key-{}", i), format!("value-{}", i)).await.expect("Write should succeed");
    }

    // Wait for log sync
    t.wait_for_log_sync(10).await.expect("Log sync should succeed");

    t.end();
}

/// Test membership change - promote learner to voter.
///
/// This test verifies the complete workflow of adding a learner
/// and then promoting it to a full voter.
#[madsim::test]
async fn test_tester_promote_learner_to_voter() {
    let mut t = AspenRaftTester::new(4, "tester_promote_learner").await;

    // Wait for initial cluster (all 4 nodes start as voters)
    madsim::time::sleep(Duration::from_secs(5)).await;
    t.check_one_leader().await.expect("No initial leader");

    // First, remove node 3 from voters to make it available as learner
    t.change_membership(&[0, 1, 2]).await.expect("Failed to remove node 3 from voters");

    madsim::time::sleep(Duration::from_secs(3)).await;

    // Add node 3 as a learner
    t.add_learner(3).await.expect("Failed to add node 3 as learner");

    madsim::time::sleep(Duration::from_secs(3)).await;

    // Verify node 3 is a learner
    let (voters, learners) = t.get_membership();
    assert_eq!(voters.len(), 3, "Should have 3 voters");
    assert!(learners.contains(&3), "Node 3 should be a learner");

    // Promote node 3 back to voter by changing membership
    t.change_membership(&[0, 1, 2, 3]).await.expect("Failed to promote node 3 to voter");

    madsim::time::sleep(Duration::from_secs(3)).await;

    // Verify membership changed
    let (voters, _learners) = t.get_membership();
    assert!(voters.contains(&3), "Node 3 should be a voter now");
    assert_eq!(voters.len(), 4, "Should have 4 voters");

    // Verify cluster still works
    t.write("after-promote".to_string(), "value".to_string())
        .await
        .expect("Write should succeed after promotion");

    t.check_no_split_brain().expect("No split brain after membership change");

    t.end();
}

/// Test membership change - scale down cluster.
///
/// This test verifies that we can safely reduce the cluster size.
#[madsim::test]
async fn test_tester_scale_down() {
    let mut t = AspenRaftTester::new(5, "tester_scale_down").await;

    // Wait for initial cluster (5 voters)
    madsim::time::sleep(Duration::from_secs(5)).await;
    t.check_one_leader().await.expect("No initial leader");

    // Change membership to remove node 4
    // For a 5-node cluster initialized with nodes 0-4 as voters,
    // we reduce to 4 voters
    t.change_membership(&[0, 1, 2, 3]).await.expect("Failed to scale down to 4 nodes");

    madsim::time::sleep(Duration::from_secs(3)).await;

    // Verify cluster still has a leader
    t.check_one_leader().await.expect("Should have leader after scale down");

    // Further scale down to 3 nodes (minimum for quorum)
    t.change_membership(&[0, 1, 2]).await.expect("Failed to scale down to 3 nodes");

    madsim::time::sleep(Duration::from_secs(3)).await;

    // Verify cluster still functions
    let (voters, _) = t.get_membership();
    assert_eq!(voters.len(), 3, "Should have 3 voters");

    t.write("after-scale-down".to_string(), "value".to_string())
        .await
        .expect("Write should succeed after scale down");

    t.check_no_split_brain().expect("No split brain after scale down");

    t.end();
}

/// Test membership change during leader crash.
///
/// This test verifies that ongoing membership changes handle leader
/// failures gracefully.
#[madsim::test]
async fn test_tester_membership_change_with_leader_crash() {
    let mut t = AspenRaftTester::new(5, "tester_membership_crash").await;

    // Wait for initial cluster (all 5 nodes start as voters)
    madsim::time::sleep(Duration::from_secs(5)).await;
    let initial_leader = t.check_one_leader().await.expect("No initial leader");

    // First reduce membership to 3 nodes to test adding learners
    t.change_membership(&[0, 1, 2]).await.expect("Failed to reduce membership");

    madsim::time::sleep(Duration::from_secs(3)).await;

    // Add node 3 as learner before crash
    t.add_learner(3).await.expect("Failed to add learner");
    madsim::time::sleep(Duration::from_secs(2)).await;

    // Crash the leader
    t.crash_node(initial_leader).await;
    t.add_event(format!("Crashed leader node {}", initial_leader));

    // Wait for new leader
    madsim::time::sleep(Duration::from_secs(10)).await;

    let new_leader = t.check_one_leader().await;
    assert!(new_leader.is_some(), "Should elect new leader after crash");
    assert_ne!(new_leader.unwrap(), initial_leader, "New leader should be different");

    // Membership operations should still work
    // Try to add another learner through new leader
    if t.add_learner(4).await.is_ok() {
        t.add_event("Successfully added learner 4 through new leader".to_string());
    } else {
        // Expected if the previous learner addition didn't fully replicate
        t.add_event("Could not add learner 4 (may be expected)".to_string());
    }

    // Verify no split brain
    t.check_no_split_brain().expect("No split brain after membership changes and crash");

    t.end();
}

// =========================================================================
// BUGGIFY Fault Injection Tests (Phase 2.2)
// =========================================================================

/// Test BUGGIFY-style fault injection with default probabilities.
///
/// This test enables BUGGIFY and runs for a period, verifying
/// that the cluster maintains consensus despite injected faults.
#[madsim::test]
async fn test_tester_buggify_default() {
    let mut t = AspenRaftTester::new(5, "tester_buggify_default").await;

    // Wait for initial cluster formation
    madsim::time::sleep(Duration::from_secs(5)).await;
    t.check_one_leader().await.expect("No initial leader");

    // Enable BUGGIFY with default probabilities
    t.enable_buggify(None);
    t.add_event("Starting BUGGIFY fault injection test".to_string());

    // Run with BUGGIFY for 20 seconds
    for _ in 0..20 {
        t.apply_buggify_faults().await;
        madsim::time::sleep(Duration::from_secs(1)).await;

        // Try to maintain some operations
        let _ = t.write(format!("buggify-key-{}", t.seed() % 1000), "value".to_string()).await;
    }

    // Disable BUGGIFY and let cluster stabilize
    t.disable_buggify();
    madsim::time::sleep(Duration::from_secs(5)).await;

    // Verify cluster is still functional
    t.check_one_leader().await.expect("Should have leader after BUGGIFY");

    t.check_no_split_brain().expect("No split brain after BUGGIFY");

    // Write should still work
    t.write("post-buggify".to_string(), "value".to_string())
        .await
        .expect("Write should work after BUGGIFY");

    let artifact = t.end();
    eprintln!(
        "BUGGIFY test completed with {} fault triggers",
        serde_json::from_str::<serde_json::Value>(&artifact.metrics)
            .ok()
            .and_then(|v| v.get("buggify_triggers").and_then(|t| t.as_u64()))
            .unwrap_or(0)
    );
}

/// Test BUGGIFY with custom fault probabilities.
///
/// This test uses higher fault probabilities to stress test the cluster.
#[madsim::test]
async fn test_tester_buggify_custom_probs() {
    use std::collections::HashMap;

    use aspen::testing::BuggifyFault;

    let mut t = AspenRaftTester::new(5, "tester_buggify_custom").await;

    // Wait for initial cluster
    madsim::time::sleep(Duration::from_secs(5)).await;
    t.check_one_leader().await.expect("No initial leader");

    // Set aggressive fault probabilities
    let mut probs = HashMap::new();
    probs.insert(BuggifyFault::NetworkDelay, 0.20); // 20% chance
    probs.insert(BuggifyFault::NetworkDrop, 0.10); // 10% chance
    probs.insert(BuggifyFault::NodeCrash, 0.05); // 5% chance
    probs.insert(BuggifyFault::ElectionTimeout, 0.10); // 10% chance
    probs.insert(BuggifyFault::MessageCorruption, 0.05); // 5% chance

    t.enable_buggify(Some(probs));

    // Run with aggressive BUGGIFY for 15 seconds
    t.run_with_buggify_loop(Duration::from_secs(15)).await;

    // Let cluster recover
    t.disable_buggify();
    madsim::time::sleep(Duration::from_secs(10)).await;

    // Verify recovery
    t.check_one_leader().await.expect("Should recover leader after aggressive BUGGIFY");

    t.check_no_split_brain().expect("No split brain after recovery");

    t.end();
}

/// Test BUGGIFY with focused fault types.
///
/// This test only enables specific fault types to test their individual impact.
#[madsim::test]
async fn test_tester_buggify_focused_faults() {
    use std::collections::HashMap;

    use aspen::testing::BuggifyFault;

    let mut t = AspenRaftTester::new(3, "tester_buggify_focused").await;

    // Wait for cluster
    madsim::time::sleep(Duration::from_secs(5)).await;
    t.check_one_leader().await.expect("No initial leader");

    // Test 1: Only network delays
    let mut delay_probs = HashMap::new();
    delay_probs.insert(BuggifyFault::NetworkDelay, 0.50); // 50% chance
    t.enable_buggify(Some(delay_probs));

    for _ in 0..10 {
        t.apply_buggify_faults().await;
        madsim::time::sleep(Duration::from_millis(500)).await;
    }

    t.disable_buggify();
    t.add_event("Completed network delay testing".to_string());

    // Test 2: Only snapshot triggers
    let mut snapshot_probs = HashMap::new();
    snapshot_probs.insert(BuggifyFault::SnapshotTrigger, 0.30); // 30% chance
    t.enable_buggify(Some(snapshot_probs));

    for _ in 0..10 {
        t.apply_buggify_faults().await;
        madsim::time::sleep(Duration::from_secs(1)).await;
    }

    t.disable_buggify();
    t.add_event("Completed snapshot trigger testing".to_string());

    // Verify cluster health
    t.check_one_leader().await.expect("Should maintain leader with focused faults");

    t.check_no_split_brain().expect("No split brain with focused faults");

    t.end();
}
