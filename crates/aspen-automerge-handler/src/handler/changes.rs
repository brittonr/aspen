//! Change operations: apply_changes, merge.

use aspen_automerge::DocumentChange;
use aspen_automerge::DocumentId;
use aspen_automerge::DocumentStore;
use aspen_client_api::AutomergeApplyChangesResultResponse;
use aspen_client_api::AutomergeMergeResultResponse;
use aspen_client_api::ClientRpcResponse;
use base64::Engine;

use super::DynAutomergeStore;

/// Handle AutomergeApplyChanges request.
pub(crate) async fn handle_apply_changes(
    store: &DynAutomergeStore,
    document_id: String,
    changes: Vec<String>,
) -> anyhow::Result<ClientRpcResponse> {
    let id = match DocumentId::from_string(&document_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::AutomergeApplyChangesResult(AutomergeApplyChangesResultResponse {
                success: false,
                changes_applied: false,
                change_count: None,
                new_heads: vec![],
                new_size: None,
                error: Some(format!("Invalid document ID: {}", e)),
            }));
        }
    };

    // Decode changes from base64
    let mut decoded_changes = Vec::with_capacity(changes.len());
    for (i, change) in changes.iter().enumerate() {
        match base64::engine::general_purpose::STANDARD.decode(change) {
            Ok(bytes) => decoded_changes.push(DocumentChange::new(bytes)),
            Err(e) => {
                return Ok(ClientRpcResponse::AutomergeApplyChangesResult(AutomergeApplyChangesResultResponse {
                    success: false,
                    changes_applied: false,
                    change_count: None,
                    new_heads: vec![],
                    new_size: None,
                    error: Some(format!("Invalid base64 for change {}: {}", i, e)),
                }));
            }
        }
    }

    match store.apply_changes(&id, decoded_changes).await {
        Ok(result) => Ok(ClientRpcResponse::AutomergeApplyChangesResult(AutomergeApplyChangesResultResponse {
            success: true,
            changes_applied: result.changes_applied,
            change_count: Some(result.change_count as u64),
            new_heads: result.new_heads,
            new_size: Some(result.new_size),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::AutomergeApplyChangesResult(AutomergeApplyChangesResultResponse {
            success: false,
            changes_applied: false,
            change_count: None,
            new_heads: vec![],
            new_size: None,
            error: Some(e.to_string()),
        })),
    }
}

/// Handle AutomergeMerge request.
pub(crate) async fn handle_merge(
    store: &DynAutomergeStore,
    target_document_id: String,
    source_document_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    let target_id = match DocumentId::from_string(&target_document_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::AutomergeMergeResult(AutomergeMergeResultResponse {
                success: false,
                changes_applied: false,
                change_count: None,
                new_heads: vec![],
                new_size: None,
                error: Some(format!("Invalid target document ID: {}", e)),
            }));
        }
    };

    let source_id = match DocumentId::from_string(&source_document_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::AutomergeMergeResult(AutomergeMergeResultResponse {
                success: false,
                changes_applied: false,
                change_count: None,
                new_heads: vec![],
                new_size: None,
                error: Some(format!("Invalid source document ID: {}", e)),
            }));
        }
    };

    match store.merge(&target_id, &source_id).await {
        Ok(result) => Ok(ClientRpcResponse::AutomergeMergeResult(AutomergeMergeResultResponse {
            success: true,
            changes_applied: result.changes_applied,
            change_count: Some(result.change_count as u64),
            new_heads: result.new_heads,
            new_size: Some(result.new_size),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::AutomergeMergeResult(AutomergeMergeResultResponse {
            success: false,
            changes_applied: false,
            change_count: None,
            new_heads: vec![],
            new_size: None,
            error: Some(e.to_string()),
        })),
    }
}
