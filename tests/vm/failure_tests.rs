//! Node failure and recovery tests for Aspen clusters.
//!
//! These tests verify cluster behavior when nodes crash and restart:
//! - Leader crash and recovery
//! - Follower crash and recovery
//! - Multiple simultaneous failures (within fault tolerance)
//! - State persistence across restarts
//!
//! Run with: `cargo nextest run --test test_vm_failure --ignored`
//!
//! Prerequisites:
//! - Root privileges
//! - KVM support
//! - Pre-built microvm runners

use std::path::PathBuf;
use std::time::Duration;

use aspen::testing::vm_manager::{VmConfig, VmManager, VmState};

/// Timeout for VM boot operations.
const VM_BOOT_TIMEOUT: Duration = Duration::from_secs(120);

/// Timeout for Raft operations.
#[allow(dead_code)]
const RAFT_TIMEOUT: Duration = Duration::from_secs(30);

/// Time to wait for leader election after failure.
const LEADER_ELECTION_TIMEOUT: Duration = Duration::from_secs(15);

/// Test helper to get the microvm runner paths.
fn get_runner_paths() -> Option<[PathBuf; 3]> {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let result_dir = base.join("result");

    let runner_base = std::env::var("ASPEN_VM_RUNNERS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| result_dir);

    if !runner_base.exists() {
        return None;
    }

    Some([
        runner_base.join("node-1/bin/microvm-run"),
        runner_base.join("node-2/bin/microvm-run"),
        runner_base.join("node-3/bin/microvm-run"),
    ])
}

/// Test VM state transitions.
#[test]
fn test_vm_state_enum() {
    // Verify all state variants exist
    let _states = [
        VmState::NotStarted,
        VmState::Booting,
        VmState::Running,
        VmState::Paused,
        VmState::Stopped,
        VmState::Error,
    ];

    // Basic equality checks
    assert_eq!(VmState::NotStarted, VmState::NotStarted);
    assert_ne!(VmState::Running, VmState::Stopped);
}

// ============================================================================
// Integration Tests (require root and KVM)
// ============================================================================

/// Test: Single follower crash and recovery.
///
/// Scenario:
/// 1. Boot 3-node cluster
/// 2. Write some data
/// 3. Kill one follower
/// 4. Verify cluster continues operating
/// 5. Restart the follower
/// 6. Verify follower catches up
#[tokio::test]
#[ignore = "requires root privileges and KVM"]
async fn test_follower_crash_and_recovery() {
    let runners = get_runner_paths().expect("microvm runners not found");
    let temp_dir = tempfile::tempdir().unwrap();

    let manager = VmManager::new(temp_dir.path().to_path_buf()).unwrap();

    // Configure and add nodes
    for (i, runner) in runners.iter().enumerate() {
        let node_id = (i + 1) as u64;
        let mut config = VmConfig::for_node(node_id, temp_dir.path());
        config.runner_path = runner.clone();
        manager.add_vm(config).await.unwrap();
    }

    // Boot and initialize cluster
    manager.start_all().await.unwrap();
    manager.wait_for_all_healthy(VM_BOOT_TIMEOUT).await.unwrap();
    manager.init_raft_cluster().await.unwrap();

    // Wait for leader election
    tokio::time::sleep(LEADER_ELECTION_TIMEOUT).await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    // Find the leader
    let mut leader_id: Option<u64> = None;
    for node_id in 1..=3 {
        let endpoint = manager.http_endpoint(node_id).await.unwrap();
        if let Ok(resp) = client
            .get(format!("{}/cluster/status", endpoint))
            .send()
            .await
            && let Ok(text) = resp.text().await
            && text.contains(&format!("\"leader_id\":{}", node_id))
        {
            leader_id = Some(node_id);
            break;
        }
    }
    let leader_id = leader_id.expect("No leader found");

    // Pick a follower to crash
    let follower_id = if leader_id == 1 { 2 } else { 1 };

    // Write initial data
    let leader_endpoint = manager.http_endpoint(leader_id).await.unwrap();
    let resp = client
        .put(format!("{}/kv/before-crash", leader_endpoint))
        .body("initial-value")
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());

    // Crash the follower
    manager.stop_vm(follower_id).await.unwrap();

    // Wait a moment
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Cluster should still be operational (2/3 nodes)
    let resp = client
        .put(format!("{}/kv/during-crash", leader_endpoint))
        .body("while-follower-down")
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());

    // Restart the follower
    manager.start_vm(follower_id).await.unwrap();

    // Wait for VM to boot
    tokio::time::sleep(VM_BOOT_TIMEOUT).await;

    // Wait for follower to catch up
    tokio::time::sleep(Duration::from_secs(10)).await;

    // Verify follower has all data
    let follower_endpoint = manager.http_endpoint(follower_id).await.unwrap();

    let resp = client
        .get(format!("{}/kv/before-crash", follower_endpoint))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
    assert_eq!(resp.text().await.unwrap(), "initial-value");

    let resp = client
        .get(format!("{}/kv/during-crash", follower_endpoint))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
    assert_eq!(resp.text().await.unwrap(), "while-follower-down");

    // Cleanup
    manager.stop_all().await.unwrap();
}

