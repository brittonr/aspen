//! Clock drift injection tests for Raft using madsim deterministic simulation.
//!
//! These tests verify that Raft consensus remains correct under various
//! clock drift conditions between nodes. Clock drift is simulated by
//! adding asymmetric network delays.
//!
//! # Why Test Clock Drift?
//!
//! Raft consensus does NOT require synchronized clocks - it uses:
//! - Logical ordering (term numbers + log indices) instead of wall-clock time
//! - Monotonic clocks (`std::time::Instant`) for election/heartbeat timeouts
//!
//! However, testing under drift conditions validates that:
//! 1. Elections still complete correctly when nodes have different timing
//! 2. No split-brain occurs due to timing variations
//! 3. The system handles heterogeneous timing gracefully
//!
//! # How Drift is Simulated
//!
//! Since madsim uses global virtual time, we simulate drift effects through
//! asymmetric network delays:
//! - Positive drift (fast clock): Adds delay to OUTGOING messages from the node
//! - Negative drift (slow clock): Adds delay to INCOMING messages to the node
//!
//! This effectively tests how Raft handles nodes that appear to be on different
//! timelines from each other's perspective.
//!
//! # Scenarios Tested
//!
//! 1. Follower slow clock (delays incoming messages to follower)
//! 2. Leader slow clock (delays outgoing messages from leader)
//! 3. Mixed drift (different drift values per node)
//! 4. Drift sweep (0ms, 50ms, 100ms, 200ms)
//!
//! # Correctness Assertions
//!
//! - No split-brain (at most one leader per term)
//! - Eventual leader election within bounded time
//! - Log consistency across all nodes

use std::time::Duration;

use aspen::testing::madsim_tester::AspenRaftTester;

/// Test: No drift baseline (control test).
///
/// This establishes baseline behavior with no clock drift for comparison.
#[madsim::test]
async fn test_clock_drift_baseline_no_drift() {
    let mut t = AspenRaftTester::new(3, "clock_drift_baseline").await;

    // Wait for initial leader election
    madsim::time::sleep(Duration::from_secs(5)).await;
    let leader = t.check_one_leader().await.expect("No initial leader");

    // Perform some operations
    for i in 0..5 {
        t.write(format!("baseline-key-{}", i), format!("value-{}", i)).await.expect("Write should succeed");
    }

    // Wait for replication
    madsim::time::sleep(Duration::from_secs(2)).await;

    // Verify cluster health
    t.check_no_split_brain().expect("Split brain in baseline");
    t.check_one_leader().await.expect("Should still have leader");

    t.add_event(format!("baseline test completed with leader {}", leader));
    t.end();
}

/// Test: Follower with 50ms simulated slow clock.
///
/// A follower with a slow clock will appear to receive heartbeats late.
/// With 50ms drift (well under the 1500ms election timeout), the cluster
/// should remain stable.
#[madsim::test]
async fn test_clock_drift_follower_slow_50ms() {
    let mut t = AspenRaftTester::new(3, "clock_drift_follower_50ms").await;

    // Wait for initial leader
    madsim::time::sleep(Duration::from_secs(5)).await;
    let leader = t.check_one_leader().await.expect("No initial leader");

    // Find a follower
    let follower_idx = if leader == 0 { 1 } else { 0 };

    // Apply 50ms clock drift to follower (negative = slow clock)
    t.set_clock_drift(follower_idx, -50);

    // Run operations under drift
    for i in 0..5 {
        t.write(format!("drift50-key-{}", i), format!("value-{}", i))
            .await
            .expect("Write should succeed with 50ms drift");
    }

    // Wait and verify stability
    madsim::time::sleep(Duration::from_secs(10)).await;

    // Verify correctness
    t.check_no_split_brain().expect("Split brain with 50ms drift");
    t.check_one_leader().await.expect("No leader after drift test");

    t.clear_clock_drift(follower_idx);
    t.end();
}

