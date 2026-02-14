//! Docs status and key origin handler functions.
//!
//! Handles: DocsStatus, GetKeyOrigin.

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::DocsStatusResultResponse;
use aspen_client_api::KeyOriginResultResponse;
use aspen_rpc_core::ClientProtocolContext;
use tracing::warn;

/// Sub-handler for docs status and key origin operations.
pub(crate) struct StatusHandler;

impl StatusHandler {
    pub(crate) fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(request, ClientRpcRequest::DocsStatus | ClientRpcRequest::GetKeyOrigin { .. })
    }

    pub(crate) async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::DocsStatus => handle_docs_status(ctx).await,
            ClientRpcRequest::GetKeyOrigin { key } => handle_get_key_origin(ctx, key).await,
            _ => Err(anyhow::anyhow!("request not handled by StatusHandler")),
        }
    }
}

async fn handle_docs_status(ctx: &ClientProtocolContext) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref docs_sync) = ctx.docs_sync else {
        return Ok(ClientRpcResponse::DocsStatusResult(DocsStatusResultResponse {
            enabled: false,
            namespace_id: None,
            author_id: None,
            entry_count: None,
            replica_open: None,
            error: None,
        }));
    };

    let namespace_id = docs_sync.namespace_id();
    let author_id = docs_sync.author_id();

    match docs_sync.get_status().await {
        Ok(status) => Ok(ClientRpcResponse::DocsStatusResult(DocsStatusResultResponse {
            enabled: status.enabled,
            namespace_id: Some(namespace_id),
            author_id: Some(author_id),
            entry_count: status.entry_count,
            replica_open: status.replica_open,
            error: None,
        })),
        Err(e) => {
            warn!(error = %e, "docs status failed");
            Ok(ClientRpcResponse::DocsStatusResult(DocsStatusResultResponse {
                enabled: true,
                namespace_id: Some(namespace_id),
                author_id: Some(author_id),
                entry_count: None,
                replica_open: Some(true),
                error: Some("status query failed".to_string()),
            }))
        }
    }
}

async fn handle_get_key_origin(ctx: &ClientProtocolContext, key: String) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref peer_manager) = ctx.peer_manager else {
        return Ok(ClientRpcResponse::KeyOriginResult(KeyOriginResultResponse {
            found: false,
            key: key.clone(),
            cluster_id: None,
            priority: None,
            timestamp_secs: None,
            is_local: None,
        }));
    };

    let Some(importer) = peer_manager.importer() else {
        return Ok(ClientRpcResponse::KeyOriginResult(KeyOriginResultResponse {
            found: false,
            key,
            cluster_id: None,
            priority: None,
            timestamp_secs: None,
            is_local: None,
        }));
    };

    match importer.get_key_origin(&key).await {
        Some(origin) => {
            let is_local = origin.is_local();
            Ok(ClientRpcResponse::KeyOriginResult(KeyOriginResultResponse {
                found: true,
                key,
                cluster_id: Some(origin.cluster_id),
                priority: Some(origin.priority),
                timestamp_secs: Some(origin.timestamp_secs),
                is_local: Some(is_local),
            }))
        }
        None => Ok(ClientRpcResponse::KeyOriginResult(KeyOriginResultResponse {
            found: false,
            key,
            cluster_id: None,
            priority: None,
            timestamp_secs: None,
            is_local: None,
        })),
    }
}
