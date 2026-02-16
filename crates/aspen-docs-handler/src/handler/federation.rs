//! Peer cluster federation handler functions.
//!
//! Handles: AddPeerCluster, RemovePeerCluster, ListPeerClusters.

use aspen_client_api::AddPeerClusterResultResponse;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::ListPeerClustersResultResponse;
use aspen_client_api::PeerClusterInfo;
use aspen_client_api::RemovePeerClusterResultResponse;
use aspen_core::AspenDocsTicket;
use aspen_rpc_core::ClientProtocolContext;
use tracing::warn;

/// Sub-handler for peer cluster federation operations.
pub(crate) struct FederationHandler;

impl FederationHandler {
    pub(crate) fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::AddPeerCluster { .. }
                | ClientRpcRequest::RemovePeerCluster { .. }
                | ClientRpcRequest::ListPeerClusters
        )
    }

    pub(crate) async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::AddPeerCluster { ticket } => handle_add_peer_cluster(ctx, ticket).await,
            ClientRpcRequest::RemovePeerCluster { cluster_id } => handle_remove_peer_cluster(ctx, cluster_id).await,
            ClientRpcRequest::ListPeerClusters => handle_list_peer_clusters(ctx).await,
            _ => Err(anyhow::anyhow!("request not handled by FederationHandler")),
        }
    }
}

async fn handle_add_peer_cluster(ctx: &ClientProtocolContext, ticket: String) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref peer_manager) = ctx.peer_manager else {
        return Ok(ClientRpcResponse::AddPeerClusterResult(AddPeerClusterResultResponse {
            is_success: false,
            cluster_id: None,
            priority: None,
            error: Some("peer sync not enabled".to_string()),
        }));
    };

    // Parse the ticket from the actual aspen_docs crate
    let parsed_ticket = match aspen_docs::ticket::AspenDocsTicket::deserialize(&ticket) {
        Ok(t) => t,
        Err(_) => {
            return Ok(ClientRpcResponse::AddPeerClusterResult(AddPeerClusterResultResponse {
                is_success: false,
                cluster_id: None,
                priority: None,
                error: Some("invalid ticket".to_string()),
            }));
        }
    };

    // Convert to trait's AspenDocsTicket type
    let docs_ticket = AspenDocsTicket {
        cluster_id: parsed_ticket.cluster_id.clone(),
        priority: parsed_ticket.priority,
    };

    let cluster_id = docs_ticket.cluster_id.clone();
    let priority = docs_ticket.priority as u32;

    match peer_manager.add_peer(docs_ticket).await {
        Ok(()) => Ok(ClientRpcResponse::AddPeerClusterResult(AddPeerClusterResultResponse {
            is_success: true,
            cluster_id: Some(cluster_id),
            priority: Some(priority),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "add peer cluster failed");
            Ok(ClientRpcResponse::AddPeerClusterResult(AddPeerClusterResultResponse {
                is_success: false,
                cluster_id: Some(cluster_id),
                priority: None,
                error: Some("peer cluster operation failed".to_string()),
            }))
        }
    }
}

async fn handle_remove_peer_cluster(
    ctx: &ClientProtocolContext,
    cluster_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref peer_manager) = ctx.peer_manager else {
        return Ok(ClientRpcResponse::RemovePeerClusterResult(RemovePeerClusterResultResponse {
            is_success: false,
            cluster_id: cluster_id.clone(),
            error: Some("peer sync not enabled".to_string()),
        }));
    };

    match peer_manager.remove_peer(&cluster_id).await {
        Ok(()) => Ok(ClientRpcResponse::RemovePeerClusterResult(RemovePeerClusterResultResponse {
            is_success: true,
            cluster_id,
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "remove peer cluster failed");
            Ok(ClientRpcResponse::RemovePeerClusterResult(RemovePeerClusterResultResponse {
                is_success: false,
                cluster_id,
                error: Some("peer cluster operation failed".to_string()),
            }))
        }
    }
}

async fn handle_list_peer_clusters(ctx: &ClientProtocolContext) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref peer_manager) = ctx.peer_manager else {
        return Ok(ClientRpcResponse::ListPeerClustersResult(ListPeerClustersResultResponse {
            peers: vec![],
            count: 0,
            error: Some("peer sync not enabled".to_string()),
        }));
    };

    let peers = peer_manager.list_peers().await;
    let count = peers.len() as u32;
    let peer_infos: Vec<PeerClusterInfo> = peers
        .into_iter()
        .map(|p| PeerClusterInfo {
            cluster_id: p.cluster_id,
            name: p.name,
            state: format!("{:?}", p.state),
            priority: p.priority,
            is_enabled: p.is_enabled,
            sync_count: p.sync_count,
            failure_count: p.failure_count,
        })
        .collect();

    Ok(ClientRpcResponse::ListPeerClustersResult(ListPeerClustersResultResponse {
        peers: peer_infos,
        count,
        error: None,
    }))
}
