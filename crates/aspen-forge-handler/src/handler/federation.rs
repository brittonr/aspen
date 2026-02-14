//! Federation sub-handler (status, discovery, trust, federate, fetch).

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ClientProtocolContext;

use super::handlers::federation::*;

pub(crate) struct FederationSubHandler;

impl FederationSubHandler {
    pub(crate) fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::ForgeGetDelegateKey { .. }
                | ClientRpcRequest::GetFederationStatus
                | ClientRpcRequest::ListDiscoveredClusters
                | ClientRpcRequest::GetDiscoveredCluster { .. }
                | ClientRpcRequest::TrustCluster { .. }
                | ClientRpcRequest::UntrustCluster { .. }
                | ClientRpcRequest::FederateRepository { .. }
                | ClientRpcRequest::ListFederatedRepositories
                | ClientRpcRequest::ForgeFetchFederated { .. }
        )
    }

    pub(crate) async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
        forge_node: &super::handlers::ForgeNodeRef,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::ForgeGetDelegateKey { repo_id } => handle_get_delegate_key(forge_node, repo_id).await,

            ClientRpcRequest::GetFederationStatus => handle_get_federation_status(ctx, forge_node).await,

            ClientRpcRequest::ListDiscoveredClusters => handle_list_discovered_clusters(ctx).await,

            ClientRpcRequest::GetDiscoveredCluster { cluster_key } => {
                handle_get_discovered_cluster(ctx, cluster_key).await
            }

            ClientRpcRequest::TrustCluster { cluster_key } => handle_trust_cluster(ctx, cluster_key).await,

            ClientRpcRequest::UntrustCluster { cluster_key } => handle_untrust_cluster(ctx, cluster_key).await,

            ClientRpcRequest::FederateRepository { repo_id, mode } => {
                handle_federate_repository(forge_node, repo_id, mode).await
            }

            ClientRpcRequest::ListFederatedRepositories => handle_list_federated_repositories(forge_node).await,

            ClientRpcRequest::ForgeFetchFederated {
                federated_id,
                remote_cluster,
            } => handle_fetch_federated(forge_node, federated_id, remote_cluster).await,

            _ => Err(anyhow::anyhow!("request not handled by FederationSubHandler")),
        }
    }
}
