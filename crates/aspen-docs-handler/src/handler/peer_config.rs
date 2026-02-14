//! Peer cluster configuration handler functions.
//!
//! Handles: GetPeerClusterStatus, UpdatePeerClusterFilter,
//! UpdatePeerClusterPriority, SetPeerClusterEnabled.

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::PeerClusterStatusResponse;
use aspen_client_api::SetPeerClusterEnabledResultResponse;
use aspen_client_api::UpdatePeerClusterFilterResultResponse;
use aspen_client_api::UpdatePeerClusterPriorityResultResponse;
use aspen_rpc_core::ClientProtocolContext;
use tracing::warn;

/// Sub-handler for peer cluster configuration operations.
pub(crate) struct PeerConfigHandler;

impl PeerConfigHandler {
    pub(crate) fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::GetPeerClusterStatus { .. }
                | ClientRpcRequest::UpdatePeerClusterFilter { .. }
                | ClientRpcRequest::UpdatePeerClusterPriority { .. }
                | ClientRpcRequest::SetPeerClusterEnabled { .. }
        )
    }

    pub(crate) async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::GetPeerClusterStatus { cluster_id } => {
                handle_get_peer_cluster_status(ctx, cluster_id).await
            }
            ClientRpcRequest::UpdatePeerClusterFilter {
                cluster_id,
                filter_type,
                prefixes,
            } => handle_update_peer_cluster_filter(ctx, cluster_id, filter_type, prefixes).await,
            ClientRpcRequest::UpdatePeerClusterPriority { cluster_id, priority } => {
                handle_update_peer_cluster_priority(ctx, cluster_id, priority).await
            }
            ClientRpcRequest::SetPeerClusterEnabled { cluster_id, enabled } => {
                handle_set_peer_cluster_enabled(ctx, cluster_id, enabled).await
            }
            _ => Err(anyhow::anyhow!("request not handled by PeerConfigHandler")),
        }
    }
}

async fn handle_get_peer_cluster_status(
    ctx: &ClientProtocolContext,
    cluster_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref peer_manager) = ctx.peer_manager else {
        return Ok(ClientRpcResponse::PeerClusterStatus(PeerClusterStatusResponse {
            found: false,
            cluster_id: cluster_id.clone(),
            state: "unknown".to_string(),
            syncing: false,
            entries_received: 0,
            entries_imported: 0,
            entries_skipped: 0,
            entries_filtered: 0,
            error: Some("peer sync not enabled".to_string()),
        }));
    };

    match peer_manager.sync_status(&cluster_id).await {
        Some(status) => Ok(ClientRpcResponse::PeerClusterStatus(PeerClusterStatusResponse {
            found: true,
            cluster_id: status.cluster_id,
            state: format!("{:?}", status.state),
            syncing: status.syncing,
            entries_received: status.entries_received,
            entries_imported: status.entries_imported,
            entries_skipped: status.entries_skipped,
            entries_filtered: status.entries_filtered,
            error: None,
        })),
        None => Ok(ClientRpcResponse::PeerClusterStatus(PeerClusterStatusResponse {
            found: false,
            cluster_id,
            state: "unknown".to_string(),
            syncing: false,
            entries_received: 0,
            entries_imported: 0,
            entries_skipped: 0,
            entries_filtered: 0,
            error: None,
        })),
    }
}

