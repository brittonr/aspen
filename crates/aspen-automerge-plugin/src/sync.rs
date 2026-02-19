//! Sync protocol operations.
//!
//! Implements a simplified sync protocol for the WASM plugin.
//! Since we can't use Automerge's native sync protocol in WASM (no
//! `automerge` crate dependency), we use a hash-based approach:
//!
//! - Generate sync message: Send the full document snapshot if the peer's known heads differ from
//!   the current document heads.
//! - Receive sync message: Store the received document snapshot, replacing the current document
//!   content.
//!
//! This is less efficient than Automerge's incremental sync but
//! works without the `automerge` crate dependency.

use aspen_client_api::AutomergeGenerateSyncMessageResultResponse;
use aspen_client_api::AutomergeReceiveSyncMessageResultResponse;
use aspen_client_api::ClientRpcResponse;
use aspen_wasm_guest_sdk::host;
use base64::Engine;

use crate::kv;
use crate::types::DocumentMetadata;
use crate::types::SimpleSyncState;
use crate::types::content_key;
use crate::types::metadata_key;
use crate::types::validate_document_id;

fn now_ms() -> u64 {
    host::current_time_ms()
}

/// KV key for sync state: `automerge:_sync:{document_id}:{peer_id}`
fn sync_state_key(document_id: &str, peer_id: &str) -> String {
    format!("automerge:_sync:{}:{}", document_id, peer_id)
}

/// Handle AutomergeGenerateSyncMessage request.
///
/// Compares the peer's known heads with the document's current heads.
/// If they differ, sends the full document as the sync message.
pub fn handle_generate_sync_message(
    document_id: String,
    peer_id: String,
    sync_state_b64: Option<String>,
) -> ClientRpcResponse {
    if let Err(e) = validate_document_id(&document_id) {
        return ClientRpcResponse::AutomergeGenerateSyncMessageResult(AutomergeGenerateSyncMessageResultResponse {
            is_success: false,
            has_message: false,
            message: None,
            sync_state: None,
            error: Some(format!("Invalid document ID: {e}")),
        });
    }

    // Load document content
    let ck = content_key(&document_id);
    let doc_b64 = match kv::get(&ck) {
        Ok(Some(data)) => data,
        Ok(None) => {
            return ClientRpcResponse::AutomergeGenerateSyncMessageResult(AutomergeGenerateSyncMessageResultResponse {
                is_success: false,
                has_message: false,
                message: None,
                sync_state: None,
                error: Some(format!("Document not found: {document_id}")),
            });
        }
        Err(e) => {
            return ClientRpcResponse::AutomergeGenerateSyncMessageResult(AutomergeGenerateSyncMessageResultResponse {
                is_success: false,
                has_message: false,
                message: None,
                sync_state: None,
                error: Some(e),
            });
        }
    };

    // Load current metadata heads
    let mk = metadata_key(&document_id);
    let current_heads: Vec<String> = kv::get(&mk)
        .ok()
        .flatten()
        .and_then(|json| DocumentMetadata::from_json(&json).ok())
        .map(|m| m.heads)
        .unwrap_or_default();

    // Load or parse peer's sync state
    let peer_state = sync_state_b64
        .and_then(|b64| base64::engine::general_purpose::STANDARD.decode(&b64).ok())
        .and_then(|bytes| serde_json::from_slice::<SimpleSyncState>(&bytes).ok())
        .or_else(|| {
            let sk = sync_state_key(&document_id, &peer_id);
            kv::get(&sk).ok().flatten().and_then(|json| serde_json::from_str(&json).ok())
        })
        .unwrap_or_else(SimpleSyncState::new);

    // Compare heads
    if peer_state.known_heads == current_heads && !current_heads.is_empty() {
        // Peer is up to date — no message needed
        let state_json = serde_json::to_vec(&peer_state).unwrap_or_default();
        let state_b64 = base64::engine::general_purpose::STANDARD.encode(&state_json);

        return ClientRpcResponse::AutomergeGenerateSyncMessageResult(AutomergeGenerateSyncMessageResultResponse {
            is_success: true,
            has_message: false,
            message: None,
            sync_state: Some(state_b64),
            error: None,
        });
    }

    // Heads differ — send full document as sync message
    // Update sync state with current heads
    let new_state = SimpleSyncState {
        known_heads: current_heads,
        last_sync_ms: now_ms(),
    };

    // Persist sync state
    let sk = sync_state_key(&document_id, &peer_id);
    if let Ok(json) = serde_json::to_string(&new_state) {
        let _ = kv::put(&sk, &json);
    }

    let state_json = serde_json::to_vec(&new_state).unwrap_or_default();
    let state_b64 = base64::engine::general_purpose::STANDARD.encode(&state_json);

    ClientRpcResponse::AutomergeGenerateSyncMessageResult(AutomergeGenerateSyncMessageResultResponse {
        is_success: true,
        has_message: true,
        message: Some(doc_b64),
        sync_state: Some(state_b64),
        error: None,
    })
}

/// Handle AutomergeReceiveSyncMessage request.
///
/// Stores the received document snapshot as the new document content.
pub fn handle_receive_sync_message(
    document_id: String,
    peer_id: String,
    message: String,
    _sync_state_b64: Option<String>,
) -> ClientRpcResponse {
    if let Err(e) = validate_document_id(&document_id) {
        return ClientRpcResponse::AutomergeReceiveSyncMessageResult(AutomergeReceiveSyncMessageResultResponse {
            is_success: false,
            changes_applied: false,
            sync_state: None,
            error: Some(format!("Invalid document ID: {e}")),
        });
    }

    // Validate that the message is valid base64
    let decoded = match base64::engine::general_purpose::STANDARD.decode(&message) {
        Ok(b) => b,
        Err(e) => {
            return ClientRpcResponse::AutomergeReceiveSyncMessageResult(AutomergeReceiveSyncMessageResultResponse {
                is_success: false,
                changes_applied: false,
                sync_state: None,
                error: Some(format!("Invalid sync message: {e}")),
            });
        }
    };

    // Store the received document
    let ck = content_key(&document_id);
    if let Err(e) = kv::put(&ck, &message) {
        return ClientRpcResponse::AutomergeReceiveSyncMessageResult(AutomergeReceiveSyncMessageResultResponse {
            is_success: false,
            changes_applied: false,
            sync_state: None,
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
        meta.size_bytes = decoded.len() as u64;
        if let Ok(new_json) = meta.to_json() {
            let _ = kv::put(&mk, &new_json);
        }
    }

    // Update sync state
    let new_state = SimpleSyncState {
        known_heads: vec![],
        last_sync_ms: now,
    };

    let sk = sync_state_key(&document_id, &peer_id);
    if let Ok(json) = serde_json::to_string(&new_state) {
        let _ = kv::put(&sk, &json);
    }

    let state_json = serde_json::to_vec(&new_state).unwrap_or_default();
    let state_b64 = base64::engine::general_purpose::STANDARD.encode(&state_json);

    ClientRpcResponse::AutomergeReceiveSyncMessageResult(AutomergeReceiveSyncMessageResultResponse {
        is_success: true,
        changes_applied: true,
        sync_state: Some(state_b64),
        error: None,
    })
}
