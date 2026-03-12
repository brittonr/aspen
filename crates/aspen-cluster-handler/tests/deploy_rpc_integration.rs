//! Integration tests for the deploy RPC path.
//!
//! Verifies the full path: coordinator → IrohNodeRpcClient → iroh QUIC →
//! target node handler → executor status updates flow back via KV.
//!
//! Uses real iroh endpoints with localhost transport.

#![cfg(feature = "deploy")]

use std::sync::Arc;

use aspen_client_api::AuthenticatedRequest;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::MAX_CLIENT_MESSAGE_SIZE;
use aspen_cluster_types::ClusterNode;
use aspen_core::ClusterController;
use aspen_deploy::DeployArtifact;
use aspen_deploy::DeployStrategy;
use aspen_deploy::DeploymentCoordinator;
use aspen_deploy::coordinator::rpc::NodeRpcClient;
use aspen_deploy::coordinator::rpc::RpcError;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::test_support::MockEndpointProvider;
use aspen_rpc_core::test_support::TestContextBuilder;
use aspen_testing::DeterministicClusterController;
use aspen_testing::DeterministicKeyValueStore;
use aspen_traits::KeyValueStore;
use iroh::Endpoint;
use iroh::protocol::AcceptError;
use iroh::protocol::ProtocolHandler;
use iroh::protocol::Router;

/// CLIENT_ALPN used for all inter-node RPCs.
const CLIENT_ALPN: &[u8] = b"aspen-client";

// ============================================================================
// Minimal CLIENT_ALPN handler for the target node
// ============================================================================

/// A minimal protocol handler that accepts CLIENT_ALPN connections and
/// dispatches NodeUpgrade/NodeRollback to the deploy handlers.
#[derive(Debug, Clone)]
struct TestDeployHandler {
    ctx: ClientProtocolContext,
}

impl ProtocolHandler for TestDeployHandler {
    async fn accept(&self, connection: iroh::endpoint::Connection) -> Result<(), AcceptError> {
        let ctx = self.ctx.clone();
        // Process streams on this connection
        while let Ok((mut send, mut recv)) = connection.accept_bi().await {
            let buffer = match recv.read_to_end(MAX_CLIENT_MESSAGE_SIZE).await {
                Ok(b) => b,
                Err(_) => break,
            };
            let auth_request: AuthenticatedRequest = match postcard::from_bytes(&buffer) {
                Ok(r) => r,
                Err(_) => break,
            };

            let response = match auth_request.request {
                ClientRpcRequest::NodeUpgrade {
                    deploy_id,
                    artifact,
                    expected_binary,
                } => aspen_cluster_handler::handler::deploy::handle_node_upgrade(
                    &ctx,
                    deploy_id,
                    artifact,
                    expected_binary,
                )
                .await
                .unwrap_or_else(|e| ClientRpcResponse::error("INTERNAL", e.to_string())),
                ClientRpcRequest::NodeRollback { deploy_id } => {
                    aspen_cluster_handler::handler::deploy::handle_node_rollback(&ctx, deploy_id)
                        .await
                        .unwrap_or_else(|e| ClientRpcResponse::error("INTERNAL", e.to_string()))
                }
                ClientRpcRequest::GetHealth => {
                    ClientRpcResponse::Health(aspen_client_api::messages::cluster::HealthResponse {
                        status: "healthy".to_string(),
                        node_id: ctx.node_id,
                        raft_node_id: Some(ctx.node_id),
                        uptime_seconds: 100,
                        is_initialized: true,
                        membership_node_count: Some(3),
                    })
                }
                _ => ClientRpcResponse::error("UNHANDLED", "unhandled request type"),
            };

            let response_bytes = match postcard::to_stdvec(&response) {
                Ok(b) => b,
                Err(_) => break,
            };
            if send.write_all(&response_bytes).await.is_err() {
                break;
            }
            if send.finish().is_err() {
                break;
            }
        }
        Ok(())
    }
}

// ============================================================================
// Test RPC client (mirrors IrohNodeRpcClient from the handler crate)
// ============================================================================

