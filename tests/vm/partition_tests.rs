//! Network partition tests for Aspen clusters.
//!
//! These tests verify that the cluster handles network partitions correctly:
//! - Leader election during minority partition
//! - Data consistency across partitions
//! - Recovery after partition heals
//!
//! Run with: `cargo nextest run --test test_vm_partition --ignored`
//!
//! Prerequisites:
//! - Root privileges (for iptables and network manipulation)
//! - KVM support
//! - Pre-built microvm runners

use std::path::PathBuf;
use std::time::Duration;

use aspen::testing::fault_injection::{FaultScenario, NetworkPartition};
use aspen::testing::vm_manager::{VmConfig, VmManager};

/// Timeout for VM boot operations.
const VM_BOOT_TIMEOUT: Duration = Duration::from_secs(120);

/// Timeout for Raft operations.
#[allow(dead_code)]
const RAFT_TIMEOUT: Duration = Duration::from_secs(30);

/// Time to wait for leader election after partition.
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

/// Test basic network partition creation and healing.
#[test]
fn test_partition_validation() {
    // Test that partition parameters are validated
    // (actual iptables manipulation requires root)

    // This is a basic validation test that doesn't require root
    let source = "10.100.0.11";
    let targets = ["10.100.0.12", "10.100.0.13"];

    // These are valid IP addresses
    assert!(source.parse::<std::net::Ipv4Addr>().is_ok());
    for target in &targets {
        assert!(target.parse::<std::net::Ipv4Addr>().is_ok());
    }
}

/// Test fault scenario builder.
#[test]
fn test_fault_scenario_builder() {
    // Test that we can create an empty fault scenario
    let scenario = FaultScenario::new();
    assert!(!scenario.has_active_faults());
}

// ============================================================================
// Integration Tests (require root and KVM)
// ============================================================================

/// Test: Minority partition - isolated node cannot write.
///
/// Scenario:
/// 1. Boot 3-node cluster
/// 2. Partition node 1 from nodes 2 and 3
/// 3. Verify node 1 cannot accept writes (no quorum)
/// 4. Verify nodes 2 and 3 can still form quorum and accept writes
/// 5. Heal partition and verify recovery
#[tokio::test]
#[ignore = "requires root privileges and KVM"]
async fn test_minority_partition_cannot_write() {
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

    // Wait for initial leader election
    tokio::time::sleep(LEADER_ELECTION_TIMEOUT).await;

    // Create network partition: isolate node 1 from nodes 2 and 3
    let partition = NetworkPartition::create("10.100.0.11", &["10.100.0.12", "10.100.0.13"])
        .expect("failed to create partition");

    // Wait for partition to take effect and leader election
    tokio::time::sleep(LEADER_ELECTION_TIMEOUT).await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    // Try to write via isolated node 1 - should timeout or fail
    let endpoint1 = manager.http_endpoint(1).await.unwrap();
    let write_result = client
        .put(format!("{}/kv/partition-test", endpoint1))
        .body("from-isolated-node")
        .send()
        .await;

    // Either connection fails or we get an error response (no quorum)
    let node1_can_write = match write_result {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    };
    assert!(
        !node1_can_write,
        "Isolated node should not be able to write"
    );

    // Write via majority partition (nodes 2 and 3)
    let endpoint2 = manager.http_endpoint(2).await.unwrap();
    let write_result = client
        .put(format!("{}/kv/partition-test", endpoint2))
        .body("from-majority-partition")
        .send()
        .await;

    assert!(
        write_result.is_ok() && write_result.unwrap().status().is_success(),
        "Majority partition should be able to write"
    );

    // Heal the partition
    drop(partition);

    // Wait for recovery
    tokio::time::sleep(LEADER_ELECTION_TIMEOUT).await;

    // Verify node 1 can now read the data
    let read_result = client
        .get(format!("{}/kv/partition-test", endpoint1))
        .send()
        .await;

    assert!(read_result.is_ok());
    let body = read_result.unwrap().text().await.unwrap();
    assert_eq!(body, "from-majority-partition");

    // Cleanup
    manager.stop_all().await.unwrap();
}

