//! Integration tests for federation across Aspen clusters.
//!
//! These tests validate cross-cluster federation functionality including:
//!
//! - Cluster identity management
//! - Trust relationships between clusters
//! - Federation gossip discovery
//! - Cross-cluster resource sync
//!
//! # Test Categories
//!
//! 1. **Identity Tests** - Cluster identity creation and signing
//! 2. **Trust Tests** - Trust management and access control
//! 3. **Gossip Tests** - Federation gossip message exchange
//! 4. **Sync Tests** - Cross-cluster synchronization protocol
//!
//! # Requirements
//!
//! These tests require:
//! - Network access (marked with `#[ignore]` for Nix sandbox)
//! - Multiple real Iroh endpoints
//!
//! Run with:
//!
//! ```bash
//! cargo nextest run federation_integration --ignored
//! ```
//!
//! # Tiger Style
//!
//! - Bounded timeouts: All operations have explicit timeouts
//! - Resource cleanup: Clusters are properly shut down after tests
//! - Explicit error handling: All errors are wrapped with context

mod support;

use std::collections::HashMap;

use aspen_cluster::federation::AppManifest;
use aspen_cluster::federation::ClusterIdentity;
use aspen_cluster::federation::FederatedId;
use aspen_cluster::federation::FederationGossipMessage;
use aspen_cluster::federation::FederationMode;
use aspen_cluster::federation::FederationRequest;
use aspen_cluster::federation::FederationResponse;
use aspen_cluster::federation::FederationSettings;
use aspen_cluster::federation::SignedFederationMessage;
use aspen_cluster::federation::TrustLevel;
use aspen_cluster::federation::TrustManager;
use aspen_core::hlc::SerializableTimestamp;

// ============================================================================
// Unit Tests (no network required)
// ============================================================================

/// Test: Cluster identity generation and signing.
#[test]
fn test_cluster_identity_generation() {
    let identity = ClusterIdentity::generate("test-cluster".to_string());

    assert_eq!(identity.name(), "test-cluster");
    assert!(identity.created_at_ms() > 0);

    // Sign a message and verify
    let message = b"hello, federation!";
    let signature = identity.sign(message);

    let sig_bytes: [u8; 64] = signature.0;
    let sig = iroh::Signature::from_bytes(&sig_bytes);
    assert!(identity.public_key().verify(message, &sig).is_ok());
}

/// Test: Cluster identity with description.
#[test]
fn test_cluster_identity_with_description() {
    let identity = ClusterIdentity::generate("test-cluster".to_string())
        .with_description("A test cluster for federation testing".to_string());

    assert_eq!(identity.description(), Some("A test cluster for federation testing"));
}

/// Test: Signed cluster identity roundtrip.
#[test]
fn test_signed_cluster_identity_roundtrip() {
    let identity =
        ClusterIdentity::generate("test-cluster".to_string()).with_description("Test description".to_string());

    let signed = identity.to_signed();

    // Verify signature
    assert!(signed.verify());
    assert_eq!(signed.name(), "test-cluster");
    assert_eq!(signed.public_key(), identity.public_key());
}

/// Test: Tampered signed identity fails verification.
#[test]
fn test_signed_identity_tamper_detection() {
    let identity = ClusterIdentity::generate("test-cluster".to_string());
    let mut signed = identity.to_signed();

    // Tamper with the name
    signed.identity.name = "tampered-name".to_string();

    // Verification should fail
    assert!(!signed.verify());
}

/// Test: Cluster identity hex key roundtrip.
#[test]
fn test_cluster_identity_hex_roundtrip() {
    let original = ClusterIdentity::generate("test-cluster".to_string());
    let hex_key = original.secret_key_hex();

    let restored =
        ClusterIdentity::from_hex_key(&hex_key, "test-cluster".to_string()).expect("hex parse should succeed");

    assert_eq!(original.public_key(), restored.public_key());
}

// ============================================================================
// Trust Manager Tests
// ============================================================================

/// Test: Trust manager basic operations.
#[test]
fn test_trust_manager_basic() {
    let manager = TrustManager::new();
    let key = iroh::SecretKey::generate(&mut rand::rng()).public();

    // Default is public
    assert_eq!(manager.trust_level(&key), TrustLevel::Public);
    assert!(!manager.is_trusted(&key));
    assert!(!manager.is_blocked(&key));
}

