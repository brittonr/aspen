//! Docs/Sync request handler.
//!
//! Handles: DocsSet, DocsGet, DocsDelete, DocsList, DocsStatus,
//! AddPeerCluster, RemovePeerCluster, ListPeerClusters, GetPeerClusterStatus,
//! UpdatePeerClusterFilter, UpdatePeerClusterPriority, SetPeerClusterEnabled, GetKeyOrigin.
//!
//! Each domain is handled by a dedicated sub-handler struct that owns
//! its routing logic, eliminating duplicate dispatch in a single match.

mod crud;
mod federation;
mod peer_config;
mod status;

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;
use crud::CrudHandler;
use federation::FederationHandler;
use peer_config::PeerConfigHandler;
use status::StatusHandler;

/// Handler for docs/sync operations.
///
/// Dispatches requests to domain-specific sub-handlers:
/// - `CrudHandler` for DocsSet, DocsGet, DocsDelete, DocsList
/// - `StatusHandler` for DocsStatus, GetKeyOrigin
/// - `FederationHandler` for AddPeerCluster, RemovePeerCluster, ListPeerClusters
/// - `PeerConfigHandler` for GetPeerClusterStatus, UpdatePeerClusterFilter,
///   UpdatePeerClusterPriority, SetPeerClusterEnabled
pub struct DocsHandler;

#[async_trait::async_trait]
impl RequestHandler for DocsHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        let crud = CrudHandler;
        let status = StatusHandler;
        let federation = FederationHandler;
        let peer_config = PeerConfigHandler;

        crud.can_handle(request)
            || status.can_handle(request)
            || federation.can_handle(request)
            || peer_config.can_handle(request)
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        let crud = CrudHandler;
        let status = StatusHandler;
        let federation = FederationHandler;
        let peer_config = PeerConfigHandler;

        if crud.can_handle(&request) {
            return crud.handle(request, ctx).await;
        }
        if status.can_handle(&request) {
            return status.handle(request, ctx).await;
        }
        if federation.can_handle(&request) {
            return federation.handle(request, ctx).await;
        }
        if peer_config.can_handle(&request) {
            return peer_config.handle(request, ctx).await;
        }

        Err(anyhow::anyhow!("request not handled by DocsHandler"))
    }

    fn name(&self) -> &'static str {
        "DocsHandler"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_handle_docs_set() {
        let handler = DocsHandler;
        assert!(handler.can_handle(&ClientRpcRequest::DocsSet {
            key: "test".to_string(),
            value: vec![1, 2, 3],
        }));
    }

    #[test]
    fn test_can_handle_docs_get() {
        let handler = DocsHandler;
        assert!(handler.can_handle(&ClientRpcRequest::DocsGet {
            key: "test".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_docs_delete() {
        let handler = DocsHandler;
        assert!(handler.can_handle(&ClientRpcRequest::DocsDelete {
            key: "test".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_docs_list() {
        let handler = DocsHandler;
        assert!(handler.can_handle(&ClientRpcRequest::DocsList {
            prefix: Some("test:".to_string()),
            limit: Some(100),
        }));
    }

    #[test]
    fn test_can_handle_docs_status() {
        let handler = DocsHandler;
        assert!(handler.can_handle(&ClientRpcRequest::DocsStatus));
    }

    #[test]
    fn test_can_handle_add_peer_cluster() {
        let handler = DocsHandler;
        assert!(handler.can_handle(&ClientRpcRequest::AddPeerCluster {
            ticket: "test-ticket".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_list_peer_clusters() {
        let handler = DocsHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ListPeerClusters));
    }

    #[test]
    fn test_can_handle_get_key_origin() {
        let handler = DocsHandler;
        assert!(handler.can_handle(&ClientRpcRequest::GetKeyOrigin {
            key: "test-key".to_string(),
        }));
    }

    #[test]
    fn test_rejects_unrelated_requests() {
        let handler = DocsHandler;

        // KV requests
        assert!(!handler.can_handle(&ClientRpcRequest::ReadKey {
            key: "test".to_string(),
        }));

        // Core requests
        assert!(!handler.can_handle(&ClientRpcRequest::Ping));
    }

    #[test]
    fn test_handler_name() {
        let handler = DocsHandler;
        assert_eq!(handler.name(), "DocsHandler");
    }
}