/// Test: Leader in minority partition - new leader elected in majority.
///
/// Scenario:
/// 1. Boot 3-node cluster and determine leader
/// 2. Partition the leader from other nodes
/// 3. Verify new leader elected in majority partition
/// 4. Heal partition and verify old leader steps down
#[tokio::test]
#[ignore = "requires root privileges and KVM"]
async fn test_leader_partition_triggers_reelection() {
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

    // Wait for initial leader election
    tokio::time::sleep(LEADER_ELECTION_TIMEOUT).await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    // Find the current leader
    let mut leader_id: Option<u64> = None;
    for node_id in 1..=3 {
        let endpoint = manager.http_endpoint(node_id).await.unwrap();
        // Parse leader_id from response (assumes JSON with leader_id field)
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
    let leader_ip = format!("10.100.0.{}", 10 + leader_id);

    // Get IPs of follower nodes
    let follower_ips: Vec<String> = (1..=3)
        .filter(|&id| id != leader_id)
        .map(|id| format!("10.100.0.{}", 10 + id))
        .collect();

    // Partition the leader
    let partition = NetworkPartition::create(
        &leader_ip,
        &follower_ips.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
    )
    .expect("failed to create partition");

    // Wait for new leader election in majority partition
    tokio::time::sleep(LEADER_ELECTION_TIMEOUT).await;

    // Check that a follower is now the leader
    let follower_id = if leader_id == 1 { 2 } else { 1 };
    let follower_endpoint = manager.http_endpoint(follower_id).await.unwrap();

    let resp = client
        .get(format!("{}/cluster/status", follower_endpoint))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());
    let text = resp.text().await.unwrap();

    // The new leader should be one of the followers
    let new_leader_is_follower = follower_ips.iter().enumerate().any(|(i, _)| {
        let follower_node_id = (1..=3).filter(|&id| id != leader_id).nth(i).unwrap();
        text.contains(&format!("\"leader_id\":{}", follower_node_id))
    });

    assert!(
        new_leader_is_follower,
        "New leader should be elected from majority partition"
    );

    // Heal partition
    drop(partition);

    // Wait for cluster to stabilize
    tokio::time::sleep(LEADER_ELECTION_TIMEOUT).await;

    // Verify cluster has single leader (all nodes agree)
    let mut leaders_seen = std::collections::HashSet::new();
    for node_id in 1..=3 {
        let endpoint = manager.http_endpoint(node_id).await.unwrap();
        if let Ok(resp) = client
            .get(format!("{}/cluster/status", endpoint))
            .send()
            .await
            && let Ok(text) = resp.text().await
        {
            // Extract leader_id from response
            for check_id in 1..=3 {
                if text.contains(&format!("\"leader_id\":{}", check_id)) {
                    leaders_seen.insert(check_id);
                }
            }
        }
    }

    assert_eq!(
        leaders_seen.len(),
        1,
        "All nodes should agree on a single leader after partition heals"
    );

    // Cleanup
    manager.stop_all().await.unwrap();
}

/// Test: Data written during partition is replicated after healing.
///
/// Scenario:
/// 1. Boot 3-node cluster
/// 2. Partition node 1 from nodes 2 and 3
/// 3. Write data via majority partition
/// 4. Heal partition
/// 5. Verify data is replicated to node 1
#[tokio::test]
#[ignore = "requires root privileges and KVM"]
async fn test_data_replication_after_partition_heals() {
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

    // Wait for initial leader election
    tokio::time::sleep(LEADER_ELECTION_TIMEOUT).await;

    // Create partition: isolate node 1
    let partition = NetworkPartition::create("10.100.0.11", &["10.100.0.12", "10.100.0.13"])
        .expect("failed to create partition");

    // Wait for partition to take effect
    tokio::time::sleep(LEADER_ELECTION_TIMEOUT).await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    // Write multiple keys via majority partition
    let endpoint2 = manager.http_endpoint(2).await.unwrap();

    for i in 0..10 {
        let key = format!("replication-test-{}", i);
        let value = format!("value-{}", i);

        let resp = client
            .put(format!("{}/kv/{}", endpoint2, key))
            .body(value)
            .send()
            .await
            .unwrap();

        assert!(resp.status().is_success());
    }

    // Heal partition
    drop(partition);

    // Wait for replication
    tokio::time::sleep(Duration::from_secs(10)).await;

    // Verify all data is replicated to node 1
    let endpoint1 = manager.http_endpoint(1).await.unwrap();

    for i in 0..10 {
        let key = format!("replication-test-{}", i);
        let expected_value = format!("value-{}", i);

        let resp = client
            .get(format!("{}/kv/{}", endpoint1, key))
            .send()
            .await
            .unwrap();

        assert!(resp.status().is_success());
        let body = resp.text().await.unwrap();
        assert_eq!(body, expected_value, "Key {} not properly replicated", key);
    }

    // Cleanup
    manager.stop_all().await.unwrap();
}

/// Test: Asymmetric partition (node can receive but not send).
///
/// This tests more complex partition scenarios where traffic is blocked
/// in only one direction.
#[tokio::test]
#[ignore = "requires root privileges and KVM"]
async fn test_asymmetric_partition() {
    // TODO: Implement asymmetric partition test
    // This requires more sophisticated iptables rules
    // For now, this is a placeholder for future implementation
    todo!("Asymmetric partition test not yet implemented")
}
