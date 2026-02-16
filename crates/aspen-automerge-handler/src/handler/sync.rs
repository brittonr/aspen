//! Sync operations: generate_sync_message, receive_sync_message.

use aspen_automerge::DocumentId;
use aspen_automerge::DocumentStore;
use aspen_client_api::AutomergeGenerateSyncMessageResultResponse;
use aspen_client_api::AutomergeReceiveSyncMessageResultResponse;
use aspen_client_api::ClientRpcResponse;
use automerge::sync;
use automerge::sync::SyncDoc;
use base64::Engine;

use super::DynAutomergeStore;

/// Handle AutomergeGenerateSyncMessage request.
pub(crate) async fn handle_generate_sync_message(
    store: &DynAutomergeStore,
    document_id: String,
    sync_state_b64: Option<String>,
) -> anyhow::Result<ClientRpcResponse> {
    let id = match DocumentId::from_string(&document_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(ClientRpcResponse::AutomergeGenerateSyncMessageResult(
                AutomergeGenerateSyncMessageResultResponse {
                    is_success: false,
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
                    is_success: false,
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
                    is_success: false,
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
                            is_success: false,
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
                            is_success: false,
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
                is_success: true,
                has_message: true,
                message: Some(msg_b64),
                sync_state: Some(new_state_b64),
                error: None,
            }))
        }
        None => Ok(ClientRpcResponse::AutomergeGenerateSyncMessageResult(AutomergeGenerateSyncMessageResultResponse {
            is_success: true,
            has_message: false,
            message: None,
            sync_state: Some(new_state_b64),
            error: None,
        })),
    }
}

/// Handle AutomergeReceiveSyncMessage request.
pub(crate) async fn handle_receive_sync_message(
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
                    is_success: false,
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
                    is_success: false,
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
                    is_success: false,
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
                    is_success: false,
                    changes_applied: false,
                    sync_state: None,
                    error: Some("Document not found".to_string()),
                },
            ));
        }
        Err(e) => {
            return Ok(ClientRpcResponse::AutomergeReceiveSyncMessageResult(
                AutomergeReceiveSyncMessageResultResponse {
                    is_success: false,
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
                            is_success: false,
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
                            is_success: false,
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
            is_success: false,
            changes_applied: false,
            sync_state: None,
            error: Some(format!("Failed to receive sync message: {}", e)),
        }));
    }

    // Check if changes were applied
    let heads_after = doc.get_heads().len();
    let changes_applied = heads_after != heads_before;

    // Save document if changes were applied
    if changes_applied && let Err(e) = store.save(&id, &mut doc).await {
        return Ok(ClientRpcResponse::AutomergeReceiveSyncMessageResult(AutomergeReceiveSyncMessageResultResponse {
            is_success: false,
            changes_applied: true,
            sync_state: None,
            error: Some(format!("Failed to save document after sync: {}", e)),
        }));
    }

    // Encode sync state
    let new_state_b64 = base64::engine::general_purpose::STANDARD.encode(sync_state.encode());

    Ok(ClientRpcResponse::AutomergeReceiveSyncMessageResult(AutomergeReceiveSyncMessageResultResponse {
        is_success: true,
        changes_applied,
        sync_state: Some(new_state_b64),
        error: None,
    }))
}