struct TestNodeRpcClient {
    endpoint: Endpoint,
    controller: Arc<dyn ClusterController>,
    #[allow(dead_code)]
    source_node_id: u64,
}

impl TestNodeRpcClient {
    fn new(endpoint: Endpoint, controller: Arc<dyn ClusterController>, source_node_id: u64) -> Self {
        Self {
            endpoint,
            controller,
            source_node_id,
        }
    }

    async fn resolve_node_addr(&self, node_id: u64) -> Result<iroh::EndpointAddr, RpcError> {
        let state = self
            .controller
            .current_state()
            .await
            .map_err(|e| RpcError::new(format!("cluster state unavailable: {e}")))?;
        for node in &state.nodes {
            if node.id == node_id {
                if let Some(addr) = node.iroh_addr() {
                    return Ok(addr.clone());
                }
                return Err(RpcError::new(format!("node {node_id} has no iroh address")));
            }
        }
        Err(RpcError::new(format!("node {node_id} not in cluster state")))
    }

    async fn send_rpc(&self, node_id: u64, request: ClientRpcRequest) -> Result<ClientRpcResponse, RpcError> {
        let addr = self.resolve_node_addr(node_id).await?;
        let timeout = std::time::Duration::from_secs(10);

        let conn = tokio::time::timeout(timeout, self.endpoint.connect(addr, CLIENT_ALPN))
            .await
            .map_err(|_| RpcError::new("connection timed out"))?
            .map_err(|e| RpcError::new(format!("connect failed: {e}")))?;

        let (mut send, mut recv) = conn.open_bi().await.map_err(|e| RpcError::new(format!("open_bi failed: {e}")))?;

        let auth = AuthenticatedRequest::unauthenticated(request);
        let bytes = postcard::to_stdvec(&auth).map_err(|e| RpcError::new(format!("serialize failed: {e}")))?;

        send.write_all(&bytes).await.map_err(|e| RpcError::new(format!("write failed: {e}")))?;
        send.finish().map_err(|e| RpcError::new(format!("finish failed: {e}")))?;

        let resp_bytes = tokio::time::timeout(timeout, recv.read_to_end(MAX_CLIENT_MESSAGE_SIZE))
            .await
            .map_err(|_| RpcError::new("response timed out"))?
            .map_err(|e| RpcError::new(format!("read failed: {e}")))?;

        let response: ClientRpcResponse =
            postcard::from_bytes(&resp_bytes).map_err(|e| RpcError::new(format!("deserialize failed: {e}")))?;

        conn.close(iroh::endpoint::VarInt::from_u32(0), b"done");
        Ok(response)
    }
}

#[async_trait::async_trait]
impl NodeRpcClient for TestNodeRpcClient {
    async fn send_upgrade(
        &self,
        node_id: u64,
        deploy_id: &str,
        artifact_ref: &str,
        _expected_binary: Option<&str>,
    ) -> Result<(), RpcError> {
        let request = ClientRpcRequest::NodeUpgrade {
            deploy_id: deploy_id.to_string(),
            artifact: artifact_ref.to_string(),
            expected_binary: None,
        };
        let resp = self.send_rpc(node_id, request).await?;
        match resp {
            ClientRpcResponse::NodeUpgradeResult(r) if r.is_accepted => Ok(()),
            ClientRpcResponse::NodeUpgradeResult(r) => Err(RpcError::new(r.error.unwrap_or_else(|| "rejected".into()))),
            ClientRpcResponse::Error(e) => Err(RpcError::new(format!("{}: {}", e.code, e.message))),
            _ => Err(RpcError::new("unexpected response")),
        }
    }

    async fn send_rollback(&self, node_id: u64, deploy_id: &str) -> Result<(), RpcError> {
        let request = ClientRpcRequest::NodeRollback {
            deploy_id: deploy_id.to_string(),
        };
        let resp = self.send_rpc(node_id, request).await?;
        match resp {
            ClientRpcResponse::NodeRollbackResult(r) if r.is_success => Ok(()),
            ClientRpcResponse::NodeRollbackResult(r) => {
                Err(RpcError::new(r.error.unwrap_or_else(|| "rejected".into())))
            }
            ClientRpcResponse::Error(e) => Err(RpcError::new(format!("{}: {}", e.code, e.message))),
            _ => Err(RpcError::new("unexpected response")),
        }
    }