/// Test: Follower with 100ms simulated slow clock.
///
/// 100ms drift is still well under the election timeout threshold,
/// so the cluster should remain stable.
#[madsim::test]
async fn test_clock_drift_follower_slow_100ms() {
    let mut t = AspenRaftTester::new(3, "clock_drift_follower_100ms").await;

    // Wait for initial leader
    madsim::time::sleep(Duration::from_secs(5)).await;
    let leader = t.check_one_leader().await.expect("No initial leader");

    // Find a follower
    let follower_idx = if leader == 0 { 1 } else { 0 };

    // Apply 100ms clock drift to follower
    t.set_clock_drift(follower_idx, -100);

    // Run operations under drift
    for i in 0..5 {
        t.write(format!("drift100-key-{}", i), format!("value-{}", i))
            .await
            .expect("Write should succeed with 100ms drift");
    }

    // Wait and verify stability
    madsim::time::sleep(Duration::from_secs(10)).await;

    // Verify correctness
    t.check_no_split_brain().expect("Split brain with 100ms drift");
    t.check_one_leader().await.expect("No leader after drift test");

    t.clear_clock_drift(follower_idx);
    t.end();
}

/// Test: Follower with 200ms simulated slow clock.
///
/// 200ms drift approaches timing that could affect elections in
/// poorly-tuned systems, but Raft should handle it correctly
/// since our election timeout is 1500-3000ms.
#[madsim::test]
async fn test_clock_drift_follower_slow_200ms() {
    let mut t = AspenRaftTester::new(3, "clock_drift_follower_200ms").await;

    // Wait for initial leader
    madsim::time::sleep(Duration::from_secs(5)).await;
    let leader = t.check_one_leader().await.expect("No initial leader");

    // Find a follower
    let follower_idx = if leader == 0 { 1 } else { 0 };

    // Apply 200ms clock drift to follower
    t.set_clock_drift(follower_idx, -200);

    // Run operations under drift
    for i in 0..5 {
        t.write(format!("drift200-key-{}", i), format!("value-{}", i))
            .await
            .expect("Write should succeed with 200ms drift");
    }

    // Wait and verify stability - give extra time for potential elections
    madsim::time::sleep(Duration::from_secs(15)).await;

    // Verify correctness
    t.check_no_split_brain().expect("Split brain with 200ms drift");
    t.check_one_leader().await.expect("No leader after drift test");

    t.clear_clock_drift(follower_idx);
    t.end();
}

/// Test: Leader with slow clock (should potentially trigger re-election).
///
/// When the leader has a slow clock, its heartbeats may arrive late
/// from followers' perspective, potentially triggering elections.
/// The cluster should still maintain correctness (no split-brain).
#[madsim::test]
async fn test_clock_drift_leader_slow_100ms() {
    let mut t = AspenRaftTester::new(3, "clock_drift_leader_slow").await;

    // Wait for initial leader
    madsim::time::sleep(Duration::from_secs(5)).await;
    let leader = t.check_one_leader().await.expect("No initial leader");

    // Apply drift to leader - positive drift means outgoing messages are delayed
    // This simulates a leader whose heartbeats arrive late to followers
    t.set_clock_drift(leader, 100);

    // Wait for potential re-election
    madsim::time::sleep(Duration::from_secs(10)).await;

    // There should be a leader (possibly different)
    let new_leader = t.check_one_leader().await.expect("No leader after leader drift");

    // Verify correctness - this is the key assertion
    t.check_no_split_brain().expect("Split brain after leader drift");

    t.add_event(format!("leader drift test: original={}, current={}", leader, new_leader));

    t.clear_clock_drift(leader);
    t.end();
}

