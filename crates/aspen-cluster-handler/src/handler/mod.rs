//! Cluster management request handler.
//!
//! Handles: InitCluster, AddLearner, ChangeMembership, PromoteLearner,
//! TriggerSnapshot, GetClusterState, GetClusterTicket, AddPeer,
//! GetClusterTicketCombined, GetClientTicket, GetDocsTicket, GetTopology.

mod client_auth;
mod init;
mod membership;
mod snapshot;
mod state;
mod tickets;
mod topology;

use aspen_client_api::AddPeerResultResponse;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;
use iroh::EndpointAddr;
use tracing::debug;
use tracing::info;
use tracing::warn;

/// Maximum number of nodes to include in cluster state response.
const MAX_CLUSTER_NODES: usize = 100;

/// Handler for cluster management operations.
pub struct ClusterHandler;

#[async_trait::async_trait]
impl RequestHandler for ClusterHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::InitCluster
                | ClientRpcRequest::AddLearner { .. }
                | ClientRpcRequest::ChangeMembership { .. }
                | ClientRpcRequest::PromoteLearner { .. }
                | ClientRpcRequest::TriggerSnapshot
                | ClientRpcRequest::GetClusterState
                | ClientRpcRequest::GetClusterTicket
                | ClientRpcRequest::AddPeer { .. }
                | ClientRpcRequest::GetClusterTicketCombined { .. }
                | ClientRpcRequest::GetClientTicket { .. }
                | ClientRpcRequest::GetDocsTicket { .. }
                | ClientRpcRequest::GetTopology { .. }
        )
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::InitCluster => init::handle_init_cluster(ctx).await,

            ClientRpcRequest::AddLearner { node_id, addr } => membership::handle_add_learner(ctx, node_id, addr).await,

            ClientRpcRequest::ChangeMembership { members } => membership::handle_change_membership(ctx, members).await,

            ClientRpcRequest::PromoteLearner {
                learner_id,
                replace_node,
                force,
            } => membership::handle_promote_learner(ctx, learner_id, replace_node, force).await,

            ClientRpcRequest::TriggerSnapshot => snapshot::handle_trigger_snapshot(ctx).await,

            ClientRpcRequest::GetClusterState => state::handle_get_cluster_state(ctx).await,

            ClientRpcRequest::GetClusterTicket => tickets::handle_get_cluster_ticket(ctx).await,

            ClientRpcRequest::AddPeer { node_id, endpoint_addr } => handle_add_peer(ctx, node_id, endpoint_addr).await,

            ClientRpcRequest::GetClusterTicketCombined { endpoint_ids } => {
                tickets::handle_get_cluster_ticket_combined(ctx, endpoint_ids).await
            }

            ClientRpcRequest::GetClientTicket { access, priority } => {
                client_auth::handle_get_client_ticket(ctx, access, priority).await
            }

            ClientRpcRequest::GetDocsTicket { read_write, priority } => {
                client_auth::handle_get_docs_ticket(ctx, read_write, priority).await
            }

            ClientRpcRequest::GetTopology { client_version } => {
                topology::handle_get_topology(ctx, client_version).await
            }

            _ => Err(anyhow::anyhow!("request not handled by ClusterHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "ClusterHandler"
    }
}

// ============================================================================
// Cluster Operation Handlers
// ============================================================================

/// Sanitize control plane error messages to prevent information leakage.
fn sanitize_control_error(e: &aspen_core::ControlPlaneError) -> String {
    use aspen_core::ControlPlaneError;
    match e {
        ControlPlaneError::NotInitialized => "cluster not initialized".to_string(),
        ControlPlaneError::InvalidRequest { reason } => format!("invalid request: {}", reason),
        ControlPlaneError::Failed { reason } => {
            // Check if reason contains leader info to provide hints
            if reason.contains("not leader") || reason.contains("ForwardToLeader") {
                "not leader".to_string()
            } else {
                "operation failed".to_string()
            }
        }
        _ => "internal error".to_string(),
    }
}

