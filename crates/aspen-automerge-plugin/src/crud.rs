//! Document CRUD operations: create, get, save, delete.

use aspen_client_api::AutomergeCreateResultResponse;
use aspen_client_api::AutomergeDeleteResultResponse;
use aspen_client_api::AutomergeGetResultResponse;
use aspen_client_api::AutomergeSaveResultResponse;
use aspen_client_api::ClientRpcResponse;
use aspen_wasm_guest_sdk::host;
use base64::Engine;

use crate::kv;
use crate::types::DocumentMetadata;
use crate::types::MAX_DOCUMENT_SIZE;
use crate::types::content_key;
use crate::types::generate_doc_id;
use crate::types::metadata_key;
use crate::types::validate_document_id;

fn now_ms() -> u64 {
    host::current_time_ms()
}

fn convert_metadata(meta: &DocumentMetadata) -> aspen_client_api::AutomergeDocumentMetadata {
    aspen_client_api::AutomergeDocumentMetadata {
        document_id: meta.document_id.clone(),
        namespace: meta.namespace.clone(),
        title: meta.title.clone(),
        description: meta.description.clone(),
        created_at_ms: meta.created_at_ms,
        updated_at_ms: meta.updated_at_ms,
        size_bytes: meta.size_bytes,
        change_count: meta.change_count,
        heads: meta.heads.clone(),
        creator_actor_id: meta.creator_actor_id.clone(),
        tags: meta.tags.clone(),
    }
}

/// Handle AutomergeCreate request.
pub fn handle_create(
    document_id: Option<String>,
    namespace: Option<String>,
    title: Option<String>,
    description: Option<String>,
    tags: Vec<String>,
) -> ClientRpcResponse {
    // Generate or validate document ID
    let doc_id = match document_id {
        Some(id) => {
            if let Err(e) = validate_document_id(&id) {
                return ClientRpcResponse::AutomergeCreateResult(AutomergeCreateResultResponse {
                    is_success: false,
                    document_id: None,
                    error: Some(format!("Invalid document ID: {e}")),
                });
            }
            id
        }
        None => generate_doc_id(),
    };

    // Check if document already exists
    let ck = content_key(&doc_id);
    match kv::get(&ck) {
        Ok(Some(_)) => {
            return ClientRpcResponse::AutomergeCreateResult(AutomergeCreateResultResponse {
                is_success: false,
                document_id: Some(doc_id.clone()),
                error: Some(format!("Document already exists: {doc_id}")),
            });
        }
        Err(e) => {
            return ClientRpcResponse::AutomergeCreateResult(AutomergeCreateResultResponse {
                is_success: false,
                document_id: None,
                error: Some(e),
            });
        }
        Ok(None) => {}
    }

    let now = now_ms();

    // Create empty document content (base64-encoded empty bytes)
    // For the WASM plugin, we store a minimal marker that represents an empty
    // Automerge document. The actual Automerge library on the client will
    // initialize the proper binary format.
    let empty_doc = base64::engine::general_purpose::STANDARD.encode([]);

    // Create metadata
    let mut meta = DocumentMetadata::new(doc_id.clone(), now);
    meta.namespace = namespace;
    meta.title = title;
    meta.description = description;
    meta.tags = tags;
    meta.size_bytes = 0;

    // Store content and metadata
    if let Err(e) = kv::put(&ck, &empty_doc) {
        return ClientRpcResponse::AutomergeCreateResult(AutomergeCreateResultResponse {
            is_success: false,
            document_id: None,
            error: Some(e),
        });
    }

    let mk = metadata_key(&doc_id);
    let meta_json = match meta.to_json() {
        Ok(j) => j,
        Err(e) => {
            return ClientRpcResponse::AutomergeCreateResult(AutomergeCreateResultResponse {
                is_success: false,
                document_id: None,
                error: Some(e),
            });
        }
    };

    if let Err(e) = kv::put(&mk, &meta_json) {
        // Clean up content key on metadata write failure
        let _ = kv::delete(&ck);
        return ClientRpcResponse::AutomergeCreateResult(AutomergeCreateResultResponse {
            is_success: false,
            document_id: None,
            error: Some(e),
        });
    }

    ClientRpcResponse::AutomergeCreateResult(AutomergeCreateResultResponse {
        is_success: true,
        document_id: Some(doc_id),
        error: None,
    })
}

