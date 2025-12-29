//! End-to-end integration tests for Forge federation.
//!
//! Tests the complete federation flow:
//! - Cluster identity creation and signing
//! - Trust management between clusters
//! - Federated ID generation
//! - Repository federation settings
//! - Federation sync protocol (handshake, list, sync)
//!
//! Note: These tests use in-memory components and don't require
//! full networking infrastructure.

use std::collections::HashMap;

use aspen::cluster::federation::{
    identity::ClusterIdentity,
    sync::{FederationRequest, FederationResponse, FEDERATION_PROTOCOL_VERSION},
    trust::{TrustLevel, TrustManager},
    types::{FederatedId, FederationMode, FederationSettings},
};

// ============================================================================
// Cluster Identity Tests
// ============================================================================

#[test]
fn test_cluster_identity_generation() {
    let identity = ClusterIdentity::generate("test-cluster".to_string());

    assert_eq!(identity.name(), "test-cluster");
    assert!(identity.description().is_none());
}

#[test]
fn test_cluster_identity_with_description() {
    let identity = ClusterIdentity::generate("my-org".to_string())
        .with_description("Production cluster for my organization".to_string());

    assert_eq!(identity.name(), "my-org");
    assert_eq!(
        identity.description(),
        Some("Production cluster for my organization")
    );
}

#[test]
fn test_cluster_identity_signing() {
    let identity = ClusterIdentity::generate("signing-test".to_string());
    let signed = identity.to_signed();

    // Verify the signature is valid
    assert!(signed.verify());
    assert_eq!(signed.name(), identity.name());
    assert_eq!(signed.public_key(), identity.public_key());
}

#[test]
fn test_cluster_identity_unique_keys() {
    let id1 = ClusterIdentity::generate("cluster-1".to_string());
    let id2 = ClusterIdentity::generate("cluster-2".to_string());

    // Each cluster should have a unique key
    assert_ne!(id1.public_key(), id2.public_key());
}

// ============================================================================
// Federated ID Tests
// ============================================================================

#[test]
fn test_federated_id_creation() {
    let origin = iroh::SecretKey::generate(&mut rand::rng()).public();
    let local_id = [0xab; 32];

    let fed_id = FederatedId::new(origin, local_id);

    assert_eq!(fed_id.origin(), origin);
    assert_eq!(fed_id.local_id(), &local_id);
}

#[test]
fn test_federated_id_short_display() {
    let origin = iroh::SecretKey::generate(&mut rand::rng()).public();
    let local_id = [0xcd; 32];

    let fed_id = FederatedId::new(origin, local_id);
    let short = fed_id.short();

    // Short format should include parts of both origin and local ID
    assert!(!short.is_empty());
    assert!(short.contains(':'));
}

#[test]
fn test_federated_id_equality() {
    let origin = iroh::SecretKey::generate(&mut rand::rng()).public();
    let local_id = [0xef; 32];

    let fed_id1 = FederatedId::new(origin, local_id);
    let fed_id2 = FederatedId::new(origin, local_id);

    assert_eq!(fed_id1, fed_id2);
}

#[test]
fn test_federated_id_different_origins() {
    let origin1 = iroh::SecretKey::generate(&mut rand::rng()).public();
    let origin2 = iroh::SecretKey::generate(&mut rand::rng()).public();
    let local_id = [0x11; 32];

    let fed_id1 = FederatedId::new(origin1, local_id);
    let fed_id2 = FederatedId::new(origin2, local_id);

    // Same local ID but different origins should be different
    assert_ne!(fed_id1, fed_id2);
}

// ============================================================================
// Federation Settings Tests
// ============================================================================

#[test]
fn test_federation_settings_public() {
    let settings = FederationSettings::public();

    assert_eq!(settings.mode, FederationMode::Public);
    assert!(settings.allowed_clusters.is_empty());
}

#[test]
fn test_federation_settings_allowlist() {
    let cluster1 = iroh::SecretKey::generate(&mut rand::rng()).public();
    let cluster2 = iroh::SecretKey::generate(&mut rand::rng()).public();

    let settings = FederationSettings::allowlist(vec![cluster1, cluster2]);

    assert_eq!(settings.mode, FederationMode::AllowList);
    assert_eq!(settings.allowed_clusters.len(), 2);
    assert!(settings.allowed_clusters.contains(&cluster1));
    assert!(settings.allowed_clusters.contains(&cluster2));
}

