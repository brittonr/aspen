//! Git bridge sub-handler (list refs, fetch, push, chunked push).

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ClientProtocolContext;

pub(crate) struct GitBridgeSubHandler;

impl GitBridgeSubHandler {
    pub(crate) fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::GitBridgeListRefs { .. }
                | ClientRpcRequest::GitBridgeFetch { .. }
                | ClientRpcRequest::GitBridgePush { .. }
                | ClientRpcRequest::GitBridgePushStart { .. }
                | ClientRpcRequest::GitBridgePushChunk { .. }
                | ClientRpcRequest::GitBridgePushComplete { .. }
        )
    }

    pub(crate) async fn handle(
        &self,
        request: ClientRpcRequest,
        _ctx: &ClientProtocolContext,
        _forge_node: &super::handlers::ForgeNodeRef,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgeListRefs { repo_id } => {
                super::handlers::git_bridge::handle_git_bridge_list_refs(_forge_node, repo_id).await
            }

            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgeFetch { repo_id, want, have } => {
                super::handlers::git_bridge::handle_git_bridge_fetch(_forge_node, repo_id, want, have).await
            }

            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgePush { repo_id, objects, refs } => {
                super::handlers::git_bridge::handle_git_bridge_push(_forge_node, repo_id, objects, refs).await
            }

            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgePushStart {
                repo_id,
                total_objects,
                total_size_bytes,
                refs,
                metadata,
            } => {
                super::handlers::git_bridge::handle_git_bridge_push_start(
                    _forge_node,
                    repo_id,
                    total_objects,
                    total_size_bytes,
                    refs,
                    metadata,
                )
                .await
            }

            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgePushChunk {
                session_id,
                chunk_id,
                total_chunks,
                objects,
                chunk_hash,
            } => {
                super::handlers::git_bridge::handle_git_bridge_push_chunk(
                    _forge_node,
                    session_id,
                    chunk_id,
                    total_chunks,
                    objects,
                    chunk_hash,
                )
                .await
            }

            #[cfg(feature = "git-bridge")]
            ClientRpcRequest::GitBridgePushComplete {
                session_id,
                content_hash,
            } => {
                super::handlers::git_bridge::handle_git_bridge_push_complete(_forge_node, session_id, content_hash)
                    .await
            }

            #[cfg(not(feature = "git-bridge"))]
            ClientRpcRequest::GitBridgeListRefs { .. }
            | ClientRpcRequest::GitBridgeFetch { .. }
            | ClientRpcRequest::GitBridgePush { .. }
            | ClientRpcRequest::GitBridgePushStart { .. }
            | ClientRpcRequest::GitBridgePushChunk { .. }
            | ClientRpcRequest::GitBridgePushComplete { .. } => Ok(ClientRpcResponse::error(
                "GIT_BRIDGE_UNAVAILABLE",
                "Git bridge feature not enabled. Rebuild with --features git-bridge",
            )),

            _ => Err(anyhow::anyhow!("request not handled by GitBridgeSubHandler")),
        }
    }
}
