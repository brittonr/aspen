//! Docs/Sync request handler.
//!
//! Handles: DocsSet, DocsGet, DocsDelete, DocsList, DocsStatus,
//! AddPeerCluster, RemovePeerCluster, ListPeerClusters, GetPeerClusterStatus,
//! UpdatePeerClusterFilter, UpdatePeerClusterPriority, SetPeerClusterEnabled, GetKeyOrigin.

use super::ClientProtocolContext;
use super::RequestHandler;
use crate::client_rpc::ClientRpcRequest;
use crate::client_rpc::ClientRpcResponse;

/// Handler for docs/sync operations.
pub struct DocsHandler;

#[async_trait::async_trait]
impl RequestHandler for DocsHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::DocsSet { .. }
                | ClientRpcRequest::DocsGet { .. }
                | ClientRpcRequest::DocsDelete { .. }
                | ClientRpcRequest::DocsList { .. }
                | ClientRpcRequest::DocsStatus
                | ClientRpcRequest::AddPeerCluster { .. }
                | ClientRpcRequest::RemovePeerCluster { .. }
                | ClientRpcRequest::ListPeerClusters
                | ClientRpcRequest::GetPeerClusterStatus { .. }
                | ClientRpcRequest::UpdatePeerClusterFilter { .. }
                | ClientRpcRequest::UpdatePeerClusterPriority { .. }
                | ClientRpcRequest::SetPeerClusterEnabled { .. }
                | ClientRpcRequest::GetKeyOrigin { .. }
        )
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        _ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        // The actual Docs implementation is complex and tightly coupled to the
        // existing client.rs code. For now, we delegate to a placeholder that
        // indicates the handler exists but the implementation remains in client.rs
        // until the full extraction is completed.
        //
        // TODO: Complete extraction of Docs handlers once the basic handler
        // infrastructure is validated with simpler handlers.
        match request {
            ClientRpcRequest::DocsSet { .. }
            | ClientRpcRequest::DocsGet { .. }
            | ClientRpcRequest::DocsDelete { .. }
            | ClientRpcRequest::DocsList { .. }
            | ClientRpcRequest::DocsStatus
            | ClientRpcRequest::AddPeerCluster { .. }
            | ClientRpcRequest::RemovePeerCluster { .. }
            | ClientRpcRequest::ListPeerClusters
            | ClientRpcRequest::GetPeerClusterStatus { .. }
            | ClientRpcRequest::UpdatePeerClusterFilter { .. }
            | ClientRpcRequest::UpdatePeerClusterPriority { .. }
            | ClientRpcRequest::SetPeerClusterEnabled { .. }
            | ClientRpcRequest::GetKeyOrigin { .. } => {
                // Placeholder: These operations remain in client.rs for now
                // The handler framework is ready but full extraction is pending
                Ok(ClientRpcResponse::error(
                    "NOT_IMPLEMENTED",
                    "Docs handler extraction in progress - operation handled by legacy path",
                ))
            }
            _ => Err(anyhow::anyhow!("request not handled by DocsHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "DocsHandler"
    }
}
