//! Patches sub-handler (create, list, get, update, approve, merge, close).

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ClientProtocolContext;

use super::handlers::cob::patches::*;

pub(crate) struct PatchesSubHandler;

impl PatchesSubHandler {
    pub(crate) fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::ForgeCreatePatch { .. }
                | ClientRpcRequest::ForgeListPatches { .. }
                | ClientRpcRequest::ForgeGetPatch { .. }
                | ClientRpcRequest::ForgeUpdatePatch { .. }
                | ClientRpcRequest::ForgeApprovePatch { .. }
                | ClientRpcRequest::ForgeMergePatch { .. }
                | ClientRpcRequest::ForgeClosePatch { .. }
        )
    }

    pub(crate) async fn handle(
        &self,
        request: ClientRpcRequest,
        _ctx: &ClientProtocolContext,
        forge_node: &super::handlers::ForgeNodeRef,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::ForgeCreatePatch {
                repo_id,
                title,
                description,
                base,
                head,
            } => handle_create_patch(forge_node, repo_id, title, description, base, head).await,

            ClientRpcRequest::ForgeListPatches { repo_id, state, limit } => {
                handle_list_patches(forge_node, repo_id, state, limit).await
            }

            ClientRpcRequest::ForgeGetPatch { repo_id, patch_id } => {
                handle_get_patch(forge_node, repo_id, patch_id).await
            }

            ClientRpcRequest::ForgeUpdatePatch {
                repo_id,
                patch_id,
                head,
                message,
            } => handle_update_patch(forge_node, repo_id, patch_id, head, message).await,

            ClientRpcRequest::ForgeApprovePatch {
                repo_id,
                patch_id,
                commit,
                message,
            } => handle_approve_patch(forge_node, repo_id, patch_id, commit, message).await,

            ClientRpcRequest::ForgeMergePatch {
                repo_id,
                patch_id,
                merge_commit,
            } => handle_merge_patch(forge_node, repo_id, patch_id, merge_commit).await,

            ClientRpcRequest::ForgeClosePatch {
                repo_id,
                patch_id,
                reason,
            } => handle_close_patch(forge_node, repo_id, patch_id, reason).await,

            _ => Err(anyhow::anyhow!("request not handled by PatchesSubHandler")),
        }
    }
}
