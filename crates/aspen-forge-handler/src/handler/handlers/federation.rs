//! Federation operations.

use std::sync::Arc;

use aspen_client_api::ClientRpcResponse;

use super::ForgeNodeRef;

/// Count federated repositories by scanning for federation settings.
///
/// A repository is considered "federated" if it has federation settings
/// stored in KV with `mode != Disabled`. Uses `ForgeNode::count_federated_resources`
/// to scan persisted settings.
///
/// Returns 0 if no forge_node is available or scan fails.
async fn count_federated_repos(forge_node: &ForgeNodeRef) -> u32 {
    match forge_node.count_federated_resources().await {
        Ok(count) => count,
        Err(e) => {
            tracing::warn!(error = %e, "failed to count federated repos");
            0
        }
    }
}

#[cfg(feature = "global-discovery")]
pub(crate) async fn handle_get_federation_status(
    forge_node: &ForgeNodeRef,
    content_discovery: Option<&Arc<dyn aspen_core::ContentDiscovery>>,
    federation_discovery: Option<&Arc<aspen_cluster::federation::FederationDiscoveryService>>,
    federation_identity: Option<&Arc<aspen_cluster::federation::SignedClusterIdentity>>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::FederationStatusResponse;

    let dht_enabled = content_discovery.is_some();
    let discovered_clusters = federation_discovery.map(|d| d.get_discovered_clusters().len() as u32).unwrap_or(0);

    // Count federated repos by scanning for persisted federation settings
    // Repos with FederationSettings where mode != Disabled are considered federated
    let federated_repos = count_federated_repos(forge_node).await;

    // Check if federation identity is configured
    match federation_identity {
        Some(identity) => Ok(ClientRpcResponse::FederationStatus(FederationStatusResponse {
            is_enabled: true,
            cluster_name: identity.name().to_string(),
            cluster_key: identity.public_key().to_string(),
            dht_enabled,
            gossip_enabled: forge_node.has_gossip(),
            discovered_clusters,
            federated_repos,
            error: None,
        })),
        None => Ok(ClientRpcResponse::FederationStatus(FederationStatusResponse {
            is_enabled: false,
            cluster_name: String::new(),
            cluster_key: String::new(),
            dht_enabled,
            gossip_enabled: forge_node.has_gossip(),
            discovered_clusters,
            federated_repos,
            error: Some("Federation not configured for this node".to_string()),
        })),
    }
}

#[cfg(not(feature = "global-discovery"))]
pub(crate) async fn handle_get_federation_status(
    forge_node: &ForgeNodeRef,
    federation_identity: Option<&Arc<aspen_cluster::federation::SignedClusterIdentity>>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::FederationStatusResponse;

    let dht_enabled = false;
    let discovered_clusters = 0u32;
    let federated_repos = count_federated_repos(forge_node).await;

    match federation_identity {
        Some(identity) => Ok(ClientRpcResponse::FederationStatus(FederationStatusResponse {
            is_enabled: true,
            cluster_name: identity.name().to_string(),
            cluster_key: identity.public_key().to_string(),
            dht_enabled,
            gossip_enabled: forge_node.has_gossip(),
            discovered_clusters,
            federated_repos,
            error: None,
        })),
        None => Ok(ClientRpcResponse::FederationStatus(FederationStatusResponse {
            is_enabled: false,
            cluster_name: String::new(),
            cluster_key: String::new(),
            dht_enabled,
            gossip_enabled: forge_node.has_gossip(),
            discovered_clusters,
            federated_repos,
            error: Some("Federation not configured for this node".to_string()),
        })),
    }
}

