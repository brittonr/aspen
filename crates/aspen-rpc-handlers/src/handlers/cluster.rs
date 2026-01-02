//! Cluster management request handler.
//!
//! Handles: InitCluster, AddLearner, ChangeMembership, PromoteLearner,
//! TriggerSnapshot, GetClusterState, GetClusterTicket, AddPeer,
//! GetClusterTicketCombined, GetClientTicket, GetDocsTicket, GetTopology.

use std::str::FromStr;

use iroh::EndpointAddr;
use iroh::EndpointId;
use iroh_gossip::proto::TopicId;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::context::ClientProtocolContext;
use crate::registry::RequestHandler;
use aspen_cluster::ticket::AspenClusterTicket;
use aspen_core::AddLearnerRequest;
use aspen_core::ChangeMembershipRequest;
use aspen_core::ClusterNode;
use aspen_core::InitRequest;
use aspen_client_rpc::AddLearnerResultResponse;
use aspen_client_rpc::AddPeerResultResponse;
use aspen_client_rpc::ChangeMembershipResultResponse;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use aspen_client_rpc::ClientTicketResponse;
use aspen_client_rpc::ClusterStateResponse;
use aspen_client_rpc::ClusterTicketResponse;
use aspen_client_rpc::DocsTicketResponse;
use aspen_client_rpc::InitResultResponse;
use aspen_client_rpc::NodeDescriptor;
use aspen_client_rpc::PromoteLearnerResultResponse;
use aspen_client_rpc::SnapshotResultResponse;
use aspen_client_rpc::TopologyResultResponse;
// TODO: Move AspenClusterTicket to a shared crate or use generic context

/// Maximum number of nodes to include in cluster state response.
const MAX_CLUSTER_NODES: usize = 100;

/// Handler for cluster management operations.
pub struct ClusterHandler;

#[async_trait::async_trait]
impl RequestHandler for ClusterHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::InitCluster
                | ClientRpcRequest::AddLearner { .. }
                | ClientRpcRequest::ChangeMembership { .. }
                | ClientRpcRequest::PromoteLearner { .. }
                | ClientRpcRequest::TriggerSnapshot
                | ClientRpcRequest::GetClusterState
                | ClientRpcRequest::GetClusterTicket
                | ClientRpcRequest::AddPeer { .. }
                | ClientRpcRequest::GetClusterTicketCombined { .. }
                | ClientRpcRequest::GetClientTicket { .. }
                | ClientRpcRequest::GetDocsTicket { .. }
                | ClientRpcRequest::GetTopology { .. }
        )
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::InitCluster => handle_init_cluster(ctx).await,

            ClientRpcRequest::AddLearner { node_id, addr } => handle_add_learner(ctx, node_id, addr).await,

            ClientRpcRequest::ChangeMembership { members } => handle_change_membership(ctx, members).await,

            ClientRpcRequest::PromoteLearner {
                learner_id,
                replace_node,
                force,
            } => handle_promote_learner(ctx, learner_id, replace_node, force).await,

            ClientRpcRequest::TriggerSnapshot => handle_trigger_snapshot(ctx).await,

            ClientRpcRequest::GetClusterState => handle_get_cluster_state(ctx).await,

            ClientRpcRequest::GetClusterTicket => handle_get_cluster_ticket(ctx).await,

            ClientRpcRequest::AddPeer { node_id, endpoint_addr } => handle_add_peer(ctx, node_id, endpoint_addr).await,

            ClientRpcRequest::GetClusterTicketCombined { endpoint_ids } => {
                handle_get_cluster_ticket_combined(ctx, endpoint_ids).await
            }

            ClientRpcRequest::GetClientTicket { access, priority } => {
                handle_get_client_ticket(ctx, access, priority).await
            }

            ClientRpcRequest::GetDocsTicket { read_write, priority } => {
                handle_get_docs_ticket(ctx, read_write, priority).await
            }

            ClientRpcRequest::GetTopology { client_version } => handle_get_topology(ctx, client_version).await,

            _ => Err(anyhow::anyhow!("request not handled by ClusterHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "ClusterHandler"
    }
}