/// Test: Leader crash triggers new election.
///
/// Scenario:
/// 1. Boot 3-node cluster
/// 2. Identify the leader
/// 3. Kill the leader
/// 4. Verify new leader elected
/// 5. Write data via new leader
/// 6. Restart old leader
/// 7. Verify old leader becomes follower
#[tokio::test]
#[ignore = "requires root privileges and KVM"]
async fn test_leader_crash_triggers_election() {
    let runners = get_runner_paths().expect("microvm runners not found");
    let temp_dir = tempfile::tempdir().unwrap();

    let manager = VmManager::new(temp_dir.path().to_path_buf()).unwrap();

    // Configure and add nodes
    for (i, runner) in runners.iter().enumerate() {
        let node_id = (i + 1) as u64;
        let mut config = VmConfig::for_node(node_id, temp_dir.path());
        config.runner_path = runner.clone();
        manager.add_vm(config).await.unwrap();
    }

    // Boot and initialize cluster
    manager.start_all().await.unwrap();
    manager.wait_for_all_healthy(VM_BOOT_TIMEOUT).await.unwrap();
    manager.init_raft_cluster().await.unwrap();

    // Wait for leader election
    tokio::time::sleep(LEADER_ELECTION_TIMEOUT).await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    // Find the current leader
    let mut leader_id: Option<u64> = None;
    for node_id in 1..=3 {
        let endpoint = manager.http_endpoint(node_id).await.unwrap();
        if let Ok(resp) = client
            .get(format!("{}/cluster/status", endpoint))
            .send()
            .await
            && let Ok(text) = resp.text().await
            && text.contains(&format!("\"leader_id\":{}", node_id))
        {
            leader_id = Some(node_id);
            break;
        }
    }
    let old_leader_id = leader_id.expect("No leader found");

    // Kill the leader
    manager.stop_vm(old_leader_id).await.unwrap();

    // Wait for new leader election
    tokio::time::sleep(LEADER_ELECTION_TIMEOUT).await;

    // Find new leader (should be one of the remaining nodes)
    let mut new_leader_id: Option<u64> = None;
    for node_id in (1..=3).filter(|&id| id != old_leader_id) {
        let endpoint = manager.http_endpoint(node_id).await.unwrap();
        if let Ok(resp) = client
            .get(format!("{}/cluster/status", endpoint))
            .send()
            .await
            && let Ok(text) = resp.text().await
        {
            for check_id in (1..=3).filter(|&id| id != old_leader_id) {
                if text.contains(&format!("\"leader_id\":{}", check_id)) {
                    new_leader_id = Some(check_id);
                    break;
                }
            }
        }
        if new_leader_id.is_some() {
            break;
        }
    }

    let new_leader_id = new_leader_id.expect("No new leader elected");
    assert_ne!(
        new_leader_id, old_leader_id,
        "New leader should be different"
    );

    // Write via new leader
    let new_leader_endpoint = manager.http_endpoint(new_leader_id).await.unwrap();
    let resp = client
        .put(format!("{}/kv/after-leader-crash", new_leader_endpoint))
        .body("written-by-new-leader")
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());

    // Restart old leader
    manager.start_vm(old_leader_id).await.unwrap();

    // Wait for old leader to boot and rejoin
    tokio::time::sleep(VM_BOOT_TIMEOUT).await;
    tokio::time::sleep(Duration::from_secs(10)).await;

    // Verify old leader has the data (it's now a follower)
    let old_leader_endpoint = manager.http_endpoint(old_leader_id).await.unwrap();
    let resp = client
        .get(format!("{}/kv/after-leader-crash", old_leader_endpoint))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
    assert_eq!(resp.text().await.unwrap(), "written-by-new-leader");

    // Cleanup
    manager.stop_all().await.unwrap();
}

