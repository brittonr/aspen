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

#[derive(Copy, Clone)]
pub(crate) struct FetchRefsTarget<'a> {
    pub(crate) fed_id_str: &'a str,
    pub(crate) peer_node_id: &'a str,
}

#[derive(Copy, Clone)]
pub(crate) struct MirrorOrigin<'a> {
    pub(crate) cluster_key: &'a str,
    pub(crate) node_id: Option<&'a str>,
    pub(crate) addr_hint: Option<&'a str>,
}

#[derive(Copy, Clone)]
pub(crate) struct FederationRuntime<'a> {
    pub(crate) cluster_identity: Option<&'a Arc<aspen_cluster::federation::ClusterIdentity>>,
    pub(crate) iroh_endpoint: Option<&'a Arc<iroh::Endpoint>>,
    pub(crate) forge_node: &'a ForgeNodeRef,
}

#[derive(Copy, Clone)]
pub(crate) struct FederationPullRequest<'a> {
    pub(crate) mirror_repo_id: Option<&'a str>,
    pub(crate) peer_node_id: Option<&'a str>,
    pub(crate) peer_addr: Option<&'a str>,
    pub(crate) repo_id: Option<&'a str>,
}

fn current_time_secs() -> u64 {
    aspen_core::utils::current_time_ms() / 1_000
}

fn federation_settings_for_mode(
    mode: aspen_cluster::federation::FederationMode,
) -> aspen_cluster::federation::FederationSettings {
    match mode {
        aspen_cluster::federation::FederationMode::Disabled => {
            aspen_cluster::federation::FederationSettings::disabled()
        }
        aspen_cluster::federation::FederationMode::Public => aspen_cluster::federation::FederationSettings::public(),
        aspen_cluster::federation::FederationMode::AllowList => {
            aspen_cluster::federation::FederationSettings::allowlist(Vec::new())
        }
    }
}

fn decode_repo_id_hex(repo_id_hex: &str) -> Option<aspen_forge::identity::RepoId> {
    let bytes = hex::decode(repo_id_hex).ok()?;
    if bytes.len() != 32 {
        return None;
    }
    let mut repo_bytes = [0u8; 32];
    repo_bytes.copy_from_slice(&bytes);
    Some(aspen_forge::identity::RepoId(repo_bytes))
}

fn federation_pull_error(msg: String) -> ClientRpcResponse {
    use aspen_client_api::FederationFetchRefsResponse;

    ClientRpcResponse::FederationPullResult(FederationFetchRefsResponse {
        is_success: false,
        fetched: 0,
        already_present: 0,
        errors: Vec::new(),
        error: Some(msg),
    })
}

async fn find_mirror_metadata_by_repo_id(forge_node: &ForgeNodeRef, mirror_repo_id: &str) -> Option<MirrorMetadata> {
    let entries = forge_node.scan_kv(MIRROR_PREFIX, Some(100)).await.ok()?;
    for (_key, value) in &entries {
        if let Some(meta) = MirrorMetadata::from_json(value)
            && meta.local_repo_id == mirror_repo_id
        {
            return Some(meta);
        }
    }
    None
}

#[allow(clippy::result_large_err)]
fn parse_repo_id_bytes_or_error(repo_id_hex: &str) -> Result<[u8; 32], ClientRpcResponse> {
    let bytes = hex::decode(repo_id_hex)
        .map_err(|e| federation_pull_error(format!("invalid repo_id hex '{}': {}", repo_id_hex, e)))?;
    if bytes.len() != 32 {
        return Err(federation_pull_error(format!("repo_id must be 32 bytes, got {}", bytes.len())));
    }
    let mut repo_id_bytes = [0u8; 32];
    repo_id_bytes.copy_from_slice(&bytes);
    Ok(repo_id_bytes)
}

