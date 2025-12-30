//! Cluster management request handler.
//!
//! Handles: InitCluster, AddLearner, ChangeMembership, PromoteLearner,
//! TriggerSnapshot, GetClusterState, GetClusterTicket, AddPeer,
//! GetClusterTicketCombined, GetClientTicket, GetDocsTicket, GetTopology.

use super::ClientProtocolContext;
use super::RequestHandler;
use crate::client_rpc::ClientRpcRequest;
use crate::client_rpc::ClientRpcResponse;

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
        _ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        // The actual cluster implementation requires complex type mapping between
        // the raft node types and the client_rpc response types. For now, we
        // delegate to a placeholder that indicates the handler exists but the
        // implementation remains in client.rs until the full extraction is completed.
        //
        // TODO: Complete extraction of Cluster handlers once the RPC type alignment
        // is completed.
        match request {
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
            | ClientRpcRequest::GetTopology { .. } => Ok(ClientRpcResponse::error(
                "NOT_IMPLEMENTED",
                "Cluster handler extraction in progress - operation handled by legacy path",
            )),
            _ => Err(anyhow::anyhow!("request not handled by ClusterHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "ClusterHandler"
    }
}
