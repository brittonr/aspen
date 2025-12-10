//! Basic VM cluster tests for Aspen.
//!
//! These tests verify fundamental cluster operations using Cloud Hypervisor microVMs.
//! They require root privileges and are ignored by default.
//!
//! Run with: `cargo nextest run --test test_vm_basic --ignored`
//!
//! Prerequisites:
//! - Root privileges (for TAP devices and network bridge)
//! - KVM support (/dev/kvm must be accessible)
//! - Pre-built microvm runners (nix build .#nixosConfigurations.x86_64-linux-aspen-node-1)
//!
//! These tests complement the in-memory tests by exercising:
//! - Real network stack (TCP/UDP, QUIC)
//! - Real disk I/O (SQLite, redb)
//! - Real process isolation
//! - Real timing and concurrency

use std::path::PathBuf;
use std::time::Duration;

use aspen::testing::vm_manager::{NetworkConfig, VmConfig, VmManager};

/// Default timeout for VM boot and health checks.
const VM_BOOT_TIMEOUT: Duration = Duration::from_secs(120);

/// Default timeout for Raft operations.
const RAFT_TIMEOUT: Duration = Duration::from_secs(30);

/// Test helper to get the microvm runner paths.
///
/// In a real setup, these would be built by Nix and available in the store.
/// For testing, we expect them to be at a well-known location.
fn get_runner_paths() -> Option<[PathBuf; 3]> {
    // Check if runners exist (built via nix build)
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let result_dir = base.join("result");

    // Try to find runners via environment variable or default paths
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

/// Test that we can create a VmManager.
#[test]
fn test_vm_manager_creation() {
    let temp_dir = tempfile::tempdir().unwrap();
    let manager = VmManager::new(temp_dir.path().to_path_buf());
    assert!(manager.is_ok());
}

/// Test VmConfig generation for nodes.
#[test]
fn test_vm_config_for_nodes() {
    let base_dir = PathBuf::from("/tmp/test-cluster");

    let config1 = VmConfig::for_node(1, &base_dir);
    let config2 = VmConfig::for_node(2, &base_dir);
    let config3 = VmConfig::for_node(3, &base_dir);

    // Verify each node has unique identifiers
    assert_eq!(config1.node_id, 1);
    assert_eq!(config2.node_id, 2);
    assert_eq!(config3.node_id, 3);

    // Verify unique IP addresses
    assert_eq!(config1.ip_address, "10.100.0.11");
    assert_eq!(config2.ip_address, "10.100.0.12");
    assert_eq!(config3.ip_address, "10.100.0.13");

    // Verify unique ports
    assert_eq!(config1.http_port, 8301);
    assert_eq!(config2.http_port, 8302);
    assert_eq!(config3.http_port, 8303);

    // Verify unique TAP devices
    assert_eq!(config1.tap_device, "aspen-1");
    assert_eq!(config2.tap_device, "aspen-2");
    assert_eq!(config3.tap_device, "aspen-3");

    // Verify unique MAC addresses
    assert_eq!(config1.mac_address, "02:00:00:01:01:01");
    assert_eq!(config2.mac_address, "02:00:00:01:01:02");
    assert_eq!(config3.mac_address, "02:00:00:01:01:03");
}

/// Test network configuration defaults.
#[test]
fn test_network_config_defaults() {
    let config = NetworkConfig::default();

    assert_eq!(config.bridge_name, "aspen-br0");
    assert_eq!(config.subnet, "10.100.0");
    assert_eq!(config.gateway, "10.100.0.1");
}

/// Test that we can add VMs to the manager.
#[tokio::test]
async fn test_add_vms_to_manager() {
    let temp_dir = tempfile::tempdir().unwrap();
    let manager = VmManager::new(temp_dir.path().to_path_buf()).unwrap();

    let config1 = VmConfig::for_node(1, temp_dir.path());
    let config2 = VmConfig::for_node(2, temp_dir.path());

    manager.add_vm(config1).await.unwrap();
    manager.add_vm(config2).await.unwrap();

    // Verify we can get endpoints
    let endpoint1 = manager.http_endpoint(1).await.unwrap();
    let endpoint2 = manager.http_endpoint(2).await.unwrap();

    assert!(endpoint1.contains("10.100.0.11"));
    assert!(endpoint2.contains("10.100.0.12"));
}

/// Test that duplicate node IDs are rejected.
#[tokio::test]
async fn test_duplicate_node_rejected() {
    let temp_dir = tempfile::tempdir().unwrap();
    let manager = VmManager::new(temp_dir.path().to_path_buf()).unwrap();

    let config1 = VmConfig::for_node(1, temp_dir.path());
    let config1_dup = VmConfig::for_node(1, temp_dir.path());

    manager.add_vm(config1).await.unwrap();
    let result = manager.add_vm(config1_dup).await;

    assert!(result.is_err());
}

/// Test that VM limit is enforced.
#[tokio::test]
async fn test_vm_limit_enforced() {
    let temp_dir = tempfile::tempdir().unwrap();
    let manager = VmManager::new(temp_dir.path().to_path_buf()).unwrap();

    // Add maximum number of VMs (10)
    for i in 1..=10 {
        let config = VmConfig::for_node(i, temp_dir.path());
        manager.add_vm(config).await.unwrap();
    }

    // 11th should fail
    let config = VmConfig::for_node(11, temp_dir.path());
    let result = manager.add_vm(config).await;

    assert!(result.is_err());
}

// ============================================================================
// Integration Tests (require root and KVM)
// ============================================================================

/// Full integration test: Boot 3-node cluster and verify health.
///
/// This test requires:
/// - Root privileges
/// - KVM access
/// - Pre-built microvm runners
#[tokio::test]
#[ignore = "requires root privileges and KVM"]
async fn test_boot_three_node_cluster() {
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

    // Start all VMs
    manager.start_all().await.unwrap();

    // Wait for all to become healthy
    manager.wait_for_all_healthy(VM_BOOT_TIMEOUT).await.unwrap();

    // Verify all endpoints are accessible
    let endpoints = manager.all_http_endpoints().await;
    assert_eq!(endpoints.len(), 3);

    // Cleanup
    manager.stop_all().await.unwrap();
}

/// Full integration test: Initialize Raft cluster and verify leader election.
#[tokio::test]
#[ignore = "requires root privileges and KVM"]
async fn test_raft_cluster_initialization() {
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

    // Start all VMs
    manager.start_all().await.unwrap();

    // Wait for all to become healthy
    manager.wait_for_all_healthy(VM_BOOT_TIMEOUT).await.unwrap();

    // Initialize the Raft cluster
    manager.init_raft_cluster().await.unwrap();

    // Give time for leader election
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Query cluster status (via HTTP API)
    let client = reqwest::Client::new();
    let endpoint = manager.http_endpoint(1).await.unwrap();

    let response = client
        .get(format!("{}/cluster/status", endpoint))
        .timeout(RAFT_TIMEOUT)
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success());

    // Cleanup
    manager.stop_all().await.unwrap();
}

/// Test write operations across the cluster.
#[tokio::test]
#[ignore = "requires root privileges and KVM"]
async fn test_cluster_write_operations() {
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

    // Give time for leader election
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Write a key-value pair
    let client = reqwest::Client::new();
    let endpoint = manager.http_endpoint(1).await.unwrap();

    let response = client
        .put(format!("{}/kv/test-key", endpoint))
        .body("test-value")
        .timeout(RAFT_TIMEOUT)
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success());

    // Read back from a different node
    let endpoint2 = manager.http_endpoint(2).await.unwrap();

    let response = client
        .get(format!("{}/kv/test-key", endpoint2))
        .timeout(RAFT_TIMEOUT)
        .send()
        .await
        .unwrap();

    assert!(response.status().is_success());
    let body = response.text().await.unwrap();
    assert_eq!(body, "test-value");

    // Cleanup
    manager.stop_all().await.unwrap();
}