// ============================================================================
// Cluster Operation Handlers
// ============================================================================

/// Sanitize control plane error messages to prevent information leakage.
fn sanitize_control_error(e: &aspen_core::ControlPlaneError) -> String {
    use aspen_core::ControlPlaneError;
    match e {
        ControlPlaneError::NotInitialized => "cluster not initialized".to_string(),
        ControlPlaneError::InvalidRequest { reason } => format!("invalid request: {}", reason),
        ControlPlaneError::Failed { reason } => {
            // Check if reason contains leader info to provide hints
            if reason.contains("not leader") || reason.contains("ForwardToLeader") {
                "not leader".to_string()
            } else {
                "operation failed".to_string()
            }
        }
        _ => "internal error".to_string(),
    }
}

async fn handle_init_cluster(ctx: &ClientProtocolContext) -> anyhow::Result<ClientRpcResponse> {
    // Build ClusterNode for the current node to initialize as single-node cluster
    let endpoint_addr = ctx.endpoint_manager.node_addr();
    let this_node = ClusterNode::with_iroh_addr(ctx.node_id, endpoint_addr.clone());

    let result = ctx
        .controller
        .init(InitRequest {
            initial_members: vec![this_node],
        })
        .await;

    Ok(ClientRpcResponse::InitResult(InitResultResponse {
        success: result.is_ok(),
        error: result.err().map(|e| sanitize_control_error(&e)),
    }))
}

async fn handle_add_learner(
    ctx: &ClientProtocolContext,
    node_id: u64,
    addr: String,
) -> anyhow::Result<ClientRpcResponse> {
    // Parse the address as either JSON EndpointAddr or bare EndpointId
    let iroh_addr = if addr.starts_with('{') {
        serde_json::from_str::<EndpointAddr>(&addr).map_err(|e| format!("invalid JSON EndpointAddr: {e}"))
    } else {
        EndpointId::from_str(&addr)
            .map(EndpointAddr::new)
            .map_err(|e| format!("invalid EndpointId: {e}"))
    };

    let result = match iroh_addr {
        Ok(iroh_addr) => {
            ctx.controller
                .add_learner(AddLearnerRequest {
                    learner: ClusterNode::with_iroh_addr(node_id, iroh_addr),
                })
                .await
        }
        Err(parse_err) => Err(aspen_core::ControlPlaneError::InvalidRequest { reason: parse_err }),
    };

    Ok(ClientRpcResponse::AddLearnerResult(AddLearnerResultResponse {
        success: result.is_ok(),
        error: result.err().map(|e| sanitize_control_error(&e)),
    }))
}

async fn handle_change_membership(
    ctx: &ClientProtocolContext,
    members: Vec<u64>,
) -> anyhow::Result<ClientRpcResponse> {
    let result = ctx.controller.change_membership(ChangeMembershipRequest { members }).await;

    Ok(ClientRpcResponse::ChangeMembershipResult(ChangeMembershipResultResponse {
        success: result.is_ok(),
        error: result.err().map(|e| sanitize_control_error(&e)),
    }))
}

async fn handle_promote_learner(
    ctx: &ClientProtocolContext,
    learner_id: u64,
    replace_node: Option<u64>,
    _force: bool,
) -> anyhow::Result<ClientRpcResponse> {
    // Get current membership to build new voter set
    let cluster_state = ctx
        .controller
        .current_state()
        .await
        .map_err(|e| anyhow::anyhow!("failed to get cluster state: {}", e))?;

    let previous_voters: Vec<u64> = cluster_state.members.clone();

    // Build new membership: add learner, optionally remove replaced node
    let mut new_members: Vec<u64> = previous_voters.clone();
    if !new_members.contains(&learner_id) {
        new_members.push(learner_id);
    }
    if let Some(replace_id) = replace_node {
        new_members.retain(|&id| id != replace_id);
    }

    let result = ctx
        .controller
        .change_membership(ChangeMembershipRequest {
            members: new_members.clone(),
        })
        .await;

    Ok(ClientRpcResponse::PromoteLearnerResult(PromoteLearnerResultResponse {
        success: result.is_ok(),
        learner_id,
        previous_voters,
        new_voters: new_members,
        message: if result.is_ok() {
            format!("promoted learner {} to voter", learner_id)
        } else {
            "promotion failed".to_string()
        },
        error: result.err().map(|e| sanitize_control_error(&e)),
    }))
}

