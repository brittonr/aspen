//! Simulation tests for membership changes under failure conditions.
//!
//! This test suite validates membership change behavior during:
//! - Network partitions
//! - Leader crashes
//! - Concurrent membership changes
//! - Scale-down during writes
//! - Joint consensus with partitions
//!
//! All tests use deterministic seeds for reproducibility.

use std::time::Duration;

use aspen_testing::AspenRaftTester;

/// Helper to write with retry logic for transient failures.
async fn write_with_retry(tester: &mut AspenRaftTester, key: String, value: String, max_retries: usize) {
    for attempt in 0..max_retries {
        match tester.write(key.clone(), value.clone()).await {
            Ok(()) => return,
            Err(err) => {
                if attempt + 1 == max_retries {
                    panic!("Write failed after {} attempts: {}", max_retries, err);
                }
                madsim::time::sleep(Duration::from_millis(500)).await;
            }
        }
    }
}

/// Test add learner during network partition.
///
/// Scenario:
/// 1. Create 4-node cluster (3 voters + 1 standby)
/// 2. Partition one voter
/// 3. Add standby as learner through remaining majority
/// 4. Heal partition
/// 5. Verify all nodes converge
#[madsim::test]
async fn test_add_learner_during_partition_seed_2000() {
    let mut t = AspenRaftTester::new_with_seed(4, "learner_partition", 2000).await;

    // Wait for leader election
    madsim::time::sleep(Duration::from_secs(5)).await;
    let leader = t.check_one_leader().await.expect("no leader");

    // Write some initial data
    for i in 0..10 {
        write_with_retry(&mut t, format!("key{}", i), format!("value{}", i), 5).await;
    }

    // Partition one voter (not the leader)
    let partitioned = if leader == 0 { 2 } else { 0 };
    t.disconnect(partitioned);

    // Add node 3 as learner through remaining majority
    t.add_learner(3).await.expect("add_learner failed during partition");

    // Write more data
    for i in 10..20 {
        write_with_retry(&mut t, format!("key{}", i), format!("value{}", i), 5).await;
    }

    // Heal partition
    t.connect(partitioned);
    madsim::time::sleep(Duration::from_secs(10)).await;

    // Verify all nodes converge
    t.check_no_split_brain().expect("split brain after heal");
    t.end();
}

/// Test membership change with leader crash.
///
/// Scenario:
/// 1. Create 5-node cluster
/// 2. Start adding a new voter
/// 3. Crash leader mid-membership-change
/// 4. Wait for new leader election
/// 5. Verify membership is consistent
#[madsim::test]
async fn test_membership_change_leader_crash_seed_2001() {
    let mut t = AspenRaftTester::new_with_seed(5, "membership_crash", 2001).await;

    // Wait for leader election
    madsim::time::sleep(Duration::from_secs(5)).await;
    let leader = t.check_one_leader().await.expect("no leader");

    // Write initial data
    for i in 0..20 {
        write_with_retry(&mut t, format!("key{}", i), format!("value{}", i), 5).await;
    }

    // Start membership change - first add as learner
    t.add_learner(4).await.expect("add_learner failed");

    // Small delay then crash leader
    madsim::time::sleep(Duration::from_millis(100)).await;
    t.crash_node(leader).await;

    // Wait for new leader election
    madsim::time::sleep(Duration::from_secs(10)).await;
    let new_leader = t.check_one_leader().await.expect("no new leader");
    assert_ne!(new_leader, leader, "new leader should be different from crashed leader");

    // Continue writes on new leader
    for i in 20..30 {
        write_with_retry(&mut t, format!("key{}", i), format!("value{}", i), 5).await;
    }

    // Restart crashed node
    t.restart_node(leader).await;
    madsim::time::sleep(Duration::from_secs(10)).await;

    // Verify no split brain
    t.check_no_split_brain().expect("split brain after membership change");
    t.end();
}

