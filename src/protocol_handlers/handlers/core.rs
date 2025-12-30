//! Core request handler for basic operations.
//!
//! Handles: Ping, GetHealth, GetRaftMetrics, GetNodeInfo, GetLeader, GetMetrics,
//! CheckpointWal, ListVaults, GetVaultKeys.

use super::ClientProtocolContext;
use super::RequestHandler;
use crate::client_rpc::ClientRpcRequest;
use crate::client_rpc::ClientRpcResponse;
use crate::client_rpc::HealthResponse;
use crate::client_rpc::MetricsResponse;
use crate::client_rpc::NodeInfoResponse;
use crate::client_rpc::RaftMetricsResponse;
use crate::client_rpc::VaultKeysResponse;
use crate::client_rpc::VaultListResponse;
use crate::coordination::AtomicCounter;
use crate::coordination::CounterConfig;
use crate::raft::constants::CLIENT_RPC_REQUEST_COUNTER;

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

                let status = match ctx.controller.get_metrics().await {
                    Ok(metrics) => {
                        if metrics.current_leader.is_some() {
                            "healthy"
                        } else {
                            "degraded"
                        }
                    }
                    Err(_) => "unhealthy",
                };

                Ok(ClientRpcResponse::Health(HealthResponse {
                    status: status.to_string(),
                    node_id: ctx.node_id,
                    raft_node_id: Some(ctx.node_id),
                    uptime_seconds,
                }))
            }

            ClientRpcRequest::GetRaftMetrics => {
                use crate::client_rpc::ReplicationProgress;

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
                let leader = ctx
                    .controller
                    .get_leader()
                    .await
                    .map_err(|e| anyhow::anyhow!("failed to get leader: {}", e))?;
                Ok(ClientRpcResponse::Leader(leader))
            }

            ClientRpcRequest::GetNodeInfo => {
                let endpoint_addr = ctx.endpoint_manager.node_addr();
                Ok(ClientRpcResponse::NodeInfo(NodeInfoResponse {
                    node_id: ctx.node_id,
                    endpoint_addr: format!("{:?}", endpoint_addr),
                }))
            }

            ClientRpcRequest::GetMetrics => {
                // Return Prometheus-format metrics from Raft
                let metrics_text = match ctx.controller.get_metrics().await {
                    Ok(metrics) => {
                        let state_value = match metrics.state {
                            openraft::ServerState::Learner => 0,
                            openraft::ServerState::Follower => 1,
                            openraft::ServerState::Candidate => 2,
                            openraft::ServerState::Leader => 3,
                            openraft::ServerState::Shutdown => 4,
                        };
                        let is_leader: u8 = u8::from(metrics.state == openraft::ServerState::Leader);
                        let last_applied = metrics.last_applied_index.unwrap_or(0);
                        let snapshot_index = metrics.snapshot_index.unwrap_or(0);

                        // Get cluster-wide request counter
                        let request_counter = {
                            let counter =
                                AtomicCounter::new(ctx.kv_store.clone(), CLIENT_RPC_REQUEST_COUNTER, CounterConfig::default());
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
                             # HELP aspen_client_rpc_requests_total Total client RPC requests processed cluster-wide\n\
                             # TYPE aspen_client_rpc_requests_total counter\n\
                             aspen_client_rpc_requests_total{{node_id=\"{}\"}} {}\n",
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

                Ok(ClientRpcResponse::Metrics(MetricsResponse { prometheus_text: metrics_text }))
            }

            ClientRpcRequest::CheckpointWal => {
                use crate::client_rpc::CheckpointWalResultResponse;
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