#[test]
fn test_federation_settings_disabled() {
    let settings = FederationSettings::disabled();

    assert_eq!(settings.mode, FederationMode::Disabled);
}

#[test]
fn test_federation_settings_is_cluster_allowed() {
    let allowed = iroh::SecretKey::generate(&mut rand::rng()).public();
    let not_allowed = iroh::SecretKey::generate(&mut rand::rng()).public();

    // Public mode - anyone can access
    let public_settings = FederationSettings::public();
    assert!(public_settings.is_cluster_allowed(&allowed));
    assert!(public_settings.is_cluster_allowed(&not_allowed));

    // Allowlist mode - only allowed clusters
    let allowlist_settings = FederationSettings::allowlist(vec![allowed]);
    assert!(allowlist_settings.is_cluster_allowed(&allowed));
    assert!(!allowlist_settings.is_cluster_allowed(&not_allowed));

    // Disabled mode - no one can access
    let disabled_settings = FederationSettings::disabled();
    assert!(!disabled_settings.is_cluster_allowed(&allowed));
    assert!(!disabled_settings.is_cluster_allowed(&not_allowed));
}

// ============================================================================
// Trust Manager Tests
// ============================================================================

#[test]
fn test_trust_manager_default_public() {
    let manager = TrustManager::new();
    let key = iroh::SecretKey::generate(&mut rand::rng()).public();

    assert_eq!(manager.trust_level(&key), TrustLevel::Public);
    assert!(!manager.is_trusted(&key));
    assert!(!manager.is_blocked(&key));
}

#[test]
fn test_trust_manager_add_trusted() {
    let manager = TrustManager::new();
    let key = iroh::SecretKey::generate(&mut rand::rng()).public();

    assert!(manager.add_trusted(key, "test-cluster".to_string(), None));
    assert!(manager.is_trusted(&key));
    assert_eq!(manager.trust_level(&key), TrustLevel::Trusted);
}

#[test]
fn test_trust_manager_remove_trusted() {
    let manager = TrustManager::new();
    let key = iroh::SecretKey::generate(&mut rand::rng()).public();

    manager.add_trusted(key, "test-cluster".to_string(), None);
    assert!(manager.is_trusted(&key));

    assert!(manager.remove_trusted(&key));
    assert!(!manager.is_trusted(&key));
}

#[test]
fn test_trust_manager_block() {
    let manager = TrustManager::new();
    let key = iroh::SecretKey::generate(&mut rand::rng()).public();

    assert!(manager.block(key));
    assert!(manager.is_blocked(&key));
    assert_eq!(manager.trust_level(&key), TrustLevel::Blocked);
}

#[test]
fn test_trust_manager_block_overrides_trust() {
    let manager = TrustManager::new();
    let key = iroh::SecretKey::generate(&mut rand::rng()).public();

    manager.add_trusted(key, "test-cluster".to_string(), None);
    assert!(manager.is_trusted(&key));

    manager.block(key);
    assert!(!manager.is_trusted(&key));
    assert!(manager.is_blocked(&key));
}

#[test]
fn test_trust_manager_trust_removes_block() {
    let manager = TrustManager::new();
    let key = iroh::SecretKey::generate(&mut rand::rng()).public();

    manager.block(key);
    assert!(manager.is_blocked(&key));

    manager.add_trusted(key, "test-cluster".to_string(), None);
    assert!(!manager.is_blocked(&key));
    assert!(manager.is_trusted(&key));
}

#[test]
fn test_trust_manager_can_access_resource() {
    let manager = TrustManager::new();
    let trusted_key = iroh::SecretKey::generate(&mut rand::rng()).public();
    let public_key = iroh::SecretKey::generate(&mut rand::rng()).public();
    let blocked_key = iroh::SecretKey::generate(&mut rand::rng()).public();

    manager.add_trusted(trusted_key, "trusted".to_string(), None);
    manager.block(blocked_key);

    // Trusted can access any mode
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
    assert!(!manager.can_access_resource(&blocked_key, &FederationMode::Disabled));
}