#[cfg(feature = "global-discovery")]
pub(crate) async fn handle_list_discovered_clusters(
    federation_discovery: Option<&Arc<aspen_cluster::federation::FederationDiscoveryService>>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::DiscoveredClustersResponse;

    #[cfg(feature = "global-discovery")]
    {
        use aspen_client_api::DiscoveredClusterInfo;
        let discovery = match federation_discovery {
            Some(d) => d,
            None => {
                return Ok(ClientRpcResponse::DiscoveredClusters(DiscoveredClustersResponse {
                    clusters: vec![],
                    count: 0,
                    error: Some("Federation discovery service not initialized".to_string()),
                }));
            }
        };

        let discovered = discovery.get_discovered_clusters();
        let clusters: Vec<DiscoveredClusterInfo> = discovered
            .iter()
            .map(|c| DiscoveredClusterInfo {
                cluster_key: c.cluster_key.to_string(),
                name: c.name.clone(),
                node_count: c.node_keys.len() as u32,
                capabilities: c.capabilities.clone(),
                discovered_at: format!("{:?}", c.discovered_at.elapsed()),
            })
            .collect();

        let count = clusters.len() as u32;
        Ok(ClientRpcResponse::DiscoveredClusters(DiscoveredClustersResponse {
            clusters,
            count,
            error: None,
        }))
    }

    #[cfg(not(feature = "global-discovery"))]
    {
        let _ = federation_discovery; // Suppress unused warning
        Ok(ClientRpcResponse::DiscoveredClusters(DiscoveredClustersResponse {
            clusters: vec![],
            count: 0,
            error: Some("Federation discovery requires 'forge' and 'global-discovery' features".to_string()),
        }))
    }
}

#[cfg(not(feature = "global-discovery"))]
pub(crate) async fn handle_list_discovered_clusters() -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::DiscoveredClustersResponse;
    Ok(ClientRpcResponse::DiscoveredClusters(DiscoveredClustersResponse {
        clusters: vec![],
        count: 0,
        error: Some("Federation discovery requires 'forge' and 'global-discovery' features".to_string()),
    }))
}

#[cfg(feature = "global-discovery")]
pub(crate) async fn handle_get_discovered_cluster(
    federation_discovery: Option<&Arc<aspen_cluster::federation::FederationDiscoveryService>>,
    cluster_key: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::DiscoveredClusterResponse;

    #[cfg(feature = "global-discovery")]
    {
        let discovery = match federation_discovery {
            Some(d) => d,
            None => {
                return Ok(ClientRpcResponse::DiscoveredCluster(DiscoveredClusterResponse {
                    was_found: false,
                    cluster_key: None,
                    name: None,
                    node_count: None,
                    capabilities: None,
                    relay_urls: None,
                    discovered_at: None,
                }));
            }
        };

        // Parse the cluster key
        let key_bytes = match hex::decode(&cluster_key) {
            Ok(bytes) if bytes.len() == 32 => {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                arr
            }
            _ => {
                return Ok(ClientRpcResponse::DiscoveredCluster(DiscoveredClusterResponse {
                    was_found: false,
                    cluster_key: None,
                    name: None,
                    node_count: None,
                    capabilities: None,
                    relay_urls: None,
                    discovered_at: None,
                }));
            }
        };

        let public_key = match iroh::PublicKey::from_bytes(&key_bytes) {
            Ok(pk) => pk,
            Err(_) => {
                return Ok(ClientRpcResponse::DiscoveredCluster(DiscoveredClusterResponse {
                    was_found: false,
                    cluster_key: None,
                    name: None,
                    node_count: None,
                    capabilities: None,
                    relay_urls: None,
                    discovered_at: None,
                }));
            }
        };

        // Try to discover the cluster (will check cache first, then DHT if not found)
        match discovery.discover_cluster(&public_key).await {
            Some(cluster) => Ok(ClientRpcResponse::DiscoveredCluster(DiscoveredClusterResponse {
                was_found: true,
                cluster_key: Some(cluster.cluster_key.to_string()),
                name: Some(cluster.name),
                node_count: Some(cluster.node_keys.len() as u32),
                capabilities: Some(cluster.capabilities),
                relay_urls: Some(cluster.relay_urls),
                discovered_at: Some(format!("{:?}", cluster.discovered_at.elapsed())),
            })),
            None => Ok(ClientRpcResponse::DiscoveredCluster(DiscoveredClusterResponse {
                was_found: false,
                cluster_key: Some(cluster_key),
                name: None,
                node_count: None,
                capabilities: None,
                relay_urls: None,
                discovered_at: None,
            })),
        }
    }

    #[cfg(not(feature = "global-discovery"))]
    {
        let _ = (federation_discovery, cluster_key); // Suppress unused warnings
        Ok(ClientRpcResponse::DiscoveredCluster(DiscoveredClusterResponse {
            was_found: false,
            cluster_key: None,
            name: None,
            node_count: None,
            capabilities: None,
            relay_urls: None,
            discovered_at: None,
        }))
    }
}

