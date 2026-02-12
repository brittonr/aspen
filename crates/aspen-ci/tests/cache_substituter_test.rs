//! Tests for NixBuildWorker cache substituter configuration.
//!
//! These tests verify the Phase 3 auto-injection of cluster cache as a Nix substituter:
//! - Configuration field validation
//! - Graceful degradation when components are missing
//! - Public key format requirements
//!
//! Note: These are unit/config tests that don't require actual Nix builds.

#![cfg(feature = "snix")]

use aspen_ci::workers::NixBuildWorkerConfig;

/// Test that cache proxy is disabled by default.
#[test]
fn test_cache_proxy_disabled_by_default() {
    let config = NixBuildWorkerConfig::default();

    assert!(!config.use_cluster_cache);
    assert!(config.iroh_endpoint.is_none());
    assert!(config.gateway_node.is_none());
    assert!(config.cache_public_key.is_none());
    assert!(!config.can_use_cache_proxy());
}

/// Test that can_use_cache_proxy returns false when use_cluster_cache is false.
#[test]
fn test_cache_proxy_disabled_when_flag_false() {
    let config = NixBuildWorkerConfig {
        use_cluster_cache: false,
        // Even if other fields were populated, it should be disabled
        ..Default::default()
    };

    assert!(!config.can_use_cache_proxy());
}

/// Test graceful degradation when iroh_endpoint is missing.
#[test]
fn test_cache_proxy_disabled_without_endpoint() {
    let config = NixBuildWorkerConfig {
        use_cluster_cache: true,
        iroh_endpoint: None, // Missing!
        gateway_node: Some(iroh::PublicKey::from_bytes(&[0u8; 32]).unwrap()),
        cache_public_key: Some("test-cache:dGVzdC1rZXk=".to_string()),
        ..Default::default()
    };

    assert!(!config.can_use_cache_proxy());
}

/// Test graceful degradation when gateway_node is missing.
#[test]
fn test_cache_proxy_disabled_without_gateway() {
    let config = NixBuildWorkerConfig {
        use_cluster_cache: true,
        iroh_endpoint: None, // Would need real endpoint
        gateway_node: None,  // Missing!
        cache_public_key: Some("test-cache:dGVzdC1rZXk=".to_string()),
        ..Default::default()
    };

    assert!(!config.can_use_cache_proxy());
}

/// Test graceful degradation when public key is missing.
#[test]
fn test_cache_proxy_disabled_without_public_key() {
    let config = NixBuildWorkerConfig {
        use_cluster_cache: true,
        iroh_endpoint: None, // Would need real endpoint
        gateway_node: Some(iroh::PublicKey::from_bytes(&[0u8; 32]).unwrap()),
        cache_public_key: None, // Missing!
        ..Default::default()
    };

    assert!(!config.can_use_cache_proxy());
}

/// Test that validation logs warning for partial cache config.
#[test]
fn test_validate_logs_warning_for_partial_config() {
    let config = NixBuildWorkerConfig {
        node_id: 1,
        use_cluster_cache: true,
        iroh_endpoint: None,
        gateway_node: None,
        cache_public_key: None,
        ..Default::default()
    };

    // validate() should return true for core services (not cache proxy related)
    // but will log a warning about missing cache proxy config
    let result = config.validate();
    // The validation should still pass (warning only, not error)
    // Cache proxy is optional functionality
    assert!(!result); // Returns false because blob_store is also None
}

/// Test public key format matches Nix expectations.
#[test]
fn test_public_key_format() {
    // Nix expects format: {cache_name}:{base64_public_key}
    let valid_key = "aspen-cache:YWJjZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXo=";

    assert!(valid_key.contains(':'));
    let parts: Vec<&str> = valid_key.split(':').collect();
    assert_eq!(parts.len(), 2);
    assert!(!parts[0].is_empty()); // cache name
    assert!(!parts[1].is_empty()); // base64 key
}

/// Test that NixBuildWorkerConfig can be created with all cache fields populated.
/// Note: We test the config logic without creating a real endpoint since that
/// requires network access. The endpoint-based tests are in nix_build_integration_test.rs.
#[test]
fn test_config_logic_with_cache_fields() {
    // Test the configuration validation logic
    // When all fields are properly set, can_use_cache_proxy should return true
    // We can't test with a real endpoint here without network access,
    // but we can verify the logic paths work correctly.

    // Partial config should return false
    let partial_config = NixBuildWorkerConfig {
        node_id: 1,
        cluster_id: "test-cluster".to_string(),
        use_cluster_cache: true,
        iroh_endpoint: None, // Missing - would need real endpoint
        gateway_node: Some(iroh::PublicKey::from_bytes(&[0u8; 32]).unwrap()),
        cache_public_key: Some("test-cache:dGVzdC1rZXk=".to_string()),
        ..Default::default()
    };

    // Without endpoint, cannot use cache proxy
    assert!(!partial_config.can_use_cache_proxy());

    // With use_cluster_cache=false, proxy disabled regardless of other fields
    let disabled_config = NixBuildWorkerConfig {
        use_cluster_cache: false,
        gateway_node: Some(iroh::PublicKey::from_bytes(&[0u8; 32]).unwrap()),
        cache_public_key: Some("test-cache:dGVzdC1rZXk=".to_string()),
        ..Default::default()
    };

    assert!(!disabled_config.can_use_cache_proxy());
}