async fn handle_update_peer_cluster_filter(
    ctx: &ClientProtocolContext,
    cluster_id: String,
    filter_type: String,
    prefixes: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref peer_manager) = ctx.peer_manager else {
        return Ok(ClientRpcResponse::UpdatePeerClusterFilterResult(UpdatePeerClusterFilterResultResponse {
            success: false,
            cluster_id: cluster_id.clone(),
            filter_type: None,
            error: Some("peer sync not enabled".to_string()),
        }));
    };

    // Parse filter type and prefixes
    let filter = match filter_type.to_lowercase().as_str() {
        "full" | "fullreplication" => aspen_core::SubscriptionFilter::FullReplication,
        "include" | "prefixfilter" => {
            let prefix_list: Vec<String> =
                prefixes.as_ref().map(|p| serde_json::from_str(p).unwrap_or_default()).unwrap_or_default();
            aspen_core::SubscriptionFilter::PrefixFilter(prefix_list)
        }
        "exclude" | "prefixexclude" => {
            let prefix_list: Vec<String> =
                prefixes.as_ref().map(|p| serde_json::from_str(p).unwrap_or_default()).unwrap_or_default();
            aspen_core::SubscriptionFilter::PrefixExclude(prefix_list)
        }
        other => {
            return Ok(ClientRpcResponse::UpdatePeerClusterFilterResult(UpdatePeerClusterFilterResultResponse {
                success: false,
                cluster_id,
                filter_type: None,
                error: Some(format!("invalid filter type: {}", other)),
            }));
        }
    };

    let Some(importer) = peer_manager.importer() else {
        return Ok(ClientRpcResponse::UpdatePeerClusterFilterResult(UpdatePeerClusterFilterResultResponse {
            success: false,
            cluster_id,
            filter_type: None,
            error: Some("importer not available".to_string()),
        }));
    };

    match importer.update_filter(&cluster_id, filter.clone()).await {
        Ok(()) => Ok(ClientRpcResponse::UpdatePeerClusterFilterResult(UpdatePeerClusterFilterResultResponse {
            success: true,
            cluster_id,
            filter_type: Some(filter_type),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "update peer cluster filter failed");
            Ok(ClientRpcResponse::UpdatePeerClusterFilterResult(UpdatePeerClusterFilterResultResponse {
                success: false,
                cluster_id,
                filter_type: None,
                error: Some("peer cluster operation failed".to_string()),
            }))
        }
    }
}

async fn handle_update_peer_cluster_priority(
    ctx: &ClientProtocolContext,
    cluster_id: String,
    priority: u32,
) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref peer_manager) = ctx.peer_manager else {
        return Ok(ClientRpcResponse::UpdatePeerClusterPriorityResult(UpdatePeerClusterPriorityResultResponse {
            success: false,
            cluster_id: cluster_id.clone(),
            previous_priority: None,
            new_priority: None,
            error: Some("peer sync not enabled".to_string()),
        }));
    };

    let Some(importer) = peer_manager.importer() else {
        return Ok(ClientRpcResponse::UpdatePeerClusterPriorityResult(UpdatePeerClusterPriorityResultResponse {
            success: false,
            cluster_id,
            previous_priority: None,
            new_priority: None,
            error: Some("importer not available".to_string()),
        }));
    };

    // Get current priority before update
    let previous_priority =
        peer_manager.list_peers().await.into_iter().find(|p| p.cluster_id == cluster_id).map(|p| p.priority);

    match importer.update_priority(&cluster_id, priority).await {
        Ok(()) => Ok(ClientRpcResponse::UpdatePeerClusterPriorityResult(UpdatePeerClusterPriorityResultResponse {
            success: true,
            cluster_id,
            previous_priority,
            new_priority: Some(priority),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "update peer cluster priority failed");
            Ok(ClientRpcResponse::UpdatePeerClusterPriorityResult(UpdatePeerClusterPriorityResultResponse {
                success: false,
                cluster_id,
                previous_priority,
                new_priority: None,
                error: Some("peer cluster operation failed".to_string()),
            }))
        }
    }
}

async fn handle_set_peer_cluster_enabled(
    ctx: &ClientProtocolContext,
    cluster_id: String,
    enabled: bool,
) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref peer_manager) = ctx.peer_manager else {
        return Ok(ClientRpcResponse::SetPeerClusterEnabledResult(SetPeerClusterEnabledResultResponse {
            success: false,
            cluster_id: cluster_id.clone(),
            enabled: None,
            error: Some("peer sync not enabled".to_string()),
        }));
    };

    let Some(importer) = peer_manager.importer() else {
        return Ok(ClientRpcResponse::SetPeerClusterEnabledResult(SetPeerClusterEnabledResultResponse {
            success: false,
            cluster_id,
            enabled: None,
            error: Some("importer not available".to_string()),
        }));
    };

    match importer.set_enabled(&cluster_id, enabled).await {
        Ok(()) => Ok(ClientRpcResponse::SetPeerClusterEnabledResult(SetPeerClusterEnabledResultResponse {
            success: true,
            cluster_id,
            enabled: Some(enabled),
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "set peer cluster enabled failed");
            Ok(ClientRpcResponse::SetPeerClusterEnabledResult(SetPeerClusterEnabledResultResponse {
                success: false,
                cluster_id,
                enabled: None,
                error: Some("peer cluster operation failed".to_string()),
            }))
        }
    }
}