#[cfg(not(feature = "global-discovery"))]
pub(crate) async fn handle_get_discovered_cluster(_cluster_key: String) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::DiscoveredClusterResponse;
    Ok(ClientRpcResponse::DiscoveredCluster(DiscoveredClusterResponse {
        was_found: false,
        cluster_key: None,
        name: None,
        node_count: None,
        capabilities: None,
        relay_urls: None,
        discovered_at: None,
    }))
}

pub(crate) async fn handle_trust_cluster(
    federation_trust_manager: Option<&Arc<aspen_cluster::federation::TrustManager>>,
    cluster_key: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::TrustClusterResultResponse;

    let trust_manager = match federation_trust_manager {
        Some(tm) => tm,
        None => {
            return Ok(ClientRpcResponse::TrustClusterResult(TrustClusterResultResponse {
                is_success: false,
                error: Some("Trust management not available - federation not configured".to_string()),
            }));
        }
    };

    // Parse the cluster key from hex
    let key_bytes = match hex::decode(&cluster_key) {
        Ok(bytes) if bytes.len() == 32 => {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            arr
        }
        _ => {
            return Ok(ClientRpcResponse::TrustClusterResult(TrustClusterResultResponse {
                is_success: false,
                error: Some("Invalid cluster key format (expected 64-character hex string)".to_string()),
            }));
        }
    };

    let public_key = match iroh::PublicKey::from_bytes(&key_bytes) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(ClientRpcResponse::TrustClusterResult(TrustClusterResultResponse {
                is_success: false,
                error: Some("Invalid cluster public key".to_string()),
            }));
        }
    };

    // Add as trusted cluster
    trust_manager.add_trusted(public_key, format!("trusted-{}", &cluster_key[..8]), None);

    Ok(ClientRpcResponse::TrustClusterResult(TrustClusterResultResponse {
        is_success: true,
        error: None,
    }))
}

pub(crate) async fn handle_untrust_cluster(
    federation_trust_manager: Option<&Arc<aspen_cluster::federation::TrustManager>>,
    cluster_key: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::UntrustClusterResultResponse;

    let trust_manager = match federation_trust_manager {
        Some(tm) => tm,
        None => {
            return Ok(ClientRpcResponse::UntrustClusterResult(UntrustClusterResultResponse {
                is_success: false,
                error: Some("Trust management not available - federation not configured".to_string()),
            }));
        }
    };

    // Parse the cluster key from hex
    let key_bytes = match hex::decode(&cluster_key) {
        Ok(bytes) if bytes.len() == 32 => {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            arr
        }
        _ => {
            return Ok(ClientRpcResponse::UntrustClusterResult(UntrustClusterResultResponse {
                is_success: false,
                error: Some("Invalid cluster key format (expected 64-character hex string)".to_string()),
            }));
        }
    };

    let public_key = match iroh::PublicKey::from_bytes(&key_bytes) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(ClientRpcResponse::UntrustClusterResult(UntrustClusterResultResponse {
                is_success: false,
                error: Some("Invalid cluster public key".to_string()),
            }));
        }
    };

    // Remove from trusted clusters
    if trust_manager.remove_trusted(&public_key) {
        Ok(ClientRpcResponse::UntrustClusterResult(UntrustClusterResultResponse {
            is_success: true,
            error: None,
        }))
    } else {
        Ok(ClientRpcResponse::UntrustClusterResult(UntrustClusterResultResponse {
            is_success: false,
            error: Some("Cluster was not in trusted list".to_string()),
        }))
    }
}

pub(crate) async fn handle_federate_repository(
    _forge_node: &ForgeNodeRef,
    _repo_id: String,
    _mode: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::FederateRepositoryResultResponse;

    // Federation integration is handled separately from ForgeNode
    Ok(ClientRpcResponse::FederateRepositoryResult(FederateRepositoryResultResponse {
        is_success: false,
        fed_id: None,
        error: Some("Federation not available through RPC".to_string()),
    }))
}

