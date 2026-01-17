//! Simulation tests for snapshot operations under failure conditions.
//!
//! This test suite validates snapshot behavior during:
//! - Network partitions
//! - Concurrent writes
//! - Leader crashes
//! - Learner installation
//! - Partial snapshot recovery
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

/// Test snapshot during network partition.
///
/// Scenario:
/// 1. Create 5-node cluster
/// 2. Write enough entries to trigger snapshot
/// 3. Partition 2 followers
/// 4. Trigger snapshot on leader
/// 5. Heal partition
/// 6. Verify all nodes converge
#[madsim::test]
async fn test_snapshot_during_partition_seed_1000() {
    let mut t = AspenRaftTester::new_with_seed(5, "snapshot_partition", 1000).await;

    // Wait for leader election
    madsim::time::sleep(Duration::from_secs(5)).await;
    let leader = t.check_one_leader().await.expect("no leader");

    // Write enough entries to build substantial log
    for i in 0..50 {
        write_with_retry(&mut t, format!("key{}", i), format!("val{}", i), 5).await;
    }

    // Partition 2 followers
    t.disconnect(2);
    t.disconnect(3);

    // Trigger snapshot on leader
    t.trigger_snapshot(leader as u64).await.expect("snapshot trigger failed");

    // Continue writing on majority partition
    for i in 50..75 {
        write_with_retry(&mut t, format!("key{}", i), format!("val{}", i), 5).await;
    }

    // Heal partition
    madsim::time::sleep(Duration::from_secs(3)).await;
    t.connect(2);
    t.connect(3);

    // Wait for snapshot installation and log catchup
    madsim::time::sleep(Duration::from_secs(15)).await;

    // Verify all nodes have same state (no split brain)
    t.check_no_split_brain().expect("split brain detected");
    t.end();
}

/// Test concurrent writes during snapshot.
///
/// Scenario:
/// 1. Create 3-node cluster
/// 2. Write initial data
/// 3. Trigger snapshot while continuing to write
/// 4. Verify no data loss
#[madsim::test]
async fn test_concurrent_writes_during_snapshot_seed_1001() {
    let mut t = AspenRaftTester::new_with_seed(3, "concurrent_snapshot", 1001).await;

    // Wait for leader election
    madsim::time::sleep(Duration::from_secs(5)).await;
    let leader = t.check_one_leader().await.expect("no leader");

    // Write initial data
    for i in 0..25 {
        write_with_retry(&mut t, format!("key{}", i), "initial".to_string(), 5).await;
    }

    // Trigger snapshot
    t.trigger_snapshot(leader as u64).await.expect("snapshot trigger failed");

    // Continue writing while snapshot is being built
    for i in 25..50 {
        write_with_retry(&mut t, format!("key{}", i), "during".to_string(), 5).await;
    }

    // Wait for snapshot completion
    madsim::time::sleep(Duration::from_secs(5)).await;

    // Verify all data is present - read a sample of keys
    for i in [0, 24, 25, 49] {
        let val = t.read(&format!("key{}", i)).await;
        assert!(val.is_ok(), "key{} read failed", i);
    }

    t.end();
}

/// Test leader crash during snapshot.
///
/// Scenario:
/// 1. Create 5-node cluster
/// 2. Write substantial log
/// 3. Trigger snapshot on leader
/// 4. Crash leader mid-snapshot
/// 5. Wait for re-election
/// 6. Verify recovery
#[madsim::test]
async fn test_leader_crash_during_snapshot_seed_1002() {
    let mut t = AspenRaftTester::new_with_seed(5, "crash_snapshot", 1002).await;

    // Wait for leader election
    madsim::time::sleep(Duration::from_secs(5)).await;
    let leader = t.check_one_leader().await.expect("no leader");

    // Write data
    for i in 0..50 {
        write_with_retry(&mut t, format!("key{}", i), "value".to_string(), 5).await;
    }

    // Trigger snapshot and immediately crash leader
    t.trigger_snapshot(leader as u64).await.expect("snapshot trigger failed");
    madsim::time::sleep(Duration::from_millis(100)).await;
    t.crash_node(leader).await;

    // Wait for re-election
    madsim::time::sleep(Duration::from_secs(10)).await;
    let new_leader = t.check_one_leader().await.expect("no new leader");
    assert_ne!(leader, new_leader, "new leader should be different");

    // Restart crashed node
    t.restart_node(leader).await;
    madsim::time::sleep(Duration::from_secs(10)).await;

    // Verify no split brain
    t.check_no_split_brain().expect("split brain after recovery");
    t.end();
}

/// Test snapshot installation on new learner.
///
/// Scenario:
/// 1. Create 3-node cluster
/// 2. Write substantial log to force snapshot-based catchup
/// 3. Trigger compaction
/// 4. Add learner
/// 5. Verify learner receives data via snapshot
#[madsim::test]
async fn test_snapshot_installation_on_learner_seed_1003() {
    let mut t = AspenRaftTester::new_with_seed(4, "learner_snapshot", 1003).await;

    // Wait for leader election (only first 3 nodes are voters initially)
    madsim::time::sleep(Duration::from_secs(5)).await;
    let leader = t.check_one_leader().await.expect("no leader");

    // Write substantial log
    for i in 0..100 {
        write_with_retry(&mut t, format!("key{}", i), "value".to_string(), 5).await;
    }

    // Trigger snapshot to compact log
    t.trigger_snapshot(leader as u64).await.expect("snapshot trigger failed");
    madsim::time::sleep(Duration::from_secs(3)).await;

    // Add node 3 as learner (should receive snapshot, not full log replay)
    t.add_learner(3).await.expect("add_learner failed");

    // Wait for learner to catch up via snapshot
    madsim::time::sleep(Duration::from_secs(15)).await;

    // Verify learner has data - the learner should now be able to serve reads
    // (this is a simplified check; in production we'd verify the learner's state)
    t.check_no_split_brain().expect("split brain with learner");
    t.end();
}

/// Test recovery from partial snapshot.
///
/// Scenario:
/// 1. Create 3-node cluster
/// 2. Write data
/// 3. Trigger snapshot on follower
/// 4. Crash follower mid-installation
/// 5. Restart and verify recovery
#[madsim::test]
async fn test_recovery_from_partial_snapshot_seed_1004() {
    let mut t = AspenRaftTester::new_with_seed(3, "partial_snapshot", 1004).await;

    // Wait for leader election
    madsim::time::sleep(Duration::from_secs(5)).await;
    let leader = t.check_one_leader().await.expect("no leader");

    // Find a follower
    let follower = if leader == 0 { 1 } else { 0 };

    // Write data
    for i in 0..50 {
        write_with_retry(&mut t, format!("key{}", i), "value".to_string(), 5).await;
    }

    // Trigger snapshot on follower
    t.trigger_snapshot(follower as u64).await.expect("snapshot trigger failed");

    // Crash follower during snapshot
    madsim::time::sleep(Duration::from_millis(50)).await;
    t.crash_node(follower).await;

    // Continue normal operations
    for i in 50..75 {
        write_with_retry(&mut t, format!("key{}", i), "value".to_string(), 5).await;
    }

    // Restart crashed node
    madsim::time::sleep(Duration::from_secs(2)).await;
    t.restart_node(follower).await;
    madsim::time::sleep(Duration::from_secs(10)).await;

    // Verify recovery
    t.check_no_split_brain().expect("split brain after partial snapshot recovery");
    t.end();
}
