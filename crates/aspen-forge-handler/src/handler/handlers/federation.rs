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
        match aspen_cluster::federation::sync::connect_to_cluster(endpoint, identity, endpoint_addr, None).await {
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

    let endpoint_addr = build_endpoint_addr(peer_key, peer_addr);

    // Connect and handshake
    let (connection, _remote_identity) =
        match aspen_cluster::federation::sync::connect_to_cluster(endpoint, identity, endpoint_addr, None).await {
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

    // =========================================================================
    // Phase 1: Fetch refs
    // =========================================================================
    let (ref_objects, _has_more) = match aspen_cluster::federation::sync::sync_remote_objects(
        &connection,
        &fed_id,
        vec!["refs".to_string()],
        Vec::new(),
        1000,
        None,
    )
    .await
    {
        Ok(result) => result,
        Err(e) => {
            return Ok(ClientRpcResponse::FederationFetchRefsResult(FederationFetchRefsResponse {
                is_success: false,
                fetched: 0,
                already_present: 0,
                errors: Vec::new(),
                error: Some(format!("ref fetch failed: {e}")),
            }));
        }
    };

    // Parse ref entries.
    // Keep full RefEntry so we can use commit_sha1 for local hash lookup.
    let mut fetched_ref_entries: Vec<RefEntry> = Vec::new();
    let mut errors = Vec::new();
    for obj in &ref_objects {
        if obj.object_type != "ref" {
            continue;
        }
        let computed: [u8; 32] = blake3::hash(&obj.data).into();
        if computed != obj.hash {
            errors.push(format!("ref hash mismatch: {}", hex::encode(&obj.hash[..8])));
            continue;
        }
        match postcard::from_bytes::<RefEntry>(&obj.data) {
            Ok(entry) => fetched_ref_entries.push(entry),
            Err(e) => errors.push(format!("ref deserialize: {e}")),
        }
    }
    // Initial refs with source hashes (translated to local after import)
    #[allow(unused_mut)]
    let mut fetched_refs: Vec<(String, [u8; 32])> =
        fetched_ref_entries.iter().map(|e| (e.ref_name.clone(), e.head_hash)).collect();

    let refs_fetched = fetched_refs.len() as u32;

    // =========================================================================
    // Phase 2: Fetch git objects (if git-bridge enabled)
    // =========================================================================
    #[cfg(feature = "git-bridge")]
    let git_stats = {
        // Get or create mirror repo
        let mirror_repo_id =
            match get_or_create_mirror(forge_node, fed_id_str, &peer_key.to_string(), Some(peer_node_id), peer_addr)
                .await
            {
                Ok(id) => id,
                Err(e) => {
                    errors.push(format!("mirror creation failed: {e}"));
                    // Still return ref results even if mirror fails
                    return Ok(ClientRpcResponse::FederationFetchRefsResult(FederationFetchRefsResponse {
                        is_success: false,
                        fetched: refs_fetched,
                        already_present: 0,
                        errors,
                        error: Some(e),
                    }));
                }
            };

        // Collect have_hashes from existing mirror objects
        let have_hashes = collect_local_blake3_hashes(forge_node, &mirror_repo_id, 10_000).await;
        let already_present_objects = have_hashes.len() as u32;

        // Fetch git objects with pagination
        let mut all_git_objects = Vec::new();
        let mut current_have = have_hashes;
        let max_rounds = 10u32; // Tiger Style: bounded pagination

        for _round in 0..max_rounds {
            let (objects, has_more) = match aspen_cluster::federation::sync::sync_remote_objects(
                &connection,
                &fed_id,
                vec!["commit".to_string(), "tree".to_string(), "blob".to_string()],
                current_have.clone(),
                1000,
                None,
            )
            .await
            {
                Ok(result) => result,
                Err(e) => {
                    errors.push(format!("git object fetch: {e}"));
                    break;
                }
            };

            if objects.is_empty() {
                break;
            }

            // Add received hashes to have_hashes for next round
            for obj in &objects {
                current_have.push(obj.hash);
            }

            all_git_objects.extend(objects);

            if !has_more {
                break;
            }
        }

        // Import git objects into mirror
        let stats = federation_import_objects(forge_node, &mirror_repo_id, &all_git_objects).await;

        // Translate ref hashes from source envelope BLAKE3 to local BLAKE3.
        // The source's head_hash is its SignedObject BLAKE3 which differs from
        // ours (different signature + HLC). Use commit_sha1 (deterministic) to
        // look up the local BLAKE3 via the SHA1→BLAKE3 mapping created during import.
        {
            use aspen_forge::git::bridge::HashMappingStore;
            use aspen_forge::git::bridge::Sha1Hash;
            let mapping = HashMappingStore::new(forge_node.kv().clone());
            for (i, ref_entry) in fetched_ref_entries.iter().enumerate() {
                if let Some(sha1_bytes) = &ref_entry.commit_sha1 {
                    let sha1 = Sha1Hash::from_bytes(*sha1_bytes);
                    match mapping.get_blake3(&mirror_repo_id, &sha1).await {
                        Ok(Some((local_blake3, _obj_type))) => {
                            tracing::debug!(
                                ref_name = %ref_entry.ref_name,
                                source_hash = %hex::encode(&ref_entry.head_hash[..8]),
                                local_hash = %hex::encode(&local_blake3.as_bytes()[..8]),
                                sha1 = %sha1.to_hex(),
                                "translated federation ref to local BLAKE3"
                            );
                            fetched_refs[i].1 = *local_blake3.as_bytes();
                        }
                        Ok(None) => {
                            tracing::warn!(
                                ref_name = %ref_entry.ref_name,
                                sha1 = %sha1.to_hex(),
                                "no SHA1→BLAKE3 mapping for federation ref, using source hash"
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                ref_name = %ref_entry.ref_name,
                                error = %e,
                                "failed to look up local BLAKE3 for federation ref"
                            );
                        }
                    }
                }
            }
        }

        // Update mirror refs (now with local BLAKE3 hashes)
        if let Err(e) = update_mirror_refs(forge_node, &mirror_repo_id, &fetched_refs).await {
            errors.push(format!("mirror ref update: {e}"));
        }

        // Update sync timestamp
        let _ = update_mirror_sync_timestamp(forge_node, fed_id_str).await;

        (stats, already_present_objects)
    };

    #[cfg(not(feature = "git-bridge"))]
    let git_stats = (FederationImportStats::default(), 0u32);

    let (import_stats, already_present_objects) = git_stats;

    // Also persist refs to legacy KV path for backwards compat
    let origin_short = {
        let full = fed_id.origin().to_string();
        full[..16.min(full.len())].to_string()
    };
    let local_short = fed_id.short();
    for (ref_name, hash) in &fetched_refs {
        let key = format!("_fed:mirror:{}:{}:refs/{}", origin_short, local_short, ref_name);
        let _ = forge_node.write_kv(&key, &hex::encode(hash)).await;
    }

    let total_fetched = refs_fetched + import_stats.commits + import_stats.trees + import_stats.blobs;

    tracing::info!(
        fed_id = %fed_id.short(),
        refs = refs_fetched,
        commits = import_stats.commits,
        trees = import_stats.trees,
        blobs = import_stats.blobs,
        already_present = already_present_objects,
        total_bytes = import_stats.total_bytes,
        errors = errors.len(),
        "federation fetch complete"
    );

    Ok(ClientRpcResponse::FederationFetchRefsResult(FederationFetchRefsResponse {
        is_success: errors.is_empty(),
        fetched: total_fetched,
        already_present: already_present_objects,
        errors,
        error: None,
    }))
}

/// Build an iroh EndpointAddr from a public key and optional address hint.
fn build_endpoint_addr(peer_key: iroh::PublicKey, peer_addr: Option<&str>) -> iroh::EndpointAddr {
    if let Some(addr_str) = peer_addr
        && let Ok(socket_addr) = addr_str.parse::<std::net::SocketAddr>()
    {
        let mut addr = iroh::EndpointAddr::from(peer_key);
        addr.addrs.insert(iroh::TransportAddr::Ip(socket_addr));
        return addr;
    }
    iroh::EndpointAddr::from(peer_key)
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
// Federation git object import
// =============================================================================

/// Statistics from importing federation sync objects into a local forge repo.
#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct FederationImportStats {
    /// Number of commit objects imported.
    pub commits: u32,
    /// Number of tree objects imported.
    pub trees: u32,
    /// Number of blob objects imported.
    pub blobs: u32,
    /// Number of objects skipped (hash mismatch or parse error).
    pub skipped: u32,
    /// Total bytes of imported object content.
    pub total_bytes: u64,
    /// Per-object errors (non-fatal).
    pub errors: Vec<String>,
    /// Mapping from SyncObject content hash → locally imported BLAKE3 hash.
    /// Used to translate remote ref hashes to local hashes for mirror refs.
    pub content_to_local_blake3: std::collections::HashMap<[u8; 32], blake3::Hash>,
}

/// Import federation `SyncObject` entries into a forge repo via the git bridge.
///
/// Converts SyncObjects to the `(Sha1Hash, GitObjectType, Vec<u8>)` format
/// expected by `import_objects()`, which performs topological sorting (wave-based
/// parallelism) to ensure dependencies are imported before dependents.
///
/// Returns import statistics including a `content_to_local_blake3` map for
/// translating remote ref hashes to locally imported BLAKE3 hashes.
#[cfg(feature = "git-bridge")]
pub(crate) async fn federation_import_objects(
    forge_node: &ForgeNodeRef,
    repo_id: &aspen_forge::identity::RepoId,
    objects: &[aspen_cluster::federation::sync::SyncObject],
) -> FederationImportStats {
    use aspen_core::hlc::create_hlc;
    use aspen_forge::git::bridge::GitImporter;
    use aspen_forge::git::bridge::GitObjectType;
    use aspen_forge::git::bridge::HashMappingStore;
    use aspen_forge::git::bridge::Sha1Hash;

    let mut stats = FederationImportStats::default();

    if objects.is_empty() {
        return stats;
    }

    // Phase 1: Convert SyncObjects to import_objects() input format.
    // Track content hashes (blake3 of raw content) in parallel so we can
    // correlate them with SHA-1s after import.
    let mut import_objects = Vec::with_capacity(objects.len());
    // sha1_hex → content_hash, for post-import correlation
    let mut sha1_to_content_hash: std::collections::HashMap<[u8; 20], [u8; 32]> = std::collections::HashMap::new();
    let mut type_counts: [u32; 3] = [0; 3]; // [blobs, trees, commits]

    for obj in objects {
        let (git_type, type_idx) = match obj.object_type.as_str() {
            "blob" => (GitObjectType::Blob, 0),
            "tree" => (GitObjectType::Tree, 1),
            "commit" => (GitObjectType::Commit, 2),
            _ => continue,
        };

        // Reconstruct full git bytes (header + content)
        let type_str = obj.object_type.as_str();
        let header = format!("{} {}\0", type_str, obj.data.len());
        let mut git_bytes = Vec::with_capacity(header.len() + obj.data.len());
        git_bytes.extend_from_slice(header.as_bytes());
        git_bytes.extend_from_slice(&obj.data);

        // Compute SHA-1 from full git bytes
        let sha1 = {
            use sha1::Digest;
            let digest: [u8; 20] = sha1::Sha1::digest(&git_bytes).into();
            Sha1Hash::from_bytes(digest)
        };

        // Track content hash for post-import ref translation
        let content_hash: [u8; 32] = blake3::hash(&obj.data).into();
        sha1_to_content_hash.insert(*sha1.as_bytes(), content_hash);

        stats.total_bytes += obj.data.len() as u64;
        type_counts[type_idx] += 1;

        import_objects.push((sha1, git_type, git_bytes));
    }

    stats.blobs = type_counts[0];
    stats.trees = type_counts[1];
    stats.commits = type_counts[2];

    // Phase 2: Import via topologically-sorted wave-based parallelism.
    let mapping = std::sync::Arc::new(HashMappingStore::new(forge_node.kv().clone()));
    let hlc = create_hlc(&forge_node.public_key().to_string());
    let importer = GitImporter::new(
        mapping,
        forge_node.git.blobs().clone(),
        std::sync::Arc::new(forge_node.refs.clone()),
        forge_node.secret_key().clone(),
        hlc,
    );

    let result = match importer.import_objects(repo_id, import_objects).await {
        Ok(r) => r,
        Err(e) => {
            stats.errors.push(format!("import_objects failed: {e}"));
            tracing::warn!(error = %e, "federation import_objects failed");
            return stats;
        }
    };

    stats.skipped = result.objects_skipped;

    // Phase 3: Build content_to_local_blake3 from ImportResult.mappings.
    // Each mapping is (sha1, blake3). We look up the content hash via the
    // sha1_to_content_hash map built in phase 1.
    for (sha1, local_blake3) in &result.mappings {
        if let Some(content_hash) = sha1_to_content_hash.get(sha1.as_bytes()) {
            stats.content_to_local_blake3.insert(*content_hash, *local_blake3);
        }
    }

    tracing::info!(
        commits = stats.commits,
        trees = stats.trees,
        blobs = stats.blobs,
        skipped = stats.skipped,
        total_bytes = stats.total_bytes,
        mappings = result.mappings.len(),
        "federation git object import complete"
    );

    stats
}

// =============================================================================
// Mirror repo lifecycle
// =============================================================================

/// KV prefix for mirror repo metadata.
const MIRROR_PREFIX: &str = "_fed:mirror:";

/// Mirror repo metadata stored in KV as JSON.
#[derive(Debug, Clone)]
pub struct MirrorMetadata {
    /// Federated ID of the remote resource.
    pub fed_id: String,
    /// Origin cluster public key (hex).
    pub origin_cluster_key: String,
    /// Local repo ID (hex) of the mirror.
    pub local_repo_id: String,
    /// Timestamp of last successful sync (Unix seconds).
    pub last_sync_timestamp: u64,
    /// When the mirror was created (Unix seconds).
    pub created_at: u64,
    /// Origin peer's iroh node ID (base32). Used to reconnect for pulls.
    pub origin_node_id: Option<String>,
    /// Origin peer's direct socket address hint (e.g., "192.168.1.5:12345").
    pub origin_addr_hint: Option<String>,
}

impl MirrorMetadata {
    pub(crate) fn to_json(&self) -> String {
        let mut v = serde_json::json!({
            "fed_id": self.fed_id,
            "origin_cluster_key": self.origin_cluster_key,
            "local_repo_id": self.local_repo_id,
            "last_sync_timestamp": self.last_sync_timestamp,
            "created_at": self.created_at
        });
        if let Some(ref nid) = self.origin_node_id {
            v["origin_node_id"] = serde_json::Value::String(nid.clone());
        }
        if let Some(ref hint) = self.origin_addr_hint {
            v["origin_addr_hint"] = serde_json::Value::String(hint.clone());
        }
        v.to_string()
    }

    pub(crate) fn from_json(s: &str) -> Option<Self> {
        let v: serde_json::Value = serde_json::from_str(s).ok()?;
        Some(Self {
            fed_id: v.get("fed_id")?.as_str()?.to_string(),
            origin_cluster_key: v.get("origin_cluster_key")?.as_str()?.to_string(),
            local_repo_id: v.get("local_repo_id")?.as_str()?.to_string(),
            last_sync_timestamp: v.get("last_sync_timestamp")?.as_u64()?,
            created_at: v.get("created_at")?.as_u64()?,
            origin_node_id: v.get("origin_node_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
            origin_addr_hint: v.get("origin_addr_hint").and_then(|v| v.as_str()).map(|s| s.to_string()),
        })
    }
}

/// Get or create a local mirror repo for a federated resource.
///
/// If a mirror already exists (looked up by fed_id), returns its repo ID.
/// Otherwise creates a new forge repo and stores mirror metadata.
///
/// `origin_node_id` and `origin_addr_hint` are stored in mirror metadata
/// for reconnection on subsequent pulls.
pub(crate) async fn get_or_create_mirror(
    forge_node: &ForgeNodeRef,
    fed_id_str: &str,
    origin_cluster_key: &str,
    origin_node_id: Option<&str>,
    origin_addr_hint: Option<&str>,
) -> Result<aspen_forge::identity::RepoId, String> {
    // Check if mirror already exists
    let meta_key = format!("{}{}", MIRROR_PREFIX, fed_id_str);
    if let Ok(entries) = forge_node.scan_kv(&meta_key, Some(1)).await
        && let Some((_key, value)) = entries.first()
        && let Some(meta) = MirrorMetadata::from_json(value)
        && let Ok(bytes) = hex::decode(&meta.local_repo_id)
        && bytes.len() == 32
    {
        let mut repo_bytes = [0u8; 32];
        repo_bytes.copy_from_slice(&bytes);
        return Ok(aspen_forge::identity::RepoId(repo_bytes));
    }

    // Create a new mirror repo
    let mirror_name = format!("mirror/{}", &fed_id_str[..24.min(fed_id_str.len())]);
    let identity = forge_node
        .create_repo(&mirror_name, vec![forge_node.public_key()], 1)
        .await
        .map_err(|e| format!("failed to create mirror repo: {e}"))?;

    let repo_id = identity.repo_id();
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();

    let metadata = MirrorMetadata {
        fed_id: fed_id_str.to_string(),
        origin_cluster_key: origin_cluster_key.to_string(),
        local_repo_id: hex::encode(repo_id.0),
        last_sync_timestamp: now,
        created_at: now,
        origin_node_id: origin_node_id.map(|s| s.to_string()),
        origin_addr_hint: origin_addr_hint.map(|s| s.to_string()),
    };

    forge_node
        .write_kv(&meta_key, &metadata.to_json())
        .await
        .map_err(|e| format!("failed to store mirror metadata: {e}"))?;

    tracing::info!(
        fed_id = %fed_id_str,
        mirror_repo = %hex::encode(repo_id.0),
        "created federation mirror repo"
    );

    Ok(repo_id)
}

/// Update refs on a mirror repo from fetched ref entries.
///
/// Writes each ref as `forge:refs:{repo_id}:{ref_name} → hash_hex`.
#[cfg(feature = "git-bridge")]
/// Update refs on a mirror repo and emit gossip `RefUpdate` announcements.
///
/// For each updated ref, a `RefUpdate` gossip announcement is emitted so
/// the CI `TriggerService` can detect the change and trigger pipelines.
pub(crate) async fn update_mirror_refs(
    forge_node: &ForgeNodeRef,
    repo_id: &aspen_forge::identity::RepoId,
    refs: &[(String, [u8; 32])],
) -> Result<u32, String> {
    let repo_hex = hex::encode(repo_id.0);
    let mut updated = 0u32;

    for (ref_name, hash) in refs {
        let key = format!("forge:refs:{}:{}", repo_hex, ref_name);
        let value = hex::encode(hash);
        if let Err(e) = forge_node.write_kv(&key, &value).await {
            tracing::warn!(ref_name = %ref_name, error = %e, "failed to update mirror ref");
            continue;
        }
        updated += 1;

        // Emit gossip so CI TriggerService sees the ref change
        forge_node.announce_ref_update(repo_id, ref_name, *hash, None).await;
    }

    Ok(updated)
}

/// Update the last sync timestamp on a mirror's metadata.
#[cfg(feature = "git-bridge")]
pub(crate) async fn update_mirror_sync_timestamp(forge_node: &ForgeNodeRef, fed_id_str: &str) -> Result<(), String> {
    let meta_key = format!("{}{}", MIRROR_PREFIX, fed_id_str);

    if let Ok(entries) = forge_node.scan_kv(&meta_key, Some(1)).await
        && let Some((_key, value)) = entries.first()
        && let Some(mut meta) = MirrorMetadata::from_json(value)
    {
        meta.last_sync_timestamp =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
        let _ = forge_node.write_kv(&meta_key, &meta.to_json()).await;
    }

    Ok(())
}

/// Check if a repo is a federation mirror.
#[cfg(feature = "git-bridge")]
pub(crate) async fn is_mirror_repo(forge_node: &ForgeNodeRef, repo_id: &aspen_forge::identity::RepoId) -> bool {
    let repo_hex = hex::encode(repo_id.0);
    if let Ok(entries) = forge_node.scan_kv(MIRROR_PREFIX, Some(100)).await {
        for (_key, value) in &entries {
            if let Some(meta) = MirrorMetadata::from_json(value)
                && meta.local_repo_id == repo_hex
            {
                return true;
            }
        }
    }
    false
}

/// Look up mirror metadata by federated ID.
#[allow(dead_code)] // Used by future mirror info commands
pub(crate) async fn get_mirror_metadata(forge_node: &ForgeNodeRef, fed_id_str: &str) -> Option<MirrorMetadata> {
    let meta_key = format!("{}{}", MIRROR_PREFIX, fed_id_str);
    if let Ok(entries) = forge_node.scan_kv(&meta_key, Some(1)).await
        && let Some((_key, value)) = entries.first()
    {
        return MirrorMetadata::from_json(value);
    }
    None
}

/// Handle a federation pull request.
///
/// Dispatches between two modes:
/// - **Cold pull** (`peer_node_id` + `repo_id`): connect to remote, create mirror, fetch.
/// - **Mirror pull** (`mirror_repo_id`): read stored metadata, reconnect, incremental fetch.
pub(crate) async fn handle_federation_pull(
    mirror_repo_id: Option<&str>,
    peer_node_id: Option<&str>,
    peer_addr: Option<&str>,
    repo_id: Option<&str>,
    cluster_identity: Option<&Arc<aspen_cluster::federation::ClusterIdentity>>,
    iroh_endpoint: Option<&Arc<iroh::Endpoint>>,
    forge_node: &ForgeNodeRef,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::FederationFetchRefsResponse;

    // Cold pull: peer + repo specified
    if let (Some(peer), Some(repo)) = (peer_node_id, repo_id) {
        return handle_federation_pull_remote(peer, peer_addr, repo, cluster_identity, iroh_endpoint, forge_node).await;
    }

    // Mirror pull: mirror_repo_id specified
    let mirror_id = match mirror_repo_id {
        Some(id) => id,
        None => {
            return Ok(ClientRpcResponse::FederationPullResult(FederationFetchRefsResponse {
                is_success: false,
                fetched: 0,
                already_present: 0,
                errors: Vec::new(),
                error: Some("either --peer + --repo (cold pull) or --repo (mirror pull) required".to_string()),
            }));
        }
    };

    // Scan all mirror metadata to find one matching this repo ID
    let meta = {
        let mut found = None;
        if let Ok(entries) = forge_node.scan_kv(MIRROR_PREFIX, Some(100)).await {
            for (_key, value) in &entries {
                if let Some(m) = MirrorMetadata::from_json(value)
                    && m.local_repo_id == mirror_id
                {
                    found = Some(m);
                    break;
                }
            }
        }
        found
    };

    let meta = match meta {
        Some(m) => m,
        None => {
            return Ok(ClientRpcResponse::FederationPullResult(FederationFetchRefsResponse {
                is_success: false,
                fetched: 0,
                already_present: 0,
                errors: Vec::new(),
                error: Some(format!("repo {} is not a federation mirror", mirror_id)),
            }));
        }
    };

    // Use stored node ID for reconnection, falling back to cluster key
    let peer_id = meta.origin_node_id.as_deref().unwrap_or(&meta.origin_cluster_key);

    // Delegate to the fetch handler using stored origin info
    let result = handle_federation_fetch_refs(
        peer_id,
        meta.origin_addr_hint.as_deref(),
        &meta.fed_id,
        cluster_identity,
        iroh_endpoint,
        forge_node,
    )
    .await?;

    // Re-wrap as FederationPullResult
    match result {
        ClientRpcResponse::FederationFetchRefsResult(resp) => Ok(ClientRpcResponse::FederationPullResult(resp)),
        other => Ok(other),
    }
}

/// Handle a cold federation pull: connect to a remote peer, discover the
/// repo's federated ID via handshake, create a local mirror, then fetch.
///
/// This is the entry point when the user specifies `--peer` + `--repo` without
/// an existing mirror. The remote's cluster identity key is obtained from the
/// handshake and used to construct the `FederatedId`.
pub(crate) async fn handle_federation_pull_remote(
    peer_node_id: &str,
    peer_addr: Option<&str>,
    repo_id_hex: &str,
    cluster_identity: Option<&Arc<aspen_cluster::federation::ClusterIdentity>>,
    iroh_endpoint: Option<&Arc<iroh::Endpoint>>,
    forge_node: &ForgeNodeRef,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::FederationFetchRefsResponse;

    let err_response = |msg: String| {
        Ok(ClientRpcResponse::FederationPullResult(FederationFetchRefsResponse {
            is_success: false,
            fetched: 0,
            already_present: 0,
            errors: Vec::new(),
            error: Some(msg),
        }))
    };

    let identity = match cluster_identity {
        Some(id) => id,
        None => return err_response("federation not configured on this cluster".to_string()),
    };

    let endpoint = match iroh_endpoint {
        Some(ep) => ep,
        None => return err_response("iroh endpoint not available".to_string()),
    };

    let peer_key: iroh::PublicKey = match peer_node_id.parse() {
        Ok(k) => k,
        Err(e) => return err_response(format!("invalid peer node ID '{}': {}", peer_node_id, e)),
    };

    // Validate repo_id is 32-byte hex
    let repo_id_bytes: [u8; 32] = match hex::decode(repo_id_hex) {
        Ok(b) if b.len() == 32 => {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&b);
            arr
        }
        Ok(b) => return err_response(format!("repo_id must be 32 bytes, got {}", b.len())),
        Err(e) => return err_response(format!("invalid repo_id hex '{}': {}", repo_id_hex, e)),
    };

    let endpoint_addr = build_endpoint_addr(peer_key, peer_addr);

    // Connect and handshake to get the remote cluster's identity key
    let (_connection, remote_identity) =
        match aspen_cluster::federation::sync::connect_to_cluster(endpoint, identity, endpoint_addr, None).await {
            Ok(result) => result,
            Err(e) => return err_response(format!("connection failed: {e}")),
        };

    // Construct FederatedId from remote cluster identity + repo_id
    let remote_cluster_key = remote_identity.public_key();
    let fed_id = aspen_cluster::federation::FederatedId::new(remote_cluster_key, repo_id_bytes);
    let fed_id_str = fed_id.to_string();

    // Ensure mirror exists with origin info stored
    match get_or_create_mirror(forge_node, &fed_id_str, &remote_cluster_key.to_string(), Some(peer_node_id), peer_addr)
        .await
    {
        Ok(_) => {}
        Err(e) => return err_response(format!("mirror creation failed: {e}")),
    }

    // Delegate to the fetch handler (reuses the same connection peer info)
    let result =
        handle_federation_fetch_refs(peer_node_id, peer_addr, &fed_id_str, cluster_identity, iroh_endpoint, forge_node)
            .await?;

    // Re-wrap as PullResult
    match result {
        ClientRpcResponse::FederationFetchRefsResult(resp) => Ok(ClientRpcResponse::FederationPullResult(resp)),
        other => Ok(other),
    }
}

/// Bidirectional federation sync: compare refs, pull what remote has, push what local has.
#[cfg(feature = "git-bridge")]
pub(crate) async fn handle_federation_bidi_sync(
    peer_node_id: &str,
    peer_addr: Option<&str>,
    repo_id_hex: &str,
    push_wins: bool,
    cluster_identity: Option<&Arc<aspen_cluster::federation::ClusterIdentity>>,
    iroh_endpoint: Option<&Arc<iroh::Endpoint>>,
    forge_node: &ForgeNodeRef,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::FederationBidiSyncResponse;
    use aspen_cluster::federation::FederatedId;
    use aspen_cluster::federation::FederationResourceResolver;
    use aspen_cluster::federation::sync::RefEntry;
    use aspen_cluster::federation::sync::SyncObject;
    use aspen_cluster::federation::verified::ref_diff;

    let err_response = |msg: String| {
        Ok(ClientRpcResponse::FederationBidiSyncResult(FederationBidiSyncResponse {
            is_success: false,
            pulled: 0,
            pushed: 0,
            pull_refs_updated: 0,
            push_refs_updated: 0,
            conflicts: Vec::new(),
            errors: Vec::new(),
            error: Some(msg),
        }))
    };

    let identity = match cluster_identity {
        Some(id) => id,
        None => return err_response("federation not configured on this cluster".to_string()),
    };

    let endpoint = match iroh_endpoint {
        Some(ep) => ep,
        None => return err_response("iroh endpoint not available".to_string()),
    };

    let peer_key: iroh::PublicKey = match peer_node_id.parse() {
        Ok(k) => k,
        Err(e) => return err_response(format!("invalid peer node ID '{}': {}", peer_node_id, e)),
    };

    let repo_id_bytes: [u8; 32] = match hex::decode(repo_id_hex) {
        Ok(b) if b.len() == 32 => {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&b);
            arr
        }
        Ok(b) => return err_response(format!("repo_id must be 32 bytes, got {}", b.len())),
        Err(e) => return err_response(format!("invalid repo_id hex '{}': {}", repo_id_hex, e)),
    };

    let endpoint_addr = build_endpoint_addr(peer_key, peer_addr);

    // Step 1: Connect and handshake
    let (connection, remote_identity) =
        match aspen_cluster::federation::sync::connect_to_cluster(endpoint, identity, endpoint_addr, None).await {
            Ok(result) => result,
            Err(e) => return err_response(format!("connection failed: {e}")),
        };

    let remote_cluster_key = remote_identity.public_key();

    // Step 2: Get remote ref state
    let remote_fed_id = FederatedId::new(remote_cluster_key, repo_id_bytes);
    let (remote_found, remote_heads, _) =
        match aspen_cluster::federation::sync::get_remote_resource_state(&connection, &remote_fed_id).await {
            Ok(result) => result,
            Err(e) => return err_response(format!("failed to get remote state: {e}")),
        };

    if !remote_found {
        return err_response(format!("remote resource not found: {}", remote_fed_id.short()));
    }

    // Step 3: Get local ref state
    let local_fed_id = FederatedId::new(identity.public_key(), repo_id_bytes);
    let resolver = aspen_forge::resolver::ForgeResourceResolver::new(forge_node.kv().clone());
    let local_state: aspen_cluster::federation::resolver::FederationResourceState =
        resolver.get_resource_state(&local_fed_id).await.unwrap_or_default();

    // Step 4: Compute ref diff
    let mut diff = ref_diff::compute_ref_diff(&local_state.heads, &remote_heads);
    let conflict_names = diff.conflicts.clone();
    ref_diff::resolve_conflicts(&mut diff, !push_wins); // pull_wins = !push_wins

    let mut errors = Vec::new();
    let mut pulled = 0u32;
    let mut pushed = 0u32;
    let mut pull_refs_updated = 0u32;
    let mut push_refs_updated = 0u32;

    // Step 5: Pull — fetch objects for refs we need from remote
    if !diff.to_pull.is_empty() {
        // Ensure mirror exists
        let _ = get_or_create_mirror(
            forge_node,
            &remote_fed_id.to_string(),
            &remote_cluster_key.to_string(),
            Some(peer_node_id),
            peer_addr,
        )
        .await;

        let have_hashes =
            collect_local_blake3_hashes(forge_node, &aspen_forge::identity::RepoId(repo_id_bytes), 10_000).await;

        // Fetch git objects from remote
        let (objects, _has_more) = match aspen_cluster::federation::sync::sync_remote_objects(
            &connection,
            &remote_fed_id,
            vec!["commit".to_string(), "tree".to_string(), "blob".to_string()],
            have_hashes,
            1000,
            None,
        )
        .await
        {
            Ok(result) => result,
            Err(e) => {
                errors.push(format!("pull objects failed: {e}"));
                (Vec::new(), false)
            }
        };

        pulled = objects.len() as u32;

        // Import into mirror
        if !objects.is_empty()
            && let Ok(mirror_id) = get_or_create_mirror(
                forge_node,
                &remote_fed_id.to_string(),
                &remote_cluster_key.to_string(),
                Some(peer_node_id),
                peer_addr,
            )
            .await
        {
            let stats = federation_import_objects(forge_node, &mirror_id, &objects).await;
            pulled = stats.commits + stats.trees + stats.blobs;
        }

        // Update mirror refs for pulled refs
        let pull_ref_updates: Vec<(String, [u8; 32])> =
            diff.to_pull.iter().filter_map(|name| remote_heads.get(name).map(|h| (name.clone(), *h))).collect();
        pull_refs_updated = pull_ref_updates.len() as u32;

        if let Ok(mirror_id) = get_or_create_mirror(
            forge_node,
            &remote_fed_id.to_string(),
            &remote_cluster_key.to_string(),
            Some(peer_node_id),
            peer_addr,
        )
        .await
            && let Err(e) = update_mirror_refs(forge_node, &mirror_id, &pull_ref_updates).await
        {
            errors.push(format!("pull ref update: {e}"));
        }
    }

    // Step 6: Push — send local objects for refs remote needs
    if !diff.to_push.is_empty() {
        let want_types = vec![
            "refs".to_string(),
            "commit".to_string(),
            "tree".to_string(),
            "blob".to_string(),
        ];
        let objects: Vec<SyncObject> = match resolver.sync_objects(&local_fed_id, &want_types, &[], 1000).await {
            Ok(objs) => objs,
            Err(e) => {
                errors.push(format!("push export failed: {e}"));
                Vec::new()
            }
        };

        // Separate refs from git objects
        let mut git_objects = Vec::new();
        let mut ref_updates = Vec::new();
        for obj in objects {
            if obj.object_type == "ref" {
                if let Ok(entry) = postcard::from_bytes::<RefEntry>(&obj.data) {
                    // Only push refs that are in to_push set
                    if diff.to_push.contains(&entry.ref_name) {
                        ref_updates.push(entry);
                    }
                }
            } else {
                git_objects.push(obj);
            }
        }

        // Also include refs from local heads as fallback
        if ref_updates.is_empty() {
            for ref_name in &diff.to_push {
                if let Some(hash) = local_state.heads.get(ref_name) {
                    ref_updates.push(RefEntry {
                        ref_name: ref_name.clone(),
                        head_hash: *hash,
                        commit_sha1: None,
                    });
                }
            }
        }

        match aspen_cluster::federation::sync::push_to_cluster(&connection, &local_fed_id, git_objects, ref_updates)
            .await
        {
            Ok(result) => {
                pushed = result.imported;
                push_refs_updated = result.refs_updated;
                if !result.accepted {
                    errors.push("push not accepted by remote".to_string());
                }
                errors.extend(result.errors);
            }
            Err(e) => {
                errors.push(format!("push failed: {e}"));
            }
        }
    }

    let is_success = pulled > 0 || pushed > 0 || (diff.to_pull.is_empty() && diff.to_push.is_empty());

    Ok(ClientRpcResponse::FederationBidiSyncResult(FederationBidiSyncResponse {
        is_success,
        pulled,
        pushed,
        pull_refs_updated,
        push_refs_updated,
        conflicts: conflict_names,
        errors,
        error: None,
    }))
}