async fn handle_trigger_snapshot(ctx: &ClientProtocolContext) -> anyhow::Result<ClientRpcResponse> {
    let result = ctx.controller.trigger_snapshot().await;

    match result {
        Ok(snapshot) => Ok(ClientRpcResponse::SnapshotResult(SnapshotResultResponse {
            success: true,
            snapshot_index: snapshot.as_ref().map(|log_id| log_id.index),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::SnapshotResult(SnapshotResultResponse {
            success: false,
            snapshot_index: None,
            error: Some(sanitize_control_error(&e)),
        })),
    }
}

async fn handle_get_cluster_state(ctx: &ClientProtocolContext) -> anyhow::Result<ClientRpcResponse> {
    // Get current cluster state from the Raft controller
    let cluster_state = ctx
        .controller
        .current_state()
        .await
        .map_err(|e| anyhow::anyhow!("failed to get cluster state: {}", e))?;

    // Get current leader from metrics
    let leader_id = ctx.controller.get_leader().await.unwrap_or(None);

    // Convert ClusterNode to NodeDescriptor with membership info
    // Tiger Style: Bounded to MAX_CLUSTER_NODES
    let mut nodes: Vec<NodeDescriptor> =
        Vec::with_capacity((cluster_state.nodes.len() + cluster_state.learners.len()).min(MAX_CLUSTER_NODES));

    // Track which nodes are voters (members)
    let member_ids: std::collections::HashSet<u64> = cluster_state.members.iter().copied().collect();

    // Add all nodes from the cluster state
    for node in cluster_state.nodes.iter().take(MAX_CLUSTER_NODES) {
        let is_voter = member_ids.contains(&node.id);
        let is_leader = leader_id == Some(node.id);

        // For our own node, use our actual endpoint address; for others use Raft-stored address
        let endpoint_addr = if node.id == ctx.node_id {
            format!("{:?}", ctx.endpoint_manager.node_addr())
        } else {
            node.iroh_addr().map(|a| format!("{:?}", a)).unwrap_or_default()
        };

        nodes.push(NodeDescriptor {
            node_id: node.id,
            endpoint_addr,
            is_voter,
            is_learner: !is_voter,
            is_leader,
        });
    }

    // Add learners that weren't in the nodes list
    for learner in cluster_state.learners.iter().take(MAX_CLUSTER_NODES - nodes.len()) {
        // Skip if already added
        if nodes.iter().any(|n| n.node_id == learner.id) {
            continue;
        }

        let endpoint_addr = learner.iroh_addr().map(|a| format!("{:?}", a)).unwrap_or_default();

        nodes.push(NodeDescriptor {
            node_id: learner.id,
            endpoint_addr,
            is_voter: false,
            is_learner: true,
            is_leader: false,
        });
    }

    Ok(ClientRpcResponse::ClusterState(ClusterStateResponse {
        nodes,
        leader_id,
        this_node_id: ctx.node_id,
    }))
}

async fn handle_get_cluster_ticket(ctx: &ClientProtocolContext) -> anyhow::Result<ClientRpcResponse> {
    let hash = blake3::hash(ctx.cluster_cookie.as_bytes());
    let topic_id = TopicId::from_bytes(*hash.as_bytes());

    let ticket = AspenClusterTicket::with_bootstrap(
        topic_id,
        ctx.cluster_cookie.clone(),
        ctx.endpoint_manager.endpoint().id(),
    );

    let ticket_str = ticket.serialize();

    Ok(ClientRpcResponse::ClusterTicket(ClusterTicketResponse {
        ticket: ticket_str,
        topic_id: format!("{:?}", topic_id),
        cluster_id: ctx.cluster_cookie.clone(),
        endpoint_id: ctx.endpoint_manager.endpoint().id().to_string(),
        bootstrap_peers: Some(1),
    }))
}

async fn handle_add_peer(
    ctx: &ClientProtocolContext,
    node_id: u64,
    endpoint_addr: String,
) -> anyhow::Result<ClientRpcResponse> {
    // Parse the endpoint address (JSON-serialized EndpointAddr)
    let parsed_addr: EndpointAddr = match serde_json::from_str(&endpoint_addr) {
        Ok(addr) => addr,
        Err(e) => {
            debug!(
                node_id = node_id,
                endpoint_addr = %endpoint_addr,
                error = %e,
                "AddPeer: failed to parse endpoint_addr as JSON EndpointAddr"
            );
            return Ok(ClientRpcResponse::AddPeerResult(AddPeerResultResponse {
                success: false,
                error: Some("invalid endpoint_addr: expected JSON-serialized EndpointAddr".to_string()),
            }));
        }
    };

    // Check if network factory is available
    let network_factory = match &ctx.network_factory {
        Some(nf) => nf,
        None => {
            warn!(node_id = node_id, "AddPeer: network_factory not available in context");
            return Ok(ClientRpcResponse::AddPeerResult(AddPeerResultResponse {
                success: false,
                error: Some("network_factory not configured for this node".to_string()),
            }));
        }
    };

    // Add peer to the network factory
    // Tiger Style: add_peer is bounded by MAX_PEERS (1000)
    let _ = network_factory
        .add_peer(node_id, format!("{:?}", parsed_addr))
        .await;

    info!(
        node_id = node_id,
        endpoint_id = %parsed_addr.id,
        "AddPeer: successfully registered peer in network factory"
    );

    Ok(ClientRpcResponse::AddPeerResult(AddPeerResultResponse {
        success: true,
        error: None,
    }))
}

async fn handle_get_cluster_ticket_combined(
    ctx: &ClientProtocolContext,
    endpoint_ids: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    let hash = blake3::hash(ctx.cluster_cookie.as_bytes());
    let topic_id = TopicId::from_bytes(*hash.as_bytes());

    // Start with this node as the first bootstrap peer
    let mut ticket = AspenClusterTicket::with_bootstrap(
        topic_id,
        ctx.cluster_cookie.clone(),
        ctx.endpoint_manager.endpoint().id(),
    );

    // Collect additional peers from:
    // 1. Explicit endpoint_ids parameter (comma-separated EndpointId strings)
    // 2. Cluster state (iroh_addr from known cluster nodes)
    let mut added_peers = 1u32; // Already added this node

    // Parse explicit endpoint_ids if provided
    if let Some(ids_str) = &endpoint_ids {
        for id_str in ids_str.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
            // Skip if we've hit the limit (Tiger Style: MAX_BOOTSTRAP_PEERS = 16)
            if added_peers >= AspenClusterTicket::MAX_BOOTSTRAP_PEERS {
                debug!(
                    max_peers = AspenClusterTicket::MAX_BOOTSTRAP_PEERS,
                    "GetClusterTicketCombined: reached max bootstrap peers, skipping remaining"
                );
                break;
            }

            if let Ok(endpoint_id) = EndpointId::from_str(id_str) {
                // Skip our own endpoint
                if endpoint_id == ctx.endpoint_manager.endpoint().id() {
                    continue;
                }
                if ticket.add_bootstrap(endpoint_id).is_ok() {
                    added_peers += 1;
                }
            }
        }
    }

    // Also try to add peers from cluster state
    if let Ok(cluster_state) = ctx.controller.current_state().await {
        for node in cluster_state.nodes.iter().take(AspenClusterTicket::MAX_BOOTSTRAP_PEERS as usize) {
            if added_peers >= AspenClusterTicket::MAX_BOOTSTRAP_PEERS {
                break;
            }

            if let Some(iroh_addr) = node.iroh_addr() {
                // Skip our own endpoint
                if iroh_addr.id == ctx.endpoint_manager.endpoint().id() {
                    continue;
                }
                if ticket.add_bootstrap(iroh_addr.id).is_ok() {
                    added_peers += 1;
                }
            }
        }
    }

    let ticket_str = ticket.serialize();

    debug!(
        bootstrap_peers = added_peers,
        "GetClusterTicketCombined: generated ticket with multiple bootstrap peers"
    );

    Ok(ClientRpcResponse::ClusterTicket(ClusterTicketResponse {
        ticket: ticket_str,
        topic_id: format!("{:?}", topic_id),
        cluster_id: ctx.cluster_cookie.clone(),
        endpoint_id: ctx.endpoint_manager.endpoint().id().to_string(),
        bootstrap_peers: Some(added_peers as usize),
    }))
}

async fn handle_get_client_ticket(
    ctx: &ClientProtocolContext,
    access: String,
    priority: u32,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_client::AspenClientTicket;
    use aspen_client::AccessLevel;

    let endpoint_addr = ctx.endpoint_manager.node_addr().clone();
    let access_level = match access.to_lowercase().as_str() {
        "write" | "readwrite" | "read_write" | "rw" => AccessLevel::ReadWrite,
        _ => AccessLevel::ReadOnly,
    };

    // Tiger Style: saturate priority to u8 range
    let priority_u8 = priority.min(255) as u8;

    let ticket = AspenClientTicket::new(&ctx.cluster_cookie, vec![endpoint_addr])
        .with_access(access_level)
        .with_priority(priority_u8);

    let ticket_str = ticket.serialize();

    Ok(ClientRpcResponse::ClientTicket(ClientTicketResponse {
        ticket: ticket_str,
        cluster_id: ctx.cluster_cookie.clone(),
        access: match access_level {
            AccessLevel::ReadOnly => "read".to_string(),
            AccessLevel::ReadWrite => "write".to_string(),
        },
        priority,
        endpoint_id: ctx.endpoint_manager.endpoint().id().to_string(),
        error: None,
    }))
}

async fn handle_get_docs_ticket(
    ctx: &ClientProtocolContext,
    read_write: bool,
    priority: u8,
) -> anyhow::Result<ClientRpcResponse> {
    use aspen_docs::ticket::AspenDocsTicket;

    let endpoint_addr = ctx.endpoint_manager.node_addr().clone();

    // Derive namespace ID from cluster cookie
    let namespace_hash = blake3::hash(format!("aspen-docs-{}", ctx.cluster_cookie).as_bytes());
    let namespace_id_str = format!("{}", namespace_hash);

    let ticket = AspenDocsTicket::new(
        ctx.cluster_cookie.clone(),
        priority,
        namespace_id_str.clone(),
        vec![endpoint_addr],
        read_write,
    );

    let ticket_str = ticket.serialize();

    Ok(ClientRpcResponse::DocsTicket(DocsTicketResponse {
        ticket: ticket_str,
        cluster_id: ctx.cluster_cookie.clone(),
        namespace_id: namespace_id_str,
        read_write,
        priority,
        endpoint_id: ctx.endpoint_manager.endpoint().id().to_string(),
        error: None,
    }))
}

async fn handle_get_topology(
    ctx: &ClientProtocolContext,
    client_version: Option<u64>,
) -> anyhow::Result<ClientRpcResponse> {
    match &ctx.topology {
        Some(topology) => {
            let topo = topology.read().await;

            // Check if client already has current version
            if let Some(cv) = client_version
                && cv == topo.version
            {
                return Ok(ClientRpcResponse::TopologyResult(TopologyResultResponse {
                    success: true,
                    version: topo.version,
                    updated: false,
                    topology_data: None,
                    shard_count: topo.shard_count() as u32,
                    error: None,
                }));
            }

            // Serialize topology for client
            let topology_data =
                serde_json::to_string(&*topo).map_err(|e| anyhow::anyhow!("failed to serialize topology: {}", e))?;

            Ok(ClientRpcResponse::TopologyResult(TopologyResultResponse {
                success: true,
                version: topo.version,
                updated: true,
                topology_data: Some(topology_data),
                shard_count: topo.shard_count() as u32,
                error: None,
            }))
        }
        None => Ok(ClientRpcResponse::TopologyResult(TopologyResultResponse {
            success: false,
            version: 0,
            updated: false,
            topology_data: None,
            shard_count: 0,
            error: Some("topology not available".to_string()),
        })),
    }
}
