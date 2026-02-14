//! Refs sub-handler (get, set, delete, CAS, list branches/tags).

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ClientProtocolContext;

use super::handlers::refs::*;

pub(crate) struct RefsSubHandler;

impl RefsSubHandler {
    pub(crate) fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::ForgeGetRef { .. }
                | ClientRpcRequest::ForgeSetRef { .. }
                | ClientRpcRequest::ForgeDeleteRef { .. }
                | ClientRpcRequest::ForgeCasRef { .. }
                | ClientRpcRequest::ForgeListBranches { .. }
                | ClientRpcRequest::ForgeListTags { .. }
        )
    }

    pub(crate) async fn handle(
        &self,
        request: ClientRpcRequest,
        _ctx: &ClientProtocolContext,
        forge_node: &super::handlers::ForgeNodeRef,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::ForgeGetRef { repo_id, ref_name } => handle_get_ref(forge_node, repo_id, ref_name).await,

            ClientRpcRequest::ForgeSetRef {
                repo_id,
                ref_name,
                hash,
                signer: _,
                signature: _,
                timestamp_ms: _,
            } => handle_set_ref(forge_node, repo_id, ref_name, hash).await,

            ClientRpcRequest::ForgeDeleteRef { repo_id, ref_name } => {
                handle_delete_ref(forge_node, repo_id, ref_name).await
            }

            ClientRpcRequest::ForgeCasRef {
                repo_id,
                ref_name,
                expected,
                new_hash,
                signer,
                signature,
                timestamp_ms,
            } => {
                handle_cas_ref(forge_node, repo_id, ref_name, expected, new_hash, signer, signature, timestamp_ms).await
            }

            ClientRpcRequest::ForgeListBranches { repo_id } => handle_list_branches(forge_node, repo_id).await,

            ClientRpcRequest::ForgeListTags { repo_id } => handle_list_tags(forge_node, repo_id).await,

            _ => Err(anyhow::anyhow!("request not handled by RefsSubHandler")),
        }
    }
}
