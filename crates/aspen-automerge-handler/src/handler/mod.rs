//! Automerge CRDT document handler.
//!
//! Handles all Automerge* operations for CRDT document management.

mod changes;
mod crud;
mod helpers;
mod query;
mod sync;

use aspen_automerge::AspenAutomergeStore;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_core::KeyValueStore;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;

/// Type alias for the Automerge store with dynamic KeyValueStore.
pub(crate) type DynAutomergeStore = AspenAutomergeStore<dyn KeyValueStore>;

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
            // CRUD operations
            ClientRpcRequest::AutomergeCreate {
                document_id,
                namespace,
                title,
                description,
                tags,
            } => crud::handle_create(&store, document_id, namespace, title, description, tags).await,

            ClientRpcRequest::AutomergeGet { document_id } => crud::handle_get(&store, document_id).await,

            ClientRpcRequest::AutomergeSave {
                document_id,
                document_bytes,
            } => crud::handle_save(&store, document_id, document_bytes).await,

            ClientRpcRequest::AutomergeDelete { document_id } => crud::handle_delete(&store, document_id).await,

            // Change operations
            ClientRpcRequest::AutomergeApplyChanges { document_id, changes } => {
                changes::handle_apply_changes(&store, document_id, changes).await
            }

            ClientRpcRequest::AutomergeMerge {
                target_document_id,
                source_document_id,
            } => changes::handle_merge(&store, target_document_id, source_document_id).await,

            // Query operations
            ClientRpcRequest::AutomergeList {
                namespace,
                tag,
                limit,
                continuation_token,
            } => query::handle_list(&store, namespace, tag, limit, continuation_token).await,

            ClientRpcRequest::AutomergeGetMetadata { document_id } => {
                query::handle_get_metadata(&store, document_id).await
            }

            ClientRpcRequest::AutomergeExists { document_id } => query::handle_exists(&store, document_id).await,

            // Sync operations
            ClientRpcRequest::AutomergeGenerateSyncMessage {
                document_id,
                peer_id: _,
                sync_state,
            } => sync::handle_generate_sync_message(&store, document_id, sync_state).await,

            ClientRpcRequest::AutomergeReceiveSyncMessage {
                document_id,
                peer_id: _,
                message,
                sync_state,
            } => sync::handle_receive_sync_message(&store, document_id, message, sync_state).await,

            _ => Err(anyhow::anyhow!("request not handled by AutomergeHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "AutomergeHandler"
    }
}
