//! Automerge CRDT document handler.
//!
//! Handles all Automerge* operations for CRDT document management.
//! This module is only available with the `automerge` feature.

use aspen_automerge::AspenAutomergeStore;
use aspen_automerge::DocumentChange;
use aspen_automerge::DocumentId;
use aspen_automerge::DocumentStore;
use aspen_automerge::ListOptions;
use aspen_client_api::AutomergeApplyChangesResultResponse;
use aspen_client_api::AutomergeCreateResultResponse;
use aspen_client_api::AutomergeDeleteResultResponse;
use aspen_client_api::AutomergeDocumentMetadata;
use aspen_client_api::AutomergeExistsResultResponse;
use aspen_client_api::AutomergeGenerateSyncMessageResultResponse;
use aspen_client_api::AutomergeGetMetadataResultResponse;
use aspen_client_api::AutomergeGetResultResponse;
use aspen_client_api::AutomergeListResultResponse;
use aspen_client_api::AutomergeMergeResultResponse;
use aspen_client_api::AutomergeReceiveSyncMessageResultResponse;
use aspen_client_api::AutomergeSaveResultResponse;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_core::KeyValueStore;
use automerge::AutoCommit;
use automerge::sync;
use automerge::sync::SyncDoc;
use base64::Engine;

use crate::context::ClientProtocolContext;
use crate::registry::RequestHandler;

/// Type alias for the Automerge store with dynamic KeyValueStore.
type DynAutomergeStore = AspenAutomergeStore<dyn KeyValueStore>;

/// Handler for Automerge CRDT document operations.
pub struct AutomergeHandler;