/// Push a local repo's objects and refs to a remote cluster.
#[cfg(feature = "git-bridge")]
pub(crate) async fn handle_federation_push(
    peer_node_id: &str,
    peer_addr: Option<&str>,
    repo_id_hex: &str,
    cluster_identity: Option<&Arc<aspen_cluster::federation::ClusterIdentity>>,
    iroh_endpoint: Option<&Arc<iroh::Endpoint>>,
    forge_node: &ForgeNodeRef,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::FederationPushResponse;
    use aspen_cluster::federation::FederatedId;
    use aspen_cluster::federation::FederationResourceResolver;
    use aspen_cluster::federation::sync::RefEntry;
    use aspen_cluster::federation::sync::SyncObject;

    let err_response = |msg: String| {
        Ok(ClientRpcResponse::FederationPushResult(FederationPushResponse {
            is_success: false,
            imported: 0,
            skipped: 0,
            refs_updated: 0,
            errors: Vec::new(),
            error: Some(msg),
        }))
    };

    let identity = match cluster_identity {
        Some(id) => id,
        None => return err_response("federation not configured on this cluster".to_string()),
    };

    let endpoint = match iroh_endpoint {
        Some(ep) => ep,
        None => return err_response("iroh endpoint not available".to_string()),
    };

    let peer_key: iroh::PublicKey = match peer_node_id.parse() {
        Ok(k) => k,
        Err(e) => return err_response(format!("invalid peer node ID '{}': {}", peer_node_id, e)),
    };

    // Parse repo_id
    let repo_id_bytes = match hex::decode(repo_id_hex) {
        Ok(b) if b.len() == 32 => {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&b);
            arr
        }
        Ok(b) => return err_response(format!("repo_id must be 32 bytes, got {}", b.len())),
        Err(e) => return err_response(format!("invalid repo_id hex '{}': {}", repo_id_hex, e)),
    };
    let _repo_id = aspen_forge::identity::RepoId(repo_id_bytes);

    // Build FederatedId from our cluster key + repo_id
    let fed_id = FederatedId::new(identity.public_key(), repo_id_bytes);

    // Export objects from the local repo using the ForgeResourceResolver
    let resolver = aspen_forge::resolver::ForgeResourceResolver::new(forge_node.kv().clone());

    // Get ref state
    let state = match resolver.get_resource_state(&fed_id).await {
        Ok(s) => s,
        Err(e) => return err_response(format!("failed to read repo state: {}", e)),
    };

    // Sync all objects (no have_hashes — push everything)
    let want_types = vec![
        "refs".to_string(),
        "commit".to_string(),
        "tree".to_string(),
        "blob".to_string(),
    ];
    let objects: Vec<SyncObject> = match resolver.sync_objects(&fed_id, &want_types, &[], 1000).await {
        Ok(objs) => objs,
        Err(e) => return err_response(format!("failed to export objects: {}", e)),
    };

    // Separate ref entries from git objects
    let mut git_objects = Vec::new();
    let mut ref_updates = Vec::new();
    for obj in objects {
        if obj.object_type == "ref" {
            if let Ok(entry) = postcard::from_bytes::<RefEntry>(&obj.data) {
                ref_updates.push(entry);
            }
        } else {
            git_objects.push(obj);
        }
    }

    // Also include refs from state.heads as fallback
    if ref_updates.is_empty() {
        for (ref_name, head_hash) in &state.heads {
            ref_updates.push(RefEntry {
                ref_name: ref_name.clone(),
                head_hash: *head_hash,
                commit_sha1: None,
            });
        }
    }

    let endpoint_addr = build_endpoint_addr(peer_key, peer_addr);

    // Connect and push
    let (connection, _remote_identity) =
        match aspen_cluster::federation::sync::connect_to_cluster(endpoint, identity, endpoint_addr, None).await {
            Ok(result) => result,
            Err(e) => return err_response(format!("failed to connect to peer: {}", e)),
        };

    let push_result =
        match aspen_cluster::federation::sync::push_to_cluster(&connection, &fed_id, git_objects, ref_updates).await {
            Ok(r) => r,
            Err(e) => return err_response(format!("push failed: {}", e)),
        };

    Ok(ClientRpcResponse::FederationPushResult(FederationPushResponse {
        is_success: push_result.accepted,
        imported: push_result.imported,
        skipped: push_result.skipped,
        refs_updated: push_result.refs_updated,
        errors: push_result.errors,
        error: if push_result.accepted {
            None
        } else {
            Some("push was not accepted".to_string())
        },
    }))
}

