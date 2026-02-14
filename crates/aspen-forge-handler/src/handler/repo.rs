//! Repository sub-handler (create, get, list).

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ClientProtocolContext;

use super::handlers::repo::*;

pub(crate) struct RepoSubHandler;

impl RepoSubHandler {
    pub(crate) fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::ForgeCreateRepo { .. }
                | ClientRpcRequest::ForgeGetRepo { .. }
                | ClientRpcRequest::ForgeListRepos { .. }
        )
    }

    pub(crate) async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
        forge_node: &super::handlers::ForgeNodeRef,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::ForgeCreateRepo {
                name,
                description,
                default_branch,
            } => handle_create_repo(forge_node, ctx, name, description, default_branch).await,

            ClientRpcRequest::ForgeGetRepo { repo_id } => handle_get_repo(forge_node, repo_id).await,

            ClientRpcRequest::ForgeListRepos { limit, offset } => handle_list_repos(ctx, limit, offset).await,

            _ => Err(anyhow::anyhow!("request not handled by RepoSubHandler")),
        }
    }
}
