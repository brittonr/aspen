//! Core request handler for basic operations.
//!
//! Handles: Ping, GetHealth, GetRaftMetrics, GetNodeInfo, GetLeader, GetMetrics,
//! CheckpointWal, ListVaults, GetVaultKeys.

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::HealthResponse;
use aspen_client_api::MetricsResponse;
use aspen_client_api::NodeInfoResponse;
use aspen_client_api::RaftMetricsResponse;
use aspen_client_api::VaultKeysResponse;
use aspen_client_api::VaultListResponse;
use aspen_constants::CLIENT_RPC_REQUEST_COUNTER;
use aspen_coordination::AtomicCounter;
use aspen_coordination::CounterConfig;

use crate::context::ClientProtocolContext;
use crate::registry::RequestHandler;

/// Handler for core operations (health, metrics, node info).
pub struct CoreHandler;

#[async_trait::async_trait]
impl RequestHandler for CoreHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::Ping
                | ClientRpcRequest::GetHealth
                | ClientRpcRequest::GetRaftMetrics
                | ClientRpcRequest::GetNodeInfo
                | ClientRpcRequest::GetLeader
                | ClientRpcRequest::GetMetrics
                | ClientRpcRequest::CheckpointWal
                | ClientRpcRequest::ListVaults
                | ClientRpcRequest::GetVaultKeys { .. }
        )
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::Ping => Ok(ClientRpcResponse::Pong),

            ClientRpcRequest::GetHealth => {
                let uptime_seconds = ctx.start_time.elapsed().as_secs();

                // Get metrics to determine health status and membership count
                let (status, is_initialized, membership_node_count) = match ctx.controller.get_metrics().await {
                    Ok(metrics) => {
                        let status = if metrics.current_leader.is_some() {
                            "healthy"
                        } else {
                            "degraded"
                        };
                        // Node is initialized if it has non-empty membership (voters + learners)
                        let node_count = metrics.voters.len() + metrics.learners.len();
                        let is_init = node_count > 0;
                        (status, is_init, Some(node_count as u32))
                    }
                    Err(_) => ("unhealthy", false, None),
                };

                Ok(ClientRpcResponse::Health(HealthResponse {
                    status: status.to_string(),
                    node_id: ctx.node_id,
                    raft_node_id: Some(ctx.node_id),
                    uptime_seconds,
                    is_initialized,
                    membership_node_count,
                }))
            }

            ClientRpcRequest::GetRaftMetrics => {
                use aspen_client_api::ReplicationProgress;

                let metrics = ctx
                    .controller
                    .get_metrics()
                    .await
                    .map_err(|e| anyhow::anyhow!("failed to get Raft metrics: {}", e))?;

                // Extract replication progress if this node is leader
                // ClusterMetrics already has unwrapped types (u64 instead of NodeId/LogId)
                let replication = metrics.replication.as_ref().map(|repl_map| {
                    repl_map
                        .iter()
                        .map(|(node_id, matched_index)| ReplicationProgress {
                            node_id: *node_id,
                            matched_index: *matched_index,
                        })
                        .collect()
                });

                Ok(ClientRpcResponse::RaftMetrics(RaftMetricsResponse {
                    node_id: ctx.node_id,
                    state: format!("{:?}", metrics.state),
                    current_leader: metrics.current_leader,
                    current_term: metrics.current_term,
                    last_log_index: metrics.last_log_index,
                    last_applied_index: metrics.last_applied_index,
                    snapshot_index: metrics.snapshot_index,
                    replication,
                }))
            }

            ClientRpcRequest::GetLeader => {
                let leader =
                    ctx.controller.get_leader().await.map_err(|e| anyhow::anyhow!("failed to get leader: {}", e))?;
                Ok(ClientRpcResponse::Leader(leader))
            }

            ClientRpcRequest::GetNodeInfo => {
                // Get peer ID and addresses from endpoint provider
                let peer_id = ctx.endpoint_manager.peer_id().await;
                let addresses = ctx.endpoint_manager.addresses().await;
                Ok(ClientRpcResponse::NodeInfo(NodeInfoResponse {
                    node_id: ctx.node_id,
                    endpoint_addr: format!("{}:{:?}", peer_id, addresses),
                }))
            }

            ClientRpcRequest::GetMetrics => {
                // Return Prometheus-format metrics from Raft
                let metrics_text = match ctx.controller.get_metrics().await {
                    Ok(metrics) => {
                        let state_value = metrics.state.as_u8();
                        let is_leader: u8 = u8::from(metrics.state.is_leader());
                        let last_applied = metrics.last_applied_index.unwrap_or(0);
                        let snapshot_index = metrics.snapshot_index.unwrap_or(0);

                        // Get cluster-wide request counter
                        let request_counter = {
                            let counter = AtomicCounter::new(
                                ctx.kv_store.clone(),
                                CLIENT_RPC_REQUEST_COUNTER,
                                CounterConfig::default(),
                            );
                            counter.get().await.unwrap_or(0)
                        };

                        format!(
                            "# HELP aspen_raft_term Current Raft term\n\
                             # TYPE aspen_raft_term gauge\n\
                             aspen_raft_term{{node_id=\"{}\"}} {}\n\
                             # HELP aspen_raft_state Raft state (0=Learner, 1=Follower, 2=Candidate, 3=Leader, 4=Shutdown)\n\
                             # TYPE aspen_raft_state gauge\n\
                             aspen_raft_state{{node_id=\"{}\"}} {}\n\
                             # HELP aspen_raft_is_leader Whether this node is the leader\n\
                             # TYPE aspen_raft_is_leader gauge\n\
                             aspen_raft_is_leader{{node_id=\"{}\"}} {}\n\
                             # HELP aspen_raft_last_log_index Last log index\n\
                             # TYPE aspen_raft_last_log_index gauge\n\
                             aspen_raft_last_log_index{{node_id=\"{}\"}} {}\n\
                             # HELP aspen_raft_last_applied_index Last applied log index\n\
                             # TYPE aspen_raft_last_applied_index gauge\n\
                             aspen_raft_last_applied_index{{node_id=\"{}\"}} {}\n\
                             # HELP aspen_raft_snapshot_index Snapshot index\n\
                             # TYPE aspen_raft_snapshot_index gauge\n\
                             aspen_raft_snapshot_index{{node_id=\"{}\"}} {}\n\
                             # HELP aspen_node_uptime_seconds Node uptime in seconds\n\
                             # TYPE aspen_node_uptime_seconds counter\n\
                             aspen_node_uptime_seconds{{node_id=\"{}\"}} {}\n\
                             # HELP aspen_client_api_requests_total Total client RPC requests processed cluster-wide\n\
                             # TYPE aspen_client_api_requests_total counter\n\
                             aspen_client_api_requests_total{{node_id=\"{}\"}} {}\n",
                            ctx.node_id,
                            metrics.current_term,
                            ctx.node_id,
                            state_value,
                            ctx.node_id,
                            is_leader,
                            ctx.node_id,
                            metrics.last_log_index.unwrap_or(0),
                            ctx.node_id,
                            last_applied,
                            ctx.node_id,
                            snapshot_index,
                            ctx.node_id,
                            ctx.start_time.elapsed().as_secs(),
                            ctx.node_id,
                            request_counter
                        )
                    }
                    Err(_) => String::new(),
                };

                Ok(ClientRpcResponse::Metrics(MetricsResponse {
                    prometheus_text: metrics_text,
                }))
            }

            ClientRpcRequest::CheckpointWal => {
                use aspen_client_api::CheckpointWalResultResponse;
                // WAL checkpoint requires direct access to the SQLite state machine,
                // which is not exposed through the ClusterController/KeyValueStore traits.
                Ok(ClientRpcResponse::CheckpointWalResult(CheckpointWalResultResponse {
                    success: false,
                    pages_checkpointed: None,
                    wal_size_before_bytes: None,
                    wal_size_after_bytes: None,
                    error: Some(
                        "WAL checkpoint requires state machine access - use trigger_snapshot for log compaction"
                            .to_string(),
                    ),
                }))
            }

            ClientRpcRequest::ListVaults => {
                // Deprecated: Vault-specific operations removed in favor of flat keyspace.
                Ok(ClientRpcResponse::VaultList(VaultListResponse {
                    vaults: vec![],
                    error: Some("ListVaults is deprecated. Use ScanKeys with a prefix instead.".to_string()),
                }))
            }

            ClientRpcRequest::GetVaultKeys { vault_name } => {
                // Deprecated: Vault-specific operations removed in favor of flat keyspace.
                Ok(ClientRpcResponse::VaultKeys(VaultKeysResponse {
                    vault: vault_name,
                    keys: vec![],
                    error: Some("GetVaultKeys is deprecated. Use ScanKeys with a prefix instead.".to_string()),
                }))
            }

            _ => Err(anyhow::anyhow!("request not handled by CoreHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "CoreHandler"
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use aspen_core::inmemory::DeterministicClusterController;
    use aspen_core::inmemory::DeterministicKeyValueStore;

    use super::*;
    use crate::context::test_support::TestContextBuilder;
    use crate::test_mocks::MockEndpointProvider;
    #[cfg(feature = "sql")]
    use crate::test_mocks::mock_sql_executor;

    async fn setup_test_context() -> ClientProtocolContext {
        let controller = Arc::new(DeterministicClusterController::new());
        let kv_store = Arc::new(DeterministicKeyValueStore::new());
        let mock_endpoint = Arc::new(MockEndpointProvider::with_seed(12345).await);

        let builder = TestContextBuilder::new()
            .with_node_id(1)
            .with_controller(controller)
            .with_kv_store(kv_store)
            .with_endpoint_manager(mock_endpoint)
            .with_cookie("test_cluster");

        #[cfg(feature = "sql")]
        let builder = builder.with_sql_executor(mock_sql_executor());

        builder.build()
    }

    #[test]
    fn test_can_handle_ping() {
        let handler = CoreHandler;
        assert!(handler.can_handle(&ClientRpcRequest::Ping));
    }

    #[test]
    fn test_can_handle_health() {
        let handler = CoreHandler;
        assert!(handler.can_handle(&ClientRpcRequest::GetHealth));
    }

    #[test]
    fn test_can_handle_raft_metrics() {
        let handler = CoreHandler;
        assert!(handler.can_handle(&ClientRpcRequest::GetRaftMetrics));
    }

    #[test]
    fn test_can_handle_node_info() {
        let handler = CoreHandler;
        assert!(handler.can_handle(&ClientRpcRequest::GetNodeInfo));
    }

    #[test]
    fn test_can_handle_leader() {
        let handler = CoreHandler;
        assert!(handler.can_handle(&ClientRpcRequest::GetLeader));
    }

    #[test]
    fn test_can_handle_metrics() {
        let handler = CoreHandler;
        assert!(handler.can_handle(&ClientRpcRequest::GetMetrics));
    }

    #[test]
    fn test_can_handle_checkpoint_wal() {
        let handler = CoreHandler;
        assert!(handler.can_handle(&ClientRpcRequest::CheckpointWal));
    }

    #[test]
    fn test_can_handle_list_vaults() {
        let handler = CoreHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ListVaults));
    }

    #[test]
    fn test_can_handle_get_vault_keys() {
        let handler = CoreHandler;
        assert!(handler.can_handle(&ClientRpcRequest::GetVaultKeys {
            vault_name: "test".to_string(),
        }));
    }

    #[test]
    fn test_rejects_unrelated_requests() {
        let handler = CoreHandler;

        // KV requests
        assert!(!handler.can_handle(&ClientRpcRequest::ReadKey {
            key: "test".to_string(),
        }));
        assert!(!handler.can_handle(&ClientRpcRequest::WriteKey {
            key: "test".to_string(),
            value: vec![],
        }));

        // Coordination requests
        assert!(!handler.can_handle(&ClientRpcRequest::LockAcquire {
            key: "test".to_string(),
            holder_id: "holder".to_string(),
            ttl_ms: 30000,
            timeout_ms: 0,
        }));

        // Cluster requests
        assert!(!handler.can_handle(&ClientRpcRequest::InitCluster));
        assert!(!handler.can_handle(&ClientRpcRequest::GetClusterState));
    }

    #[test]
    fn test_handler_name() {
        let handler = CoreHandler;
        assert_eq!(handler.name(), "CoreHandler");
    }

    #[tokio::test]
    async fn test_handle_ping() {
        let ctx = setup_test_context().await;
        let handler = CoreHandler;

        let result = handler.handle(ClientRpcRequest::Ping, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::Pong => (),
            other => panic!("expected Pong, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_handle_health() {
        let ctx = setup_test_context().await;
        let handler = CoreHandler;

        let result = handler.handle(ClientRpcRequest::GetHealth, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::Health(response) => {
                assert_eq!(response.node_id, 1);
                // uptime_seconds is u64 so always >= 0
                // Status should be one of: healthy, degraded, unhealthy
                assert!(
                    response.status == "healthy" || response.status == "degraded" || response.status == "unhealthy"
                );
            }
            other => panic!("expected Health, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_handle_node_info() {
        let ctx = setup_test_context().await;
        let handler = CoreHandler;

        let result = handler.handle(ClientRpcRequest::GetNodeInfo, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::NodeInfo(response) => {
                assert_eq!(response.node_id, 1);
                assert!(!response.endpoint_addr.is_empty());
            }
            other => panic!("expected NodeInfo, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_handle_checkpoint_wal_returns_not_supported() {
        let ctx = setup_test_context().await;
        let handler = CoreHandler;

        let result = handler.handle(ClientRpcRequest::CheckpointWal, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::CheckpointWalResult(response) => {
                // WAL checkpoint is not supported via trait interface
                assert!(!response.success);
                assert!(response.error.is_some());
            }
            other => panic!("expected CheckpointWalResult, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_handle_list_vaults_deprecated() {
        let ctx = setup_test_context().await;
        let handler = CoreHandler;

        let result = handler.handle(ClientRpcRequest::ListVaults, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::VaultList(response) => {
                // ListVaults is deprecated
                assert!(response.vaults.is_empty());
                assert!(response.error.is_some());
                assert!(response.error.unwrap().contains("deprecated"));
            }
            other => panic!("expected VaultList, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_handle_get_vault_keys_deprecated() {
        let ctx = setup_test_context().await;
        let handler = CoreHandler;

        let request = ClientRpcRequest::GetVaultKeys {
            vault_name: "test_vault".to_string(),
        };

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_ok());

        match result.unwrap() {
            ClientRpcResponse::VaultKeys(response) => {
                // GetVaultKeys is deprecated
                assert_eq!(response.vault, "test_vault");
                assert!(response.keys.is_empty());
                assert!(response.error.is_some());
                assert!(response.error.unwrap().contains("deprecated"));
            }
            other => panic!("expected VaultKeys, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_handle_unhandled_request() {
        let ctx = setup_test_context().await;
        let handler = CoreHandler;

        // This request is not handled by CoreHandler
        let request = ClientRpcRequest::ReadKey {
            key: "test".to_string(),
        };

        let result = handler.handle(request, &ctx).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not handled"));
    }
}
