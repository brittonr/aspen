#![cfg(feature = "simulation")]
//! Targeted BUGGIFY fault injection tests.
//!
//! Each test exercises a specific fault type with 100% probability to verify
//! the cluster handles it gracefully. Uses `apply_single_fault` for
//! deterministic, targeted injection.

use std::collections::HashMap;
use std::time::Duration;

use aspen_testing::AspenRaftTester;
use aspen_testing::BuggifyFault;

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

/// Slow storage fault: inject high-latency delays on all links, verify
/// cluster survives and writes eventually succeed.
#[madsim::test]
async fn test_buggify_slow_storage_seed_7001() {
    let mut t = AspenRaftTester::new_with_seed(3, "buggify_slow_storage", 7001).await;
    madsim::time::sleep(Duration::from_secs(5)).await;
    let _leader = t.check_one_leader().await.expect("no leader");

    // Write baseline data
    for i in 0..5 {
        write_with_retry(&mut t, format!("pre_{i}"), format!("val_{i}"), 5).await;
    }

    // Inject slow storage fault (100% probability)
    t.apply_single_fault(BuggifyFault::SlowStorage).await;

    // Wait for fault to clear (handler holds delay for 3s then clears)
    madsim::time::sleep(Duration::from_secs(5)).await;

    // Cluster should recover and accept writes again
    let _leader = t.check_one_leader().await.expect("no leader after slow storage");

    for i in 5..10 {
        write_with_retry(&mut t, format!("post_{i}"), format!("val_{i}"), 10).await;
    }

    // Verify reads
    let val = t.read("pre_0").await.expect("read failed");
    assert_eq!(val, Some("val_0".to_string()));
    let val = t.read("post_5").await.expect("read failed");
    assert_eq!(val, Some("val_5".to_string()));

    t.check_no_split_brain().expect("split brain after slow storage fault");
    t.end();
}

/// Snapshot transfer reset fault: crash a follower during snapshot trigger,
/// restart it, verify it catches up.
#[madsim::test]
async fn test_buggify_snapshot_transfer_reset_seed_7002() {
    let mut t = AspenRaftTester::new_with_seed(5, "buggify_snapshot_reset", 7002).await;
    madsim::time::sleep(Duration::from_secs(5)).await;
    let _leader = t.check_one_leader().await.expect("no leader");

    // Write enough data to build meaningful snapshot
    for i in 0..30 {
        write_with_retry(&mut t, format!("snap_{i}"), format!("val_{i}"), 5).await;
    }

    // Inject snapshot transfer reset (100% probability)
    t.apply_single_fault(BuggifyFault::SnapshotTransferReset).await;

    // Wait for restarted node to catch up
    madsim::time::sleep(Duration::from_secs(10)).await;

    // Cluster should have a leader
    let _leader = t.check_one_leader().await.expect("no leader after snapshot transfer reset");

    // Writes should succeed
    for i in 30..35 {
        write_with_retry(&mut t, format!("snap_{i}"), format!("val_{i}"), 10).await;
    }

    // Verify data
    let val = t.read("snap_0").await.expect("read failed");
    assert_eq!(val, Some("val_0".to_string()));

    t.check_no_split_brain().expect("split brain after snapshot transfer reset");
    t.end();
}

/// Network partition fault: verify partition + heal through buggify path.
#[madsim::test]
async fn test_buggify_network_partition_seed_7003() {
    let mut t = AspenRaftTester::new_with_seed(5, "buggify_partition", 7003).await;
    madsim::time::sleep(Duration::from_secs(5)).await;
    let _leader = t.check_one_leader().await.expect("no leader");

    for i in 0..10 {
        write_with_retry(&mut t, format!("part_{i}"), format!("val_{i}"), 5).await;
    }

    // Inject network partition (100% probability)
    t.apply_single_fault(BuggifyFault::NetworkPartition).await;

    // Partition heals after 3s inside the handler; wait for stabilization
    madsim::time::sleep(Duration::from_secs(8)).await;

    let _leader = t.check_one_leader().await.expect("no leader after partition heal");

    for i in 10..15 {
        write_with_retry(&mut t, format!("part_{i}"), format!("val_{i}"), 10).await;
    }

    t.check_no_split_brain().expect("split brain after buggify partition");
    t.end();
}

/// Node crash fault: crash a random node, verify cluster continues.
#[madsim::test]
async fn test_buggify_node_crash_seed_7004() {
    let mut t = AspenRaftTester::new_with_seed(5, "buggify_crash", 7004).await;
    madsim::time::sleep(Duration::from_secs(5)).await;
    let _leader = t.check_one_leader().await.expect("no leader");

    for i in 0..10 {
        write_with_retry(&mut t, format!("crash_{i}"), format!("val_{i}"), 5).await;
    }

    // Inject node crash (100% probability)
    t.apply_single_fault(BuggifyFault::NodeCrash).await;

    madsim::time::sleep(Duration::from_secs(8)).await;

    // Cluster should still elect a leader (4 of 5 remain)
    let _leader = t.check_one_leader().await.expect("no leader after node crash");

    // Writes should succeed on remaining majority
    for i in 10..15 {
        write_with_retry(&mut t, format!("crash_{i}"), format!("val_{i}"), 10).await;
    }

    t.check_no_split_brain().expect("split brain after buggify crash");
    t.end();
}

/// Combined buggify loop: enable all faults with default probabilities,
/// run writes concurrently for 15 simulated seconds, verify no split brain.
#[madsim::test]
async fn test_buggify_combined_faults_seed_7005() {
    let mut t = AspenRaftTester::new_with_seed(5, "buggify_combined", 7005).await;
    madsim::time::sleep(Duration::from_secs(5)).await;
    let _leader = t.check_one_leader().await.expect("no leader");

    // Seed initial data
    for i in 0..5 {
        write_with_retry(&mut t, format!("combo_{i}"), format!("val_{i}"), 5).await;
    }

    // Enable all buggify faults with elevated probabilities
    let mut probs = HashMap::new();
    probs.insert(BuggifyFault::NetworkDelay, 0.15);
    probs.insert(BuggifyFault::NetworkDrop, 0.05);
    probs.insert(BuggifyFault::SlowStorage, 0.10);
    probs.insert(BuggifyFault::SnapshotTrigger, 0.05);
    t.enable_buggify(Some(probs));

    // Run fault injection loop for 15 simulated seconds
    t.run_with_buggify_loop(Duration::from_secs(15)).await;

    t.disable_buggify();

    // Allow cluster to stabilize after faults
    madsim::time::sleep(Duration::from_secs(10)).await;

    // Should still have a leader
    let _leader = t.check_one_leader().await.expect("no leader after combined faults");

    // Original data should still be readable
    let val = t.read("combo_0").await;
    assert!(val.is_ok(), "reads should work after combined faults");

    t.check_no_split_brain().expect("split brain after combined faults");
    t.end();
}