#[test]
fn test_trust_manager_pending_request() {
    let manager = TrustManager::new();
    let identity = ClusterIdentity::generate("requester".to_string());
    let signed = identity.to_signed();

    assert!(manager.add_trust_request(signed.clone(), Some("please trust me".to_string())));

    let pending = manager.list_pending_requests();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].message, Some("please trust me".to_string()));
}

#[test]
fn test_trust_manager_accept_request() {
    let manager = TrustManager::new();
    let identity = ClusterIdentity::generate("requester".to_string());
    let signed = identity.to_signed();
    let key = signed.public_key();

    manager.add_trust_request(signed, None);
    assert!(!manager.is_trusted(&key));

    assert!(manager.accept_trust_request(&key));
    assert!(manager.is_trusted(&key));
    assert!(manager.list_pending_requests().is_empty());
}

#[test]
fn test_trust_manager_reject_request() {
    let manager = TrustManager::new();
    let identity = ClusterIdentity::generate("requester".to_string());
    let signed = identity.to_signed();
    let key = signed.public_key();

    manager.add_trust_request(signed, None);
    assert!(manager.reject_trust_request(&key));
    assert!(!manager.is_trusted(&key));
    assert!(manager.list_pending_requests().is_empty());
}

#[test]
fn test_trust_manager_reject_blocked_request() {
    let manager = TrustManager::new();
    let identity = ClusterIdentity::generate("blocked-requester".to_string());
    let signed = identity.to_signed();
    let key = signed.public_key();

    // Block the cluster first
    manager.block(key);

    // Trust request from blocked cluster should be rejected
    assert!(!manager.add_trust_request(signed, None));
    assert!(manager.list_pending_requests().is_empty());
}

#[test]
fn test_trust_manager_with_initial_trusted() {
    let key1 = iroh::SecretKey::generate(&mut rand::rng()).public();
    let key2 = iroh::SecretKey::generate(&mut rand::rng()).public();

    let manager = TrustManager::with_trusted(vec![key1, key2]);

    assert!(manager.is_trusted(&key1));
    assert!(manager.is_trusted(&key2));
    assert_eq!(manager.list_trusted().len(), 2);
}

// ============================================================================
// Federation Protocol Message Tests
// ============================================================================

#[test]
fn test_handshake_request_serialization() {
    let identity = ClusterIdentity::generate("test-cluster".to_string());

    let request = FederationRequest::Handshake {
        identity: identity.to_signed(),
        protocol_version: FEDERATION_PROTOCOL_VERSION,
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
            assert_eq!(protocol_version, FEDERATION_PROTOCOL_VERSION);
            assert_eq!(capabilities, vec!["forge"]);
        }
        _ => panic!("expected Handshake"),
    }
}

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
            resource_type,
            limit,
            ..
        } => {
            assert_eq!(resource_type, Some("forge:repo".to_string()));
            assert_eq!(limit, 100);
        }
        _ => panic!("expected ListResources"),
    }
}

#[test]
fn test_get_resource_state_request_serialization() {
    let origin = iroh::SecretKey::generate(&mut rand::rng()).public();
    let fed_id = FederatedId::new(origin, [0xab; 32]);

    let request = FederationRequest::GetResourceState { fed_id };

    let bytes = postcard::to_allocvec(&request).unwrap();
    let parsed: FederationRequest = postcard::from_bytes(&bytes).unwrap();

    match parsed {
        FederationRequest::GetResourceState { fed_id: parsed_id } => {
            assert_eq!(parsed_id, fed_id);
        }
        _ => panic!("expected GetResourceState"),
    }
}

#[test]
fn test_sync_objects_request_serialization() {
    let origin = iroh::SecretKey::generate(&mut rand::rng()).public();
    let fed_id = FederatedId::new(origin, [0xcd; 32]);

    let request = FederationRequest::SyncObjects {
        fed_id,
        want_types: vec!["blob".to_string(), "commit".to_string()],
        have_hashes: vec![[0x11; 32], [0x22; 32]],
        limit: 500,
    };

    let bytes = postcard::to_allocvec(&request).unwrap();
    let parsed: FederationRequest = postcard::from_bytes(&bytes).unwrap();

    match parsed {
        FederationRequest::SyncObjects {
            want_types,
            have_hashes,
            limit,
            ..
        } => {
            assert_eq!(want_types, vec!["blob", "commit"]);
            assert_eq!(have_hashes.len(), 2);
            assert_eq!(limit, 500);
        }
        _ => panic!("expected SyncObjects"),
    }
}

