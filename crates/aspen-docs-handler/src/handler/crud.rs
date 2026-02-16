//! Docs CRUD handler functions.
//!
//! Handles: DocsSet, DocsGet, DocsDelete, DocsList.

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::DocsDeleteResultResponse;
use aspen_client_api::DocsGetResultResponse;
use aspen_client_api::DocsListEntry;
use aspen_client_api::DocsListResultResponse;
use aspen_client_api::DocsSetResultResponse;
use aspen_rpc_core::ClientProtocolContext;
use tracing::warn;

/// Sub-handler for docs CRUD operations.
pub(crate) struct CrudHandler;

impl CrudHandler {
    pub(crate) fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::DocsSet { .. }
                | ClientRpcRequest::DocsGet { .. }
                | ClientRpcRequest::DocsDelete { .. }
                | ClientRpcRequest::DocsList { .. }
        )
    }

    pub(crate) async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::DocsSet { key, value } => handle_docs_set(ctx, key, value).await,
            ClientRpcRequest::DocsGet { key } => handle_docs_get(ctx, key).await,
            ClientRpcRequest::DocsDelete { key } => handle_docs_delete(ctx, key).await,
            ClientRpcRequest::DocsList { prefix, limit } => handle_docs_list(ctx, prefix, limit).await,
            _ => Err(anyhow::anyhow!("request not handled by CrudHandler")),
        }
    }
}

async fn handle_docs_set(
    ctx: &ClientProtocolContext,
    key: String,
    value: Vec<u8>,
) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref docs_sync) = ctx.docs_sync else {
        return Ok(ClientRpcResponse::DocsSetResult(DocsSetResultResponse {
            is_success: false,
            key: None,
            size: None,
            error: Some("docs not enabled".to_string()),
        }));
    };

    let value_len = value.len() as u64;
    match docs_sync.set_entry(key.as_bytes().to_vec(), value).await {
        Ok(()) => Ok(ClientRpcResponse::DocsSetResult(DocsSetResultResponse {
            is_success: true,
            key: Some(key),
            size: Some(value_len),
            error: None,
        })),
        Err(e) => {
            warn!(key = %key, error = %e, "docs set failed");
            Ok(ClientRpcResponse::DocsSetResult(DocsSetResultResponse {
                is_success: false,
                key: Some(key),
                size: None,
                error: Some("docs operation failed".to_string()),
            }))
        }
    }
}

async fn handle_docs_get(ctx: &ClientProtocolContext, key: String) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref docs_sync) = ctx.docs_sync else {
        return Ok(ClientRpcResponse::DocsGetResult(DocsGetResultResponse {
            was_found: false,
            value: None,
            size: None,
            error: Some("docs not enabled".to_string()),
        }));
    };

    match docs_sync.get_entry(key.as_bytes()).await {
        Ok(Some((value, size, _hash))) => Ok(ClientRpcResponse::DocsGetResult(DocsGetResultResponse {
            was_found: true,
            value: Some(value),
            size: Some(size),
            error: None,
        })),
        Ok(None) => Ok(ClientRpcResponse::DocsGetResult(DocsGetResultResponse {
            was_found: false,
            value: None,
            size: None,
            error: None,
        })),
        Err(e) => {
            warn!(key = %key, error = %e, "docs get failed");
            Ok(ClientRpcResponse::DocsGetResult(DocsGetResultResponse {
                was_found: false,
                value: None,
                size: None,
                error: Some("docs operation failed".to_string()),
            }))
        }
    }
}

async fn handle_docs_delete(ctx: &ClientProtocolContext, key: String) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref docs_sync) = ctx.docs_sync else {
        return Ok(ClientRpcResponse::DocsDeleteResult(DocsDeleteResultResponse {
            is_success: false,
            error: Some("docs not enabled".to_string()),
        }));
    };

    match docs_sync.delete_entry(key.as_bytes().to_vec()).await {
        Ok(()) => Ok(ClientRpcResponse::DocsDeleteResult(DocsDeleteResultResponse {
            is_success: true,
            error: None,
        })),
        Err(e) => {
            warn!(key = %key, error = %e, "docs delete failed");
            Ok(ClientRpcResponse::DocsDeleteResult(DocsDeleteResultResponse {
                is_success: false,
                error: Some("docs operation failed".to_string()),
            }))
        }
    }
}

async fn handle_docs_list(
    ctx: &ClientProtocolContext,
    prefix: Option<String>,
    limit: Option<u32>,
) -> anyhow::Result<ClientRpcResponse> {
    let Some(ref docs_sync) = ctx.docs_sync else {
        return Ok(ClientRpcResponse::DocsListResult(DocsListResultResponse {
            entries: vec![],
            count: 0,
            has_more: false,
            error: Some("docs not enabled".to_string()),
        }));
    };

    match docs_sync.list_entries(prefix, limit).await {
        Ok(entries) => {
            let max_entries = limit.unwrap_or(100) as usize;
            let has_more = entries.len() > max_entries;
            let mut result_entries = entries;
            if has_more {
                result_entries.pop(); // Remove the extra entry used for has_more detection
            }

            let count = result_entries.len() as u32;
            let docs_entries = result_entries
                .into_iter()
                .map(|entry| DocsListEntry {
                    key: entry.key,
                    size_bytes: entry.size_bytes,
                    hash: entry.hash,
                })
                .collect();

            Ok(ClientRpcResponse::DocsListResult(DocsListResultResponse {
                entries: docs_entries,
                count,
                has_more,
                error: None,
            }))
        }
        Err(e) => {
            warn!(error = %e, "docs list failed");
            Ok(ClientRpcResponse::DocsListResult(DocsListResultResponse {
                entries: vec![],
                count: 0,
                has_more: false,
                error: Some("docs list operation failed".to_string()),
            }))
        }
    }
}
