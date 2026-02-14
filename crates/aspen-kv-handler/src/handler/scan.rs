//! Scan handler functions.
//!
//! Handles: ScanKeys.

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::ScanEntry;
use aspen_client_api::ScanResultResponse;
use aspen_core::KeyValueStore;
use aspen_core::ScanRequest;
use aspen_rpc_core::ClientProtocolContext;

use crate::error_sanitization::sanitize_kv_error;

/// Sub-handler for scan operations.
pub(crate) struct ScanHandler;

impl ScanHandler {
    pub(crate) fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(request, ClientRpcRequest::ScanKeys { .. })
    }

    pub(crate) async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::ScanKeys {
                prefix,
                limit,
                continuation_token,
            } => handle_scan_keys(ctx, prefix, limit, continuation_token).await,
            _ => Err(anyhow::anyhow!("request not handled by ScanHandler")),
        }
    }
}

async fn handle_scan_keys(
    ctx: &ClientProtocolContext,
    prefix: String,
    limit: Option<u32>,
    continuation_token: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    let result = ctx
        .kv_store
        .scan(ScanRequest {
            prefix,
            limit,
            continuation_token,
        })
        .await;

    match result {
        Ok(scan_resp) => {
            // Convert from api::KeyValueWithRevision to client_rpc::ScanEntry
            let entries: Vec<ScanEntry> = scan_resp
                .entries
                .into_iter()
                .map(|e| ScanEntry {
                    key: e.key,
                    value: e.value,
                    version: e.version,
                    create_revision: e.create_revision,
                    mod_revision: e.mod_revision,
                })
                .collect();

            Ok(ClientRpcResponse::ScanResult(ScanResultResponse {
                entries,
                count: scan_resp.count,
                is_truncated: scan_resp.is_truncated,
                continuation_token: scan_resp.continuation_token,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::ScanResult(ScanResultResponse {
            entries: vec![],
            count: 0,
            is_truncated: false,
            continuation_token: None,
            // HIGH-4: Sanitize error messages to prevent information leakage
            error: Some(sanitize_kv_error(&e)),
        })),
    }
}