#[async_trait::async_trait]
impl RequestHandler for AutomergeHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::AutomergeCreate { .. }
                | ClientRpcRequest::AutomergeGet { .. }
                | ClientRpcRequest::AutomergeSave { .. }
                | ClientRpcRequest::AutomergeDelete { .. }
                | ClientRpcRequest::AutomergeApplyChanges { .. }
                | ClientRpcRequest::AutomergeMerge { .. }
                | ClientRpcRequest::AutomergeList { .. }
                | ClientRpcRequest::AutomergeGetMetadata { .. }
                | ClientRpcRequest::AutomergeExists { .. }
                | ClientRpcRequest::AutomergeGenerateSyncMessage { .. }
                | ClientRpcRequest::AutomergeReceiveSyncMessage { .. }
        )
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        // Create automerge store wrapping the KV store
        let store = AspenAutomergeStore::new(ctx.kv_store.clone());

        match request {
            ClientRpcRequest::AutomergeCreate {
                document_id,
                namespace,
                title,
                description,
                tags,
            } => handle_create(&store, document_id, namespace, title, description, tags).await,

            ClientRpcRequest::AutomergeGet { document_id } => handle_get(&store, document_id).await,

            ClientRpcRequest::AutomergeSave {
                document_id,
                document_bytes,
            } => handle_save(&store, document_id, document_bytes).await,

            ClientRpcRequest::AutomergeDelete { document_id } => handle_delete(&store, document_id).await,

            ClientRpcRequest::AutomergeApplyChanges { document_id, changes } => {
                handle_apply_changes(&store, document_id, changes).await
            }

            ClientRpcRequest::AutomergeMerge {
                target_document_id,
                source_document_id,
            } => handle_merge(&store, target_document_id, source_document_id).await,

            ClientRpcRequest::AutomergeList {
                namespace,
                tag,
                limit,
                continuation_token,
            } => handle_list(&store, namespace, tag, limit, continuation_token).await,

            ClientRpcRequest::AutomergeGetMetadata { document_id } => handle_get_metadata(&store, document_id).await,

            ClientRpcRequest::AutomergeExists { document_id } => handle_exists(&store, document_id).await,

            ClientRpcRequest::AutomergeGenerateSyncMessage {
                document_id,
                peer_id: _,
                sync_state,
            } => handle_generate_sync_message(&store, document_id, sync_state).await,

            ClientRpcRequest::AutomergeReceiveSyncMessage {
                document_id,
                peer_id: _,
                message,
                sync_state,
            } => handle_receive_sync_message(&store, document_id, message, sync_state).await,

            _ => Err(anyhow::anyhow!("request not handled by AutomergeHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "AutomergeHandler"
    }
}

// ============================================================================
// Document CRUD Operations
// ============================================================================

async fn handle_create(
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
                    success: false,
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
            success: true,
            document_id: Some(created_id.to_string()),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::AutomergeCreateResult(AutomergeCreateResultResponse {
            success: false,
            document_id: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_get(store: &DynAutomergeStore, document_id: String) -> anyhow::Result<ClientRpcResponse> {
    let id = match DocumentId::from_string(&document_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::AutomergeGetResult(AutomergeGetResultResponse {
                success: false,
                found: false,
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
                success: true,
                found: true,
                document_id: Some(document_id),
                document_bytes: Some(encoded),
                metadata,
                error: None,
            }))
        }
        Ok(None) => Ok(ClientRpcResponse::AutomergeGetResult(AutomergeGetResultResponse {
            success: true,
            found: false,
            document_id: Some(document_id),
            document_bytes: None,
            metadata: None,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::AutomergeGetResult(AutomergeGetResultResponse {
            success: false,
            found: false,
            document_id: Some(document_id),
            document_bytes: None,
            metadata: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_save(
    store: &DynAutomergeStore,
    document_id: String,
    document_bytes: String,
) -> anyhow::Result<ClientRpcResponse> {
    let id = match DocumentId::from_string(&document_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::AutomergeSaveResult(AutomergeSaveResultResponse {
                success: false,
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
                success: false,
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
                success: false,
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
                success: true,
                size_bytes: Some(saved_bytes.len() as u64),
                change_count: Some(change_count),
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::AutomergeSaveResult(AutomergeSaveResultResponse {
            success: false,
            size_bytes: None,
            change_count: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_delete(store: &DynAutomergeStore, document_id: String) -> anyhow::Result<ClientRpcResponse> {
    let id = match DocumentId::from_string(&document_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::AutomergeDeleteResult(AutomergeDeleteResultResponse {
                success: false,
                existed: false,
                error: Some(format!("Invalid document ID: {}", e)),
            }));
        }
    };

    match store.delete(&id).await {
        Ok(existed) => Ok(ClientRpcResponse::AutomergeDeleteResult(AutomergeDeleteResultResponse {
            success: true,
            existed,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::AutomergeDeleteResult(AutomergeDeleteResultResponse {
            success: false,
            existed: false,
            error: Some(e.to_string()),
        })),
    }
}

// ============================================================================
// Change Operations
// ============================================================================

async fn handle_apply_changes(
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

async fn handle_merge(
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

// ============================================================================
// Query Operations
// ============================================================================

async fn handle_list(
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

async fn handle_get_metadata(store: &DynAutomergeStore, document_id: String) -> anyhow::Result<ClientRpcResponse> {
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

async fn handle_exists(store: &DynAutomergeStore, document_id: String) -> anyhow::Result<ClientRpcResponse> {
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

// ============================================================================
// Sync Operations
// ============================================================================

async fn handle_generate_sync_message(
    store: &DynAutomergeStore,
    document_id: String,
    sync_state_b64: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    let id = match DocumentId::from_string(&document_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::AutomergeGenerateSyncMessageResult(
                AutomergeGenerateSyncMessageResultResponse {
                    success: false,
                    has_message: false,
                    message: None,
                    sync_state: None,
                    error: Some(format!("Invalid document ID: {}", e)),
                },
            ));
        }
    };

    // Get document
    let mut doc = match store.get(&id).await {
        Ok(Some(d)) => d,
        Ok(None) => {
            return Ok(ClientRpcResponse::AutomergeGenerateSyncMessageResult(
                AutomergeGenerateSyncMessageResultResponse {
                    success: false,
                    has_message: false,
                    message: None,
                    sync_state: None,
                    error: Some("Document not found".to_string()),
                },
            ));
        }
        Err(e) => {
            return Ok(ClientRpcResponse::AutomergeGenerateSyncMessageResult(
                AutomergeGenerateSyncMessageResultResponse {
                    success: false,
                    has_message: false,
                    message: None,
                    sync_state: None,
                    error: Some(e.to_string()),
                },
            ));
        }
    };

    // Decode or create sync state
    let mut sync_state = match sync_state_b64 {
        Some(b64) => {
            let bytes = match base64::engine::general_purpose::STANDARD.decode(&b64) {
                Ok(b) => b,
                Err(e) => {
                    return Ok(ClientRpcResponse::AutomergeGenerateSyncMessageResult(
                        AutomergeGenerateSyncMessageResultResponse {
                            success: false,
                            has_message: false,
                            message: None,
                            sync_state: None,
                            error: Some(format!("Invalid sync state: {}", e)),
                        },
                    ));
                }
            };
            match sync::State::decode(&bytes) {
                Ok(s) => s,
                Err(e) => {
                    return Ok(ClientRpcResponse::AutomergeGenerateSyncMessageResult(
                        AutomergeGenerateSyncMessageResultResponse {
                            success: false,
                            has_message: false,
                            message: None,
                            sync_state: None,
                            error: Some(format!("Invalid sync state: {}", e)),
                        },
                    ));
                }
            }
        }
        None => sync::State::new(),
    };

    // Generate sync message
    let message = doc.sync().generate_sync_message(&mut sync_state);

    // Encode sync state
    let new_state_b64 = base64::engine::general_purpose::STANDARD.encode(sync_state.encode());

    match message {
        Some(msg) => {
            let msg_b64 = base64::engine::general_purpose::STANDARD.encode(msg.encode());
            Ok(ClientRpcResponse::AutomergeGenerateSyncMessageResult(AutomergeGenerateSyncMessageResultResponse {
                success: true,
                has_message: true,
                message: Some(msg_b64),
                sync_state: Some(new_state_b64),
                error: None,
            }))
        }
        None => Ok(ClientRpcResponse::AutomergeGenerateSyncMessageResult(AutomergeGenerateSyncMessageResultResponse {
            success: true,
            has_message: false,
            message: None,
            sync_state: Some(new_state_b64),
            error: None,
        })),
    }
}

async fn handle_receive_sync_message(
    store: &DynAutomergeStore,
    document_id: String,
    message_b64: String,
    sync_state_b64: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    let id = match DocumentId::from_string(&document_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::AutomergeReceiveSyncMessageResult(
                AutomergeReceiveSyncMessageResultResponse {
                    success: false,
                    changes_applied: false,
                    sync_state: None,
                    error: Some(format!("Invalid document ID: {}", e)),
                },
            ));
        }
    };

    // Decode message
    let msg_bytes = match base64::engine::general_purpose::STANDARD.decode(&message_b64) {
        Ok(b) => b,
        Err(e) => {
            return Ok(ClientRpcResponse::AutomergeReceiveSyncMessageResult(
                AutomergeReceiveSyncMessageResultResponse {
                    success: false,
                    changes_applied: false,
                    sync_state: None,
                    error: Some(format!("Invalid message bytes: {}", e)),
                },
            ));
        }
    };

    let message = match sync::Message::decode(&msg_bytes) {
        Ok(m) => m,
        Err(e) => {
            return Ok(ClientRpcResponse::AutomergeReceiveSyncMessageResult(
                AutomergeReceiveSyncMessageResultResponse {
                    success: false,
                    changes_applied: false,
                    sync_state: None,
                    error: Some(format!("Invalid sync message: {}", e)),
                },
            ));
        }
    };

    // Get document
    let mut doc = match store.get(&id).await {
        Ok(Some(d)) => d,
        Ok(None) => {
            return Ok(ClientRpcResponse::AutomergeReceiveSyncMessageResult(
                AutomergeReceiveSyncMessageResultResponse {
                    success: false,
                    changes_applied: false,
                    sync_state: None,
                    error: Some("Document not found".to_string()),
                },
            ));
        }
        Err(e) => {
            return Ok(ClientRpcResponse::AutomergeReceiveSyncMessageResult(
                AutomergeReceiveSyncMessageResultResponse {
                    success: false,
                    changes_applied: false,
                    sync_state: None,
                    error: Some(e.to_string()),
                },
            ));
        }
    };

    // Decode or create sync state
    let mut sync_state = match sync_state_b64 {
        Some(b64) => {
            let bytes = match base64::engine::general_purpose::STANDARD.decode(&b64) {
                Ok(b) => b,
                Err(e) => {
                    return Ok(ClientRpcResponse::AutomergeReceiveSyncMessageResult(
                        AutomergeReceiveSyncMessageResultResponse {
                            success: false,
                            changes_applied: false,
                            sync_state: None,
                            error: Some(format!("Invalid sync state: {}", e)),
                        },
                    ));
                }
            };
            match sync::State::decode(&bytes) {
                Ok(s) => s,
                Err(e) => {
                    return Ok(ClientRpcResponse::AutomergeReceiveSyncMessageResult(
                        AutomergeReceiveSyncMessageResultResponse {
                            success: false,
                            changes_applied: false,
                            sync_state: None,
                            error: Some(format!("Invalid sync state: {}", e)),
                        },
                    ));
                }
            }
        }
        None => sync::State::new(),
    };

    // Track if document had changes before
    let heads_before = doc.get_heads().len();

    // Receive sync message
    if let Err(e) = doc.sync().receive_sync_message(&mut sync_state, message) {
        return Ok(ClientRpcResponse::AutomergeReceiveSyncMessageResult(AutomergeReceiveSyncMessageResultResponse {
            success: false,
            changes_applied: false,
            sync_state: None,
            error: Some(format!("Failed to receive sync message: {}", e)),
        }));
    }

    // Check if changes were applied
    let heads_after = doc.get_heads().len();
    let changes_applied = heads_after != heads_before;

    // Save document if changes were applied
    if changes_applied {
        if let Err(e) = store.save(&id, &mut doc).await {
            return Ok(ClientRpcResponse::AutomergeReceiveSyncMessageResult(
                AutomergeReceiveSyncMessageResultResponse {
                    success: false,
                    changes_applied: true,
                    sync_state: None,
                    error: Some(format!("Failed to save document after sync: {}", e)),
                },
            ));
        }
    }

    // Encode sync state
    let new_state_b64 = base64::engine::general_purpose::STANDARD.encode(sync_state.encode());

    Ok(ClientRpcResponse::AutomergeReceiveSyncMessageResult(AutomergeReceiveSyncMessageResultResponse {
        success: true,
        changes_applied,
        sync_state: Some(new_state_b64),
        error: None,
    }))
}

// ============================================================================
// Helper Functions
// ============================================================================

fn convert_metadata(meta: aspen_automerge::DocumentMetadata) -> AutomergeDocumentMetadata {
    AutomergeDocumentMetadata {
        document_id: meta.id.to_string(),
        namespace: meta.namespace,
        title: meta.title,
        description: meta.description,
        created_at_ms: meta.created_at_ms,
        updated_at_ms: meta.updated_at_ms,
        size_bytes: meta.size_bytes,
        change_count: meta.change_count,
        heads: meta.heads,
        creator_actor_id: meta.creator_actor_id,
        tags: meta.tags,
    }
}
