//! Forge (decentralized Git) request handler.
//!
//! Handles all Forge* operations for decentralized version control.
//! This module is only available with the `forge` feature.

use super::ClientProtocolContext;
use super::RequestHandler;
use crate::client_rpc::ClientRpcRequest;
use crate::client_rpc::ClientRpcResponse;

/// Handler for Forge operations.
pub struct ForgeHandler;

#[async_trait::async_trait]
impl RequestHandler for ForgeHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::ForgeCreateRepo { .. }
                | ClientRpcRequest::ForgeGetRepo { .. }
                | ClientRpcRequest::ForgeListRepos { .. }
                | ClientRpcRequest::ForgeStoreBlob { .. }
                | ClientRpcRequest::ForgeGetBlob { .. }
                | ClientRpcRequest::ForgeCreateTree { .. }
                | ClientRpcRequest::ForgeGetTree { .. }
                | ClientRpcRequest::ForgeCommit { .. }
                | ClientRpcRequest::ForgeGetCommit { .. }
                | ClientRpcRequest::ForgeLog { .. }
                | ClientRpcRequest::ForgeGetRef { .. }
                | ClientRpcRequest::ForgeSetRef { .. }
                | ClientRpcRequest::ForgeDeleteRef { .. }
                | ClientRpcRequest::ForgeCasRef { .. }
                | ClientRpcRequest::ForgeListBranches { .. }
                | ClientRpcRequest::ForgeListTags { .. }
                | ClientRpcRequest::ForgeCreateIssue { .. }
                | ClientRpcRequest::ForgeListIssues { .. }
                | ClientRpcRequest::ForgeGetIssue { .. }
                | ClientRpcRequest::ForgeCommentIssue { .. }
                | ClientRpcRequest::ForgeCloseIssue { .. }
                | ClientRpcRequest::ForgeReopenIssue { .. }
                | ClientRpcRequest::ForgeCreatePatch { .. }
                | ClientRpcRequest::ForgeListPatches { .. }
                | ClientRpcRequest::ForgeGetPatch { .. }
                | ClientRpcRequest::ForgeUpdatePatch { .. }
                | ClientRpcRequest::ForgeApprovePatch { .. }
                | ClientRpcRequest::ForgeMergePatch { .. }
                | ClientRpcRequest::ForgeClosePatch { .. }
                | ClientRpcRequest::ForgeGetDelegateKey { .. }
                | ClientRpcRequest::GetFederationStatus
                | ClientRpcRequest::ListDiscoveredClusters
                | ClientRpcRequest::GetDiscoveredCluster { .. }
                | ClientRpcRequest::TrustCluster { .. }
                | ClientRpcRequest::UntrustCluster { .. }
                | ClientRpcRequest::FederateRepository { .. }
                | ClientRpcRequest::ListFederatedRepositories
                | ClientRpcRequest::ForgeFetchFederated { .. }
                | ClientRpcRequest::GitBridgeListRefs { .. }
                | ClientRpcRequest::GitBridgeFetch { .. }
                | ClientRpcRequest::GitBridgePush { .. }
        )
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        // Check if forge node is available
        let _forge_node = match &ctx.forge_node {
            Some(node) => node,
            None => {
                return Ok(ClientRpcResponse::error(
                    "FORGE_UNAVAILABLE",
                    "Forge feature not configured on this node",
                ));
            }
        };

        // The actual Forge implementation is complex and tightly coupled to the
        // existing client.rs code. For now, we delegate to a placeholder that
        // indicates the handler exists but the implementation remains in client.rs
        // until the full extraction is completed.
        //
        // TODO: Complete extraction of Forge handlers once the basic handler
        // infrastructure is validated with simpler handlers.
        match request {
            ClientRpcRequest::ForgeCreateRepo { .. }
            | ClientRpcRequest::ForgeGetRepo { .. }
            | ClientRpcRequest::ForgeListRepos { .. }
            | ClientRpcRequest::ForgeStoreBlob { .. }
            | ClientRpcRequest::ForgeGetBlob { .. }
            | ClientRpcRequest::ForgeCreateTree { .. }
            | ClientRpcRequest::ForgeGetTree { .. }
            | ClientRpcRequest::ForgeCommit { .. }
            | ClientRpcRequest::ForgeGetCommit { .. }
            | ClientRpcRequest::ForgeLog { .. }
            | ClientRpcRequest::ForgeGetRef { .. }
            | ClientRpcRequest::ForgeSetRef { .. }
            | ClientRpcRequest::ForgeDeleteRef { .. }
            | ClientRpcRequest::ForgeCasRef { .. }
            | ClientRpcRequest::ForgeListBranches { .. }
            | ClientRpcRequest::ForgeListTags { .. }
            | ClientRpcRequest::ForgeCreateIssue { .. }
            | ClientRpcRequest::ForgeListIssues { .. }
            | ClientRpcRequest::ForgeGetIssue { .. }
            | ClientRpcRequest::ForgeCommentIssue { .. }
            | ClientRpcRequest::ForgeCloseIssue { .. }
            | ClientRpcRequest::ForgeReopenIssue { .. }
            | ClientRpcRequest::ForgeCreatePatch { .. }
            | ClientRpcRequest::ForgeListPatches { .. }
            | ClientRpcRequest::ForgeGetPatch { .. }
            | ClientRpcRequest::ForgeUpdatePatch { .. }
            | ClientRpcRequest::ForgeApprovePatch { .. }
            | ClientRpcRequest::ForgeMergePatch { .. }
            | ClientRpcRequest::ForgeClosePatch { .. }
            | ClientRpcRequest::ForgeGetDelegateKey { .. }
            | ClientRpcRequest::GetFederationStatus
            | ClientRpcRequest::ListDiscoveredClusters
            | ClientRpcRequest::GetDiscoveredCluster { .. }
            | ClientRpcRequest::TrustCluster { .. }
            | ClientRpcRequest::UntrustCluster { .. }
            | ClientRpcRequest::FederateRepository { .. }
            | ClientRpcRequest::ListFederatedRepositories
            | ClientRpcRequest::ForgeFetchFederated { .. }
            | ClientRpcRequest::GitBridgeListRefs { .. }
            | ClientRpcRequest::GitBridgeFetch { .. }
            | ClientRpcRequest::GitBridgePush { .. } => {
                // Placeholder: These operations remain in client.rs for now
                // The handler framework is ready but full extraction is pending
                Ok(ClientRpcResponse::error(
                    "NOT_IMPLEMENTED",
                    "Forge handler extraction in progress - operation handled by legacy path",
                ))
            }
            _ => Err(anyhow::anyhow!("request not handled by ForgeHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "ForgeHandler"
    }
}