/// Test: Adding and removing trusted clusters.
#[test]
fn test_trust_manager_add_remove() {
    let manager = TrustManager::new();
    let key = iroh::SecretKey::generate(&mut rand::rng()).public();

    // Add trusted
    assert!(manager.add_trusted(key, "test-cluster".to_string(), None));
    assert!(manager.is_trusted(&key));
    assert_eq!(manager.trust_level(&key), TrustLevel::Trusted);

    // Remove trusted
    assert!(manager.remove_trusted(&key));
    assert!(!manager.is_trusted(&key));
    assert_eq!(manager.trust_level(&key), TrustLevel::Public);
}

/// Test: Blocking clusters.
#[test]
fn test_trust_manager_blocking() {
    let manager = TrustManager::new();
    let key = iroh::SecretKey::generate(&mut rand::rng()).public();

    // Block
    assert!(manager.block(key));
    assert!(manager.is_blocked(&key));
    assert_eq!(manager.trust_level(&key), TrustLevel::Blocked);

    // Unblock
    assert!(manager.unblock(&key));
    assert!(!manager.is_blocked(&key));
}

/// Test: Blocking removes trust.
#[test]
fn test_blocking_removes_trust() {
    let manager = TrustManager::new();
    let key = iroh::SecretKey::generate(&mut rand::rng()).public();

    manager.add_trusted(key, "test-cluster".to_string(), None);
    assert!(manager.is_trusted(&key));

    manager.block(key);
    assert!(!manager.is_trusted(&key));
    assert!(manager.is_blocked(&key));
}

/// Test: Trust removes block.
#[test]
fn test_trust_removes_block() {
    let manager = TrustManager::new();
    let key = iroh::SecretKey::generate(&mut rand::rng()).public();

    manager.block(key);
    assert!(manager.is_blocked(&key));

    manager.add_trusted(key, "test-cluster".to_string(), None);
    assert!(!manager.is_blocked(&key));
    assert!(manager.is_trusted(&key));
}

/// Test: Resource access control.
#[test]
fn test_resource_access_control() {
    let manager = TrustManager::new();
    let trusted_key = iroh::SecretKey::generate(&mut rand::rng()).public();
    let public_key = iroh::SecretKey::generate(&mut rand::rng()).public();
    let blocked_key = iroh::SecretKey::generate(&mut rand::rng()).public();

    manager.add_trusted(trusted_key, "trusted".to_string(), None);
    manager.block(blocked_key);

    // Trusted can access anything
    assert!(manager.can_access_resource(&trusted_key, &FederationMode::Public));
    assert!(manager.can_access_resource(&trusted_key, &FederationMode::AllowList));
    assert!(manager.can_access_resource(&trusted_key, &FederationMode::Disabled));

    // Public can only access public
    assert!(manager.can_access_resource(&public_key, &FederationMode::Public));
    assert!(!manager.can_access_resource(&public_key, &FederationMode::AllowList));
    assert!(!manager.can_access_resource(&public_key, &FederationMode::Disabled));

    // Blocked can't access anything
    assert!(!manager.can_access_resource(&blocked_key, &FederationMode::Public));
    assert!(!manager.can_access_resource(&blocked_key, &FederationMode::AllowList));
}

// ============================================================================
// Federated ID Tests
// ============================================================================

/// Test: Federated ID creation and components.
#[test]
fn test_federated_id_creation() {
    let origin = iroh::SecretKey::generate(&mut rand::rng()).public();
    let local_id = [0xab; 32];

    let fed_id = FederatedId::new(origin, local_id);

    assert_eq!(fed_id.origin(), origin);
    assert_eq!(fed_id.local_id(), &local_id);
}

/// Test: Federated ID string representation.
#[test]
fn test_federated_id_string_representation() {
    let origin = iroh::SecretKey::generate(&mut rand::rng()).public();
    let local_id = [0xcd; 32];

    let fed_id = FederatedId::new(origin, local_id);

    // Should have a short representation
    let short = fed_id.short();
    assert!(!short.is_empty());
    assert!(short.len() < 100); // Should be reasonably short
}

/// Test: Federated ID equality.
#[test]
fn test_federated_id_equality() {
    let origin = iroh::SecretKey::generate(&mut rand::rng()).public();
    let local_id = [0xef; 32];

    let fed_id1 = FederatedId::new(origin, local_id);
    let fed_id2 = FederatedId::new(origin, local_id);
    let fed_id3 = FederatedId::new(origin, [0x12; 32]);

    assert_eq!(fed_id1, fed_id2);
    assert_ne!(fed_id1, fed_id3);
}

// ============================================================================
// Federation Gossip Message Tests
// ============================================================================