/// Test: Two simultaneous failures (exceeds fault tolerance).
///
/// Scenario:
/// 1. Boot 3-node cluster
/// 2. Kill two nodes simultaneously
/// 3. Verify remaining node cannot accept writes
/// 4. Restart one node
/// 5. Verify cluster recovers with 2/3 nodes
#[tokio::test]
#[ignore = "requires root privileges and KVM"]
async fn test_two_node_failure_loses_quorum() {
    let runners = get_runner_paths().expect("microvm runners not found");
    let temp_dir = tempfile::tempdir().unwrap();

    let manager = VmManager::new(temp_dir.path().to_path_buf()).unwrap();

    // Configure and add nodes
    for (i, runner) in runners.iter().enumerate() {
        let node_id = (i + 1) as u64;
        let mut config = VmConfig::for_node(node_id, temp_dir.path());
        config.runner_path = runner.clone();
        manager.add_vm(config).await.unwrap();
    }

    // Boot and initialize cluster
    manager.start_all().await.unwrap();
    manager.wait_for_all_healthy(VM_BOOT_TIMEOUT).await.unwrap();
    manager.init_raft_cluster().await.unwrap();

    // Wait for leader election
    tokio::time::sleep(LEADER_ELECTION_TIMEOUT).await;

    // Kill two nodes
    manager.stop_vm(1).await.unwrap();
    manager.stop_vm(2).await.unwrap();

    // Wait a moment
    tokio::time::sleep(Duration::from_secs(5)).await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    // Remaining node (3) cannot accept writes
    let endpoint3 = manager.http_endpoint(3).await.unwrap();
    let write_result = client
        .put(format!("{}/kv/no-quorum", endpoint3))
        .body("should-fail")
        .send()
        .await;

    // Should either fail to connect or return error
    let can_write = match write_result {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    };
    assert!(!can_write, "Node without quorum should not accept writes");

    // Restart node 1
    manager.start_vm(1).await.unwrap();

    // Wait for boot and recovery
    tokio::time::sleep(VM_BOOT_TIMEOUT).await;
    tokio::time::sleep(LEADER_ELECTION_TIMEOUT).await;

    // Now cluster should be operational with 2/3 nodes
    let resp = client
        .put(format!("{}/kv/after-recovery", endpoint3))
        .body("quorum-restored")
        .send()
        .await
        .unwrap();
    assert!(
        resp.status().is_success(),
        "Cluster with quorum should accept writes"
    );

    // Cleanup
    manager.stop_all().await.unwrap();
}