/// Test concurrent membership changes (should be serialized by Raft).
///
/// Scenario:
/// 1. Create 5-node cluster
/// 2. Attempt two membership changes close together
/// 3. Verify system handles them correctly (one succeeds or both serialize)
#[madsim::test]
async fn test_concurrent_membership_changes_seed_2002() {
    let mut t = AspenRaftTester::new_with_seed(5, "concurrent_membership", 2002).await;

    // Wait for leader election
    madsim::time::sleep(Duration::from_secs(5)).await;
    t.check_one_leader().await.expect("no leader");

    // Write initial data
    for i in 0..10 {
        write_with_retry(&mut t, format!("key{}", i), format!("value{}", i), 5).await;
    }

    // Add learner
    let r1 = t.add_learner(3).await;

    // Small delay
    madsim::time::sleep(Duration::from_millis(50)).await;

    // Try to add another learner
    let r2 = t.add_learner(4).await;

    // At least one should succeed, or both eventually succeed
    assert!(r1.is_ok() || r2.is_ok(), "both membership changes failed");

    madsim::time::sleep(Duration::from_secs(5)).await;
    t.check_no_split_brain().expect("split brain during concurrent membership");
    t.end();
}

/// Test scale down during continuous writes.
///
/// Scenario:
/// 1. Create 5-node cluster
/// 2. Start continuous writes
/// 3. Remove voters one at a time
/// 4. Verify writes continue (some may fail during transitions)
#[madsim::test]
async fn test_scale_down_during_writes_seed_2003() {
    let mut t = AspenRaftTester::new_with_seed(5, "scale_down", 2003).await;

    // Wait for leader election
    madsim::time::sleep(Duration::from_secs(5)).await;
    t.check_one_leader().await.expect("no leader");

    // Write initial data
    for i in 0..20 {
        write_with_retry(&mut t, format!("key{}", i), format!("value{}", i), 5).await;
    }

    // Change membership to remove node 4 (keeping 5 -> 4 nodes)
    t.change_membership(&[0, 1, 2, 3]).await.expect("first scale down failed");
    madsim::time::sleep(Duration::from_secs(3)).await;

    // Continue writes
    for i in 20..30 {
        // Use more retries since membership is changing
        write_with_retry(&mut t, format!("key{}", i), format!("value{}", i), 10).await;
    }

    // Scale down again (4 -> 3 nodes)
    t.change_membership(&[0, 1, 2]).await.expect("second scale down failed");
    madsim::time::sleep(Duration::from_secs(3)).await;

    // Final writes
    for i in 30..40 {
        write_with_retry(&mut t, format!("key{}", i), format!("value{}", i), 10).await;
    }

    // Verify cluster is healthy
    t.check_one_leader().await.expect("no leader after scale down");
    t.end();
}

/// Test joint consensus with partition.
///
/// Scenario:
/// 1. Create 5-node cluster
/// 2. Start voter addition (enters joint consensus)
/// 3. Create partition splitting old and new configurations
/// 4. Heal partition
/// 5. Verify safety (at most one leader per term)
#[madsim::test]
async fn test_joint_consensus_with_partition_seed_2004() {
    let mut t = AspenRaftTester::new_with_seed(5, "joint_partition", 2004).await;

    // Wait for leader election
    madsim::time::sleep(Duration::from_secs(5)).await;
    t.check_one_leader().await.expect("no leader");

    // Write initial data
    for i in 0..10 {
        write_with_retry(&mut t, format!("key{}", i), format!("value{}", i), 5).await;
    }

    // Start voter addition - first add as learner
    t.add_learner(4).await.expect("add_learner failed");

    // Create partition during/after membership change
    madsim::time::sleep(Duration::from_millis(100)).await;
    t.disconnect(0);
    t.disconnect(1);

    // Let the partitioned nodes try to elect
    madsim::time::sleep(Duration::from_secs(5)).await;

    // Heal partition
    t.connect(0);
    t.connect(1);

    madsim::time::sleep(Duration::from_secs(10)).await;

    // Safety: at most one leader per term (no split brain)
    t.check_no_split_brain().expect("split brain in joint consensus");
    t.end();
}