/// Handle AutomergeGet request.
pub fn handle_get(document_id: String) -> ClientRpcResponse {
    if let Err(e) = validate_document_id(&document_id) {
        return ClientRpcResponse::AutomergeGetResult(AutomergeGetResultResponse {
            is_success: false,
            was_found: false,
            document_id: None,
            document_bytes: None,
            metadata: None,
            error: Some(format!("Invalid document ID: {e}")),
        });
    }

    let ck = content_key(&document_id);
    match kv::get(&ck) {
        Ok(Some(doc_b64)) => {
            // Load metadata
            let mk = metadata_key(&document_id);
            let metadata = kv::get(&mk)
                .ok()
                .flatten()
                .and_then(|json| DocumentMetadata::from_json(&json).ok())
                .map(|m| convert_metadata(&m));

            ClientRpcResponse::AutomergeGetResult(AutomergeGetResultResponse {
                is_success: true,
                was_found: true,
                document_id: Some(document_id),
                document_bytes: Some(doc_b64),
                metadata,
                error: None,
            })
        }
        Ok(None) => ClientRpcResponse::AutomergeGetResult(AutomergeGetResultResponse {
            is_success: true,
            was_found: false,
            document_id: Some(document_id),
            document_bytes: None,
            metadata: None,
            error: None,
        }),
        Err(e) => ClientRpcResponse::AutomergeGetResult(AutomergeGetResultResponse {
            is_success: false,
            was_found: false,
            document_id: Some(document_id),
            document_bytes: None,
            metadata: None,
            error: Some(e),
        }),
    }
}

/// Handle AutomergeSave request.
pub fn handle_save(document_id: String, document_bytes: String) -> ClientRpcResponse {
    if let Err(e) = validate_document_id(&document_id) {
        return ClientRpcResponse::AutomergeSaveResult(AutomergeSaveResultResponse {
            is_success: false,
            size_bytes: None,
            change_count: None,
            error: Some(format!("Invalid document ID: {e}")),
        });
    }

    // Validate base64 and size
    let decoded = match base64::engine::general_purpose::STANDARD.decode(&document_bytes) {
        Ok(b) => b,
        Err(e) => {
            return ClientRpcResponse::AutomergeSaveResult(AutomergeSaveResultResponse {
                is_success: false,
                size_bytes: None,
                change_count: None,
                error: Some(format!("Invalid base64 document bytes: {e}")),
            });
        }
    };

    if decoded.len() > MAX_DOCUMENT_SIZE {
        return ClientRpcResponse::AutomergeSaveResult(AutomergeSaveResultResponse {
            is_success: false,
            size_bytes: None,
            change_count: None,
            error: Some(format!("Document too large: {} bytes (max {})", decoded.len(), MAX_DOCUMENT_SIZE)),
        });
    }

    let size = decoded.len() as u64;

    // Store document content
    let ck = content_key(&document_id);
    if let Err(e) = kv::put(&ck, &document_bytes) {
        return ClientRpcResponse::AutomergeSaveResult(AutomergeSaveResultResponse {
            is_success: false,
            size_bytes: None,
            change_count: None,
            error: Some(e),
        });
    }

    // Update metadata
    let mk = metadata_key(&document_id);
    let now = now_ms();
    let mut meta = kv::get(&mk)
        .ok()
        .flatten()
        .and_then(|json| DocumentMetadata::from_json(&json).ok())
        .unwrap_or_else(|| DocumentMetadata::new(document_id.clone(), now));

    meta.updated_at_ms = now;
    meta.size_bytes = size;

    if let Ok(json) = meta.to_json() {
        let _ = kv::put(&mk, &json);
    }

    ClientRpcResponse::AutomergeSaveResult(AutomergeSaveResultResponse {
        is_success: true,
        size_bytes: Some(size),
        change_count: None,
        error: None,
    })
}

/// Handle AutomergeDelete request.
pub fn handle_delete(document_id: String) -> ClientRpcResponse {
    if let Err(e) = validate_document_id(&document_id) {
        return ClientRpcResponse::AutomergeDeleteResult(AutomergeDeleteResultResponse {
            is_success: false,
            existed: false,
            error: Some(format!("Invalid document ID: {e}")),
        });
    }

    let ck = content_key(&document_id);
    let existed = matches!(kv::get(&ck), Ok(Some(_)));

    // Delete content and metadata
    let _ = kv::delete(&ck);
    let mk = metadata_key(&document_id);
    let _ = kv::delete(&mk);

    ClientRpcResponse::AutomergeDeleteResult(AutomergeDeleteResultResponse {
        is_success: true,
        existed,
        error: None,
    })
}