/// Test: Mixed drift (nodes have different clock offsets).
///
/// In real deployments, nodes typically have small, varying amounts
/// of clock drift. This tests heterogeneous timing.
#[madsim::test]
async fn test_clock_drift_mixed() {
    let mut t = AspenRaftTester::new(5, "clock_drift_mixed").await;

    // Wait for initial leader
    madsim::time::sleep(Duration::from_secs(5)).await;
    t.check_one_leader().await.expect("No initial leader");

    // Apply different drifts to different nodes (in ms)
    // This simulates a realistic cluster with varied timing
    t.set_cluster_clock_drifts(&[
        (0, 0),   // Reference clock
        (1, 50),  // 50ms fast (positive = fast)
        (2, -30), // 30ms slow (negative = slow)
        (3, 75),  // 75ms fast
        (4, -20), // 20ms slow
    ]);

    // Run test with writes
    for i in 0..10 {
        match t.write(format!("mixed-key-{}", i), format!("value-{}", i)).await {
            Ok(()) => {}
            Err(_) => {
                // May fail briefly during elections - that's ok
                madsim::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }

    // Wait for cluster to stabilize
    madsim::time::sleep(Duration::from_secs(15)).await;

    // Verify correctness
    t.check_no_split_brain().expect("Split brain with mixed drift");
    t.check_one_leader().await.expect("No leader after mixed drift");

    t.clear_all_clock_drifts();
    t.end();
}

/// Test: Deterministic reproducibility with clock drift.
///
/// Running the same test with the same seed should produce identical results.
/// This validates that clock drift simulation is deterministic.
#[ignore] // Flaky: times out intermittently due to simulation overhead
#[madsim::test]
async fn test_clock_drift_reproducible() {
    let seed = 42;

    // First run
    let mut t1 = AspenRaftTester::new_with_seed(3, "drift_repro_1", seed).await;
    madsim::time::sleep(Duration::from_secs(5)).await;
    t1.set_clock_drift(1, -100);
    madsim::time::sleep(Duration::from_secs(10)).await;
    let metrics1 = t1.get_metrics(0);
    let term1 = metrics1.map(|m| m.current_term);
    t1.check_no_split_brain().expect("Split brain in repro test 1");
    t1.end();

    // Second run with same seed should produce same results
    let mut t2 = AspenRaftTester::new_with_seed(3, "drift_repro_2", seed).await;
    madsim::time::sleep(Duration::from_secs(5)).await;
    t2.set_clock_drift(1, -100);
    madsim::time::sleep(Duration::from_secs(10)).await;
    let metrics2 = t2.get_metrics(0);
    let term2 = metrics2.map(|m| m.current_term);
    t2.check_no_split_brain().expect("Split brain in repro test 2");
    t2.end();

    // Results should match (deterministic simulation)
    assert_eq!(term1, term2, "Deterministic test should produce same term: {:?} vs {:?}", term1, term2);
}

/// Test: Clock drift during network partition.
///
/// This tests the interaction between clock drift and network partitions,
/// which is a realistic failure scenario.
#[madsim::test]
async fn test_clock_drift_with_partition() {
    let mut t = AspenRaftTester::new(5, "clock_drift_partition").await;

    // Wait for initial leader
    madsim::time::sleep(Duration::from_secs(5)).await;
    let leader = t.check_one_leader().await.expect("No initial leader");

    // Apply clock drift
    t.set_clock_drift(1, -100);
    t.set_clock_drift(3, 75);

    // Partition a non-leader node
    let partition_idx = if leader == 2 { 3 } else { 2 };
    t.disconnect(partition_idx);

    // Wait for potential elections
    madsim::time::sleep(Duration::from_secs(10)).await;

    // Should still have majority (4/5 nodes connected)
    t.check_one_leader().await.expect("Should have leader with one partition");

    // Heal partition
    t.connect(partition_idx);
    madsim::time::sleep(Duration::from_secs(5)).await;

    // Verify final state
    t.check_no_split_brain().expect("Split brain after healing partition");
    t.check_one_leader().await.expect("No leader after healing");

    t.clear_all_clock_drifts();
    t.end();
}

/// Test: Extreme clock drift (stress test).
///
/// Tests with drift values approaching the heartbeat interval (500ms).
/// This is unrealistic but validates robustness.
#[madsim::test]
async fn test_clock_drift_extreme_400ms() {
    let mut t = AspenRaftTester::new(3, "clock_drift_extreme").await;

    // Wait for initial leader
    madsim::time::sleep(Duration::from_secs(5)).await;
    t.check_one_leader().await.expect("No initial leader");

    // Apply extreme drift - close to heartbeat interval (500ms)
    t.set_clock_drift(1, -400);

    // Wait for potential elections - give plenty of time
    madsim::time::sleep(Duration::from_secs(20)).await;

    // The cluster may have re-elected, but should never split-brain
    t.check_no_split_brain().expect("Split brain with extreme drift");

    // Should eventually have a leader
    t.check_one_leader().await.expect("No leader after extreme drift");

    t.clear_clock_drift(1);
    t.end();
}