    async fn check_health(&self, node_id: u64) -> Result<bool, RpcError> {
        let resp = self.send_rpc(node_id, ClientRpcRequest::GetHealth).await?;
        match resp {
            ClientRpcResponse::Health(h) => Ok(h.status == "healthy"),
            _ => Ok(false),
        }
    }
}

// ============================================================================
// Test helpers
// ============================================================================

/// Create a real iroh endpoint bound to localhost.
async fn create_endpoint(seed: u64) -> Endpoint {
    let mut key_bytes = [0u8; 32];
    key_bytes[0..8].copy_from_slice(&seed.to_le_bytes());
    let secret_key = iroh::SecretKey::from_bytes(&key_bytes);

    Endpoint::builder()
        .secret_key(secret_key)
        .alpns(vec![CLIENT_ALPN.to_vec()])
        .bind_addr_v4("127.0.0.1:0".parse().unwrap())
        .bind()
        .await
        .expect("failed to bind endpoint")
}

/// Build a ClientProtocolContext for a target node.
async fn build_target_context(node_id: u64) -> ClientProtocolContext {
    let mock_ep = Arc::new(MockEndpointProvider::with_seed(node_id * 1000).await);
    let kv = Arc::new(DeterministicKeyValueStore::new()) as Arc<dyn KeyValueStore>;
    let controller = DeterministicClusterController::new();

    TestContextBuilder::new()
        .with_node_id(node_id)
        .with_kv_store(kv)
        .with_controller(controller)
        .with_endpoint_manager(mock_ep)
        .with_cookie("test-cluster")
        .build()
}

/// Set up a target node: create endpoint, attach handler, return address.
///
/// The returned `Router` must be held alive for the handler to work.
async fn setup_target_node(node_id: u64, seed: u64) -> (Endpoint, iroh::EndpointAddr, Router) {
    let endpoint = create_endpoint(seed).await;
    let mut addr = endpoint.addr();
    // Add actual socket addresses for local connectivity
    for sock in endpoint.bound_sockets() {
        let mut fixed = sock;
        if fixed.ip().is_unspecified() {
            fixed.set_ip(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST));
        }
        addr.addrs.insert(iroh::TransportAddr::Ip(fixed));
    }

    let ctx = build_target_context(node_id).await;
    let handler = TestDeployHandler { ctx };

    let router = Router::builder(endpoint.clone()).accept(CLIENT_ALPN, handler).spawn();

    (endpoint, addr, router)
}

// ============================================================================
// Tests
// ============================================================================

/// Test: send_upgrade RPC reaches target node and is accepted.
#[tokio::test]
async fn test_send_upgrade_rpc_reaches_target() {
    let (_target_ep, target_addr, _router) = setup_target_node(2, 200).await;

    let coord_ep = create_endpoint(100).await;

    let controller = DeterministicClusterController::new();
    let target_node = ClusterNode::with_iroh_addr(2, target_addr);
    let coord_node = ClusterNode::new(1, "coordinator", None);
    controller
        .init(aspen_cluster_types::InitRequest {
            initial_members: vec![coord_node, target_node],
        })
        .await
        .unwrap();

    let rpc_client = TestNodeRpcClient::new(coord_ep, controller, 1);

    let result = rpc_client.send_upgrade(2, "deploy-test-1", "/nix/store/abc-aspen", None).await;
    assert!(result.is_ok(), "send_upgrade should succeed, got: {:?}", result.err());
}

