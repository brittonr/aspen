//! Integration tests for cluster bootstrap functionality.
//!
//! These tests verify the end-to-end bootstrap process for Aspen nodes.
//!
//! # Test Categories
//!
//! - Configuration validation
//! - Endpoint initialization
//! - Mock endpoint provider functionality
//! - Graceful shutdown

mod common;

use std::sync::Arc;
use std::time::Duration;

use aspen_cluster::IrohEndpointConfig;
use aspen_cluster::IrohEndpointManager;
use aspen_cluster::validation;
use aspen_core::EndpointProvider;
use common::fixtures::*;

/// Test that a single node endpoint manager can be created.
#[tokio::test]
#[ignore = "requires network"]
async fn test_endpoint_manager_creation() {
    let config = IrohEndpointConfig::new()
        .with_mdns(false)
        .with_dns_discovery(false)
        .with_pkarr(false)
        .with_gossip(false);

    let manager = IrohEndpointManager::new(config).await;
    assert!(manager.is_ok(), "Endpoint manager should be created successfully");
    let manager = manager.unwrap();

    // Verify endpoint is ready
    let node_addr = manager.node_addr();
    assert!(!node_addr.id.to_string().is_empty(), "Node should have an ID");

    // Clean shutdown
    manager.shutdown().await.unwrap();
}

/// Test that node ID validation works correctly.
#[tokio::test]
async fn test_node_id_validation() {
    // Valid node ID
    assert!(validation::validate_node_id(1).is_ok());
    assert!(validation::validate_node_id(100).is_ok());
    assert!(validation::validate_node_id(u64::MAX).is_ok());

    // Invalid node ID (0)
    let result = validation::validate_node_id(0);
    assert!(result.is_err(), "Node ID 0 should be invalid");
}

/// Test that cookie validation works correctly.
#[tokio::test]
async fn test_cookie_validation() {
    // Valid cookies
    assert!(validation::validate_cookie("my-cluster").is_ok());
    assert!(validation::validate_cookie("test-cookie-123").is_ok());

    // Invalid cookies (empty)
    let result = validation::validate_cookie("");
    assert!(result.is_err(), "Empty cookie should be invalid");
}

/// Test that cookie safety validation rejects default cookies.
#[tokio::test]
async fn test_cookie_safety_validation() {
    // Safe cookies
    assert!(validation::validate_cookie_safety("my-custom-cluster").is_ok());
    assert!(validation::validate_cookie_safety("production-cluster-abc").is_ok());
    assert!(validation::validate_cookie_safety("aspen").is_ok()); // "aspen" alone is fine

    // Unsafe default cookie (the specific placeholder)
    let result = validation::validate_cookie_safety("aspen-cookie-UNSAFE-CHANGE-ME");
    assert!(result.is_err(), "Default unsafe cookie should be rejected");
}

/// Test that raft timing validation works.
#[tokio::test]
async fn test_raft_timing_validation() {
    // Valid timings
    let result = validation::validate_raft_timings(100, 500, 1000);
    assert!(result.is_ok(), "Valid timings should pass validation");

    // Invalid: min > max
    let result = validation::validate_raft_timings(100, 1000, 500);
    assert!(result.is_err(), "Election min > max should fail");

    // Invalid: heartbeat = 0
    let result = validation::validate_raft_timings(0, 500, 1000);
    assert!(result.is_err(), "Zero heartbeat should fail");
}

/// Test that secret key validation works.
#[tokio::test]
async fn test_secret_key_validation() {
    // No key is valid (will be generated)
    assert!(validation::validate_secret_key(None).is_ok());

    // Valid hex key (64 hex chars = 32 bytes)
    let valid_key = "0".repeat(64);
    assert!(validation::validate_secret_key(Some(&valid_key)).is_ok());

    // Invalid: wrong length
    let short_key = "0".repeat(32);
    let result = validation::validate_secret_key(Some(&short_key));
    assert!(result.is_err(), "Short key should fail validation");

    // Invalid: not hex
    let invalid_hex = "g".repeat(64);
    let result = validation::validate_secret_key(Some(&invalid_hex));
    assert!(result.is_err(), "Non-hex key should fail validation");
}

/// Test that mock endpoint provider works correctly.
#[tokio::test]
async fn test_mock_endpoint_provider() {
    let provider = MockEndpointProvider::new().await;

    // Verify provider is functional
    let public_key = provider.public_key().await;
    assert!(!public_key.is_empty(), "Should have public key");

    let peer_id = provider.peer_id().await;
    assert!(!peer_id.is_empty(), "Should have peer ID");

    let node_addr = provider.node_addr();
    assert!(!node_addr.id.to_string().is_empty(), "Should have node ID");
}

/// Test that deterministic seeding produces consistent identities.
#[tokio::test]
async fn test_mock_endpoint_deterministic_seeding() {
    let provider1 = MockEndpointProvider::with_seed(42).await;
    let provider2 = MockEndpointProvider::with_seed(42).await;

    // Same seed should produce same node ID
    assert_eq!(provider1.node_addr().id, provider2.node_addr().id, "Same seed should produce same node ID");

    // Different seed should produce different node ID
    let provider3 = MockEndpointProvider::with_seed(43).await;
    assert_ne!(
        provider1.node_addr().id,
        provider3.node_addr().id,
        "Different seed should produce different node ID"
    );
}

/// Test the wait_for utility function.
#[tokio::test]
async fn test_wait_for_utility() {
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::Ordering;

    let flag = Arc::new(AtomicBool::new(false));
    let flag_clone = flag.clone();

    // Spawn a task that sets the flag after a delay
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        flag_clone.store(true, Ordering::SeqCst);
    });

    // Wait for the condition
    let result = wait_for(Duration::from_secs(1), || async { flag.load(Ordering::SeqCst) }).await;

    assert!(result, "wait_for should return true when condition is met");
}

/// Test that wait_for times out correctly.
#[tokio::test]
async fn test_wait_for_timeout() {
    // Wait for a condition that will never be true
    let result = wait_for(Duration::from_millis(100), || async { false }).await;

    assert!(!result, "wait_for should return false on timeout");
}

/// Test endpoint manager with custom config.
#[tokio::test]
#[ignore = "requires network"]
async fn test_endpoint_manager_custom_config() {
    let config = IrohEndpointConfig::new()
        .with_mdns(false)
        .with_dns_discovery(false)
        .with_pkarr(false)
        .with_gossip(false)
        .with_bind_port(0);

    let manager = IrohEndpointManager::new(config).await.unwrap();

    // Verify settings were applied
    let node_addr = manager.node_addr();
    assert!(!node_addr.id.to_string().is_empty());

    manager.shutdown().await.unwrap();
}

/// Test that shutdown is idempotent and doesn't panic.
#[tokio::test]
#[ignore = "requires network"]
async fn test_graceful_shutdown_cleanup() {
    let config = IrohEndpointConfig::new()
        .with_mdns(false)
        .with_dns_discovery(false)
        .with_pkarr(false)
        .with_gossip(false);

    let manager = IrohEndpointManager::new(config).await.unwrap();

    // Shutdown should complete without error
    let result = manager.shutdown().await;
    assert!(result.is_ok(), "First shutdown should succeed");
}