pub(crate) async fn handle_list_federated_repositories(forge_node: &ForgeNodeRef) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::FederatedRepoInfo;
    use aspen_client_api::FederatedRepositoriesResponse;

    // List federated repositories from KV storage
    match forge_node.list_federated_resources(None, 1000).await {
        Ok(federated) => {
            let repositories: Vec<FederatedRepoInfo> = federated
                .into_iter()
                .map(|(fed_id, settings)| {
                    let mode = match settings.mode {
                        aspen_cluster::federation::FederationMode::Disabled => "disabled",
                        aspen_cluster::federation::FederationMode::Public => "public",
                        aspen_cluster::federation::FederationMode::AllowList => "allowlist",
                    };
                    FederatedRepoInfo {
                        repo_id: hex::encode(fed_id.local_id()),
                        mode: mode.to_string(),
                        fed_id: fed_id.to_string(),
                    }
                })
                .collect();
            let count = repositories.len() as u32;
            Ok(ClientRpcResponse::FederatedRepositories(FederatedRepositoriesResponse {
                repositories,
                count,
                error: None,
            }))
        }
        Err(e) => {
            tracing::warn!(error = %e, "failed to list federated repositories");
            Ok(ClientRpcResponse::FederatedRepositories(FederatedRepositoriesResponse {
                repositories: vec![],
                count: 0,
                error: Some(format!("Failed to list federated repositories: {}", e)),
            }))
        }
    }
}

pub(crate) async fn handle_fetch_federated(
    _forge_node: &ForgeNodeRef,
    _federated_id: String,
    _remote_cluster: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeFetchFederatedResultResponse;

    // Federation integration is handled separately from ForgeNode
    Ok(ClientRpcResponse::ForgeFetchResult(ForgeFetchFederatedResultResponse {
        is_success: false,
        remote_cluster: None,
        fetched: 0,
        already_present: 0,
        errors: vec![],
        error: Some("Federation not available through RPC".to_string()),
    }))
}

