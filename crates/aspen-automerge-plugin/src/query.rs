//! Query operations: list, get metadata, exists.

use aspen_client_api::AutomergeExistsResultResponse;
use aspen_client_api::AutomergeGetMetadataResultResponse;
use aspen_client_api::AutomergeListResultResponse;
use aspen_client_api::ClientRpcResponse;

use crate::kv;
use crate::types::DEFAULT_LIST_LIMIT;
use crate::types::DOC_META_PREFIX;
use crate::types::DocumentMetadata;
use crate::types::MAX_SCAN_RESULTS;
use crate::types::content_key;
use crate::types::doc_id_from_metadata_key;
use crate::types::metadata_key;
use crate::types::validate_document_id;

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

/// Handle AutomergeList request.
pub fn handle_list(
    namespace: Option<String>,
    tag: Option<String>,
    limit: Option<u32>,
    _continuation_token: Option<String>,
) -> ClientRpcResponse {
    let scan_limit = limit.unwrap_or(DEFAULT_LIST_LIMIT).min(MAX_SCAN_RESULTS);

    match kv::scan(DOC_META_PREFIX, scan_limit) {
        Ok(entries) => {
            let mut documents = Vec::new();

            for (key, value) in &entries {
                // Skip entries that don't have a valid doc ID
                if doc_id_from_metadata_key(key).is_none() {
                    continue;
                }

                if let Ok(meta) = DocumentMetadata::from_json(value) {
                    // Apply namespace filter
                    if let Some(ref ns) = namespace
                        && meta.namespace.as_ref() != Some(ns)
                    {
                        continue;
                    }

                    // Apply tag filter
                    if let Some(ref t) = tag
                        && !meta.tags.contains(t)
                    {
                        continue;
                    }

                    documents.push(convert_metadata(&meta));
                }
            }

            ClientRpcResponse::AutomergeListResult(AutomergeListResultResponse {
                is_success: true,
                documents,
                has_more: false,
                continuation_token: None,
                error: None,
            })
        }
        Err(e) => ClientRpcResponse::AutomergeListResult(AutomergeListResultResponse {
            is_success: false,
            documents: vec![],
            has_more: false,
            continuation_token: None,
            error: Some(e),
        }),
    }
}

/// Handle AutomergeGetMetadata request.
pub fn handle_get_metadata(document_id: String) -> ClientRpcResponse {
    if let Err(e) = validate_document_id(&document_id) {
        return ClientRpcResponse::AutomergeGetMetadataResult(AutomergeGetMetadataResultResponse {
            is_success: false,
            was_found: false,
            metadata: None,
            error: Some(format!("Invalid document ID: {e}")),
        });
    }

    let mk = metadata_key(&document_id);
    match kv::get(&mk) {
        Ok(Some(json)) => match DocumentMetadata::from_json(&json) {
            Ok(meta) => ClientRpcResponse::AutomergeGetMetadataResult(AutomergeGetMetadataResultResponse {
                is_success: true,
                was_found: true,
                metadata: Some(convert_metadata(&meta)),
                error: None,
            }),
            Err(e) => ClientRpcResponse::AutomergeGetMetadataResult(AutomergeGetMetadataResultResponse {
                is_success: false,
                was_found: true,
                metadata: None,
                error: Some(e),
            }),
        },
        Ok(None) => ClientRpcResponse::AutomergeGetMetadataResult(AutomergeGetMetadataResultResponse {
            is_success: true,
            was_found: false,
            metadata: None,
            error: None,
        }),
        Err(e) => ClientRpcResponse::AutomergeGetMetadataResult(AutomergeGetMetadataResultResponse {
            is_success: false,
            was_found: false,
            metadata: None,
            error: Some(e),
        }),
    }
}

/// Handle AutomergeExists request.
pub fn handle_exists(document_id: String) -> ClientRpcResponse {
    if let Err(e) = validate_document_id(&document_id) {
        return ClientRpcResponse::AutomergeExistsResult(AutomergeExistsResultResponse {
            is_success: false,
            does_exist: false,
            error: Some(format!("Invalid document ID: {e}")),
        });
    }

    let ck = content_key(&document_id);
    match kv::get(&ck) {
        Ok(Some(_)) => ClientRpcResponse::AutomergeExistsResult(AutomergeExistsResultResponse {
            is_success: true,
            does_exist: true,
            error: None,
        }),
        Ok(None) => ClientRpcResponse::AutomergeExistsResult(AutomergeExistsResultResponse {
            is_success: true,
            does_exist: false,
            error: None,
        }),
        Err(e) => ClientRpcResponse::AutomergeExistsResult(AutomergeExistsResultResponse {
            is_success: false,
            does_exist: false,
            error: Some(e),
        }),
    }
}
