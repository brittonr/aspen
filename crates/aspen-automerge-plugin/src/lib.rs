//! WASM guest plugin for Automerge CRDT document management.
//!
//! Handles document CRUD, change application, merge, listing, metadata,
//! existence checks, and a simplified sync protocol using the host-provided
//! KV store and blob store.
//!
//! ## Architecture
//!
//! Documents are stored in the KV store with keys:
//! - Content: `automerge:{document_id}` → base64-encoded document bytes
//! - Metadata: `automerge:_meta:{document_id}` → JSON metadata
//! - Sync state: `automerge:_sync:{document_id}:{peer_id}` → JSON sync state
//!
//! The plugin manages document storage without linking the full `automerge`
//! crate. Document bytes are opaque base64 blobs; the Automerge CRDT logic
//! (merge, conflict resolution) happens client-side. The plugin provides:
//! - Durable storage with Raft consensus
//! - Namespace-isolated KV operations
//! - Metadata indexing for list/search
//! - A simplified sync protocol for peer discovery

mod changes;
mod crud;
mod kv;
mod query;
mod sync;
mod types;

use aspen_wasm_guest_sdk::AspenPlugin;
use aspen_wasm_guest_sdk::ClientRpcRequest;
use aspen_wasm_guest_sdk::ClientRpcResponse;
use aspen_wasm_guest_sdk::PluginInfo;
use aspen_wasm_guest_sdk::PluginPermissions;
use aspen_wasm_guest_sdk::register_plugin;

struct AutomergePlugin;

impl AspenPlugin for AutomergePlugin {
    fn info() -> PluginInfo {
        PluginInfo {
            name: "automerge".to_string(),
            version: "0.1.0".to_string(),
            handles: vec![
                "AutomergeCreate".to_string(),
                "AutomergeGet".to_string(),
                "AutomergeSave".to_string(),
                "AutomergeDelete".to_string(),
                "AutomergeApplyChanges".to_string(),
                "AutomergeMerge".to_string(),
                "AutomergeList".to_string(),
                "AutomergeGetMetadata".to_string(),
                "AutomergeExists".to_string(),
                "AutomergeGenerateSyncMessage".to_string(),
                "AutomergeReceiveSyncMessage".to_string(),
            ],
            priority: 935,
            app_id: Some("automerge".to_string()),
            kv_prefixes: vec!["automerge:".to_string()],
            permissions: PluginPermissions {
                kv_read: true,
                kv_write: true,
                blob_read: true,
                blob_write: true,
                randomness: true,
                ..PluginPermissions::default()
            },
        }
    }

    fn handle(request: ClientRpcRequest) -> ClientRpcResponse {
        match request {
            // CRUD
            ClientRpcRequest::AutomergeCreate {
                document_id,
                namespace,
                title,
                description,
                tags,
            } => crud::handle_create(document_id, namespace, title, description, tags),

            ClientRpcRequest::AutomergeGet { document_id } => crud::handle_get(document_id),

            ClientRpcRequest::AutomergeSave {
                document_id,
                document_bytes,
            } => crud::handle_save(document_id, document_bytes),

            ClientRpcRequest::AutomergeDelete { document_id } => crud::handle_delete(document_id),

            // Changes
            ClientRpcRequest::AutomergeApplyChanges { document_id, changes } => {
                changes::handle_apply_changes(document_id, changes)
            }

            ClientRpcRequest::AutomergeMerge {
                target_document_id,
                source_document_id,
            } => changes::handle_merge(target_document_id, source_document_id),

            // Query
            ClientRpcRequest::AutomergeList {
                namespace,
                tag,
                limit,
                continuation_token,
            } => query::handle_list(namespace, tag, limit, continuation_token),

            ClientRpcRequest::AutomergeGetMetadata { document_id } => query::handle_get_metadata(document_id),

            ClientRpcRequest::AutomergeExists { document_id } => query::handle_exists(document_id),

            // Sync
            ClientRpcRequest::AutomergeGenerateSyncMessage {
                document_id,
                peer_id,
                sync_state,
            } => sync::handle_generate_sync_message(document_id, peer_id, sync_state),

            ClientRpcRequest::AutomergeReceiveSyncMessage {
                document_id,
                peer_id,
                message,
                sync_state,
            } => sync::handle_receive_sync_message(document_id, peer_id, message, sync_state),

            _ => ClientRpcResponse::Error(aspen_wasm_guest_sdk::response::error_response(
                "UNHANDLED",
                "automerge plugin does not handle this request type",
            )),
        }
    }
}

register_plugin!(AutomergePlugin);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_info_matches_manifest() {
        let manifest_bytes = include_bytes!("../plugin.json");
        let manifest: PluginInfo = serde_json::from_slice(manifest_bytes).expect("plugin.json should be valid");
        let info = AutomergePlugin::info();
        assert_eq!(info.name, manifest.name, "name mismatch between code and plugin.json");
        assert_eq!(info.handles, manifest.handles, "handles mismatch between code and plugin.json");
        assert_eq!(info.priority, manifest.priority, "priority mismatch between code and plugin.json");
        assert_eq!(info.version, manifest.version, "version mismatch between code and plugin.json");
        assert_eq!(info.app_id, manifest.app_id, "app_id mismatch between code and plugin.json");
        assert_eq!(info.kv_prefixes, manifest.kv_prefixes, "kv_prefixes mismatch between code and plugin.json");
    }
}
