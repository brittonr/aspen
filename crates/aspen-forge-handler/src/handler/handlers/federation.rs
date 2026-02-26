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
