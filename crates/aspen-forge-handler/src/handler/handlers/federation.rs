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
    forge_node: &ForgeNodeRef,
    repo_id: String,
    mode: String,
    cluster_identity: Option<&Arc<aspen_cluster::federation::ClusterIdentity>>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::FederateRepositoryResultResponse;
    use aspen_cluster::federation::FederatedId;
    use aspen_cluster::federation::FederationSettings;
    use aspen_forge::identity::RepoId;

    let identity = match cluster_identity {
        Some(id) => id,
        None => {
            return Ok(ClientRpcResponse::FederateRepositoryResult(FederateRepositoryResultResponse {
                is_success: false,
                fed_id: None,
                error: Some("federation not configured on this cluster".to_string()),
            }));
        }
    };

    // Parse repo ID
    let repo_id_parsed = match RepoId::from_hex(&repo_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::FederateRepositoryResult(FederateRepositoryResultResponse {
                is_success: false,
                fed_id: None,
                error: Some(format!("invalid repo ID: {e}")),
            }));
        }
    };

    // Verify repo exists
    if let Err(e) = forge_node.get_repo(&repo_id_parsed).await {
        return Ok(ClientRpcResponse::FederateRepositoryResult(FederateRepositoryResultResponse {
            is_success: false,
            fed_id: None,
            error: Some(format!("repo not found: {e}")),
        }));
    }

    // Construct federated ID from cluster pubkey + repo hash
    let fed_id = FederatedId::new(identity.public_key(), repo_id_parsed.0);

    // Parse federation mode
    let fed_mode = match mode.as_str() {
        "public" => aspen_cluster::federation::FederationMode::Public,
        "allowlist" => aspen_cluster::federation::FederationMode::AllowList,
        "disabled" => aspen_cluster::federation::FederationMode::Disabled,
        _ => aspen_cluster::federation::FederationMode::Public,
    };

    let settings = FederationSettings {
        mode: fed_mode,
        resource_type: Some("forge:repo".to_string()),
        ..Default::default()
    };

    // Persist to KV
    if let Err(e) = forge_node.set_federation_settings(&fed_id, &settings).await {
        return Ok(ClientRpcResponse::FederateRepositoryResult(FederateRepositoryResultResponse {
            is_success: false,
            fed_id: None,
            error: Some(format!("failed to persist federation settings: {e}")),
        }));
    }

    tracing::info!(
        repo_id = %repo_id,
        fed_id = %fed_id,
        mode = %mode,
        "repository federated"
    );

    Ok(ClientRpcResponse::FederateRepositoryResult(FederateRepositoryResultResponse {
        is_success: true,
        fed_id: Some(fed_id.to_string()),
        error: None,
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

/// Handle a one-shot federation sync pull from a remote cluster.
///
/// Connects to the remote peer via iroh QUIC, performs a federation handshake,
/// lists resources, and returns the remote cluster's identity and resource state.
pub(crate) async fn handle_federation_sync_peer(
    peer_node_id: &str,
    peer_addr: Option<&str>,
    cluster_identity: Option<&Arc<aspen_cluster::federation::ClusterIdentity>>,
    iroh_endpoint: Option<&Arc<iroh::Endpoint>>,
    forge_node: &ForgeNodeRef,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::FederationSyncPeerResponse;
    use aspen_client_api::SyncPeerResourceInfo;

    let identity = match cluster_identity {
        Some(id) => id,
        None => {
            return Ok(ClientRpcResponse::FederationSyncPeerResult(FederationSyncPeerResponse {
                is_success: false,
                remote_cluster_name: None,
                remote_cluster_key: None,
                trusted: None,
                resources: Vec::new(),
                error: Some("federation not configured on this cluster".to_string()),
            }));
        }
    };

    let endpoint = match iroh_endpoint {
        Some(ep) => ep,
        None => {
            return Ok(ClientRpcResponse::FederationSyncPeerResult(FederationSyncPeerResponse {
                is_success: false,
                remote_cluster_name: None,
                remote_cluster_key: None,
                trusted: None,
                resources: Vec::new(),
                error: Some("iroh endpoint not available".to_string()),
            }));
        }
    };

    // Parse the peer node ID
    let peer_key: iroh::PublicKey = peer_node_id
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid peer node ID '{}': {}", peer_node_id, e))?;

    // Build endpoint address with optional direct address hint
    let endpoint_addr = if let Some(addr_str) = peer_addr {
        if let Ok(socket_addr) = addr_str.parse::<std::net::SocketAddr>() {
            tracing::debug!(peer = %peer_key, addr = %socket_addr, "using direct address hint for federation peer");
            let mut addr = iroh::EndpointAddr::from(peer_key);
            addr.addrs.insert(iroh::TransportAddr::Ip(socket_addr));
            addr
        } else {
            tracing::warn!(addr = %addr_str, "invalid peer address hint, using node ID only");
            iroh::EndpointAddr::from(peer_key)
        }
    } else {
        iroh::EndpointAddr::from(peer_key)
    };

    // Connect and perform handshake
    let (connection, remote_identity) =
        match aspen_cluster::federation::sync::connect_to_cluster(endpoint, identity, endpoint_addr).await {
            Ok(result) => result,
            Err(e) => {
                return Ok(ClientRpcResponse::FederationSyncPeerResult(FederationSyncPeerResponse {
                    is_success: false,
                    remote_cluster_name: None,
                    remote_cluster_key: None,
                    trusted: None,
                    resources: Vec::new(),
                    error: Some(format!("connection failed: {e}")),
                }));
            }
        };

    // List resources on the remote cluster
    let resources =
        match aspen_cluster::federation::sync::list_remote_resources(&connection, Some("forge:repo"), 100).await {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(error = %e, "failed to list remote resources, returning handshake only");
                Vec::new()
            }
        };

    // Query ref heads for each resource and store locally
    let mut resource_infos = Vec::with_capacity(resources.len());
    for r in &resources {
        let (was_found, heads, _metadata) =
            match aspen_cluster::federation::sync::get_remote_resource_state(&connection, &r.fed_id).await {
                Ok(result) => result,
                Err(e) => {
                    tracing::warn!(fed_id = %r.fed_id.short(), error = %e, "failed to get resource state");
                    (false, std::collections::HashMap::new(), None)
                }
            };

        let ref_heads: Vec<(String, String)> = if was_found {
            heads.iter().map(|(name, hash)| (name.clone(), hex::encode(hash))).collect()
        } else {
            Vec::new()
        };

        // Store synced refs in local KV for persistence (best-effort)
        let origin_short = {
            let full = r.fed_id.origin().to_string();
            full[..16.min(full.len())].to_string()
        };
        let local_short = r.fed_id.short();
        for (ref_name, hex_hash) in &ref_heads {
            let key = format!("_fed:sync:{}:{}:refs/{}", origin_short, local_short, ref_name);
            if let Err(e) = forge_node.write_kv(&key, hex_hash).await {
                tracing::warn!(key = %key, error = %e, "failed to persist synced ref");
            }
        }

        resource_infos.push(SyncPeerResourceInfo {
            resource_type: r.resource_type.clone(),
            ref_count: ref_heads.len() as u32,
            ref_names: ref_heads.iter().map(|(name, _)| name.clone()).collect(),
            ref_heads,
            fed_id: Some(r.fed_id.to_string()),
        });
    }

    Ok(ClientRpcResponse::FederationSyncPeerResult(FederationSyncPeerResponse {
        is_success: true,
        remote_cluster_name: Some(remote_identity.name().to_string()),
        remote_cluster_key: Some(remote_identity.public_key().to_string()),
        trusted: Some(true), // We connected, so handshake succeeded
        resources: resource_infos,
        error: None,
    }))
}

/// Fetch ref objects from a remote federated repository and persist locally.
///
/// Connects to the remote peer, performs a federation handshake, then calls
/// `SyncObjects` with `want_types: ["refs"]`. Received refs are written to
/// local KV under `_fed:mirror:{origin_short}:{local_id_short}:refs/{name}`.
pub(crate) async fn handle_federation_fetch_refs(
    peer_node_id: &str,
    peer_addr: Option<&str>,
    fed_id_str: &str,
    cluster_identity: Option<&Arc<aspen_cluster::federation::ClusterIdentity>>,
    iroh_endpoint: Option<&Arc<iroh::Endpoint>>,
    forge_node: &ForgeNodeRef,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::FederationFetchRefsResponse;
    use aspen_cluster::federation::FederatedId;
    use aspen_cluster::federation::sync::RefEntry;

    let identity = match cluster_identity {
        Some(id) => id,
        None => {
            return Ok(ClientRpcResponse::FederationFetchRefsResult(FederationFetchRefsResponse {
                is_success: false,
                fetched: 0,
                already_present: 0,
                errors: Vec::new(),
                error: Some("federation not configured on this cluster".to_string()),
            }));
        }
    };

    let endpoint = match iroh_endpoint {
        Some(ep) => ep,
        None => {
            return Ok(ClientRpcResponse::FederationFetchRefsResult(FederationFetchRefsResponse {
                is_success: false,
                fetched: 0,
                already_present: 0,
                errors: Vec::new(),
                error: Some("iroh endpoint not available".to_string()),
            }));
        }
    };

    // Parse the federated ID
    let fed_id: FederatedId = match fed_id_str.parse() {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::FederationFetchRefsResult(FederationFetchRefsResponse {
                is_success: false,
                fetched: 0,
                already_present: 0,
                errors: Vec::new(),
                error: Some(format!("invalid fed_id '{}': {}", fed_id_str, e)),
            }));
        }
    };

    // Parse the peer node ID
    let peer_key: iroh::PublicKey = match peer_node_id.parse() {
        Ok(k) => k,
        Err(e) => {
            return Ok(ClientRpcResponse::FederationFetchRefsResult(FederationFetchRefsResponse {
                is_success: false,
                fetched: 0,
                already_present: 0,
                errors: Vec::new(),
                error: Some(format!("invalid peer node ID '{}': {}", peer_node_id, e)),
            }));
        }
    };

    // Build endpoint address
    let endpoint_addr = if let Some(addr_str) = peer_addr {
        if let Ok(socket_addr) = addr_str.parse::<std::net::SocketAddr>() {
            let mut addr = iroh::EndpointAddr::from(peer_key);
            addr.addrs.insert(iroh::TransportAddr::Ip(socket_addr));
            addr
        } else {
            iroh::EndpointAddr::from(peer_key)
        }
    } else {
        iroh::EndpointAddr::from(peer_key)
    };

    // Connect and handshake
    let (connection, _remote_identity) =
        match aspen_cluster::federation::sync::connect_to_cluster(endpoint, identity, endpoint_addr).await {
            Ok(result) => result,
            Err(e) => {
                return Ok(ClientRpcResponse::FederationFetchRefsResult(FederationFetchRefsResponse {
                    is_success: false,
                    fetched: 0,
                    already_present: 0,
                    errors: Vec::new(),
                    error: Some(format!("connection failed: {e}")),
                }));
            }
        };

    // Collect have_hashes from existing mirrored refs
    let origin_short = {
        let full = fed_id.origin().to_string();
        full[..16.min(full.len())].to_string()
    };
    let local_short = fed_id.short();
    let mirror_prefix = format!("_fed:mirror:{}:{}:refs/", origin_short, local_short);

    let mut have_hashes = Vec::new();
    if let Ok(scan_result) = forge_node.scan_kv(&mirror_prefix, Some(1000)).await {
        for entry in &scan_result {
            // Reconstruct the RefEntry to compute its hash for have_hashes
            let ref_name = entry.0.strip_prefix(&mirror_prefix).unwrap_or(&entry.0);
            if let Ok(hash_bytes) = hex::decode(entry.1.trim())
                && hash_bytes.len() == 32
            {
                let mut head_hash = [0u8; 32];
                head_hash.copy_from_slice(&hash_bytes);
                let ref_entry = RefEntry {
                    ref_name: ref_name.to_string(),
                    head_hash,
                };
                let data = postcard::to_allocvec(&ref_entry).unwrap_or_default();
                let hash: [u8; 32] = blake3::hash(&data).into();
                have_hashes.push(hash);
            }
        }
    }

    let already_present = have_hashes.len() as u32;

    // Fetch ref objects
    let (objects, _has_more) = match aspen_cluster::federation::sync::sync_remote_objects(
        &connection,
        &fed_id,
        vec!["refs".to_string()],
        have_hashes,
        1000,
        None, // No delegate verification for ref entries
    )
    .await
    {
        Ok(result) => result,
        Err(e) => {
            return Ok(ClientRpcResponse::FederationFetchRefsResult(FederationFetchRefsResponse {
                is_success: false,
                fetched: 0,
                already_present,
                errors: Vec::new(),
                error: Some(format!("sync objects failed: {e}")),
            }));
        }
    };

    // Parse and persist received refs
    let mut fetched = 0u32;
    let mut errors = Vec::new();
    for obj in &objects {
        if obj.object_type != "ref" {
            continue;
        }

        // Verify BLAKE3 hash
        let computed: [u8; 32] = blake3::hash(&obj.data).into();
        if computed != obj.hash {
            errors.push(format!("hash mismatch for object {}", hex::encode(&obj.hash[..8])));
            continue;
        }

        // Deserialize RefEntry
        let ref_entry: RefEntry = match postcard::from_bytes(&obj.data) {
            Ok(e) => e,
            Err(e) => {
                errors.push(format!("failed to deserialize ref entry: {e}"));
                continue;
            }
        };

        // Persist to local KV
        let key = format!("{}{}", mirror_prefix, ref_entry.ref_name);
        let value = hex::encode(ref_entry.head_hash);
        if let Err(e) = forge_node.write_kv(&key, &value).await {
            errors.push(format!("failed to write {}: {e}", ref_entry.ref_name));
            continue;
        }

        fetched += 1;
    }

    tracing::info!(
        fed_id = %fed_id.short(),
        fetched = fetched,
        already_present = already_present,
        errors = errors.len(),
        "federation ref fetch complete"
    );

    Ok(ClientRpcResponse::FederationFetchRefsResult(FederationFetchRefsResponse {
        is_success: errors.is_empty(),
        fetched,
        already_present,
        errors,
        error: None,
    }))
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
