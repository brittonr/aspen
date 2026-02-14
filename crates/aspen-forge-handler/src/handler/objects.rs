//! Git objects sub-handler (blob, tree, commit, log).

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ClientProtocolContext;

use super::handlers::log::*;
use super::handlers::objects::*;

pub(crate) struct ObjectsSubHandler;

impl ObjectsSubHandler {
    pub(crate) fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::ForgeStoreBlob { .. }
                | ClientRpcRequest::ForgeGetBlob { .. }
                | ClientRpcRequest::ForgeCreateTree { .. }
                | ClientRpcRequest::ForgeGetTree { .. }
                | ClientRpcRequest::ForgeCommit { .. }
                | ClientRpcRequest::ForgeGetCommit { .. }
                | ClientRpcRequest::ForgeLog { .. }
        )
    }

    pub(crate) async fn handle(
        &self,
        request: ClientRpcRequest,
        _ctx: &ClientProtocolContext,
        forge_node: &super::handlers::ForgeNodeRef,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::ForgeStoreBlob { repo_id, content } => {
                handle_store_blob(forge_node, repo_id, content).await
            }

            ClientRpcRequest::ForgeGetBlob { hash } => handle_get_blob(forge_node, hash).await,

            ClientRpcRequest::ForgeCreateTree { repo_id, entries_json } => {
                handle_create_tree(forge_node, repo_id, entries_json).await
            }

            ClientRpcRequest::ForgeGetTree { hash } => handle_get_tree(forge_node, hash).await,

            ClientRpcRequest::ForgeCommit {
                repo_id,
                tree,
                parents,
                message,
            } => handle_commit(forge_node, repo_id, tree, parents, message).await,

            ClientRpcRequest::ForgeGetCommit { hash } => handle_get_commit(forge_node, hash).await,

            ClientRpcRequest::ForgeLog {
                repo_id,
                ref_name,
                limit,
            } => handle_log(forge_node, repo_id, ref_name, limit).await,

            _ => Err(anyhow::anyhow!("request not handled by ObjectsSubHandler")),
        }
    }
}