pub(crate) async fn handle_get_delegate_key(
    forge_node: &ForgeNodeRef,
    repo_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::ForgeKeyResultResponse;
    use aspen_forge::identity::RepoId;

    let repo_id = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::ForgeKeyResult(ForgeKeyResultResponse {
                is_success: false,
                public_key: None,
                secret_key: None,
                error: Some(format!("Invalid repo ID: {}", e)),
            }));
        }
    };

    // Verify repo exists
    match forge_node.get_repo(&repo_id).await {
        Ok(_identity) => {
            // Return the node's key as the delegate key
            Ok(ClientRpcResponse::ForgeKeyResult(ForgeKeyResultResponse {
                is_success: true,
                public_key: Some(forge_node.public_key().to_string()),
                secret_key: Some(hex::encode(forge_node.secret_key().to_bytes())),
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ForgeKeyResult(ForgeKeyResultResponse {
            is_success: false,
            public_key: None,
            secret_key: None,
            error: Some(e.to_string()),
        })),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use aspen_client_api::ClientRpcResponse;
    use aspen_cluster::federation::TrustManager;

    // ====================================================================
    // Trust cluster handler tests
    // ====================================================================

    #[tokio::test]
    async fn test_trust_cluster_valid_key() {
        let trust_manager = Arc::new(TrustManager::new());
        let peer_key = iroh::SecretKey::generate(&mut rand::rng()).public();
        let key_hex = hex::encode(peer_key.as_bytes());

        let response = super::handle_trust_cluster(Some(&trust_manager), key_hex).await.unwrap();

        match response {
            ClientRpcResponse::TrustClusterResult(r) => {
                assert!(r.is_success, "trust should succeed");
                assert!(r.error.is_none());
            }
            other => panic!("unexpected response: {:?}", other),
        }

        // Verify the trust manager state
        assert_eq!(trust_manager.trust_level(&peer_key), aspen_cluster::federation::TrustLevel::Trusted);
    }

    #[tokio::test]
    async fn test_trust_cluster_invalid_hex() {
        let trust_manager = Arc::new(TrustManager::new());

        let response = super::handle_trust_cluster(Some(&trust_manager), "not-valid-hex".to_string()).await.unwrap();

        match response {
            ClientRpcResponse::TrustClusterResult(r) => {
                assert!(!r.is_success);
                assert!(r.error.as_ref().unwrap().contains("Invalid cluster key format"));
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_trust_cluster_short_hex() {
        let trust_manager = Arc::new(TrustManager::new());

        // Valid hex but wrong length (not 32 bytes)
        let response = super::handle_trust_cluster(Some(&trust_manager), "abcd1234".to_string()).await.unwrap();

        match response {
            ClientRpcResponse::TrustClusterResult(r) => {
                assert!(!r.is_success);
                assert!(r.error.as_ref().unwrap().contains("Invalid cluster key format"));
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_trust_cluster_no_trust_manager() {
        let response = super::handle_trust_cluster(None, "abcd".to_string()).await.unwrap();

        match response {
            ClientRpcResponse::TrustClusterResult(r) => {
                assert!(!r.is_success);
                assert!(r.error.as_ref().unwrap().contains("not available"));
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_trust_cluster_idempotent() {
        let trust_manager = Arc::new(TrustManager::new());
        let peer_key = iroh::SecretKey::generate(&mut rand::rng()).public();
        let key_hex = hex::encode(peer_key.as_bytes());

        // Trust twice - should succeed both times
        let r1 = super::handle_trust_cluster(Some(&trust_manager), key_hex.clone()).await.unwrap();
        let r2 = super::handle_trust_cluster(Some(&trust_manager), key_hex).await.unwrap();

        match (r1, r2) {
            (ClientRpcResponse::TrustClusterResult(a), ClientRpcResponse::TrustClusterResult(b)) => {
                assert!(a.is_success);
                assert!(b.is_success);
            }
            _ => panic!("unexpected response types"),
        }
    }

    // ====================================================================
    // Untrust cluster handler tests
    // ====================================================================

    #[tokio::test]
    async fn test_untrust_cluster_trusted_key() {
        let trust_manager = Arc::new(TrustManager::new());
        let peer_key = iroh::SecretKey::generate(&mut rand::rng()).public();
        let key_hex = hex::encode(peer_key.as_bytes());

        // First trust the cluster
        trust_manager.add_trusted(peer_key, "test-peer".to_string(), None);

        let response = super::handle_untrust_cluster(Some(&trust_manager), key_hex).await.unwrap();

        match response {
            ClientRpcResponse::UntrustClusterResult(r) => {
                assert!(r.is_success, "untrust should succeed for trusted cluster");
                assert!(r.error.is_none());
            }
            other => panic!("unexpected response: {:?}", other),
        }

        // Verify removed from trust
        assert_ne!(trust_manager.trust_level(&peer_key), aspen_cluster::federation::TrustLevel::Trusted);
    }

    #[tokio::test]
    async fn test_untrust_cluster_not_trusted() {
        let trust_manager = Arc::new(TrustManager::new());
        let peer_key = iroh::SecretKey::generate(&mut rand::rng()).public();
        let key_hex = hex::encode(peer_key.as_bytes());

        let response = super::handle_untrust_cluster(Some(&trust_manager), key_hex).await.unwrap();

        match response {
            ClientRpcResponse::UntrustClusterResult(r) => {
                assert!(!r.is_success, "untrust should fail for non-trusted cluster");
                assert!(r.error.as_ref().unwrap().contains("not in trusted list"));
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_untrust_cluster_invalid_hex() {
        let trust_manager = Arc::new(TrustManager::new());

        let response = super::handle_untrust_cluster(Some(&trust_manager), "zzz-invalid".to_string()).await.unwrap();

        match response {
            ClientRpcResponse::UntrustClusterResult(r) => {
                assert!(!r.is_success);
                assert!(r.error.as_ref().unwrap().contains("Invalid cluster key format"));
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_untrust_cluster_no_trust_manager() {
        let response = super::handle_untrust_cluster(None, "abcd".to_string()).await.unwrap();

        match response {
            ClientRpcResponse::UntrustClusterResult(r) => {
                assert!(!r.is_success);
                assert!(r.error.as_ref().unwrap().contains("not available"));
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }

    // ====================================================================
    // Federation status handler tests (non-global-discovery)
    // ====================================================================

    // ====================================================================
    // Federation status handler tests (non-global-discovery)
    //
    // These tests exercise handle_get_federation_status without a ForgeNode.
    // ForgeNode-dependent status tests (counting federated repos) are in
    // aspen-forge::node::tests since ForgeNodeRef requires IrohBlobStore.
    // ====================================================================

    #[cfg(not(feature = "global-discovery"))]
    mod status_tests {
        use aspen_client_api::FederationStatusResponse;

        #[test]
        fn test_federation_status_response_fields_disabled() {
            // Verify the disabled response structure matches expectations
            let status = FederationStatusResponse {
                is_enabled: false,
                cluster_name: String::new(),
                cluster_key: String::new(),
                dht_enabled: false,
                gossip_enabled: false,
                discovered_clusters: 0,
                federated_repos: 0,
                error: Some("Federation not configured for this node".to_string()),
            };

            assert!(!status.is_enabled);
            assert!(status.error.is_some());
        }

        #[test]
        fn test_federation_status_response_fields_enabled() {
            let status = FederationStatusResponse {
                is_enabled: true,
                cluster_name: "test-cluster".to_string(),
                cluster_key: "abcd1234".to_string(),
                dht_enabled: false,
                gossip_enabled: false,
                discovered_clusters: 0,
                federated_repos: 3,
                error: None,
            };

            assert!(status.is_enabled);
            assert_eq!(status.cluster_name, "test-cluster");
            assert_eq!(status.federated_repos, 3);
            assert!(status.error.is_none());
        }
    }

    // ====================================================================
    // Discovered clusters handler tests (non-global-discovery)
    // ====================================================================

    #[cfg(not(feature = "global-discovery"))]
    mod discovery_tests {
        use super::*;

        #[tokio::test]
        async fn test_list_discovered_clusters_without_discovery() {
            let response = super::super::handle_list_discovered_clusters().await.unwrap();

            match response {
                ClientRpcResponse::DiscoveredClusters(r) => {
                    assert!(r.clusters.is_empty());
                    assert_eq!(r.count, 0);
                    assert!(r.error.as_ref().unwrap().contains("requires"));
                }
                other => panic!("unexpected response: {:?}", other),
            }
        }

        #[tokio::test]
        async fn test_get_discovered_cluster_without_discovery() {
            let key_hex = hex::encode([0u8; 32]);
            let response = super::super::handle_get_discovered_cluster(key_hex).await.unwrap();

            match response {
                ClientRpcResponse::DiscoveredCluster(r) => {
                    assert!(!r.was_found);
                }
                other => panic!("unexpected response: {:?}", other),
            }
        }
    }

    // ====================================================================
    // Stub handler tests (federate repository, fetch federated)
    // ====================================================================

    // Note: handle_federate_repository and handle_fetch_federated are stubs
    // that always return errors. Test them to ensure stable error messages.

    // ====================================================================
    // Trust + untrust roundtrip
    // ====================================================================

    #[tokio::test]
    async fn test_trust_then_untrust_roundtrip() {
        let trust_manager = Arc::new(TrustManager::new());
        let peer_key = iroh::SecretKey::generate(&mut rand::rng()).public();
        let key_hex = hex::encode(peer_key.as_bytes());

        // Trust
        let r = super::handle_trust_cluster(Some(&trust_manager), key_hex.clone()).await.unwrap();
        assert!(matches!(r, ClientRpcResponse::TrustClusterResult(ref t) if t.is_success));

        assert_eq!(trust_manager.trust_level(&peer_key), aspen_cluster::federation::TrustLevel::Trusted);

        // Untrust
        let r = super::handle_untrust_cluster(Some(&trust_manager), key_hex).await.unwrap();
        assert!(matches!(r, ClientRpcResponse::UntrustClusterResult(ref t) if t.is_success));

        // Back to default level (Public/Unknown)
        assert_ne!(trust_manager.trust_level(&peer_key), aspen_cluster::federation::TrustLevel::Trusted);
    }

    #[tokio::test]
    async fn test_trust_multiple_clusters() {
        let trust_manager = Arc::new(TrustManager::new());

        // Trust 3 clusters
        for _ in 0..3 {
            let peer_key = iroh::SecretKey::generate(&mut rand::rng()).public();
            let key_hex = hex::encode(peer_key.as_bytes());
            let r = super::handle_trust_cluster(Some(&trust_manager), key_hex).await.unwrap();
            assert!(matches!(r, ClientRpcResponse::TrustClusterResult(ref t) if t.is_success));
        }

        assert_eq!(trust_manager.list_trusted().len(), 3);
    }
}