#[cfg(feature = "global-discovery")]
pub(crate) async fn handle_get_federation_status(
    forge_node: &ForgeNodeRef,
    content_discovery: Option<&Arc<dyn aspen_core::ContentDiscovery>>,
    federation_discovery: Option<&Arc<aspen_cluster::federation::FederationDiscoveryService>>,
    federation_identity: Option<&Arc<aspen_cluster::federation::SignedClusterIdentity>>,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::FederationStatusResponse;

    let is_dht_enabled = content_discovery.is_some();
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
            dht_enabled: is_dht_enabled,
            gossip_enabled: forge_node.has_gossip(),
            discovered_clusters,
            federated_repos,
            error: None,
        })),
        None => Ok(ClientRpcResponse::FederationStatus(FederationStatusResponse {
            is_enabled: false,
            cluster_name: String::new(),
            cluster_key: String::new(),
            dht_enabled: is_dht_enabled,
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

    let is_dht_enabled = false;
    let discovered_clusters = 0u32;
    let federated_repos = count_federated_repos(forge_node).await;

    match federation_identity {
        Some(identity) => Ok(ClientRpcResponse::FederationStatus(FederationStatusResponse {
            is_enabled: true,
            cluster_name: identity.name().to_string(),
            cluster_key: identity.public_key().to_string(),
            dht_enabled: is_dht_enabled,
            gossip_enabled: forge_node.has_gossip(),
            discovered_clusters,
            federated_repos,
            error: None,
        })),
        None => Ok(ClientRpcResponse::FederationStatus(FederationStatusResponse {
            is_enabled: false,
            cluster_name: String::new(),
            cluster_key: String::new(),
            dht_enabled: is_dht_enabled,
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

    let settings = federation_settings_for_mode(fed_mode).with_resource_type("forge:repo");

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

    // Load stored credential for this peer (if any)
    let credential = aspen_cluster::federation::token_store::load_credential_for_peer(forge_node.kv(), &peer_key).await;

    // Connect and perform handshake
    let connect_result = match aspen_cluster::federation::sync::connect_to_cluster(
        endpoint,
        identity,
        endpoint_addr,
        credential,
    )
    .await
    {
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
    let connection = connect_result.connection;
    let remote_identity = connect_result.identity;

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

/// Paginate git objects from an open sync session until exhausted or bounded.
///
/// Drives `session.sync_objects` for up to `max_rounds`, accumulating all
/// returned objects. Finishes the session when done. Returns accumulated
/// objects and any per-round errors (non-fatal; the caller extends its own
/// error list).
#[cfg(feature = "git-bridge")]
async fn paginate_git_objects_from_session(
    mut session: aspen_cluster::federation::sync::SyncSession,
    fed_id: &aspen_cluster::federation::FederatedId,
    mut have_hashes: Vec<[u8; 32]>,
    max_rounds: u32,
) -> (Vec<aspen_cluster::federation::sync::SyncObject>, Vec<String>) {
    let mut all_objects = Vec::new();
    let mut errors = Vec::new();

    for _round in 0..max_rounds {
        let (objects, has_more) = match session
            .sync_objects(
                fed_id,
                vec!["commit".to_string(), "tree".to_string(), "blob".to_string()],
                have_hashes.clone(),
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

        for obj in &objects {
            have_hashes.push(obj.hash);
        }
        all_objects.extend(objects);

        if !has_more {
            break;
        }
    }

    if let Err(e) = session.finish().await {
        tracing::debug!(error = %e, "sync session finish (non-fatal)");
    }

    (all_objects, errors)
}

/// Translate fetched ref hashes from source BLAKE3 to locally imported BLAKE3.
///
/// Uses the SHA-1 → BLAKE3 mapping store built during object import to remap
/// each ref's `head_hash`. Updates `fetched_refs` in place. Refs whose SHA-1
/// can't be resolved are left with the source hash and logged as warnings.
#[cfg(feature = "git-bridge")]
async fn translate_federation_refs_to_local(
    forge_node: &ForgeNodeRef,
    mirror_repo_id: &aspen_forge::identity::RepoId,
    fetched_ref_entries: &[aspen_cluster::federation::sync::RefEntry],
    fetched_refs: &mut [(String, [u8; 32])],
) {
    use aspen_forge::git::bridge::HashMappingStore;
    use aspen_forge::git::bridge::Sha1Hash;

    let mapping = HashMappingStore::new(forge_node.kv().clone());
    for (i, ref_entry) in fetched_ref_entries.iter().enumerate() {
        if let Some(sha1_bytes) = &ref_entry.commit_sha1 {
            let sha1 = Sha1Hash::from_bytes(*sha1_bytes);
            match mapping.get_blake3(mirror_repo_id, &sha1).await {
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

/// Validates config for a federation fetch-refs call, returning the four
/// extracted values or an early-exit error response.
#[allow(clippy::result_large_err, clippy::type_complexity)]
fn validate_fetch_refs_config<'a>(
    cluster_identity: Option<&'a Arc<aspen_cluster::federation::ClusterIdentity>>,
    iroh_endpoint: Option<&'a Arc<iroh::Endpoint>>,
    target: FetchRefsTarget<'a>,
) -> Result<
    (
        &'a Arc<aspen_cluster::federation::ClusterIdentity>,
        &'a Arc<iroh::Endpoint>,
        aspen_cluster::federation::FederatedId,
        iroh::PublicKey,
    ),
    ClientRpcResponse,
> {
    use aspen_client_api::FederationFetchRefsResponse;
    use aspen_cluster::federation::FederatedId;

    let err = |msg: String| {
        Err(ClientRpcResponse::FederationFetchRefsResult(FederationFetchRefsResponse {
            is_success: false,
            fetched: 0,
            already_present: 0,
            errors: Vec::new(),
            error: Some(msg),
        }))
    };

    debug_assert!(!target.fed_id_str.is_empty(), "fetch-refs target must include a federated id");
    debug_assert!(!target.peer_node_id.is_empty(), "fetch-refs target must include a peer id");
    let identity = match cluster_identity {
        Some(id) => id,
        None => return err("federation not configured on this cluster".to_string()),
    };
    let endpoint = match iroh_endpoint {
        Some(ep) => ep,
        None => return err("iroh endpoint not available".to_string()),
    };
    let fed_id: FederatedId = match target.fed_id_str.parse() {
        Ok(id) => id,
        Err(e) => return err(format!("invalid fed_id '{}': {}", target.fed_id_str, e)),
    };
    let peer_key: iroh::PublicKey = match target.peer_node_id.parse() {
        Ok(k) => k,
        Err(e) => return err(format!("invalid peer node ID '{}': {}", target.peer_node_id, e)),
    };

    Ok((identity, endpoint, fed_id, peer_key))
}

/// Connects to a remote federation peer, fetches the ref objects, and parses them.
///
/// Returns `(connect_result, fetched_ref_entries, fetched_refs, refs_fetched, errors)`
/// or an early-exit error response.
#[allow(clippy::result_large_err, clippy::type_complexity)]
async fn connect_and_fetch_remote_refs(
    endpoint: &Arc<iroh::Endpoint>,
    identity: &Arc<aspen_cluster::federation::ClusterIdentity>,
    endpoint_addr: iroh::EndpointAddr,
    credential: Option<aspen_auth::Credential>,
    fed_id: &aspen_cluster::federation::FederatedId,
) -> Result<
    (
        aspen_cluster::federation::sync::ConnectResult,
        Vec<aspen_cluster::federation::sync::RefEntry>,
        Vec<(String, [u8; 32])>,
        u32,
        Vec<String>,
    ),
    ClientRpcResponse,
> {
    use aspen_client_api::FederationFetchRefsResponse;
    use aspen_cluster::federation::sync::RefEntry;

    let err = |msg: String| {
        Err(ClientRpcResponse::FederationFetchRefsResult(FederationFetchRefsResponse {
            is_success: false,
            fetched: 0,
            already_present: 0,
            errors: Vec::new(),
            error: Some(msg),
        }))
    };

    let connect_result = match aspen_cluster::federation::sync::connect_to_cluster(
        endpoint,
        identity,
        endpoint_addr,
        credential,
    )
    .await
    {
        Ok(result) => result,
        Err(e) => return err(format!("connection failed: {e}")),
    };

    // Phase 1: Fetch refs
    #[allow(deprecated)] // Single-shot ref fetch; SyncSession not needed
    let (ref_objects, _has_more) = match aspen_cluster::federation::sync::sync_remote_objects(
        &connect_result.connection,
        fed_id,
        vec!["refs".to_string()],
        Vec::new(),
        1000,
        None,
    )
    .await
    {
        Ok(result) => result,
        Err(e) => return err(format!("ref fetch failed: {e}")),
    };

    // Parse ref entries.
    // Keep full RefEntry so we can use commit_sha1 for local hash lookup.
    let mut fetched_ref_entries: Vec<RefEntry> = Vec::with_capacity(ref_objects.len());
    let mut errors = Vec::with_capacity(ref_objects.len());
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
    let fetched_refs: Vec<(String, [u8; 32])> =
        fetched_ref_entries.iter().map(|entry| (entry.ref_name.clone(), entry.head_hash)).collect();
    let refs_fetched = u32::try_from(fetched_refs.len()).unwrap_or(u32::MAX);
    debug_assert!(fetched_ref_entries.len() <= ref_objects.len(), "parsed refs are a subset of fetched objects");
    debug_assert!(errors.len() <= ref_objects.len(), "each fetched object yields at most one parse error");

    Ok((connect_result, fetched_ref_entries, fetched_refs, refs_fetched, errors))
}

/// Persists fetched refs to the legacy KV path for backwards compatibility.
async fn persist_refs_to_legacy_kv(
    forge_node: &ForgeNodeRef,
    fed_id: &aspen_cluster::federation::FederatedId,
    fetched_refs: &[(String, [u8; 32])],
) {
    let origin_short = {
        let full = fed_id.origin().to_string();
        full[..16.min(full.len())].to_string()
    };
    let local_short = fed_id.short();
    for (ref_name, hash) in fetched_refs {
        let key = format!("_fed:mirror:{}:{}:refs/{}", origin_short, local_short, ref_name);
        if let Err(error) = forge_node.write_kv(&key, &hex::encode(hash)).await {
            tracing::warn!(ref_name = %ref_name, error = %error, "failed to persist legacy federation ref");
        }
    }
}

/// Fetches and imports git objects into the local mirror repo for a federation peer.
///
/// Gets or creates the mirror repo, paginates git objects from the remote via
/// `SyncSession`, imports them, and updates mirror refs. Returns
/// `(import_stats, already_present_objects)` on success, or an early-exit
/// response on failure.
///
/// Errors are appended in-place to the `errors` vec; the caller retains
/// ownership and inspects it when building the final response.
#[cfg(feature = "git-bridge")]
#[allow(clippy::result_large_err, clippy::too_many_arguments)]
async fn fetch_and_import_git_objects(
    forge_node: &ForgeNodeRef,
    connection: &iroh::endpoint::Connection,
    fed_id: &aspen_cluster::federation::FederatedId,
    fed_id_str: &str,
    peer_key: &iroh::PublicKey,
    peer_node_id: &str,
    peer_addr: Option<&str>,
    fetched_ref_entries: &[aspen_cluster::federation::sync::RefEntry],
    fetched_refs: &mut [(String, [u8; 32])],
    refs_fetched: u32,
    errors: &mut Vec<String>,
) -> Result<(FederationImportStats, u32), ClientRpcResponse> {
    use aspen_client_api::FederationFetchRefsResponse;

    let err_refs = |errs: Vec<String>, msg: String| {
        Err(ClientRpcResponse::FederationFetchRefsResult(FederationFetchRefsResponse {
            is_success: false,
            fetched: refs_fetched,
            already_present: 0,
            errors: errs,
            error: Some(msg),
        }))
    };

    let peer_cluster_key = peer_key.to_string();
    let mirror_repo_id = match get_or_create_mirror(forge_node, fed_id_str, MirrorOrigin {
        cluster_key: &peer_cluster_key,
        node_id: Some(peer_node_id),
        addr_hint: peer_addr,
    })
    .await
    {
        Ok(id) => id,
        Err(e) => {
            errors.push(format!("mirror creation failed: {e}"));
            return err_refs(errors.clone(), e);
        }
    };

    let have_hashes = collect_local_blake3_hashes(forge_node, &mirror_repo_id, 10_000).await;
    let already_present_objects = have_hashes.len() as u32;

    let max_rounds = 10u32; // Tiger Style: bounded pagination
    let session = match aspen_cluster::federation::sync::SyncSession::open(connection).await {
        Ok(s) => s,
        Err(e) => {
            errors.push(format!("failed to open sync session: {e}"));
            return Err(ClientRpcResponse::FederationFetchRefsResult(FederationFetchRefsResponse {
                is_success: false,
                fetched: refs_fetched,
                already_present: already_present_objects,
                errors: errors.clone(),
                error: Some(format!("sync session open failed: {e}")),
            }));
        }
    };

    let (all_git_objects, session_errors) =
        paginate_git_objects_from_session(session, fed_id, have_hashes, max_rounds).await;
    errors.extend(session_errors);

    let stats = federation_import_objects(forge_node, &mirror_repo_id, &all_git_objects).await;

    translate_federation_refs_to_local(forge_node, &mirror_repo_id, fetched_ref_entries, fetched_refs).await;

    if let Err(e) = update_mirror_refs(forge_node, &mirror_repo_id, fetched_refs).await {
        errors.push(format!("mirror ref update: {e}"));
    }
    if let Err(e) = update_mirror_sync_timestamp(forge_node, fed_id_str).await {
        errors.push(format!("mirror sync timestamp: {e}"));
    }

    Ok((stats, already_present_objects))
}

/// Fetch ref objects from a remote federated repository and persist locally.
///
/// Connects to the remote peer, performs a federation handshake, then calls
/// `SyncObjects` with `want_types: ["refs"]`. Received refs are written to
/// local KV under `_fed:mirror:{origin_short}:{local_id_short}:refs/{name}`.
pub(crate) async fn handle_federation_fetch_refs(
    target: FetchRefsTarget<'_>,
    peer_addr: Option<&str>,
    cluster_identity: Option<&Arc<aspen_cluster::federation::ClusterIdentity>>,
    iroh_endpoint: Option<&Arc<iroh::Endpoint>>,
    forge_node: &ForgeNodeRef,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client_api::FederationFetchRefsResponse;

    let (identity, endpoint, fed_id, peer_key) =
        match validate_fetch_refs_config(cluster_identity, iroh_endpoint, target) {
            Ok(v) => v,
            Err(r) => return Ok(r),
        };

    let endpoint_addr = build_endpoint_addr(peer_key, peer_addr);
    let credential = aspen_cluster::federation::token_store::load_credential_for_peer(forge_node.kv(), &peer_key).await;

    #[allow(unused_mut, unused_variables)] // some bindings only consumed by git-bridge feature block
    let (connect_result, fetched_ref_entries, mut fetched_refs, refs_fetched, mut errors) =
        match connect_and_fetch_remote_refs(endpoint, identity, endpoint_addr, credential, &fed_id).await {
            Ok(v) => v,
            Err(r) => return Ok(r),
        };

    #[cfg(feature = "git-bridge")]
    let git_stats = {
        let r = fetch_and_import_git_objects(
            forge_node,
            &connect_result.connection,
            &fed_id,
            target.fed_id_str,
            &peer_key,
            target.peer_node_id,
            peer_addr,
            &fetched_ref_entries,
            &mut fetched_refs,
            refs_fetched,
            &mut errors,
        )
        .await;
        match r {
            Ok(v) => v,
            Err(r) => return Ok(r),
        }
    };

    #[cfg(not(feature = "git-bridge"))]
    let git_stats = (FederationImportStats::default(), 0u32);

    let (import_stats, already_present_objects) = git_stats;

    persist_refs_to_legacy_kv(forge_node, &fed_id, &fetched_refs).await;

    let total_fetched = refs_fetched
        .saturating_add(import_stats.commits)
        .saturating_add(import_stats.trees)
        .saturating_add(import_stats.blobs);
    debug_assert!(already_present_objects <= 10_000, "mirror import scan keeps already-present count bounded");
    debug_assert!(total_fetched >= refs_fetched, "git import totals include fetched ref objects");

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
    /// Mapping from git SHA-1 → locally imported BLAKE3 envelope hash.
    /// Used to translate remote ref hashes to local hashes for mirror refs.
    /// Keyed by SHA-1 (20 bytes) for deterministic lookup.
    pub sha1_to_local_blake3: std::collections::HashMap<[u8; 20], blake3::Hash>,
}

/// Raw data prepared from SyncObjects for the convergent import loop.
#[cfg(feature = "git-bridge")]
struct PreparedImportData {
    entries: Vec<(aspen_forge::git::bridge::Sha1Hash, aspen_forge::git::bridge::GitObjectType, Vec<u8>)>,
    sha1_to_envelope: std::collections::HashMap<[u8; 20], blake3::Hash>,
    re_sha1_to_origin: std::collections::HashMap<[u8; 20], ([u8; 20], aspen_forge::git::bridge::GitObjectType)>,
    /// [blobs, trees, commits]
    type_counts: [u32; 3],
    total_bytes: u64,
}

/// Convert SyncObjects into the format expected by the convergent import loop.
///
/// Also builds SHA-1 → origin BLAKE3 index for remap entries and a
/// re-serialized SHA-1 → origin SHA-1 map for cross-pass resolution.
#[cfg(feature = "git-bridge")]
fn prepare_objects_for_import(objects: &[aspen_cluster::federation::sync::SyncObject]) -> PreparedImportData {
    use aspen_forge::git::bridge::GitObjectType;
    use aspen_forge::git::bridge::Sha1Hash;

    let mut entries = Vec::with_capacity(objects.len());
    let mut type_counts: [u32; 3] = [0; 3];
    let mut sha1_to_envelope = std::collections::HashMap::with_capacity(objects.len());
    let mut re_sha1_to_origin = std::collections::HashMap::with_capacity(objects.len());
    let mut total_bytes = 0u64;

    for obj in objects {
        let (git_type, type_idx) = match obj.object_type.as_str() {
            "blob" => (GitObjectType::Blob, 0),
            "tree" => (GitObjectType::Tree, 1),
            "commit" => (GitObjectType::Commit, 2),
            _ => continue,
        };

        let type_str = obj.object_type.as_str();
        let header = format!("{} {}\0", type_str, obj.data.len());
        let mut git_bytes = Vec::with_capacity(header.len() + obj.data.len());
        git_bytes.extend_from_slice(header.as_bytes());
        git_bytes.extend_from_slice(&obj.data);

        let sha1 = {
            use sha1::Digest;
            let digest: [u8; 20] = sha1::Sha1::digest(&git_bytes).into();
            Sha1Hash::from_bytes(digest)
        };

        if let Some(env_bytes) = obj.envelope_hash {
            sha1_to_envelope.insert(*sha1.as_bytes(), blake3::Hash::from_bytes(env_bytes));
        } else {
            tracing::debug!(
                sha1 = sha1.to_hex(),
                object_type = obj.object_type.as_str(),
                "SyncObject has no envelope_hash, skipping remap"
            );
        }

        if let Some(origin_sha1_bytes) = obj.origin_sha1
            && origin_sha1_bytes != *sha1.as_bytes()
        {
            re_sha1_to_origin.insert(*sha1.as_bytes(), (origin_sha1_bytes, git_type));
        }

        total_bytes += obj.data.len() as u64;
        type_counts[type_idx] += 1;
        entries.push((sha1, git_type, git_bytes));
    }

    PreparedImportData {
        entries,
        sha1_to_envelope,
        re_sha1_to_origin,
        type_counts,
        total_bytes,
    }
}

/// Pre-populate origin SHA-1 → BLAKE3 mappings from already-imported objects.
///
/// Trees reference sub-objects by the origin cluster's SHA-1. Pre-populating
/// from previously imported objects gives the first convergent pass access to
/// mappings that would otherwise require extra passes.
///
/// Returns the number of mappings pre-populated.
#[cfg(feature = "git-bridge")]
async fn pre_populate_origin_sha1_mappings(
    mapping: &std::sync::Arc<aspen_forge::git::bridge::HashMappingStore<dyn aspen_core::KeyValueStore>>,
    repo_id: &aspen_forge::identity::RepoId,
    objects: &[aspen_cluster::federation::sync::SyncObject],
) -> u32 {
    use aspen_forge::git::bridge::GitObjectType;
    use aspen_forge::git::bridge::Sha1Hash;

    let mut pre_populated = 0u32;
    for obj in objects {
        let Some(origin_sha1_bytes) = obj.origin_sha1 else {
            continue;
        };
        let git_type = match obj.object_type.as_str() {
            "blob" => GitObjectType::Blob,
            "tree" => GitObjectType::Tree,
            "commit" => GitObjectType::Commit,
            _ => continue,
        };

        let type_str = obj.object_type.as_str();
        let header = format!("{} {}\0", type_str, obj.data.len());
        let computed_sha1 = {
            use sha1::Digest;
            let mut hasher = sha1::Sha1::new();
            hasher.update(header.as_bytes());
            hasher.update(&obj.data);
            let digest: [u8; 20] = hasher.finalize().into();
            Sha1Hash::from_bytes(digest)
        };

        let origin_sha1 = Sha1Hash::from_bytes(origin_sha1_bytes);
        if origin_sha1_bytes == *computed_sha1.as_bytes() {
            continue;
        }

        if let Ok(Some((existing_blake3, _))) = mapping.get_blake3(repo_id, &computed_sha1).await {
            if let Err(e) = mapping.store_batch(repo_id, &[(existing_blake3, origin_sha1, git_type)]).await {
                tracing::debug!(
                    error = %e,
                    origin_sha1 = %origin_sha1.to_hex(),
                    "failed to pre-populate origin SHA-1 mapping (non-fatal)"
                );
            } else {
                pre_populated += 1;
            }
        }
    }
    pre_populated
}

/// Store original SHA-1 → local BLAKE3 mappings for git client lookups.
///
/// The SHA-1 in import mappings is from re-serialized content, which may
/// differ from the original git push SHA-1 for trees/commits. When SyncObjects
/// carry `origin_sha1`, an additional mapping is stored so git clients can
/// find objects by their original SHA-1.
///
/// Returns `(matched, stored, same, missed)` counters.
#[cfg(feature = "git-bridge")]
async fn store_origin_sha1_mappings(
    forge_node: &ForgeNodeRef,
    repo_id: &aspen_forge::identity::RepoId,
    objects: &[aspen_cluster::federation::sync::SyncObject],
    stats: &mut FederationImportStats,
) -> (u32, u32, u32, u32) {
    use aspen_forge::git::bridge::GitObjectType;
    use aspen_forge::git::bridge::HashMappingStore;
    use aspen_forge::git::bridge::Sha1Hash;

    let mapping_store = HashMappingStore::new(forge_node.kv().clone());
    let mut matched = 0u32;
    let mut stored = 0u32;
    let mut same = 0u32;
    let mut missed = 0u32;

    for obj in objects {
        let Some(origin_sha1_bytes) = obj.origin_sha1 else {
            continue;
        };

        let import_sha1 = {
            use sha1::Digest;
            let header = format!("{} {}\0", obj.object_type, obj.data.len());
            let mut hasher = sha1::Sha1::new();
            hasher.update(header.as_bytes());
            hasher.update(&obj.data);
            let digest: [u8; 20] = hasher.finalize().into();
            digest
        };

        let local_blake3 = if let Some(b3) = stats.sha1_to_local_blake3.get(&import_sha1) {
            Some(*b3)
        } else {
            let import_sha1_hash = Sha1Hash::from_bytes(import_sha1);
            match mapping_store.get_blake3(repo_id, &import_sha1_hash).await {
                Ok(Some((b3, _))) => Some(b3),
                _ => None,
            }
        };

        if let Some(local_blake3) = local_blake3 {
            let origin_sha1 = Sha1Hash::from_bytes(origin_sha1_bytes);
            let git_type = match obj.object_type.as_str() {
                "blob" => GitObjectType::Blob,
                "tree" => GitObjectType::Tree,
                "commit" => GitObjectType::Commit,
                _ => continue,
            };

            matched += 1;
            if origin_sha1_bytes != import_sha1 {
                if let Err(e) = mapping_store.store_batch(repo_id, &[(local_blake3, origin_sha1, git_type)]).await {
                    tracing::debug!(error = %e, "failed to store origin SHA-1 mapping (non-fatal)");
                } else {
                    stored += 1;
                }
            } else {
                same += 1;
            }

            stats.sha1_to_local_blake3.insert(origin_sha1_bytes, local_blake3);
        } else {
            missed += 1;
        }
    }

    (matched, stored, same, missed)
}

/// Runs the convergent import loop for federation git objects.
///
/// Each pass filters to unmapped objects, calls `import_objects`, and collects
/// newly created SHA-1→BLAKE3 mappings. Repeats until fixed-point or
/// `max_convergence_passes` is reached. Stalled objects are retried with a
/// raw-bytes force-import to break circular mapping dependencies.
///
/// Returns `(all_mappings, total_skipped)`.
#[cfg(feature = "git-bridge")]
async fn run_convergent_import_loop<K, B>(
    importer: &aspen_forge::git::bridge::GitImporter<K, B>,
    mapping: &std::sync::Arc<aspen_forge::git::bridge::HashMappingStore<K>>,
    repo_id: &aspen_forge::identity::RepoId,
    import_objects: Vec<(aspen_forge::git::bridge::Sha1Hash, aspen_forge::git::bridge::GitObjectType, Vec<u8>)>,
    sha1_to_envelope: &std::collections::HashMap<[u8; 20], blake3::Hash>,
    re_sha1_to_origin: &std::collections::HashMap<[u8; 20], ([u8; 20], aspen_forge::git::bridge::GitObjectType)>,
    stats: &mut FederationImportStats,
) -> (Vec<(aspen_forge::git::bridge::Sha1Hash, blake3::Hash)>, u32)
where
    K: aspen_core::KeyValueStore + ?Sized,
    B: aspen_blob::BlobStore,
{
    use aspen_forge::git::bridge::GitObjectType;
    use aspen_forge::git::bridge::Sha1Hash;

    let mut all_mappings = Vec::new();
    let mut total_skipped = 0u32;
    let max_convergence_passes = 30u32;
    let mut remaining_objects = import_objects;

    for pass in 0..max_convergence_passes {
        // Filter to objects that still lack SHA-1 mappings.
        let mut unmapped = Vec::new();
        let mut pass_skipped = 0u32;
        for (sha1, git_type, git_bytes) in remaining_objects {
            if mapping.has_sha1(repo_id, &sha1).await.unwrap_or(false) {
                pass_skipped += 1;
            } else {
                unmapped.push((sha1, git_type, git_bytes));
            }
        }

        if unmapped.is_empty() {
            tracing::info!(
                pass = pass,
                total_imported = all_mappings.len(),
                total_skipped = total_skipped,
                "convergent import: all objects mapped"
            );
            total_skipped += pass_skipped;
            break;
        }

        let attempt_count = unmapped.len() as u32;

        // Build a SHA-1 set of failed objects for cheap lookup after import.
        // We need to recover the original (sha1, git_type, git_bytes) tuples
        // for failed objects to retry. Build a map keyed by SHA-1 bytes.
        let mut retry_map: std::collections::HashMap<[u8; 20], (Sha1Hash, GitObjectType, Vec<u8>)> =
            unmapped.iter().map(|(sha1, gt, gb)| (*sha1.as_bytes(), (*sha1, *gt, gb.clone()))).collect();

        match importer.import_objects(repo_id, unmapped).await {
            Ok(r) => {
                let newly_imported = r.objects_imported;
                total_skipped += r.objects_skipped + pass_skipped;

                // Write origin→mirror BLAKE3 remap entries for this pass.
                let mut remap_batch: Vec<(blake3::Hash, blake3::Hash)> = Vec::new();
                for (sha1, mirror_blake3) in &r.mappings {
                    if let Some(origin_blake3) = sha1_to_envelope.get(sha1.as_bytes()) {
                        remap_batch.push((*origin_blake3, *mirror_blake3));
                    }
                }
                let remap_count = remap_batch.len() as u32;
                if !remap_batch.is_empty() {
                    // Chunk into MAX_HASH_MAPPING_BATCH_SIZE to stay within batch limits
                    for chunk in remap_batch.chunks(1000) {
                        if let Err(e) = mapping.write_remap_batch(repo_id, chunk).await {
                            tracing::warn!(error = %e, "failed to write remap batch (non-fatal)");
                        }
                    }
                }
                tracing::info!(pass = pass, remap_entries = remap_count, "wrote origin→mirror BLAKE3 remap entries");

                // Store origin SHA-1 → local BLAKE3 mappings for newly imported
                // objects. This is critical for multi-pass convergence: tree entries
                // reference sub-objects by their ORIGINAL SHA-1 (from the source
                // cluster's mapping store). If a subtree's re-serialized SHA-1
                // differs from its origin SHA-1 (due to entry ordering or mode
                // formatting), parent trees can't resolve the reference. By storing
                // origin_sha1 → blake3 after each pass, the next pass can find
                // subtrees by their origin SHA-1.
                let mut origin_mappings_this_pass = 0u32;
                for (re_sha1, local_blake3) in &r.mappings {
                    if let Some((origin_sha1_bytes, git_type)) = re_sha1_to_origin.get(re_sha1.as_bytes()) {
                        let origin_sha1 = Sha1Hash::from_bytes(*origin_sha1_bytes);
                        if let Err(e) = mapping.store_batch(repo_id, &[(*local_blake3, origin_sha1, *git_type)]).await {
                            tracing::debug!(
                                error = %e,
                                origin_sha1 = %origin_sha1.to_hex(),
                                "failed to store origin SHA-1 mapping in convergent pass (non-fatal)"
                            );
                        } else {
                            origin_mappings_this_pass += 1;
                        }
                    }
                }
                if origin_mappings_this_pass > 0 {
                    tracing::info!(
                        pass = pass,
                        origin_mappings = origin_mappings_this_pass,
                        "stored origin SHA-1 → BLAKE3 mappings for cross-pass resolution"
                    );
                }

                all_mappings.extend(r.mappings);

                tracing::info!(
                    pass = pass,
                    attempted = attempt_count,
                    imported = newly_imported,
                    skipped = r.objects_skipped,
                    failed = r.failures.len(),
                    cumulative_mapped = all_mappings.len(),
                    "convergent import pass complete"
                );

                if newly_imported == 0 {
                    // No progress — try a final pass with relaxed dep checking.
                    // Objects whose SHA-1 validates against their content are safe
                    // to store even if dependency mappings are incomplete. The
                    // missing deps may be among the stuck objects themselves
                    // (circular dependency at the mapping level, not the data level).
                    let stuck_count = r.failures.len();
                    if stuck_count > 0 {
                        tracing::info!(
                            pass = pass,
                            stuck = stuck_count,
                            "convergent loop stalled, attempting final pass with raw-bytes import"
                        );

                        // Collect stuck objects from the retry map
                        let mut stuck_objects: Vec<(Sha1Hash, GitObjectType, Vec<u8>)> = Vec::new();
                        for (sha1, _err) in &r.failures {
                            if let Some(entry) = retry_map.remove(sha1.as_bytes()) {
                                stuck_objects.push(entry);
                            }
                        }

                        // Force-store each stuck object by directly writing its
                        // blob and mapping, bypassing dependency resolution.
                        let mut force_imported = 0u32;
                        for (sha1, git_type, git_bytes) in &stuck_objects {
                            // Validate: SHA-1 of content must match the SHA-1 key
                            let check_sha1 = {
                                use sha1::Digest;
                                let digest: [u8; 20] = sha1::Sha1::digest(git_bytes).into();
                                Sha1Hash::from_bytes(digest)
                            };
                            if check_sha1 != *sha1 {
                                stats.errors.push(format!(
                                    "stuck object {} failed SHA-1 validation in final pass",
                                    sha1.to_hex()
                                ));
                                continue;
                            }

                            // Store the raw bytes directly via blob store + KV
                            match importer.import_object(repo_id, git_bytes).await {
                                Ok(blake3_hash) => {
                                    all_mappings.push((*sha1, blake3_hash));
                                    force_imported += 1;

                                    // Also store origin SHA-1 mapping if applicable
                                    if let Some((origin_sha1_bytes, _)) = re_sha1_to_origin.get(sha1.as_bytes()) {
                                        let origin_sha1 = Sha1Hash::from_bytes(*origin_sha1_bytes);
                                        let _ = mapping
                                            .store_batch(repo_id, &[(blake3_hash, origin_sha1, *git_type)])
                                            .await;
                                    }
                                }
                                Err(e) => {
                                    stats.errors.push(format!(
                                        "stuck object {} final pass import failed: {}",
                                        sha1.to_hex(),
                                        e
                                    ));
                                }
                            }
                        }

                        if force_imported > 0 {
                            tracing::info!(
                                force_imported = force_imported,
                                remaining = stuck_objects.len() as u32 - force_imported,
                                "final pass imported stuck objects, continuing convergent loop"
                            );
                            // The newly imported objects may unblock others.
                            // Collect remaining stuck objects for another loop iteration.
                            remaining_objects = Vec::new();
                            for (sha1, git_type, git_bytes) in stuck_objects {
                                if !mapping.has_sha1(repo_id, &sha1).await.unwrap_or(false) {
                                    remaining_objects.push((sha1, git_type, git_bytes));
                                }
                            }
                            if remaining_objects.is_empty() {
                                break;
                            }
                            continue; // Continue the convergent loop
                        }

                        // Final pass made no progress — log remaining stuck objects
                        let preview: Vec<String> =
                            stuck_objects.iter().take(20).map(|(sha1, _, _)| sha1.to_hex()).collect();
                        tracing::warn!(
                            stuck = stuck_objects.len(),
                            first_20 = ?preview,
                            "final pass made no progress, objects have genuinely unresolvable dependencies"
                        );
                        for (sha1, _git_type, _git_bytes) in &stuck_objects {
                            stats.errors.push(format!("stuck object {}: unresolvable after final pass", sha1.to_hex()));
                        }
                    }
                    break;
                }

                // Collect failed objects for next pass.
                remaining_objects = Vec::with_capacity(r.failures.len());
                for (sha1, _err) in &r.failures {
                    if let Some(entry) = retry_map.remove(sha1.as_bytes()) {
                        remaining_objects.push(entry);
                    }
                }

                if remaining_objects.is_empty() {
                    break;
                }
            }
            Err(e) => {
                stats.errors.push(format!("convergent pass {pass} failed: {e}"));
                tracing::warn!(pass = pass, error = %e, "convergent import pass error");
                // Recover all objects from the retry map for next attempt
                remaining_objects = retry_map.into_values().collect();
            }
        }
    }

    (all_mappings, total_skipped)
}

/// Builds `sha1_to_local_blake3` from mappings and stores origin SHA-1 entries (phases 3 and 4).
#[cfg(feature = "git-bridge")]
async fn finalize_import_stats(
    forge_node: &ForgeNodeRef,
    repo_id: &aspen_forge::identity::RepoId,
    objects: &[aspen_cluster::federation::sync::SyncObject],
    stats: &mut FederationImportStats,
    all_mappings: Vec<(aspen_forge::git::bridge::Sha1Hash, blake3::Hash)>,
    total_skipped: u32,
) {
    stats.skipped = total_skipped;

    // Phase 3: Build sha1_to_local_blake3 from all mappings.
    // Each mapping is (sha1, blake3). Keyed directly by SHA-1 bytes —
    // no content_hash indirection needed since SHA-1 is deterministic.
    for (sha1, local_blake3) in &all_mappings {
        stats.sha1_to_local_blake3.insert(*sha1.as_bytes(), *local_blake3);
    }

    // Phase 4: Store ORIGINAL SHA-1 → local BLAKE3 mappings.
    let origin_sha1_count = objects.iter().filter(|o| o.origin_sha1.is_some()).count();
    let (origin_sha1_matched, origin_mappings_stored, origin_sha1_same, origin_sha1_missed) =
        store_origin_sha1_mappings(forge_node, repo_id, objects, stats).await;

    tracing::info!(
        total = origin_sha1_count,
        matched = origin_sha1_matched,
        stored = origin_mappings_stored,
        same = origin_sha1_same,
        missed = origin_sha1_missed,
        "Phase 4: origin SHA-1 mapping results"
    );

    if origin_mappings_stored > 0 {
        tracing::info!(count = origin_mappings_stored, "stored origin SHA-1 → BLAKE3 mappings for git client lookups");
    }

    tracing::info!(
        commits = stats.commits,
        trees = stats.trees,
        blobs = stats.blobs,
        skipped = stats.skipped,
        total_bytes = stats.total_bytes,
        mappings = all_mappings.len(),
        errors = stats.errors.len(),
        "federation git object import complete"
    );
}

/// Import federation `SyncObject` entries into a forge repo via the git bridge.
///
/// Converts SyncObjects to the `(Sha1Hash, GitObjectType, Vec<u8>)` format
/// expected by `import_objects()`, then runs a convergent retry loop:
///
/// 1. Each pass calls `import_objects` (wave-based parallel, partial-success)
/// 2. Failed objects (missing cross-batch dependencies) are retried next pass
/// 3. Loop terminates when no new objects are imported or after 10 passes
///
/// This handles cross-batch dependencies where trees reference blobs from
/// a different sync batch. Typical convergence: 1-3 passes.
///
/// Returns import statistics including a `sha1_to_local_blake3` map for
/// translating remote ref hashes to locally imported BLAKE3 hashes.
#[cfg(feature = "git-bridge")]
pub(crate) async fn federation_import_objects(
    forge_node: &ForgeNodeRef,
    repo_id: &aspen_forge::identity::RepoId,
    objects: &[aspen_cluster::federation::sync::SyncObject],
) -> FederationImportStats {
    use aspen_core::hlc::create_hlc;
    use aspen_forge::git::bridge::GitImporter;
    use aspen_forge::git::bridge::HashMappingStore;

    let mut stats = FederationImportStats::default();

    if objects.is_empty() {
        return stats;
    }

    // Phase 1: Prepare SyncObjects for the convergent import loop.
    let prepared = prepare_objects_for_import(objects);
    stats.blobs = prepared.type_counts[0];
    stats.trees = prepared.type_counts[1];
    stats.commits = prepared.type_counts[2];
    stats.total_bytes = prepared.total_bytes;

    // Phase 2: Convergent import loop — see run_convergent_import_loop for details.
    let mapping = std::sync::Arc::new(HashMappingStore::new(forge_node.kv().clone()));
    let hlc = create_hlc(&forge_node.public_key().to_string());
    let importer = GitImporter::new(
        mapping.clone(),
        forge_node.git.blobs().clone(),
        std::sync::Arc::new(forge_node.refs.clone()),
        forge_node.secret_key().clone(),
        hlc,
    );

    // Phase 1.5: Pre-populate origin SHA-1 → BLAKE3 mappings from previously
    // imported objects to reduce convergent loop passes.
    let pre_populated = pre_populate_origin_sha1_mappings(&mapping, repo_id, objects).await;
    if pre_populated > 0 {
        tracing::info!(
            pre_populated = pre_populated,
            "pre-populated origin SHA-1 → BLAKE3 mappings from previously imported objects"
        );
    }

    let (all_mappings, total_skipped) = run_convergent_import_loop(
        &importer,
        &mapping,
        repo_id,
        prepared.entries,
        &prepared.sha1_to_envelope,
        &prepared.re_sha1_to_origin,
        &mut stats,
    )
    .await;

    finalize_import_stats(forge_node, repo_id, objects, &mut stats, all_mappings, total_skipped).await;

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

async fn load_existing_mirror_repo_id(
    forge_node: &ForgeNodeRef,
    meta_key: &str,
) -> Option<aspen_forge::identity::RepoId> {
    let entries = forge_node.scan_kv(meta_key, Some(1)).await.ok()?;
    let (_key, value) = entries.first()?;
    let meta = MirrorMetadata::from_json(value)?;
    decode_repo_id_hex(&meta.local_repo_id)
}

/// Get or create a local mirror repo for a federated resource.
///
/// If a mirror already exists (looked up by fed_id), returns its repo ID.
/// Otherwise creates a new forge repo and stores mirror metadata.
///
/// `origin.node_id` and `origin.addr_hint` are stored in mirror metadata
/// for reconnection on subsequent pulls.
pub(crate) async fn get_or_create_mirror(
    forge_node: &ForgeNodeRef,
    fed_id_str: &str,
    origin: MirrorOrigin<'_>,
) -> Result<aspen_forge::identity::RepoId, String> {
    let meta_key = format!("{}{}", MIRROR_PREFIX, fed_id_str);
    if let Some(repo_id) = load_existing_mirror_repo_id(forge_node, &meta_key).await {
        return Ok(repo_id);
    }

    let name_hash = blake3::hash(fed_id_str.as_bytes());
    let mirror_name = format!("mirror/{}", &hex::encode(name_hash.as_bytes())[..24]);
    let identity = forge_node
        .create_repo(&mirror_name, vec![forge_node.public_key()], 1)
        .await
        .map_err(|e| format!("failed to create mirror repo: {e}"))?;

    let repo_id = identity.repo_id();
    let now = current_time_secs();
    let metadata = MirrorMetadata {
        fed_id: fed_id_str.to_string(),
        origin_cluster_key: origin.cluster_key.to_string(),
        local_repo_id: hex::encode(repo_id.0),
        last_sync_timestamp: now,
        created_at: now,
        origin_node_id: origin.node_id.map(str::to_string),
        origin_addr_hint: origin.addr_hint.map(str::to_string),
    };
    debug_assert!(metadata.created_at == metadata.last_sync_timestamp, "new mirror timestamps start aligned");
    debug_assert!(mirror_name.len() > "mirror/".len(), "mirror names keep a hashed suffix");

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
        meta.last_sync_timestamp = current_time_secs();
        forge_node
            .write_kv(&meta_key, &meta.to_json())
            .await
            .map_err(|e| format!("failed to update mirror metadata: {e}"))?;
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
    request: FederationPullRequest<'_>,
    runtime: FederationRuntime<'_>,
) -> anyhow::Result<ClientRpcResponse> {
    debug_assert!(
        request.peer_node_id.is_some() || request.mirror_repo_id.is_some(),
        "pull request selects either cold or mirror mode"
    );
    if let (Some(peer_node_id), Some(repo_id)) = (request.peer_node_id, request.repo_id) {
        return handle_federation_pull_remote(peer_node_id, request.peer_addr, repo_id, runtime).await;
    }

    let mirror_repo_id = match request.mirror_repo_id {
        Some(repo_id) => repo_id,
        None => {
            return Ok(federation_pull_error(
                "either --peer + --repo (cold pull) or --repo (mirror pull) required".to_string(),
            ));
        }
    };
    let meta = match find_mirror_metadata_by_repo_id(runtime.forge_node, mirror_repo_id).await {
        Some(meta) => meta,
        None => return Ok(federation_pull_error(format!("repo {} is not a federation mirror", mirror_repo_id))),
    };

    debug_assert!(
        meta.origin_node_id.is_some() || !meta.origin_cluster_key.is_empty(),
        "mirror metadata preserves at least one remote identity handle"
    );
    let peer_id = meta.origin_node_id.as_deref().unwrap_or(&meta.origin_cluster_key);
    let result = handle_federation_fetch_refs(
        FetchRefsTarget {
            fed_id_str: &meta.fed_id,
            peer_node_id: peer_id,
        },
        meta.origin_addr_hint.as_deref(),
        runtime.cluster_identity,
        runtime.iroh_endpoint,
        runtime.forge_node,
    )
    .await?;

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
    runtime: FederationRuntime<'_>,
) -> anyhow::Result<ClientRpcResponse> {
    let identity = match runtime.cluster_identity {
        Some(identity) => identity,
        None => return Ok(federation_pull_error("federation not configured on this cluster".to_string())),
    };
    let endpoint = match runtime.iroh_endpoint {
        Some(endpoint) => endpoint,
        None => return Ok(federation_pull_error("iroh endpoint not available".to_string())),
    };

    debug_assert!(!peer_node_id.is_empty(), "remote pull requires a peer node id");
    debug_assert!(!repo_id_hex.is_empty(), "remote pull requires a repo id");
    let peer_key: iroh::PublicKey = match peer_node_id.parse() {
        Ok(peer_key) => peer_key,
        Err(e) => return Ok(federation_pull_error(format!("invalid peer node ID '{}': {}", peer_node_id, e))),
    };
    let repo_id_bytes = match parse_repo_id_bytes_or_error(repo_id_hex) {
        Ok(bytes) => bytes,
        Err(response) => return Ok(response),
    };

    let endpoint_addr = build_endpoint_addr(peer_key, peer_addr);
    let credential =
        aspen_cluster::federation::token_store::load_credential_for_peer(runtime.forge_node.kv(), &peer_key).await;
    let connect_result = match aspen_cluster::federation::sync::connect_to_cluster(
        endpoint,
        identity,
        endpoint_addr,
        credential,
    )
    .await
    {
        Ok(result) => result,
        Err(e) => return Ok(federation_pull_error(format!("connection failed: {e}"))),
    };

    let remote_cluster_key = connect_result.identity.public_key();
    let fed_id = aspen_cluster::federation::FederatedId::new(remote_cluster_key, repo_id_bytes);
    let fed_id_str = fed_id.to_string();
    let remote_cluster_key_str = remote_cluster_key.to_string();
    match get_or_create_mirror(runtime.forge_node, &fed_id_str, MirrorOrigin {
        cluster_key: &remote_cluster_key_str,
        node_id: Some(peer_node_id),
        addr_hint: peer_addr,
    })
    .await
    {
        Ok(_) => {}
        Err(e) => return Ok(federation_pull_error(format!("mirror creation failed: {e}"))),
    }

    let result = handle_federation_fetch_refs(
        FetchRefsTarget {
            fed_id_str: &fed_id_str,
            peer_node_id,
        },
        peer_addr,
        runtime.cluster_identity,
        runtime.iroh_endpoint,
        runtime.forge_node,
    )
    .await?;

    match result {
        ClientRpcResponse::FederationFetchRefsResult(resp) => Ok(ClientRpcResponse::FederationPullResult(resp)),
        other => Ok(other),
    }
}

/// Execute the pull phase of a bidi sync: fetch objects the remote has and import them.
///
/// Returns `(pulled_objects, pull_refs_updated, errors)`.
#[cfg(feature = "git-bridge")]
#[allow(clippy::too_many_arguments)]
async fn execute_bidi_pull(
    forge_node: &ForgeNodeRef,
    connection: &iroh::endpoint::Connection,
    remote_fed_id: &aspen_cluster::federation::FederatedId,
    remote_cluster_key: iroh::PublicKey,
    peer_node_id: &str,
    peer_addr: Option<&str>,
    to_pull: &[String],
    remote_heads: &std::collections::HashMap<String, [u8; 32]>,
    repo_id_bytes: [u8; 32],
) -> (u32, u32, Vec<String>) {
    let mut errors = Vec::new();
    let remote_cluster_key_str = remote_cluster_key.to_string();

    // Ensure mirror repo exists before fetching
    let _ = get_or_create_mirror(forge_node, &remote_fed_id.to_string(), MirrorOrigin {
        cluster_key: &remote_cluster_key_str,
        node_id: Some(peer_node_id),
        addr_hint: peer_addr,
    })
    .await;

    let have_hashes =
        collect_local_blake3_hashes(forge_node, &aspen_forge::identity::RepoId(repo_id_bytes), 10_000).await;

    #[allow(deprecated)] // Single-shot pull; SyncSession not needed
    let (objects, _has_more) = match aspen_cluster::federation::sync::sync_remote_objects(
        connection,
        remote_fed_id,
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

    let mut pulled = objects.len() as u32;

    if !objects.is_empty()
        && let Ok(mirror_id) = get_or_create_mirror(forge_node, &remote_fed_id.to_string(), MirrorOrigin {
            cluster_key: &remote_cluster_key_str,
            node_id: Some(peer_node_id),
            addr_hint: peer_addr,
        })
        .await
    {
        let stats = federation_import_objects(forge_node, &mirror_id, &objects).await;
        pulled = stats.commits + stats.trees + stats.blobs;
    }

    let pull_ref_updates: Vec<(String, [u8; 32])> =
        to_pull.iter().filter_map(|name| remote_heads.get(name).map(|h| (name.clone(), *h))).collect();
    let pull_refs_updated = pull_ref_updates.len() as u32;

    if let Ok(mirror_id) = get_or_create_mirror(forge_node, &remote_fed_id.to_string(), MirrorOrigin {
        cluster_key: &remote_cluster_key_str,
        node_id: Some(peer_node_id),
        addr_hint: peer_addr,
    })
    .await
        && let Err(e) = update_mirror_refs(forge_node, &mirror_id, &pull_ref_updates).await
    {
        errors.push(format!("pull ref update: {e}"));
    }

    (pulled, pull_refs_updated, errors)
}

/// Execute the push phase of a bidi sync: export local objects and send them to the remote.
///
/// Returns `(pushed_objects, push_refs_updated, errors)`.
#[cfg(feature = "git-bridge")]
async fn execute_bidi_push(
    connection: &iroh::endpoint::Connection,
    forge_node: &ForgeNodeRef,
    local_fed_id: &aspen_cluster::federation::FederatedId,
    to_push: &[String],
    local_heads: &std::collections::HashMap<String, [u8; 32]>,
) -> (u32, u32, Vec<String>) {
    use aspen_cluster::federation::FederationResourceResolver;
    use aspen_cluster::federation::sync::RefEntry;
    use aspen_cluster::federation::sync::SyncObject;

    let mut errors = Vec::new();
    let mut pushed = 0u32;
    let mut push_refs_updated = 0u32;

    let resolver = aspen_forge::resolver::ForgeResourceResolver::new(forge_node.kv().clone());
    let want_types = vec![
        "refs".to_string(),
        "commit".to_string(),
        "tree".to_string(),
        "blob".to_string(),
    ];
    let objects: Vec<SyncObject> = match resolver.sync_objects(local_fed_id, &want_types, &[], 1000).await {
        Ok(objs) => objs,
        Err(e) => {
            errors.push(format!("push export failed: {e}"));
            Vec::new()
        }
    };

    let mut git_objects = Vec::new();
    let mut ref_updates = Vec::new();
    for obj in objects {
        if obj.object_type == "ref" {
            if let Ok(entry) = postcard::from_bytes::<RefEntry>(&obj.data)
                && to_push.contains(&entry.ref_name)
            {
                ref_updates.push(entry);
            }
        } else {
            git_objects.push(obj);
        }
    }

    // Fallback: use local heads directly if resolver returned no ref objects
    if ref_updates.is_empty() {
        for ref_name in to_push {
            if let Some(hash) = local_heads.get(ref_name) {
                ref_updates.push(RefEntry {
                    ref_name: ref_name.clone(),
                    head_hash: *hash,
                    commit_sha1: None,
                });
            }
        }
    }

    match aspen_cluster::federation::sync::push_to_cluster(connection, local_fed_id, git_objects, ref_updates).await {
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

    (pushed, push_refs_updated, errors)
}

/// Parses and validates the arguments for a bidi-sync call.
///
/// Returns `(identity, endpoint, peer_key, repo_id_bytes)` or an early-exit
/// error response if any argument is missing or malformed.
#[cfg(feature = "git-bridge")]
#[allow(clippy::result_large_err, clippy::type_complexity)]
fn parse_bidi_sync_args<'a>(
    peer_node_id: &str,
    repo_id_hex: &str,
    cluster_identity: Option<&'a Arc<aspen_cluster::federation::ClusterIdentity>>,
    iroh_endpoint: Option<&'a Arc<iroh::Endpoint>>,
) -> Result<
    (
        &'a Arc<aspen_cluster::federation::ClusterIdentity>,
        &'a Arc<iroh::Endpoint>,
        iroh::PublicKey,
        [u8; 32],
    ),
    ClientRpcResponse,
> {
    use aspen_client_api::FederationBidiSyncResponse;

    let err = |msg: String| {
        Err(ClientRpcResponse::FederationBidiSyncResult(FederationBidiSyncResponse {
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
        None => return err("federation not configured on this cluster".to_string()),
    };
    let endpoint = match iroh_endpoint {
        Some(ep) => ep,
        None => return err("iroh endpoint not available".to_string()),
    };
    let peer_key: iroh::PublicKey = match peer_node_id.parse() {
        Ok(k) => k,
        Err(e) => return err(format!("invalid peer node ID '{}': {}", peer_node_id, e)),
    };
    let repo_id_bytes: [u8; 32] = match hex::decode(repo_id_hex) {
        Ok(b) if b.len() == 32 => {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&b);
            arr
        }
        Ok(b) => return err(format!("repo_id must be 32 bytes, got {}", b.len())),
        Err(e) => return err(format!("invalid repo_id hex '{}': {}", repo_id_hex, e)),
    };

    Ok((identity, endpoint, peer_key, repo_id_bytes))
}

/// Builds the `FederationBidiSyncResult` response from sync outcome values.
///
/// Pass `error: Some(msg)` for top-level failures; `diff_empty` should be
/// `true` only when both `to_pull` and `to_push` are empty (no work needed).
#[cfg(feature = "git-bridge")]
#[allow(clippy::too_many_arguments)]
fn build_bidi_sync_response(
    pulled: u32,
    pushed: u32,
    pull_refs_updated: u32,
    push_refs_updated: u32,
    conflicts: Vec<String>,
    errors: Vec<String>,
    diff_empty: bool,
    error: Option<String>,
) -> ClientRpcResponse {
    use aspen_client_api::FederationBidiSyncResponse;

    let is_success = error.is_none() && (pulled > 0 || pushed > 0 || diff_empty);
    ClientRpcResponse::FederationBidiSyncResult(FederationBidiSyncResponse {
        is_success,
        pulled,
        pushed,
        pull_refs_updated,
        push_refs_updated,
        conflicts,
        errors,
        error,
    })
}

/// Connects to a federation peer, fetches remote ref state, and gets local ref state.
///
/// Returns `(connect_result, remote_fed_id, remote_heads, local_fed_id, local_state)`
/// or an error response on failure.
#[cfg(feature = "git-bridge")]
async fn bidi_connect_and_get_state(
    endpoint: &Arc<iroh::Endpoint>,
    identity: &Arc<aspen_cluster::federation::ClusterIdentity>,
    endpoint_addr: iroh::EndpointAddr,
    credential: Option<aspen_auth::Credential>,
    repo_id_bytes: [u8; 32],
    forge_node: &ForgeNodeRef,
) -> Result<
    (
        aspen_cluster::federation::sync::ConnectResult,
        aspen_cluster::federation::FederatedId,
        std::collections::HashMap<String, [u8; 32]>,
        aspen_cluster::federation::FederatedId,
        aspen_cluster::federation::resolver::FederationResourceState,
    ),
    ClientRpcResponse,
> {
    use aspen_cluster::federation::FederatedId;
    use aspen_cluster::federation::FederationResourceResolver;

    let connect_result = match aspen_cluster::federation::sync::connect_to_cluster(
        endpoint,
        identity,
        endpoint_addr,
        credential,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            return Err(build_bidi_sync_response(
                0,
                0,
                0,
                0,
                Vec::new(),
                Vec::new(),
                false,
                Some(format!("connection failed: {e}")),
            ));
        }
    };

    let remote_cluster_key = connect_result.identity.public_key();
    let remote_fed_id = FederatedId::new(remote_cluster_key, repo_id_bytes);

    let (remote_found, remote_heads, _) =
        match aspen_cluster::federation::sync::get_remote_resource_state(&connect_result.connection, &remote_fed_id)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                return Err(build_bidi_sync_response(
                    0,
                    0,
                    0,
                    0,
                    Vec::new(),
                    Vec::new(),
                    false,
                    Some(format!("failed to get remote state: {e}")),
                ));
            }
        };

    if !remote_found {
        return Err(build_bidi_sync_response(
            0,
            0,
            0,
            0,
            Vec::new(),
            Vec::new(),
            false,
            Some(format!("remote resource not found: {}", remote_fed_id.short())),
        ));
    }

    let local_fed_id = FederatedId::new(identity.public_key(), repo_id_bytes);
    let resolver = aspen_forge::resolver::ForgeResourceResolver::new(forge_node.kv().clone());
    let local_state = resolver.get_resource_state(&local_fed_id).await.unwrap_or_default();

    Ok((connect_result, remote_fed_id, remote_heads, local_fed_id, local_state))
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
    use aspen_cluster::federation::verified::ref_diff;

    let (identity, endpoint, peer_key, repo_id_bytes) =
        match parse_bidi_sync_args(peer_node_id, repo_id_hex, cluster_identity, iroh_endpoint) {
            Ok(v) => v,
            Err(r) => return Ok(r),
        };

    let endpoint_addr = build_endpoint_addr(peer_key, peer_addr);
    let credential = aspen_cluster::federation::token_store::load_credential_for_peer(forge_node.kv(), &peer_key).await;

    let (connect_result, remote_fed_id, remote_heads, local_fed_id, local_state) = match bidi_connect_and_get_state(
        endpoint,
        identity,
        endpoint_addr,
        credential,
        repo_id_bytes,
        forge_node,
    )
    .await
    {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };

    let mut diff = ref_diff::compute_ref_diff(&local_state.heads, &remote_heads);
    let conflict_names = diff.conflicts.clone();
    ref_diff::resolve_conflicts(&mut diff, !push_wins); // pull_wins = !push_wins

    let (mut errors, mut pulled, mut pushed) = (Vec::<String>::new(), 0u32, 0u32);
    let (mut pull_refs_updated, mut push_refs_updated) = (0u32, 0u32);

    if !diff.to_pull.is_empty() {
        let (p, rpu, errs) = execute_bidi_pull(
            forge_node,
            &connect_result.connection,
            &remote_fed_id,
            connect_result.identity.public_key(),
            peer_node_id,
            peer_addr,
            &diff.to_pull,
            &remote_heads,
            repo_id_bytes,
        )
        .await;
        pulled = p;
        pull_refs_updated = rpu;
        errors.extend(errs);
    }

    if !diff.to_push.is_empty() {
        let (p, pru, errs) =
            execute_bidi_push(&connect_result.connection, forge_node, &local_fed_id, &diff.to_push, &local_state.heads)
                .await;
        pushed = p;
        push_refs_updated = pru;
        errors.extend(errs);
    }

    let diff_empty = diff.to_pull.is_empty() && diff.to_push.is_empty();
    Ok(build_bidi_sync_response(
        pulled,
        pushed,
        pull_refs_updated,
        push_refs_updated,
        conflict_names,
        errors,
        diff_empty,
        None,
    ))
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

    // Load stored credential for this peer (if any)
    let credential = aspen_cluster::federation::token_store::load_credential_for_peer(forge_node.kv(), &peer_key).await;

    // Connect and push
    let connect_result = match aspen_cluster::federation::sync::connect_to_cluster(
        endpoint,
        identity,
        endpoint_addr,
        credential,
    )
    .await
    {
        Ok(result) => result,
        Err(e) => return err_response(format!("failed to connect to peer: {}", e)),
    };

    let push_result = match aspen_cluster::federation::sync::push_to_cluster(
        &connect_result.connection,
        &fed_id,
        git_objects,
        ref_updates,
    )
    .await
    {
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

/// Collect SHA-1 hashes of locally imported git objects for a mirror repo.
///
/// Scans the SHA-1→BLAKE3 hash mapping store (`forge:hashmap:sha1:{repo}:`)
/// and returns SHA-1 hashes zero-padded to 32 bytes for the federation wire
/// format. The exporter truncates back to 20 bytes for c2e lookup.
///
/// Returns at most `limit` hashes.
#[cfg(feature = "git-bridge")]
pub(crate) async fn collect_local_sha1_hashes(
    forge_node: &ForgeNodeRef,
    repo_id: &aspen_forge::identity::RepoId,
    limit: u32,
) -> Vec<[u8; 32]> {
    let prefix = format!("forge:hashmap:sha1:{}:", repo_id.to_hex());
    let scan_req = aspen_core::ScanRequest {
        prefix: prefix.clone(),
        limit_results: Some(limit.min(50_000)),
        continuation_token: None,
    };

    let entries = match forge_node.kv().scan(scan_req).await {
        Ok(result) => result.entries,
        Err(e) => {
            tracing::warn!(error = %e, "failed to scan local SHA-1 hashes for mirror");
            return Vec::new();
        }
    };

    let mut hashes = Vec::with_capacity(entries.len());
    for entry in &entries {
        // Key: forge:hashmap:sha1:{repo_id}:{sha1_hex}
        if let Some(sha1_hex) = entry.key.strip_prefix(&prefix)
            && let Ok(bytes) = hex::decode(sha1_hex)
            && bytes.len() == 20
        {
            let mut sha1 = [0u8; 20];
            sha1.copy_from_slice(&bytes);
            hashes.push(aspen_forge::resolver::sha1_to_have_hash(&sha1));
        }
    }

    tracing::debug!(
        repo_id = %repo_id.to_hex(),
        count = hashes.len(),
        "collected local SHA-1 hashes for incremental sync"
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

    // ====================================================================
    // prepare_objects_for_import characterization tests
    // ====================================================================

    #[cfg(feature = "git-bridge")]
    mod prepare_import_tests {
        use aspen_cluster::federation::sync::SyncObject;

        fn make_sync_object(object_type: &str, data: &[u8]) -> SyncObject {
            SyncObject {
                object_type: object_type.to_string(),
                hash: [0u8; 32],
                data: data.to_vec(),
                signature: None,
                signer: None,
                envelope_hash: None,
                origin_sha1: None,
            }
        }

        #[test]
        fn test_prepare_objects_empty() {
            let result = super::super::prepare_objects_for_import(&[]);
            assert!(result.entries.is_empty());
            assert!(result.sha1_to_envelope.is_empty());
            assert!(result.re_sha1_to_origin.is_empty());
            assert_eq!(result.type_counts, [0, 0, 0]);
            assert_eq!(result.total_bytes, 0);
        }

        #[test]
        fn test_prepare_objects_type_counts() {
            let objects = vec![
                make_sync_object("blob", b"blob content"),
                make_sync_object("blob", b"blob2"),
                make_sync_object("tree", b"tree data"),
                make_sync_object("commit", b"commit data"),
                make_sync_object("commit", b"commit2"),
                make_sync_object("commit", b"commit3"),
            ];
            let result = super::super::prepare_objects_for_import(&objects);
            // type_counts: [blobs, trees, commits]
            assert_eq!(result.type_counts[0], 2, "blobs");
            assert_eq!(result.type_counts[1], 1, "trees");
            assert_eq!(result.type_counts[2], 3, "commits");
            assert_eq!(result.entries.len(), 6);
        }

        #[test]
        fn test_prepare_objects_total_bytes() {
            let objects = vec![make_sync_object("blob", b"hello"), make_sync_object("tree", b"world!!")];
            let result = super::super::prepare_objects_for_import(&objects);
            // total_bytes counts obj.data.len(), not the git-formatted bytes
            assert_eq!(result.total_bytes, 5 + 7);
        }

        #[test]
        fn test_prepare_objects_skips_unknown_types() {
            let objects = vec![
                make_sync_object("ref", b"ref data"),
                make_sync_object("cob", b"cob data"),
                make_sync_object("blob", b"real blob"),
            ];
            let result = super::super::prepare_objects_for_import(&objects);
            assert_eq!(result.entries.len(), 1, "only blob should be included");
            assert_eq!(result.type_counts[0], 1);
            // total_bytes only counts the accepted object
            assert_eq!(result.total_bytes, 9);
        }

        #[test]
        fn test_prepare_objects_envelope_hash_populated() {
            let envelope = [0xabu8; 32];
            let mut obj = make_sync_object("blob", b"content");
            obj.envelope_hash = Some(envelope);

            let result = super::super::prepare_objects_for_import(&[obj]);
            assert_eq!(result.sha1_to_envelope.len(), 1);
            let stored = result.sha1_to_envelope.values().next().unwrap();
            assert_eq!(stored.as_bytes(), &envelope);
        }

        #[test]
        fn test_prepare_objects_no_envelope_hash_not_inserted() {
            let obj = make_sync_object("blob", b"content");
            // envelope_hash is None
            let result = super::super::prepare_objects_for_import(&[obj]);
            assert!(result.sha1_to_envelope.is_empty());
        }

        #[test]
        fn test_prepare_objects_origin_sha1_remap_when_differs() {
            use sha1::Digest as _;
            let data = b"tree content";
            let header = format!("tree {}\0", data.len());
            let mut hasher = sha1::Sha1::new();
            hasher.update(header.as_bytes());
            hasher.update(data);
            let computed_sha1: [u8; 20] = hasher.finalize().into();

            // origin_sha1 differs from computed sha1
            let origin = [0x11u8; 20];
            assert_ne!(computed_sha1, origin);

            let mut obj = make_sync_object("tree", data);
            obj.origin_sha1 = Some(origin);

            let result = super::super::prepare_objects_for_import(&[obj]);
            // re_sha1_to_origin should contain computed_sha1 -> (origin, Tree)
            assert_eq!(result.re_sha1_to_origin.len(), 1);
            let (stored_origin, _git_type) = result.re_sha1_to_origin.get(&computed_sha1).unwrap();
            assert_eq!(stored_origin, &origin);
        }
    }
}