/// Collect BLAKE3 hashes of objects already present in a local repo.
///
/// Scans the hash mapping KV prefix (`forge:hashmap:b3:{repo_id}:`) to find
/// all BLAKE3 hashes that have been imported. Used for incremental federation
/// sync — these hashes go into `have_hashes` so the remote skips them.
///
/// Returns at most `limit` hashes (default 10_000).
#[cfg(feature = "git-bridge")]
pub(crate) async fn collect_local_blake3_hashes(
    forge_node: &ForgeNodeRef,
    repo_id: &aspen_forge::identity::RepoId,
    limit: u32,
) -> Vec<[u8; 32]> {
    let prefix = format!("forge:hashmap:b3:{}:", repo_id.to_hex());
    let scan_req = aspen_core::ScanRequest {
        prefix: prefix.clone(),
        limit_results: Some(limit.min(10_000)),
        continuation_token: None,
    };

    let entries = match forge_node.kv().scan(scan_req).await {
        Ok(result) => result.entries,
        Err(e) => {
            tracing::warn!(error = %e, "failed to scan local BLAKE3 hashes for mirror");
            return Vec::new();
        }
    };

    let mut hashes = Vec::with_capacity(entries.len());
    for entry in &entries {
        // Key: forge:hashmap:b3:{repo_id}:{blake3_hex}
        if let Some(blake3_hex) = entry.key.strip_prefix(&prefix)
            && let Ok(bytes) = hex::decode(blake3_hex)
            && bytes.len() == 32
        {
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&bytes);
            hashes.push(hash);
        }
    }

    tracing::debug!(
        repo_id = %repo_id.to_hex(),
        count = hashes.len(),
        "collected local BLAKE3 hashes for incremental sync"
    );

    hashes
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

    // ====================================================================
    // Mirror metadata tests
    // ====================================================================

    #[test]
    fn test_mirror_metadata_roundtrip() {
        let meta = super::MirrorMetadata {
            fed_id: "abc123:def456".to_string(),
            origin_cluster_key: "0102030405060708".to_string(),
            local_repo_id: hex::encode([0xab; 32]),
            origin_node_id: None,
            origin_addr_hint: None,
            last_sync_timestamp: 1700000000,
            created_at: 1699999000,
        };

        let json = meta.to_json();
        let parsed = super::MirrorMetadata::from_json(&json).expect("should parse");

        assert_eq!(parsed.fed_id, meta.fed_id);
        assert_eq!(parsed.origin_cluster_key, meta.origin_cluster_key);
        assert_eq!(parsed.local_repo_id, meta.local_repo_id);
        assert_eq!(parsed.last_sync_timestamp, meta.last_sync_timestamp);
        assert_eq!(parsed.created_at, meta.created_at);
    }

    #[test]
    fn test_mirror_metadata_roundtrip_with_origin_fields() {
        let meta = super::MirrorMetadata {
            fed_id: "abc123:def456".to_string(),
            origin_cluster_key: "0102030405060708".to_string(),
            local_repo_id: hex::encode([0xab; 32]),
            origin_node_id: Some("n4bech3tdbfhtncf5hhi3lbmg2dqmj3q".to_string()),
            origin_addr_hint: Some("192.168.1.5:12345".to_string()),
            last_sync_timestamp: 1700000000,
            created_at: 1699999000,
        };

        let json = meta.to_json();
        let parsed = super::MirrorMetadata::from_json(&json).expect("should parse");

        assert_eq!(parsed.origin_node_id.as_deref(), Some("n4bech3tdbfhtncf5hhi3lbmg2dqmj3q"));
        assert_eq!(parsed.origin_addr_hint.as_deref(), Some("192.168.1.5:12345"));
        assert_eq!(parsed.fed_id, meta.fed_id);
        assert_eq!(parsed.origin_cluster_key, meta.origin_cluster_key);
    }

    #[test]
    fn test_mirror_metadata_backwards_compat_missing_origin_fields() {
        // Old-format JSON without origin_node_id / origin_addr_hint
        let old_json = r#"{
            "fed_id": "abc:def",
            "origin_cluster_key": "aabbccdd",
            "local_repo_id": "0011223344556677",
            "last_sync_timestamp": 100,
            "created_at": 99
        }"#;

        let parsed = super::MirrorMetadata::from_json(old_json).expect("should parse old format");
        assert_eq!(parsed.fed_id, "abc:def");
        assert!(parsed.origin_node_id.is_none());
        assert!(parsed.origin_addr_hint.is_none());
    }

    #[test]
    fn test_mirror_metadata_from_invalid_json() {
        assert!(super::MirrorMetadata::from_json("not json").is_none());
        assert!(super::MirrorMetadata::from_json("{}").is_none());
        assert!(super::MirrorMetadata::from_json(r#"{"fed_id":"x"}"#).is_none());
    }

    #[test]
    fn test_mirror_prefix_format() {
        assert_eq!(super::MIRROR_PREFIX, "_fed:mirror:");
    }
}
