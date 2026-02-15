//! Read handler functions.
//!
//! Handles: ReadKey, BatchRead.

use aspen_client_api::BatchReadResultResponse;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::ReadResultResponse;
use aspen_core::KeyValueStore;
use aspen_core::KeyValueStoreError;
use aspen_core::ReadRequest;
use aspen_core::validate_client_key;
use aspen_rpc_core::ClientProtocolContext;

use crate::error_sanitization::sanitize_kv_error;

/// Sub-handler for read operations.
pub(crate) struct ReadHandler;

impl ReadHandler {
    pub(crate) fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(request, ClientRpcRequest::ReadKey { .. } | ClientRpcRequest::BatchRead { .. })
    }

    pub(crate) async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::ReadKey { key } => handle_read_key(ctx, key).await,
            ClientRpcRequest::BatchRead { keys } => handle_batch_read(ctx, keys).await,
            _ => Err(anyhow::anyhow!("request not handled by ReadHandler")),
        }
    }
}

async fn handle_read_key(ctx: &ClientProtocolContext, key: String) -> anyhow::Result<ClientRpcResponse> {
    let result = ctx.kv_store.read(ReadRequest::new(key)).await;

    match result {
        Ok(resp) => {
            let value = resp.kv.map(|kv| kv.value.into_bytes());
            Ok(ClientRpcResponse::ReadResult(ReadResultResponse {
                value,
                found: true,
                error: None,
            }))
        }
        Err(e) => {
            // HIGH-4: Sanitize error messages to prevent information leakage
            let (found, error) = match &e {
                KeyValueStoreError::NotFound { .. } => (false, None),
                other => (false, Some(sanitize_kv_error(other))),
            };
            Ok(ClientRpcResponse::ReadResult(ReadResultResponse {
                value: None,
                found,
                error,
            }))
        }
    }
}

async fn handle_batch_read(ctx: &ClientProtocolContext, keys: Vec<String>) -> anyhow::Result<ClientRpcResponse> {
    debug_assert!(
        keys.len() <= aspen_core::MAX_SETMULTI_KEYS as usize,
        "batch read exceeds MAX_SETMULTI_KEYS: {}",
        keys.len()
    );

    // Validate all keys
    for key in &keys {
        if let Err(e) = validate_client_key(key) {
            return Ok(ClientRpcResponse::BatchReadResult(BatchReadResultResponse {
                success: false,
                values: None,
                error: Some(e.to_string()),
            }));
        }
    }

    // Read all keys atomically
    let mut values = Vec::with_capacity(keys.len());
    for key in &keys {
        let request = ReadRequest::new(key.clone());
        match ctx.kv_store.read(request).await {
            Ok(result) => {
                // Key exists - return the value
                let value_bytes = result.kv.map(|kv| kv.value.into_bytes());
                values.push(value_bytes);
            }
            Err(KeyValueStoreError::NotFound { .. }) => {
                // Key doesn't exist - return None for this position
                values.push(None);
            }
            Err(e) => {
                // Real error - fail the entire batch
                return Ok(ClientRpcResponse::BatchReadResult(BatchReadResultResponse {
                    success: false,
                    values: None,
                    error: Some(e.to_string()),
                }));
            }
        }
    }

    debug_assert_eq!(
        values.len(),
        keys.len(),
        "batch read result count mismatch: got {} values for {} keys",
        values.len(),
        keys.len()
    );

    Ok(ClientRpcResponse::BatchReadResult(BatchReadResultResponse {
        success: true,
        values: Some(values),
        error: None,
    }))
}