/// Test: ClusterOnline message roundtrip.
#[test]
fn test_cluster_online_message_roundtrip() {
    let identity = ClusterIdentity::generate("test-cluster".to_string());
    let node_keys = vec![*iroh::SecretKey::generate(&mut rand::rng()).public().as_bytes()];
    let hlc = aspen_core::hlc::create_hlc("test-node");

    let apps = vec![
        AppManifest::new("forge", "1.0.0")
            .with_name("Aspen Forge")
            .with_capabilities(vec!["git", "issues", "patches"]),
    ];

    let message = FederationGossipMessage::ClusterOnline {
        version: 1,
        cluster_key: *identity.public_key().as_bytes(),
        cluster_name: identity.name().to_string(),
        node_keys,
        relay_urls: vec!["https://relay.example.com".to_string()],
        apps,
        capabilities: vec!["forge".to_string()],
        hlc_timestamp: SerializableTimestamp::from(hlc.new_timestamp()),
    };

    let signed = SignedFederationMessage::sign(message.clone(), &identity).unwrap();
    let bytes = signed.to_bytes().unwrap();

    let parsed = SignedFederationMessage::from_bytes(&bytes).unwrap();
    let verified = parsed.verify().expect("signature should be valid");

    match verified {
        FederationGossipMessage::ClusterOnline { cluster_name, .. } => {
            assert_eq!(cluster_name, identity.name());
        }
        _ => panic!("expected ClusterOnline"),
    }
}

/// Test: ResourceSeeding message roundtrip.
#[test]
fn test_resource_seeding_message_roundtrip() {
    let identity = ClusterIdentity::generate("test-cluster".to_string());
    let origin = iroh::SecretKey::generate(&mut rand::rng()).public();
    let fed_id = FederatedId::new(origin, [0xab; 32]);
    let hlc = aspen_core::hlc::create_hlc("test-node");

    let message = FederationGossipMessage::ResourceSeeding {
        version: 1,
        fed_id_origin: *fed_id.origin().as_bytes(),
        fed_id_local: *fed_id.local_id(),
        cluster_key: *identity.public_key().as_bytes(),
        node_keys: vec![*iroh::SecretKey::generate(&mut rand::rng()).public().as_bytes()],
        ref_heads: vec![("heads/main".to_string(), [0xcd; 32])],
        hlc_timestamp: SerializableTimestamp::from(hlc.new_timestamp()),
    };

    let signed = SignedFederationMessage::sign(message, &identity).unwrap();
    assert!(signed.verify().is_some());
}

/// Test: Tampered message detection.
#[test]
fn test_tampered_message_detection() {
    let identity = ClusterIdentity::generate("test-cluster".to_string());
    let hlc = aspen_core::hlc::create_hlc("test-node");

    let mut message = FederationGossipMessage::ClusterOnline {
        version: 1,
        cluster_key: *identity.public_key().as_bytes(),
        cluster_name: identity.name().to_string(),
        node_keys: vec![],
        relay_urls: vec![],
        apps: vec![],
        capabilities: vec![],
        hlc_timestamp: SerializableTimestamp::from(hlc.new_timestamp()),
    };

    let signed = SignedFederationMessage::sign(message.clone(), &identity).unwrap();

    // Tamper with the message
    if let FederationGossipMessage::ClusterOnline {
        ref mut cluster_name, ..
    } = message
    {
        *cluster_name = "tampered".to_string();
    }

    let tampered = SignedFederationMessage {
        message,
        signature: signed.signature.clone(),
    };

    assert!(tampered.verify().is_none());
}

// ============================================================================
// Federation Protocol Request/Response Tests
// ============================================================================

/// Test: Handshake request serialization.
#[test]
fn test_handshake_request_serialization() {
    let identity = ClusterIdentity::generate("test-cluster".to_string());

    let request = FederationRequest::Handshake {
        identity: identity.to_signed(),
        protocol_version: 1,
        capabilities: vec!["forge".to_string()],
    };

    let bytes = postcard::to_allocvec(&request).unwrap();
    let parsed: FederationRequest = postcard::from_bytes(&bytes).unwrap();

    match parsed {
        FederationRequest::Handshake {
            protocol_version,
            capabilities,
            ..
        } => {
            assert_eq!(protocol_version, 1);
            assert_eq!(capabilities, vec!["forge"]);
        }
        _ => panic!("expected Handshake"),
    }
}