#[test]
fn test_handshake_response_serialization() {
    let identity = ClusterIdentity::generate("remote-cluster".to_string());

    let response = FederationResponse::Handshake {
        identity: identity.to_signed(),
        protocol_version: FEDERATION_PROTOCOL_VERSION,
        capabilities: vec!["forge".to_string()],
        trusted: true,
    };

    let bytes = postcard::to_allocvec(&response).unwrap();
    let parsed: FederationResponse = postcard::from_bytes(&bytes).unwrap();

    match parsed {
        FederationResponse::Handshake {
            protocol_version,
            trusted,
            ..
        } => {
            assert_eq!(protocol_version, FEDERATION_PROTOCOL_VERSION);
            assert!(trusted);
        }
        _ => panic!("expected Handshake response"),
    }
}

#[test]
fn test_resource_state_response_serialization() {
    let mut heads = HashMap::new();
    heads.insert("heads/main".to_string(), [0xef; 32]);
    heads.insert("heads/develop".to_string(), [0xfe; 32]);

    let response = FederationResponse::ResourceState {
        found: true,
        heads,
        metadata: None,
    };

    let bytes = postcard::to_allocvec(&response).unwrap();
    let parsed: FederationResponse = postcard::from_bytes(&bytes).unwrap();

    match parsed {
        FederationResponse::ResourceState { found, heads, .. } => {
            assert!(found);
            assert_eq!(heads.len(), 2);
            assert!(heads.contains_key("heads/main"));
        }
        _ => panic!("expected ResourceState"),
    }
}

#[test]
fn test_error_response_serialization() {
    let response = FederationResponse::Error {
        code: "NOT_FOUND".to_string(),
        message: "Resource not found".to_string(),
    };

    let bytes = postcard::to_allocvec(&response).unwrap();
    let parsed: FederationResponse = postcard::from_bytes(&bytes).unwrap();

    match parsed {
        FederationResponse::Error { code, message } => {
            assert_eq!(code, "NOT_FOUND");
            assert_eq!(message, "Resource not found");
        }
        _ => panic!("expected Error response"),
    }
}

// ============================================================================
// Content Hash Verification Tests
// ============================================================================

#[test]
fn test_content_hash_verification() {
    use aspen::cluster::federation::trust::verify_content_hash;

    let data = b"hello world";
    let hash = *blake3::hash(data).as_bytes();

    assert!(verify_content_hash(data, &hash));
    assert!(!verify_content_hash(b"different data", &hash));
    assert!(!verify_content_hash(data, &[0xff; 32]));
}

// ============================================================================
// Combined Flow Tests
// ============================================================================

#[test]
fn test_federate_and_check_access() {
    // Simulate federating a repository and checking access

    // Create two clusters
    let cluster_a = ClusterIdentity::generate("cluster-a".to_string());
    let cluster_b = ClusterIdentity::generate("cluster-b".to_string());

    // Create trust manager for cluster A
    let trust_manager = TrustManager::new();

    // Create a federated repository on cluster A
    let origin = cluster_a.public_key();
    let repo_id = [0xaa; 32];
    let fed_id = FederatedId::new(origin, repo_id);

    // Initially federate as public
    let mut resource_settings: HashMap<FederatedId, FederationSettings> = HashMap::new();
    resource_settings.insert(fed_id, FederationSettings::public());

    // Cluster B (untrusted) should be able to access public resource
    assert!(trust_manager.can_access_resource(&cluster_b.public_key(), &FederationMode::Public));

    // Change to allowlist mode with only cluster B allowed
    let settings = FederationSettings::allowlist(vec![cluster_b.public_key()]);
    resource_settings.insert(fed_id, settings);

    // Cluster B should still have access via allowlist
    assert!(resource_settings
        .get(&fed_id)
        .unwrap()
        .is_cluster_allowed(&cluster_b.public_key()));

    // Create cluster C (not in allowlist)
    let cluster_c = ClusterIdentity::generate("cluster-c".to_string());
    assert!(!resource_settings
        .get(&fed_id)
        .unwrap()
        .is_cluster_allowed(&cluster_c.public_key()));

    // If we trust cluster C, it can access via trust manager
    trust_manager.add_trusted(cluster_c.public_key(), "cluster-c".to_string(), None);
    assert!(trust_manager.can_access_resource(&cluster_c.public_key(), &FederationMode::AllowList));
}

