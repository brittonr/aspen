//! Change application and merge operations.
//!
//! Since the WASM plugin doesn't link the full `automerge` crate,
//! changes are stored as opaque base64 blobs. The plugin manages
//! the change log and lets clients apply changes locally.
//!
//! ## Design
//!
//! - Apply changes: Append change blobs to the document content, storing them as a combined base64
//!   document snapshot sent by the client.
//! - Merge: Combine two documents by concatenating their change sets. The client is responsible for
//!   producing a valid merged Automerge snapshot.

use aspen_client_api::AutomergeApplyChangesResultResponse;
use aspen_client_api::AutomergeMergeResultResponse;
use aspen_client_api::ClientRpcResponse;
use aspen_wasm_guest_sdk::host;
use base64::Engine;

use crate::kv;
use crate::types::DocumentMetadata;
use crate::types::MAX_BATCH_CHANGES;
use crate::types::MAX_CHANGE_SIZE;
use crate::types::MAX_DOCUMENT_SIZE;
use crate::types::content_key;
use crate::types::metadata_key;
use crate::types::validate_document_id;

fn now_ms() -> u64 {
    host::current_time_ms()
}

/// Handle AutomergeApplyChanges request.
///
/// Changes are base64-encoded Automerge change bytes. The plugin stores
/// them by appending to the document blob. The actual Automerge merge
/// logic happens client-side; the plugin provides durable storage.
pub fn handle_apply_changes(document_id: String, changes: Vec<String>) -> ClientRpcResponse {
    if let Err(e) = validate_document_id(&document_id) {
        return ClientRpcResponse::AutomergeApplyChangesResult(AutomergeApplyChangesResultResponse {
            is_success: false,
            changes_applied: false,
            change_count: None,
            new_heads: vec![],
            new_size: None,
            error: Some(format!("Invalid document ID: {e}")),
        });
    }

    if changes.len() > MAX_BATCH_CHANGES {
        return ClientRpcResponse::AutomergeApplyChangesResult(AutomergeApplyChangesResultResponse {
            is_success: false,
            changes_applied: false,
            change_count: None,
            new_heads: vec![],
            new_size: None,
            error: Some(format!("Too many changes: {} (max {})", changes.len(), MAX_BATCH_CHANGES)),
        });
    }

    // Validate individual change sizes
    for (i, change_b64) in changes.iter().enumerate() {
        match base64::engine::general_purpose::STANDARD.decode(change_b64) {
            Ok(bytes) => {
                if bytes.len() > MAX_CHANGE_SIZE {
                    return ClientRpcResponse::AutomergeApplyChangesResult(AutomergeApplyChangesResultResponse {
                        is_success: false,
                        changes_applied: false,
                        change_count: None,
                        new_heads: vec![],
                        new_size: None,
                        error: Some(format!("Change {} too large: {} bytes (max {})", i, bytes.len(), MAX_CHANGE_SIZE)),
                    });
                }
            }
            Err(e) => {
                return ClientRpcResponse::AutomergeApplyChangesResult(AutomergeApplyChangesResultResponse {
                    is_success: false,
                    changes_applied: false,
                    change_count: None,
                    new_heads: vec![],
                    new_size: None,
                    error: Some(format!("Invalid base64 in change {}: {e}", i)),
                });
            }
        }
    }

    let ck = content_key(&document_id);

    // Load existing document
    let existing = match kv::get(&ck) {
        Ok(Some(data)) => data,
        Ok(None) => {
            // Create a new empty document if it doesn't exist
            base64::engine::general_purpose::STANDARD.encode([])
        }
        Err(e) => {
            return ClientRpcResponse::AutomergeApplyChangesResult(AutomergeApplyChangesResultResponse {
                is_success: false,
                changes_applied: false,
                change_count: None,
                new_heads: vec![],
                new_size: None,
                error: Some(e),
            });
        }
    };

    // Decode existing document
    let mut doc_bytes: Vec<u8> = base64::engine::general_purpose::STANDARD.decode(&existing).unwrap_or_default();

    // Append all change bytes
    let change_count = changes.len() as u64;
    for change_b64 in &changes {
        if let Ok(change_bytes) = base64::engine::general_purpose::STANDARD.decode(change_b64) {
            doc_bytes.extend_from_slice(&change_bytes);
        }
    }

    // Check total size
    if doc_bytes.len() > MAX_DOCUMENT_SIZE {
        return ClientRpcResponse::AutomergeApplyChangesResult(AutomergeApplyChangesResultResponse {
            is_success: false,
            changes_applied: false,
            change_count: None,
            new_heads: vec![],
            new_size: None,
            error: Some(format!(
                "Document would exceed maximum size: {} bytes (max {})",
                doc_bytes.len(),
                MAX_DOCUMENT_SIZE
            )),
        });
    }

    let new_size = doc_bytes.len() as u64;

    // Store updated document
    let new_b64 = base64::engine::general_purpose::STANDARD.encode(&doc_bytes);
    if let Err(e) = kv::put(&ck, &new_b64) {
        return ClientRpcResponse::AutomergeApplyChangesResult(AutomergeApplyChangesResultResponse {
            is_success: false,
            changes_applied: false,
            change_count: None,
            new_heads: vec![],
            new_size: None,
            error: Some(e),
        });
    }

    // Update metadata
    let mk = metadata_key(&document_id);
    let now = now_ms();
    if let Ok(Some(json)) = kv::get(&mk)
        && let Ok(mut meta) = DocumentMetadata::from_json(&json)
    {
        meta.updated_at_ms = now;
        meta.size_bytes = new_size;
        meta.change_count += change_count;
        if let Ok(new_json) = meta.to_json() {
            let _ = kv::put(&mk, &new_json);
        }
    }

    ClientRpcResponse::AutomergeApplyChangesResult(AutomergeApplyChangesResultResponse {
        is_success: true,
        changes_applied: change_count > 0,
        change_count: Some(change_count),
        new_heads: vec![],
        new_size: Some(new_size),
        error: None,
    })
}