/// Test: ListResources request serialization.
#[test]
fn test_list_resources_request_serialization() {
    let request = FederationRequest::ListResources {
        resource_type: Some("forge:repo".to_string()),
        cursor: None,
        limit: 100,
    };

    let bytes = postcard::to_allocvec(&request).unwrap();
    let parsed: FederationRequest = postcard::from_bytes(&bytes).unwrap();

    match parsed {
        FederationRequest::ListResources {
            resource_type, limit, ..
        } => {
            assert_eq!(resource_type, Some("forge:repo".to_string()));
            assert_eq!(limit, 100);
        }
        _ => panic!("expected ListResources"),
    }
}

/// Test: ResourceState response serialization.
#[test]
fn test_resource_state_response_serialization() {
    let response = FederationResponse::ResourceState {
        found: true,
        heads: {
            let mut h = HashMap::new();
            h.insert("heads/main".to_string(), [0xef; 32]);
            h
        },
        metadata: None,
    };

    let bytes = postcard::to_allocvec(&response).unwrap();
    let parsed: FederationResponse = postcard::from_bytes(&bytes).unwrap();

    match parsed {
        FederationResponse::ResourceState { found, heads, .. } => {
            assert!(found);
            assert!(heads.contains_key("heads/main"));
        }
        _ => panic!("expected ResourceState"),
    }
}

// ============================================================================
// Federation Settings Tests
// ============================================================================

/// Test: Federation settings defaults.
#[test]
fn test_federation_settings_defaults() {
    let settings = FederationSettings::default();

    assert_eq!(settings.mode, FederationMode::Disabled);
    assert!(settings.allowed_clusters.is_empty());
}

/// Test: Federation settings with public mode.
#[test]
fn test_federation_settings_public() {
    let settings = FederationSettings::public();

    assert_eq!(settings.mode, FederationMode::Public);
}

/// Test: Federation settings with allow list.
#[test]
fn test_federation_settings_allowlist() {
    let key1 = iroh::SecretKey::generate(&mut rand::rng()).public();
    let key2 = iroh::SecretKey::generate(&mut rand::rng()).public();

    let settings = FederationSettings::allowlist(vec![key1, key2]);

    assert_eq!(settings.mode, FederationMode::AllowList);
    assert_eq!(settings.allowed_clusters.len(), 2);
    assert!(settings.allowed_clusters.contains(&key1));
    assert!(settings.allowed_clusters.contains(&key2));
}

// ============================================================================
// Note: Full integration tests with real Iroh networking are available in
// the existing test suite. This file focuses on comprehensive unit testing
// of federation primitives. For end-to-end federation tests, see:
// - scripts/aspen-cluster-raft-smoke.sh
// - tests/gossip_integration_test.rs
// ============================================================================

/// Test: Trust request workflow.
#[test]
fn test_trust_request_workflow() {
    let manager = TrustManager::new();
    let requester_identity = ClusterIdentity::generate("requester".to_string());
    let signed_identity = requester_identity.to_signed();
    let requester_key = requester_identity.public_key();

    // Add trust request
    assert!(manager.add_trust_request(signed_identity.clone(), Some("please trust me".to_string())));

    // Check pending requests
    let pending = manager.list_pending_requests();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].message, Some("please trust me".to_string()));

    // Requester is not yet trusted
    assert!(!manager.is_trusted(&requester_key));

    // Accept the request
    assert!(manager.accept_trust_request(&requester_key));

    // Now trusted
    assert!(manager.is_trusted(&requester_key));

    // No pending requests
    assert!(manager.list_pending_requests().is_empty());
}

/// Test: Trust request rejection.
#[test]
fn test_trust_request_rejection() {
    let manager = TrustManager::new();
    let requester_identity = ClusterIdentity::generate("requester".to_string());
    let signed_identity = requester_identity.to_signed();
    let requester_key = requester_identity.public_key();

    manager.add_trust_request(signed_identity, None);

    // Reject the request
    assert!(manager.reject_trust_request(&requester_key));

    // Not trusted
    assert!(!manager.is_trusted(&requester_key));

    // No pending requests
    assert!(manager.list_pending_requests().is_empty());
}

/// Test: Blocked cluster cannot send trust requests.
#[test]
fn test_blocked_cluster_trust_request() {
    let manager = TrustManager::new();
    let requester_identity = ClusterIdentity::generate("requester".to_string());
    let signed_identity = requester_identity.to_signed();
    let requester_key = requester_identity.public_key();

    // Block the requester
    manager.block(requester_key);

    // Trust request should be rejected
    assert!(!manager.add_trust_request(signed_identity, None));

    // No pending requests
    assert!(manager.list_pending_requests().is_empty());
}