async fn handle_add_peer(
    ctx: &ClientProtocolContext,
    node_id: u64,
    endpoint_addr: String,
) -> anyhow::Result<ClientRpcResponse> {
    // Parse the endpoint address (JSON-serialized EndpointAddr)
    let parsed_addr: EndpointAddr = match serde_json::from_str(&endpoint_addr) {
        Ok(addr) => addr,
        Err(e) => {
            debug!(
                node_id = node_id,
                endpoint_addr = %endpoint_addr,
                error = %e,
                "AddPeer: failed to parse endpoint_addr as JSON EndpointAddr"
            );
            return Ok(ClientRpcResponse::AddPeerResult(AddPeerResultResponse {
                success: false,
                error: Some("invalid endpoint_addr: expected JSON-serialized EndpointAddr".to_string()),
            }));
        }
    };

    // Check if network factory is available
    let network_factory = match &ctx.network_factory {
        Some(nf) => nf,
        None => {
            warn!(node_id = node_id, "AddPeer: network_factory not available in context");
            return Ok(ClientRpcResponse::AddPeerResult(AddPeerResultResponse {
                success: false,
                error: Some("network_factory not configured for this node".to_string()),
            }));
        }
    };

    // Add peer to the network factory
    // Tiger Style: add_peer is bounded by MAX_PEERS (1000)
    // Use JSON serialization for the address (matches CoreNetworkFactory::add_peer expectation)
    let addr_json = serde_json::to_string(&parsed_addr).unwrap_or_default();
    let _ = network_factory.add_peer(node_id, addr_json).await;

    info!(
        node_id = node_id,
        endpoint_id = %parsed_addr.id,
        "AddPeer: successfully registered peer in network factory"
    );

    Ok(ClientRpcResponse::AddPeerResult(AddPeerResultResponse {
        success: true,
        error: None,
    }))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use aspen_rpc_core::test_support::MockEndpointProvider;
    use aspen_rpc_core::test_support::TestContextBuilder;
    use aspen_testing::DeterministicClusterController;
    use aspen_testing::DeterministicKeyValueStore;

    use super::*;

    async fn setup_test_context() -> ClientProtocolContext {
        let controller = Arc::new(DeterministicClusterController::new());
        let kv_store = Arc::new(DeterministicKeyValueStore::new());
        let mock_endpoint = Arc::new(MockEndpointProvider::with_seed(12345).await);

        TestContextBuilder::new()
            .with_node_id(1)
            .with_controller(controller)
            .with_kv_store(kv_store)
            .with_endpoint_manager(mock_endpoint)
            .with_cookie("test_cluster")
            .build()
    }

    #[test]
    fn test_can_handle_init_cluster() {
        let handler = ClusterHandler;
        assert!(handler.can_handle(&ClientRpcRequest::InitCluster));
    }

    #[test]
    fn test_can_handle_add_learner() {
        let handler = ClusterHandler;
        assert!(handler.can_handle(&ClientRpcRequest::AddLearner {
            node_id: 2,
            addr: "test_addr".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_change_membership() {
        let handler = ClusterHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ChangeMembership { members: vec![1, 2, 3] }));
    }

    #[test]
    fn test_can_handle_promote_learner() {
        let handler = ClusterHandler;
        assert!(handler.can_handle(&ClientRpcRequest::PromoteLearner {
            learner_id: 2,
            replace_node: None,
            force: false,
        }));
    }

    #[test]
    fn test_can_handle_trigger_snapshot() {
        let handler = ClusterHandler;
        assert!(handler.can_handle(&ClientRpcRequest::TriggerSnapshot));
    }

    #[test]
    fn test_can_handle_get_cluster_state() {
        let handler = ClusterHandler;
        assert!(handler.can_handle(&ClientRpcRequest::GetClusterState));
    }

    #[test]
    fn test_can_handle_get_cluster_ticket() {
        let handler = ClusterHandler;
        assert!(handler.can_handle(&ClientRpcRequest::GetClusterTicket));
    }

    #[test]
    fn test_can_handle_add_peer() {
        let handler = ClusterHandler;
        assert!(handler.can_handle(&ClientRpcRequest::AddPeer {
            node_id: 2,
            endpoint_addr: "{}".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_get_cluster_ticket_combined() {
        let handler = ClusterHandler;
        assert!(handler.can_handle(&ClientRpcRequest::GetClusterTicketCombined { endpoint_ids: None }));
    }

    #[test]
    fn test_can_handle_get_client_ticket() {
        let handler = ClusterHandler;
        assert!(handler.can_handle(&ClientRpcRequest::GetClientTicket {
            access: "read".to_string(),
            priority: 0,
        }));
    }

    #[test]
    fn test_can_handle_get_docs_ticket() {
        let handler = ClusterHandler;
        assert!(handler.can_handle(&ClientRpcRequest::GetDocsTicket {
            read_write: false,
            priority: 0,
        }));
    }

    #[test]
    fn test_can_handle_get_topology() {
        let handler = ClusterHandler;
        assert!(handler.can_handle(&ClientRpcRequest::GetTopology { client_version: None }));
    }

    #[test]
    fn test_rejects_unrelated_requests() {
        let handler = ClusterHandler;

        // Core requests
        assert!(!handler.can_handle(&ClientRpcRequest::Ping));
        assert!(!handler.can_handle(&ClientRpcRequest::GetHealth));

        // KV requests
        assert!(!handler.can_handle(&ClientRpcRequest::ReadKey {
            key: "test".to_string(),
        }));

        // Coordination requests
        assert!(!handler.can_handle(&ClientRpcRequest::LockAcquire {
            key: "test".to_string(),
            holder_id: "holder".to_string(),
            ttl_ms: 30000,
            timeout_ms: 0,
        }));
    }

    #[test]
    fn test_handler_name() {
        let handler = ClusterHandler;
        assert_eq!(handler.name(), "ClusterHandler");
    }

    #[tokio::test]
    async fn test_handle_get_cluster_ticket() {
        let ctx = setup_test_context().await;
        let handler = ClusterHandler;

        let result = handler.handle(ClientRpcRequest::GetClusterTicket, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::ClusterTicket(response) => {
                assert!(!response.ticket.is_empty());
                assert_eq!(response.cluster_id, "test_cluster");
                assert!(!response.topic_id.is_empty());
            }
            other => panic!("expected ClusterTicket, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_handle_get_cluster_ticket_combined() {
        let ctx = setup_test_context().await;
        let handler = ClusterHandler;

        let request = ClientRpcRequest::GetClusterTicketCombined { endpoint_ids: None };

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::ClusterTicket(response) => {
                assert!(!response.ticket.is_empty());
                assert_eq!(response.cluster_id, "test_cluster");
            }
            other => panic!("expected ClusterTicket, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_handle_get_client_ticket() {
        let ctx = setup_test_context().await;
        let handler = ClusterHandler;

        let request = ClientRpcRequest::GetClientTicket {
            access: "write".to_string(),
            priority: 5,
        };

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::ClientTicket(response) => {
                assert!(!response.ticket.is_empty());
                assert_eq!(response.cluster_id, "test_cluster");
                assert_eq!(response.access, "write");
                assert_eq!(response.priority, 5);
            }
            other => panic!("expected ClientTicket, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_handle_get_client_ticket_read_only() {
        let ctx = setup_test_context().await;
        let handler = ClusterHandler;

        let request = ClientRpcRequest::GetClientTicket {
            access: "read".to_string(),
            priority: 0,
        };

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::ClientTicket(response) => {
                assert_eq!(response.access, "read");
            }
            other => panic!("expected ClientTicket, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_handle_get_docs_ticket() {
        let ctx = setup_test_context().await;
        let handler = ClusterHandler;

        let request = ClientRpcRequest::GetDocsTicket {
            read_write: true,
            priority: 3,
        };

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::DocsTicket(response) => {
                // Without docs feature, ticket is empty with error
                #[cfg(not(feature = "docs"))]
                {
                    assert!(response.error.is_some());
                }
                // With docs feature, ticket is populated
                #[cfg(feature = "docs")]
                {
                    assert!(!response.ticket.is_empty());
                    assert_eq!(response.cluster_id, "test_cluster");
                    assert!(response.read_write);
                    assert_eq!(response.priority, 3);
                }
            }
            other => panic!("expected DocsTicket, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_handle_get_topology_not_available() {
        let ctx = setup_test_context().await;
        let handler = ClusterHandler;

        let request = ClientRpcRequest::GetTopology { client_version: None };

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::TopologyResult(response) => {
                // Topology not configured in test context
                assert!(!response.success);
                assert!(response.error.is_some());
            }
            other => panic!("expected TopologyResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_handle_add_peer_no_network_factory() {
        let ctx = setup_test_context().await;
        let handler = ClusterHandler;

        // Test with network_factory not configured
        let request = ClientRpcRequest::AddPeer {
            node_id: 2,
            endpoint_addr: "{}".to_string(),
        };

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::AddPeerResult(response) => {
                // network_factory is not available in test context
                assert!(!response.success);
                assert!(response.error.is_some());
            }
            other => panic!("expected AddPeerResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_handle_add_peer_invalid_json() {
        let ctx = setup_test_context().await;
        let handler = ClusterHandler;

        let request = ClientRpcRequest::AddPeer {
            node_id: 2,
            endpoint_addr: "not valid json".to_string(),
        };

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::AddPeerResult(response) => {
                assert!(!response.success);
                assert!(response.error.is_some());
                assert!(response.error.unwrap().contains("invalid endpoint_addr"));
            }
            other => panic!("expected AddPeerResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_handle_unhandled_request() {
        let ctx = setup_test_context().await;
        let handler = ClusterHandler;

        // This request is not handled by ClusterHandler
        let request = ClientRpcRequest::Ping;

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not handled"));
    }

    #[test]
    fn test_sanitize_control_error_not_initialized() {
        use aspen_core::ControlPlaneError;
        let error = ControlPlaneError::NotInitialized;
        let sanitized = sanitize_control_error(&error);
        assert_eq!(sanitized, "cluster not initialized");
    }

    #[test]
    fn test_sanitize_control_error_invalid_request() {
        use aspen_core::ControlPlaneError;
        let error = ControlPlaneError::InvalidRequest {
            reason: "test reason".to_string(),
        };
        let sanitized = sanitize_control_error(&error);
        assert!(sanitized.contains("invalid request"));
        assert!(sanitized.contains("test reason"));
    }

    #[test]
    fn test_sanitize_control_error_not_leader() {
        use aspen_core::ControlPlaneError;
        let error = ControlPlaneError::Failed {
            reason: "not leader".to_string(),
        };
        let sanitized = sanitize_control_error(&error);
        assert_eq!(sanitized, "not leader");
    }

    #[test]
    fn test_sanitize_control_error_forward_to_leader() {
        use aspen_core::ControlPlaneError;
        let error = ControlPlaneError::Failed {
            reason: "ForwardToLeader node 2".to_string(),
        };
        let sanitized = sanitize_control_error(&error);
        assert_eq!(sanitized, "not leader");
    }

    #[test]
    fn test_sanitize_control_error_generic_failed() {
        use aspen_core::ControlPlaneError;
        let error = ControlPlaneError::Failed {
            reason: "some other failure".to_string(),
        };
        let sanitized = sanitize_control_error(&error);
        assert_eq!(sanitized, "operation failed");
    }
}