/// Test: DrainState integration — in-flight ops block drain, new ops rejected during drain.
///
/// Validates the core invariant: when a node upgrade triggers drain,
/// in-flight RPCs complete before drain finishes, and new RPCs are rejected
/// with the equivalent of NOT_LEADER.
#[tokio::test]
async fn test_drain_blocks_until_inflight_completes_and_rejects_new() {
    use std::time::Duration;

    use aspen_cluster::upgrade::DrainState;

    let drain_state = DrainState::new();

    // Simulate an in-flight RPC (try_start_op before drain)
    assert!(drain_state.try_start_op(), "op should start before drain");
    assert_eq!(drain_state.in_flight_count(), 1);

    let drain_state_for_drain = drain_state.clone();
    let drain_state_for_op = drain_state.clone();

    // Spawn the drain (like the executor does)
    let drain_handle = tokio::spawn(async move {
        aspen_cluster::upgrade::drain::execute_drain(&drain_state_for_drain, Duration::from_secs(5)).await
    });

    // Give drain a moment to set the flag
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Verify: new RPCs should be rejected during drain
    assert!(drain_state.is_draining(), "should be draining");
    assert!(!drain_state.try_start_op(), "new ops should be rejected during drain");

    // Complete the in-flight RPC
    drain_state_for_op.finish_op();

    // Drain should complete now
    let result = drain_handle.await.unwrap();
    assert!(result.completed, "drain should have completed cleanly");
    assert_eq!(result.cancelled_ops, 0, "no ops should have been cancelled");
}

/// Test: BlobHash artifact with blob_store None returns rejected with clear error.
#[tokio::test]
async fn test_blob_artifact_without_blob_store_rejected() {
    let ctx = build_target_context(1).await;

    // blob_store is None by default in test context
    let result = aspen_cluster_handler::handler::deploy::handle_node_upgrade(
        &ctx,
        "deploy-blob-1".to_string(),
        "deadbeef".repeat(8),
    )
    .await
    .expect("handler should not return Err");

    match result {
        ClientRpcResponse::NodeUpgradeResult(r) => {
            assert!(!r.is_accepted, "should reject blob upgrade without blob store");
            let err = r.error.expect("should have error message");
            assert!(err.contains("blob") && err.contains("feature"), "error should mention blob feature: {err}");
        }
        other => panic!("expected NodeUpgradeResult, got: {other:?}"),
    }
}

/// Test: send_rollback RPC reaches target node and is accepted.
#[tokio::test]
async fn test_send_rollback_rpc_reaches_target() {
    let (_target_ep, target_addr, _router) = setup_target_node(2, 300).await;

    let coord_ep = create_endpoint(101).await;

    let controller = DeterministicClusterController::new();
    let target_node = ClusterNode::with_iroh_addr(2, target_addr);
    let coord_node = ClusterNode::new(1, "coordinator", None);
    controller
        .init(aspen_cluster_types::InitRequest {
            initial_members: vec![coord_node, target_node],
        })
        .await
        .unwrap();

    let rpc_client = TestNodeRpcClient::new(coord_ep, controller, 1);

    let result = rpc_client.send_rollback(2, "deploy-test-1").await;
    assert!(result.is_ok(), "send_rollback should succeed, got: {:?}", result.err());
}

/// Test: health check RPC reaches target node and returns healthy.
#[tokio::test]
async fn test_health_check_rpc_reaches_target() {
    let (_target_ep, target_addr, _router) = setup_target_node(2, 400).await;

    let coord_ep = create_endpoint(102).await;

    let controller = DeterministicClusterController::new();
    let target_node = ClusterNode::with_iroh_addr(2, target_addr);
    let coord_node = ClusterNode::new(1, "coordinator", None);
    controller
        .init(aspen_cluster_types::InitRequest {
            initial_members: vec![coord_node, target_node],
        })
        .await
        .unwrap();

    let rpc_client = TestNodeRpcClient::new(coord_ep, controller, 1);

    let result = rpc_client.check_health(2).await;
    assert!(result.is_ok());
    assert!(result.unwrap(), "node should report healthy");
}

/// Test: send_upgrade to unknown node returns error.
#[tokio::test]
async fn test_send_upgrade_unknown_node_fails() {
    let coord_ep = create_endpoint(103).await;

    let controller = DeterministicClusterController::new();
    let coord_node = ClusterNode::new(1, "coordinator", None);
    controller
        .init(aspen_cluster_types::InitRequest {
            initial_members: vec![coord_node],
        })
        .await
        .unwrap();

    let rpc_client = TestNodeRpcClient::new(coord_ep, controller, 1);

    let result = rpc_client.send_upgrade(99, "deploy-1", "/nix/store/abc", None).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message.contains("node 99"), "error should mention node 99: {}", err.message);
}

