//! Cluster ticket generation handlers.
//!
//! Handles: GetClusterTicket, GetClusterTicketCombined.

use std::str::FromStr;

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::ClusterTicketResponse;
use aspen_rpc_core::ClientProtocolContext;
use aspen_ticket::AspenClusterTicket;
use iroh::EndpointId;
use iroh_gossip::proto::TopicId;
use tracing::debug;

pub(crate) async fn handle_get_cluster_ticket(ctx: &ClientProtocolContext) -> anyhow::Result<ClientRpcResponse> {
    let hash = blake3::hash(ctx.cluster_cookie.as_bytes());
    let topic_id = TopicId::from_bytes(*hash.as_bytes());

    // Use full endpoint address with direct socket addresses for better connectivity
    let endpoint_addr = ctx.endpoint_manager.endpoint().addr();
    let ticket = AspenClusterTicket::with_bootstrap_addr(topic_id, ctx.cluster_cookie.clone(), &endpoint_addr);

    let ticket_str = ticket.serialize();

    Ok(ClientRpcResponse::ClusterTicket(ClusterTicketResponse {
        ticket: ticket_str,
        topic_id: format!("{:?}", topic_id),
        cluster_id: ctx.cluster_cookie.clone(),
        endpoint_id: ctx.endpoint_manager.endpoint().id().to_string(),
        bootstrap_peers: Some(1),
    }))
}

pub(crate) async fn handle_get_cluster_ticket_combined(
    ctx: &ClientProtocolContext,
    endpoint_ids: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    let hash = blake3::hash(ctx.cluster_cookie.as_bytes());
    let topic_id = TopicId::from_bytes(*hash.as_bytes());

    // Start with this node as the first bootstrap peer (use full address for better connectivity)
    let endpoint_addr = ctx.endpoint_manager.endpoint().addr();
    let mut ticket = AspenClusterTicket::with_bootstrap_addr(topic_id, ctx.cluster_cookie.clone(), &endpoint_addr);

    // Collect additional peers from:
    // 1. Explicit endpoint_ids parameter (comma-separated EndpointId strings)
    // 2. Cluster state (iroh_addr from known cluster nodes)
    let mut added_peers = 1u32; // Already added this node

    // Parse explicit endpoint_ids if provided
    if let Some(ids_str) = &endpoint_ids {
        for id_str in ids_str.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
            // Skip if we've hit the limit (Tiger Style: MAX_BOOTSTRAP_PEERS = 16)
            if added_peers as usize >= AspenClusterTicket::MAX_BOOTSTRAP_PEERS {
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

    // Also try to add peers from cluster state (using full addresses for better connectivity)
    if let Ok(cluster_state) = ctx.controller.current_state().await {
        for node in cluster_state.nodes.iter().take(AspenClusterTicket::MAX_BOOTSTRAP_PEERS) {
            if added_peers as usize >= AspenClusterTicket::MAX_BOOTSTRAP_PEERS {
                break;
            }

            if let Some(iroh_addr) = node.iroh_addr() {
                // Skip our own endpoint
                if iroh_addr.id == ctx.endpoint_manager.endpoint().id() {
                    continue;
                }
                // Use full endpoint address with direct socket addresses
                if ticket.add_bootstrap_addr(iroh_addr).is_ok() {
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
        bootstrap_peers: Some(added_peers),
    }))
}