#[test]
fn test_federation_identity_exchange() {
    // Simulate identity exchange during handshake

    let cluster_a = ClusterIdentity::generate("cluster-a".to_string());
    let cluster_b = ClusterIdentity::generate("cluster-b".to_string());

    // Cluster A creates a signed identity
    let signed_a = cluster_a.to_signed();

    // Cluster B verifies the signature
    assert!(signed_a.verify());
    assert_eq!(signed_a.name(), "cluster-a");

    // Cluster B creates trust manager and trusts cluster A
    let trust_b = TrustManager::new();
    trust_b.add_trusted(
        signed_a.public_key(),
        signed_a.name().to_string(),
        Some("Manually trusted".to_string()),
    );

    // Verify the trust relationship
    assert!(trust_b.is_trusted(&cluster_a.public_key()));

    // Cluster B creates signed identity
    let signed_b = cluster_b.to_signed();

    // Cluster A verifies cluster B's identity
    assert!(signed_b.verify());
    assert_eq!(signed_b.name(), "cluster-b");
}

#[test]
fn test_multiple_federated_repos() {
    // Test managing multiple federated repositories

    let cluster = ClusterIdentity::generate("main-cluster".to_string());
    let origin = cluster.public_key();

    let mut settings: HashMap<FederatedId, FederationSettings> = HashMap::new();

    // Create 3 repos with different federation modes
    let repo1 = FederatedId::new(origin, [0x01; 32]);
    let repo2 = FederatedId::new(origin, [0x02; 32]);
    let repo3 = FederatedId::new(origin, [0x03; 32]);

    settings.insert(repo1, FederationSettings::public());
    settings.insert(
        repo2,
        FederationSettings::allowlist(vec![iroh::SecretKey::generate(&mut rand::rng()).public()]),
    );
    settings.insert(repo3, FederationSettings::disabled());

    // Verify we can list federated repos
    let federated_repos: Vec<_> = settings
        .iter()
        .filter(|(_, s)| !matches!(s.mode, FederationMode::Disabled))
        .map(|(id, _)| *id)
        .collect();

    assert_eq!(federated_repos.len(), 2);
    assert!(federated_repos.contains(&repo1));
    assert!(federated_repos.contains(&repo2));
    assert!(!federated_repos.contains(&repo3));
}

// ============================================================================
// Stress Tests (Tiger Style limits)
// ============================================================================

#[test]
fn test_trust_manager_capacity_limits() {
    use aspen::cluster::federation::trust::MAX_TRUSTED_CLUSTERS;

    let manager = TrustManager::new();

    // Add up to the maximum trusted clusters
    for i in 0..MAX_TRUSTED_CLUSTERS {
        let key = iroh::SecretKey::generate(&mut rand::rng()).public();
        assert!(
            manager.add_trusted(key, format!("cluster-{}", i), None),
            "Should allow adding trusted cluster at index {}",
            i
        );
    }

    // Verify we hit the limit
    let extra_key = iroh::SecretKey::generate(&mut rand::rng()).public();
    assert!(
        !manager.add_trusted(extra_key, "over-limit".to_string(), None),
        "Should reject adding cluster over limit"
    );

    assert_eq!(manager.list_trusted().len(), MAX_TRUSTED_CLUSTERS);
}

#[test]
fn test_federation_settings_many_allowed_clusters() {
    // Create settings with many allowed clusters
    let allowed: Vec<_> = (0..100)
        .map(|_| iroh::SecretKey::generate(&mut rand::rng()).public())
        .collect();

    let settings = FederationSettings::allowlist(allowed.clone());

    // All should be accessible
    for key in &allowed {
        assert!(settings.is_cluster_allowed(key));
    }

    // Random key should not be accessible
    let random_key = iroh::SecretKey::generate(&mut rand::rng()).public();
    assert!(!settings.is_cluster_allowed(&random_key));
}