/// Test: node address resolution failure when node has no iroh address.
#[tokio::test]
async fn test_node_without_iroh_addr_fails_clearly() {
    let coord_ep = create_endpoint(104).await;

    let controller = DeterministicClusterController::new();
    // Node 2 has no iroh address (legacy format)
    let node1 = ClusterNode::new(1, "node1", None);
    let node2 = ClusterNode::new(2, "node2-legacy", None);
    controller
        .init(aspen_cluster_types::InitRequest {
            initial_members: vec![node1, node2],
        })
        .await
        .unwrap();

    let rpc_client = TestNodeRpcClient::new(coord_ep, controller, 1);

    let result = rpc_client.send_upgrade(2, "deploy-1", "/nix/store/abc", None).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.message.contains("no iroh") || err.message.contains("endpoint address"),
        "should report missing address: {}",
        err.message
    );
}

/// Test: full coordinator → RPC → target path with 3-node cluster.
///
/// Sets up two target nodes, runs the deployment coordinator, and verifies
/// that upgrade RPCs are delivered to follower nodes over real iroh QUIC.
#[tokio::test]
async fn test_coordinator_sends_real_rpcs_to_followers() {
    let (_ep2, addr2, _r2) = setup_target_node(2, 500).await;
    let (_ep3, addr3, _r3) = setup_target_node(3, 600).await;

    let coord_ep = create_endpoint(105).await;
    let coord_addr = coord_ep.addr();

    // Build cluster state with all 3 nodes
    let controller = DeterministicClusterController::new();
    let node1 = ClusterNode::with_iroh_addr(1, coord_addr);
    let node2 = ClusterNode::with_iroh_addr(2, addr2);
    let node3 = ClusterNode::with_iroh_addr(3, addr3);
    controller
        .init(aspen_cluster_types::InitRequest {
            initial_members: vec![node1, node2, node3],
        })
        .await
        .unwrap();

    let kv = Arc::new(DeterministicKeyValueStore::new()) as Arc<dyn KeyValueStore>;
    let rpc_client = Arc::new(TestNodeRpcClient::new(coord_ep, controller, 1));

    let coordinator: DeploymentCoordinator<_, _, dyn aspen_traits::ClusterController> =
        DeploymentCoordinator::with_timeouts(
            kv.clone(),
            rpc_client,
            1, // leader
            5, // health_timeout_secs
            1, // poll_interval_secs
        );

    let now_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;

    let record = coordinator
        .start_deployment(
            "deploy-integration-1".into(),
            DeployArtifact::NixStorePath("/nix/store/xyz-aspen-node".into()),
            DeployStrategy::rolling(1),
            &[1, 2, 3],
            now_ms,
        )
        .await
        .expect("start_deployment should succeed");

    assert_eq!(record.deploy_id, "deploy-integration-1");

    // Run the deployment. Followers (2, 3) have real handlers so their
    // upgrade RPCs succeed. Leader (1) doesn't have a CLIENT_ALPN handler
    // attached so its self-upgrade RPC will fail. That's expected — we're
    // testing that the coordinator actually sends real RPCs over iroh.
    let result = coordinator.run_deployment("deploy-integration-1").await;

    match result {
        Ok(record) => {
            // If it fully succeeds, great — check followers are healthy
            for node in &record.nodes {
                if node.node_id != 1 {
                    assert_eq!(
                        node.status,
                        aspen_deploy::NodeDeployStatus::Healthy,
                        "follower node {} should be healthy",
                        node.node_id
                    );
                }
            }
        }
        Err(e) => {
            // Expected: leader self-upgrade fails (no handler on coord_ep).
            // The error should reference node 1 (the leader), not 2 or 3.
            let msg = e.to_string();
            assert!(
                msg.contains("node 1") || msg.contains("connection"),
                "expected leader-related failure, got: {msg}"
            );
        }
    }
}
