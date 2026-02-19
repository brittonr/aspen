//! Write handler functions.
//!
//! Handles: WriteKey, BatchWrite.

use aspen_client_api::BatchWriteOperation;
use aspen_client_api::BatchWriteResultResponse;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::WriteResultResponse;
use aspen_core::BatchOperation;
use aspen_core::KeyValueStore;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;
use aspen_core::validate_client_key;
use aspen_rpc_core::ClientProtocolContext;

use crate::error_sanitization::sanitize_kv_error;
use crate::verified::bytes_to_string_lossy;

/// Sub-handler for write operations.
pub(crate) struct WriteHandler;

impl WriteHandler {
    pub(crate) fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(request, ClientRpcRequest::WriteKey { .. } | ClientRpcRequest::BatchWrite { .. })
    }

    pub(crate) async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::WriteKey { key, value } => handle_write_key(ctx, key, value).await,
            ClientRpcRequest::BatchWrite { operations } => handle_batch_write(ctx, operations).await,
            _ => Err(anyhow::anyhow!("request not handled by WriteHandler")),
        }
    }
}

async fn handle_write_key(
    ctx: &ClientProtocolContext,
    key: String,
    value: Vec<u8>,
) -> anyhow::Result<ClientRpcResponse> {
    // Validate key against reserved _system: prefix
    if let Err(vault_err) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::WriteResult(WriteResultResponse {
            is_success: false,
            error: Some(vault_err.to_string()),
        }));
    }

    let result = ctx
        .kv_store
        .write(WriteRequest {
            command: WriteCommand::Set {
                key,
                value: bytes_to_string_lossy(&value),
            },
        })
        .await;

    match result {
        Ok(_) => Ok(ClientRpcResponse::WriteResult(WriteResultResponse {
            is_success: true,
            error: None,
        })),
        // Surface NOT_LEADER as a top-level error code so the client can
        // detect it and rotate to the current Raft leader.
        Err(aspen_core::KeyValueStoreError::NotLeader { leader, .. }) => {
            let msg = if let Some(id) = leader {
                format!("not leader; leader is node {}", id)
            } else {
                "not leader; leader unknown".to_string()
            };
            Ok(ClientRpcResponse::error("NOT_LEADER", msg))
        }
        Err(e) => Ok(ClientRpcResponse::WriteResult(WriteResultResponse {
            is_success: false,
            error: Some(sanitize_kv_error(&e)),
        })),
    }
}

async fn handle_batch_write(
    ctx: &ClientProtocolContext,
    operations: Vec<BatchWriteOperation>,
) -> anyhow::Result<ClientRpcResponse> {
    debug_assert!(
        operations.len() <= aspen_core::MAX_SETMULTI_KEYS as usize,
        "batch write exceeds MAX_SETMULTI_KEYS: {}",
        operations.len()
    );

    // Validate all keys
    for op in &operations {
        let key = match op {
            BatchWriteOperation::Set { key, .. } => key,
            BatchWriteOperation::Delete { key } => key,
        };
        if let Err(e) = validate_client_key(key) {
            return Ok(ClientRpcResponse::BatchWriteResult(BatchWriteResultResponse {
                is_success: false,
                operations_applied: None,
                error: Some(e.to_string()),
            }));
        }
    }

    // Convert to internal batch operations
    let batch_ops: Vec<BatchOperation> = operations
        .iter()
        .map(|op| match op {
            BatchWriteOperation::Set { key, value } => BatchOperation::Set {
                key: key.clone(),
                value: bytes_to_string_lossy(value),
            },
            BatchWriteOperation::Delete { key } => BatchOperation::Delete { key: key.clone() },
        })
        .collect();

    let request = WriteRequest {
        command: WriteCommand::Batch { operations: batch_ops },
    };

    match ctx.kv_store.write(request).await {
        Ok(result) => Ok(ClientRpcResponse::BatchWriteResult(BatchWriteResultResponse {
            is_success: true,
            operations_applied: result.batch_applied,
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
        Err(e) => Ok(ClientRpcResponse::BatchWriteResult(BatchWriteResultResponse {
            is_success: false,
            operations_applied: None,
            error: Some(sanitize_kv_error(&e)),
        })),
    }
}