/// Handle AutomergeMerge request.
///
/// Merges the source document into the target by concatenating their bytes.
/// The client is expected to send a properly merged snapshot via AutomergeSave
/// after this operation.
pub fn handle_merge(target_document_id: String, source_document_id: String) -> ClientRpcResponse {
    if let Err(e) = validate_document_id(&target_document_id) {
        return ClientRpcResponse::AutomergeMergeResult(AutomergeMergeResultResponse {
            is_success: false,
            changes_applied: false,
            change_count: None,
            new_heads: vec![],
            new_size: None,
            error: Some(format!("Invalid target document ID: {e}")),
        });
    }
    if let Err(e) = validate_document_id(&source_document_id) {
        return ClientRpcResponse::AutomergeMergeResult(AutomergeMergeResultResponse {
            is_success: false,
            changes_applied: false,
            change_count: None,
            new_heads: vec![],
            new_size: None,
            error: Some(format!("Invalid source document ID: {e}")),
        });
    }

    // Load both documents
    let target_ck = content_key(&target_document_id);
    let source_ck = content_key(&source_document_id);

    let target_b64 = match kv::get(&target_ck) {
        Ok(Some(data)) => data,
        Ok(None) => {
            return ClientRpcResponse::AutomergeMergeResult(AutomergeMergeResultResponse {
                is_success: false,
                changes_applied: false,
                change_count: None,
                new_heads: vec![],
                new_size: None,
                error: Some(format!("Target document not found: {target_document_id}")),
            });
        }
        Err(e) => {
            return ClientRpcResponse::AutomergeMergeResult(AutomergeMergeResultResponse {
                is_success: false,
                changes_applied: false,
                change_count: None,
                new_heads: vec![],
                new_size: None,
                error: Some(e),
            });
        }
    };

    let source_b64 = match kv::get(&source_ck) {
        Ok(Some(data)) => data,
        Ok(None) => {
            return ClientRpcResponse::AutomergeMergeResult(AutomergeMergeResultResponse {
                is_success: false,
                changes_applied: false,
                change_count: None,
                new_heads: vec![],
                new_size: None,
                error: Some(format!("Source document not found: {source_document_id}")),
            });
        }
        Err(e) => {
            return ClientRpcResponse::AutomergeMergeResult(AutomergeMergeResultResponse {
                is_success: false,
                changes_applied: false,
                change_count: None,
                new_heads: vec![],
                new_size: None,
                error: Some(e),
            });
        }
    };

    // Decode and concatenate
    let mut target_bytes = base64::engine::general_purpose::STANDARD.decode(&target_b64).unwrap_or_default();
    let source_bytes = base64::engine::general_purpose::STANDARD.decode(&source_b64).unwrap_or_default();

    target_bytes.extend_from_slice(&source_bytes);

    if target_bytes.len() > MAX_DOCUMENT_SIZE {
        return ClientRpcResponse::AutomergeMergeResult(AutomergeMergeResultResponse {
            is_success: false,
            changes_applied: false,
            change_count: None,
            new_heads: vec![],
            new_size: None,
            error: Some(format!("Merged document would exceed maximum size: {} bytes", target_bytes.len())),
        });
    }

    let new_size = target_bytes.len() as u64;

    // Store merged document
    let merged_b64 = base64::engine::general_purpose::STANDARD.encode(&target_bytes);
    if let Err(e) = kv::put(&target_ck, &merged_b64) {
        return ClientRpcResponse::AutomergeMergeResult(AutomergeMergeResultResponse {
            is_success: false,
            changes_applied: false,
            change_count: None,
            new_heads: vec![],
            new_size: None,
            error: Some(e),
        });
    }

    // Update target metadata
    let mk = metadata_key(&target_document_id);
    let now = now_ms();
    if let Ok(Some(json)) = kv::get(&mk)
        && let Ok(mut meta) = DocumentMetadata::from_json(&json)
    {
        meta.updated_at_ms = now;
        meta.size_bytes = new_size;
        meta.change_count += 1;
        if let Ok(new_json) = meta.to_json() {
            let _ = kv::put(&mk, &new_json);
        }
    }

    ClientRpcResponse::AutomergeMergeResult(AutomergeMergeResultResponse {
        is_success: true,
        changes_applied: true,
        change_count: Some(1),
        new_heads: vec![],
        new_size: Some(new_size),
        error: None,
    })
}