/// Test: Data persistence across node restart.
///
/// Scenario:
/// 1. Boot 3-node cluster
/// 2. Write data
/// 3. Stop all nodes gracefully
/// 4. Restart all nodes
/// 5. Verify data persisted
#[tokio::test]
#[ignore = "requires root privileges and KVM"]
async fn test_data_persistence_across_restart() {
    let runners = get_runner_paths().expect("microvm runners not found");
    let temp_dir = tempfile::tempdir().unwrap();

    let manager = VmManager::new(temp_dir.path().to_path_buf()).unwrap();

    // Configure and add nodes
    for (i, runner) in runners.iter().enumerate() {
        let node_id = (i + 1) as u64;
        let mut config = VmConfig::for_node(node_id, temp_dir.path());
        config.runner_path = runner.clone();
        manager.add_vm(config).await.unwrap();
    }

    // Boot and initialize cluster
    manager.start_all().await.unwrap();
    manager.wait_for_all_healthy(VM_BOOT_TIMEOUT).await.unwrap();
    manager.init_raft_cluster().await.unwrap();

    // Wait for leader election
    tokio::time::sleep(LEADER_ELECTION_TIMEOUT).await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    // Write data
    let endpoint1 = manager.http_endpoint(1).await.unwrap();
    for i in 0..5 {
        let resp = client
            .put(format!("{}/kv/persist-test-{}", endpoint1, i))
            .body(format!("value-{}", i))
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success());
    }

    // Stop all nodes gracefully
    manager.stop_all().await.unwrap();

    // Wait a moment
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Restart all nodes
    manager.start_all().await.unwrap();
    manager.wait_for_all_healthy(VM_BOOT_TIMEOUT).await.unwrap();

    // Wait for cluster to stabilize
    tokio::time::sleep(LEADER_ELECTION_TIMEOUT).await;

    // Verify data persisted
    for i in 0..5 {
        let resp = client
            .get(format!("{}/kv/persist-test-{}", endpoint1, i))
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success());
        assert_eq!(resp.text().await.unwrap(), format!("value-{}", i));
    }

    // Cleanup
    manager.stop_all().await.unwrap();
}

/// Test: Rolling restart (one node at a time).
///
/// Scenario:
/// 1. Boot 3-node cluster
/// 2. Restart each node one at a time
/// 3. Verify cluster remains operational throughout
/// 4. Verify data consistency
#[tokio::test]
#[ignore = "requires root privileges and KVM"]
async fn test_rolling_restart() {
    let runners = get_runner_paths().expect("microvm runners not found");
    let temp_dir = tempfile::tempdir().unwrap();

    let manager = VmManager::new(temp_dir.path().to_path_buf()).unwrap();

    // Configure and add nodes
    for (i, runner) in runners.iter().enumerate() {
        let node_id = (i + 1) as u64;
        let mut config = VmConfig::for_node(node_id, temp_dir.path());
        config.runner_path = runner.clone();
        manager.add_vm(config).await.unwrap();
    }

    // Boot and initialize cluster
    manager.start_all().await.unwrap();
    manager.wait_for_all_healthy(VM_BOOT_TIMEOUT).await.unwrap();
    manager.init_raft_cluster().await.unwrap();

    // Wait for leader election
    tokio::time::sleep(LEADER_ELECTION_TIMEOUT).await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    // Rolling restart each node
    for restart_id in 1..=3 {
        // Find a node that's still up for writes
        let write_id = if restart_id == 1 { 2 } else { 1 };
        let write_endpoint = manager.http_endpoint(write_id).await.unwrap();

        // Write data before restarting this node
        let resp = client
            .put(format!(
                "{}/kv/before-restart-{}",
                write_endpoint, restart_id
            ))
            .body(format!("value-before-{}", restart_id))
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success());

        // Stop this node
        manager.stop_vm(restart_id).await.unwrap();

        // Wait briefly
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Cluster should still accept writes
        let resp = client
            .put(format!(
                "{}/kv/during-restart-{}",
                write_endpoint, restart_id
            ))
            .body(format!("value-during-{}", restart_id))
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "Cluster should remain operational during rolling restart"
        );

        // Restart the node
        manager.start_vm(restart_id).await.unwrap();

        // Wait for it to rejoin
        tokio::time::sleep(VM_BOOT_TIMEOUT).await;
        tokio::time::sleep(Duration::from_secs(5)).await;
    }

    // Verify all data is consistent across all nodes
    for node_id in 1..=3 {
        let endpoint = manager.http_endpoint(node_id).await.unwrap();

        for check_id in 1..=3 {
            let resp = client
                .get(format!("{}/kv/before-restart-{}", endpoint, check_id))
                .send()
                .await
                .unwrap();
            assert!(resp.status().is_success());
            assert_eq!(
                resp.text().await.unwrap(),
                format!("value-before-{}", check_id)
            );

            let resp = client
                .get(format!("{}/kv/during-restart-{}", endpoint, check_id))
                .send()
                .await
                .unwrap();
            assert!(resp.status().is_success());
            assert_eq!(
                resp.text().await.unwrap(),
                format!("value-during-{}", check_id)
            );
        }
    }

    // Cleanup
    manager.stop_all().await.unwrap();
}
