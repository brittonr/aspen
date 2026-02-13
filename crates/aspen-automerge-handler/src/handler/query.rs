//! Query operations: list, get_metadata, exists.

use aspen_automerge::DocumentId;
use aspen_automerge::DocumentStore;
use aspen_automerge::ListOptions;
use aspen_client_api::AutomergeExistsResultResponse;
use aspen_client_api::AutomergeGetMetadataResultResponse;
use aspen_client_api::AutomergeListResultResponse;
use aspen_client_api::ClientRpcResponse;

use super::DynAutomergeStore;
use super::helpers::convert_metadata;

/// Handle AutomergeList request.
pub(crate) async fn handle_list(
    store: &DynAutomergeStore,
    namespace: Option<String>,
    tag: Option<String>,
    limit: Option<u32>,
    continuation_token: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    let options = ListOptions {
        namespace,
        tag,
        limit,
        continuation_token,
    };

    match store.list(options).await {
        Ok(result) => Ok(ClientRpcResponse::AutomergeListResult(AutomergeListResultResponse {
            success: true,
            documents: result.documents.into_iter().map(convert_metadata).collect(),
            has_more: result.has_more,
            continuation_token: result.continuation_token,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::AutomergeListResult(AutomergeListResultResponse {
            success: false,
            documents: vec![],
            has_more: false,
            continuation_token: None,
            error: Some(e.to_string()),
        })),
    }
}

/// Handle AutomergeGetMetadata request.
pub(crate) async fn handle_get_metadata(
    store: &DynAutomergeStore,
    document_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    let id = match DocumentId::from_string(&document_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::AutomergeGetMetadataResult(AutomergeGetMetadataResultResponse {
                success: false,
                found: false,
                metadata: None,
                error: Some(format!("Invalid document ID: {}", e)),
            }));
        }
    };

    match store.get_metadata(&id).await {
        Ok(Some(metadata)) => Ok(ClientRpcResponse::AutomergeGetMetadataResult(AutomergeGetMetadataResultResponse {
            success: true,
            found: true,
            metadata: Some(convert_metadata(metadata)),
            error: None,
        })),
        Ok(None) => Ok(ClientRpcResponse::AutomergeGetMetadataResult(AutomergeGetMetadataResultResponse {
            success: true,
            found: false,
            metadata: None,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::AutomergeGetMetadataResult(AutomergeGetMetadataResultResponse {
            success: false,
            found: false,
            metadata: None,
            error: Some(e.to_string()),
        })),
    }
}

/// Handle AutomergeExists request.
pub(crate) async fn handle_exists(store: &DynAutomergeStore, document_id: String) -> anyhow::Result<ClientRpcResponse> {
    let id = match DocumentId::from_string(&document_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::AutomergeExistsResult(AutomergeExistsResultResponse {
                success: false,
                exists: false,
                error: Some(format!("Invalid document ID: {}", e)),
            }));
        }
    };

    match store.exists(&id).await {
        Ok(exists) => Ok(ClientRpcResponse::AutomergeExistsResult(AutomergeExistsResultResponse {
            success: true,
            exists,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::AutomergeExistsResult(AutomergeExistsResultResponse {
            success: false,
            exists: false,
            error: Some(e.to_string()),
        })),
    }
}
