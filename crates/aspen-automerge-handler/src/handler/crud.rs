//! Document CRUD operations: create, get, save, delete.

use aspen_automerge::DocumentId;
use aspen_automerge::DocumentStore;
use aspen_client_api::AutomergeCreateResultResponse;
use aspen_client_api::AutomergeDeleteResultResponse;
use aspen_client_api::AutomergeGetResultResponse;
use aspen_client_api::AutomergeSaveResultResponse;
use aspen_client_api::ClientRpcResponse;
use automerge::AutoCommit;
use base64::Engine;

use super::DynAutomergeStore;
use super::helpers::convert_metadata;

/// Handle AutomergeCreate request.
pub(crate) async fn handle_create(
    store: &DynAutomergeStore,
    document_id: Option<String>,
    namespace: Option<String>,
    title: Option<String>,
    description: Option<String>,
    tags: Vec<String>,
) -> anyhow::Result<ClientRpcResponse> {
    // Parse document ID if provided
    let id = match document_id {
        Some(id_str) => match DocumentId::from_string(&id_str) {
            Ok(id) => Some(id),
            Err(e) => {
                return Ok(ClientRpcResponse::AutomergeCreateResult(AutomergeCreateResultResponse {
                    is_success: false,
                    document_id: None,
                    error: Some(format!("Invalid document ID: {}", e)),
                }));
            }
        },
        None => None,
    };

    // Build metadata
    let mut metadata = aspen_automerge::DocumentMetadata::new(id.clone().unwrap_or_default());
    if let Some(ns) = namespace {
        metadata = metadata.with_namespace(ns);
    }
    if let Some(t) = title {
        metadata = metadata.with_title(t);
    }
    if let Some(d) = description {
        metadata = metadata.with_description(d);
    }
    for tag in tags {
        metadata = metadata.with_tag(tag);
    }

    match store.create(id, Some(metadata)).await {
        Ok(created_id) => Ok(ClientRpcResponse::AutomergeCreateResult(AutomergeCreateResultResponse {
            is_success: true,
            document_id: Some(created_id.to_string()),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::AutomergeCreateResult(AutomergeCreateResultResponse {
            is_success: false,
            document_id: None,
            error: Some(e.to_string()),
        })),
    }
}

/// Handle AutomergeGet request.
pub(crate) async fn handle_get(store: &DynAutomergeStore, document_id: String) -> anyhow::Result<ClientRpcResponse> {
    let id = match DocumentId::from_string(&document_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::AutomergeGetResult(AutomergeGetResultResponse {
                is_success: false,
                was_found: false,
                document_id: None,
                document_bytes: None,
                metadata: None,
                error: Some(format!("Invalid document ID: {}", e)),
            }));
        }
    };

    match store.get(&id).await {
        Ok(Some(mut doc)) => {
            // Serialize document to bytes
            let bytes = doc.save();
            let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);

            // Get metadata
            let metadata = store.get_metadata(&id).await.ok().flatten().map(convert_metadata);

            Ok(ClientRpcResponse::AutomergeGetResult(AutomergeGetResultResponse {
                is_success: true,
                was_found: true,
                document_id: Some(document_id),
                document_bytes: Some(encoded),
                metadata,
                error: None,
            }))
        }
        Ok(None) => Ok(ClientRpcResponse::AutomergeGetResult(AutomergeGetResultResponse {
            is_success: true,
            was_found: false,
            document_id: Some(document_id),
            document_bytes: None,
            metadata: None,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::AutomergeGetResult(AutomergeGetResultResponse {
            is_success: false,
            was_found: false,
            document_id: Some(document_id),
            document_bytes: None,
            metadata: None,
            error: Some(e.to_string()),
        })),
    }
}

/// Handle AutomergeSave request.
pub(crate) async fn handle_save(
    store: &DynAutomergeStore,
    document_id: String,
    document_bytes: String,
) -> anyhow::Result<ClientRpcResponse> {
    let id = match DocumentId::from_string(&document_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::AutomergeSaveResult(AutomergeSaveResultResponse {
                is_success: false,
                size_bytes: None,
                change_count: None,
                error: Some(format!("Invalid document ID: {}", e)),
            }));
        }
    };

    // Decode base64
    let bytes = match base64::engine::general_purpose::STANDARD.decode(&document_bytes) {
        Ok(b) => b,
        Err(e) => {
            return Ok(ClientRpcResponse::AutomergeSaveResult(AutomergeSaveResultResponse {
                is_success: false,
                size_bytes: None,
                change_count: None,
                error: Some(format!("Invalid base64 document bytes: {}", e)),
            }));
        }
    };

    // Load document from bytes
    let mut doc = match AutoCommit::load(&bytes) {
        Ok(d) => d,
        Err(e) => {
            return Ok(ClientRpcResponse::AutomergeSaveResult(AutomergeSaveResultResponse {
                is_success: false,
                size_bytes: None,
                change_count: None,
                error: Some(format!("Invalid Automerge document: {}", e)),
            }));
        }
    };

    match store.save(&id, &mut doc).await {
        Ok(()) => {
            let saved_bytes = doc.save();
            let change_count = doc.get_changes(&[]).len() as u64;

            Ok(ClientRpcResponse::AutomergeSaveResult(AutomergeSaveResultResponse {
                is_success: true,
                size_bytes: Some(saved_bytes.len() as u64),
                change_count: Some(change_count),
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::AutomergeSaveResult(AutomergeSaveResultResponse {
            is_success: false,
            size_bytes: None,
            change_count: None,
            error: Some(e.to_string()),
        })),
    }
}

/// Handle AutomergeDelete request.
pub(crate) async fn handle_delete(store: &DynAutomergeStore, document_id: String) -> anyhow::Result<ClientRpcResponse> {
    let id = match DocumentId::from_string(&document_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::AutomergeDeleteResult(AutomergeDeleteResultResponse {
                is_success: false,
                existed: false,
                error: Some(format!("Invalid document ID: {}", e)),
            }));
        }
    };

    match store.delete(&id).await {
        Ok(existed) => Ok(ClientRpcResponse::AutomergeDeleteResult(AutomergeDeleteResultResponse {
            is_success: true,
            existed,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::AutomergeDeleteResult(AutomergeDeleteResultResponse {
            is_success: false,
            existed: false,
            error: Some(e.to_string()),
        })),
    }
}
