//! Delete handler functions.
//!
//! Handles: DeleteKey.

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::DeleteResultResponse;
use aspen_core::KeyValueStore;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;
use aspen_core::validate_client_key;
use aspen_rpc_core::ClientProtocolContext;

use crate::error_sanitization::sanitize_kv_error;

/// Sub-handler for delete operations.
pub(crate) struct DeleteHandler;

impl DeleteHandler {
    pub(crate) fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(request, ClientRpcRequest::DeleteKey { .. })
    }

    pub(crate) async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::DeleteKey { key } => handle_delete_key(ctx, key).await,
            _ => Err(anyhow::anyhow!("request not handled by DeleteHandler")),
        }
    }
}

async fn handle_delete_key(ctx: &ClientProtocolContext, key: String) -> anyhow::Result<ClientRpcResponse> {
    // Validate key against reserved _system: prefix
    if let Err(vault_err) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::DeleteResult(DeleteResultResponse {
            key,
            was_deleted: false,
            error: Some(vault_err.to_string()),
        }));
    }

    let result = ctx
        .kv_store
        .write(WriteRequest {
            command: WriteCommand::Delete { key: key.clone() },
        })
        .await;

    match result {
        Ok(_) => Ok(ClientRpcResponse::DeleteResult(DeleteResultResponse {
            key,
            was_deleted: true,
            error: None,
        })),
        Err(aspen_core::KeyValueStoreError::NotLeader { leader, .. }) => {
            let msg = if let Some(id) = leader {
                format!("not leader; leader is node {}", id)
            } else {
                "not leader; leader unknown".to_string()
            };
            Ok(ClientRpcResponse::error("NOT_LEADER", msg))
        }
        Err(e) => Ok(ClientRpcResponse::DeleteResult(DeleteResultResponse {
            key,
            was_deleted: false,
            error: Some(sanitize_kv_error(&e)),
        })),
    }
}
