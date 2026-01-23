//! Iroh client for connecting to Aspen nodes over QUIC.
//!
//! This module provides the client-side implementation for connecting to
//! Aspen nodes using Iroh's peer-to-peer transport with the Client ALPN.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use aspen_client::AddLearnerResultResponse;
use aspen_client::AspenClusterTicket;
use aspen_client::CLIENT_ALPN;
use aspen_client::ChangeMembershipResultResponse;
use aspen_client::CheckpointWalResultResponse;
use aspen_client::ClientRpcRequest;
use aspen_client::ClientRpcResponse;
use aspen_client::ClusterStateResponse;
use aspen_client::ClusterTicketResponse;
use aspen_client::DeleteResultResponse;
use aspen_client::HealthResponse;
use aspen_client::InitResultResponse;
use aspen_client::MAX_CLIENT_MESSAGE_SIZE;
use aspen_client::MetricsResponse;
use aspen_client::NodeDescriptor;
use aspen_client::NodeInfoResponse;
use aspen_client::PromoteLearnerResultResponse;
use aspen_client::RaftMetricsResponse;
use aspen_client::ReadResultResponse;
use aspen_client::ScanResultResponse;
use aspen_client::SnapshotResultResponse;
use aspen_client::SqlResultResponse;
use aspen_client::WriteResultResponse;
use iroh::Endpoint;
use iroh::EndpointAddr;
use iroh::SecretKey;
use tokio::sync::RwLock;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::types::JobInfo;
use crate::types::QueueStats;
use crate::types::WorkerInfo;
use crate::types::WorkerPoolInfo;

/// Timeout for individual RPC calls.
/// Reduced for TUI to keep UI responsive during network issues.
const RPC_TIMEOUT: Duration = Duration::from_secs(2); // Reduced from 10s

/// Connection retry delay.
/// Shorter delay for TUI to fail fast and keep UI responsive.
const RETRY_DELAY: Duration = Duration::from_millis(500); // Reduced from 5s

/// Maximum connection retries before giving up.
/// Balance between responsiveness and reliability on lossy networks.
const MAX_RETRIES: u32 = 3;

/// Iroh client for TUI connections to Aspen nodes.
pub struct IrohClient {
    /// Iroh endpoint for making connections.
    endpoint: Endpoint,
    /// Target node address.
    target_addr: Arc<RwLock<EndpointAddr>>,
    /// Connection status.
    connected: Arc<RwLock<bool>>,
}

impl IrohClient {
    /// Create a new Iroh client for TUI connections.
    ///
    /// # Arguments
    /// * `target_addr` - The endpoint address of the target node
    ///
    /// # Returns
    /// A new IrohClient instance.
    pub async fn new(target_addr: EndpointAddr) -> Result<Self> {
        // Generate a new secret key for the TUI client
        use rand::RngCore;
        let mut key_bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut key_bytes);
        let secret_key = SecretKey::from(key_bytes);

        // Build the Iroh endpoint with TUI ALPN
        let endpoint = Endpoint::builder()
            .secret_key(secret_key)
            .alpns(vec![CLIENT_ALPN.to_vec()])
            .bind()
            .await
            .context("failed to bind Iroh endpoint")?;

        info!(
            node_id = %endpoint.id(),
            target_node_id = %target_addr.id,
            "TUI client endpoint created"
        );

        Ok(Self {
            endpoint,
            target_addr: Arc::new(RwLock::new(target_addr)),
            connected: Arc::new(RwLock::new(false)),
        })
    }

    /// Update the target node address.
    pub async fn set_target(&self, addr: EndpointAddr) {
        let mut target = self.target_addr.write().await;
        *target = addr;

        // Mark as disconnected so next request will reconnect
        let mut connected = self.connected.write().await;
        *connected = false;
    }

    /// Send an RPC request to the target node.
    async fn send_rpc(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        let target_addr = self.target_addr.read().await.clone();

        debug!(
            target_node_id = %target_addr.id,
            request_type = ?request,
            "sending TUI RPC request"
        );

        // Connect to the target node
        let connection = self.endpoint.connect(target_addr, CLIENT_ALPN).await.context("failed to connect to node")?;

        // Open a bidirectional stream for the RPC
        let (mut send, mut recv) = connection.open_bi().await.context("failed to open stream")?;

        // Serialize and send the request
        let request_bytes = postcard::to_stdvec(&request).context("failed to serialize request")?;

        send.write_all(&request_bytes).await.context("failed to send request")?;
        send.finish().context("failed to finish send stream")?;

        // Read the response
        let response_bytes = recv.read_to_end(MAX_CLIENT_MESSAGE_SIZE).await.context("failed to read response")?;

        // Deserialize the response
        let response: ClientRpcResponse =
            postcard::from_bytes(&response_bytes).context("failed to deserialize response")?;

        // Check for error response
        if let ClientRpcResponse::Error(err) = &response {
            anyhow::bail!("RPC error {}: {}", err.code, err.message);
        }

        // Mark as connected on successful RPC
        let mut connected = self.connected.write().await;
        *connected = true;

        Ok(response)
    }

    /// Send an RPC request with automatic retry on failure.
    async fn send_rpc_with_retry(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        let mut retries = 0;

        loop {
            match tokio::time::timeout(RPC_TIMEOUT, self.send_rpc(request.clone())).await {
                Ok(Ok(response)) => return Ok(response),
                Ok(Err(err)) => {
                    warn!(
                        error = %err,
                        retries,
                        max_retries = MAX_RETRIES,
                        "RPC request failed"
                    );

                    if retries >= MAX_RETRIES {
                        return Err(err).context("max retries exceeded");
                    }

                    retries += 1;
                    tokio::time::sleep(RETRY_DELAY).await;
                }
                Err(_) => {
                    warn!(retries, max_retries = MAX_RETRIES, "RPC request timed out");

                    if retries >= MAX_RETRIES {
                        anyhow::bail!("RPC request timed out after {} retries", MAX_RETRIES);
                    }

                    retries += 1;
                    tokio::time::sleep(RETRY_DELAY).await;
                }
            }
        }
    }

    /// Check if the client is connected.
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    /// Get health status from the node.
    pub async fn get_health(&self) -> Result<HealthResponse> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::GetHealth).await?;

        match response {
            ClientRpcResponse::Health(health) => Ok(health),
            _ => anyhow::bail!("unexpected response type for GetHealth"),
        }
    }

    /// Get Raft metrics from the node.
    pub async fn get_raft_metrics(&self) -> Result<RaftMetricsResponse> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::GetRaftMetrics).await?;

        match response {
            ClientRpcResponse::RaftMetrics(metrics) => Ok(metrics),
            _ => anyhow::bail!("unexpected response type for GetRaftMetrics"),
        }
    }

    /// Get the current leader node ID.
    pub async fn get_leader(&self) -> Result<Option<u64>> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::GetLeader).await?;

        match response {
            ClientRpcResponse::Leader(leader) => Ok(leader),
            _ => anyhow::bail!("unexpected response type for GetLeader"),
        }
    }

    /// Get node information.
    pub async fn get_node_info(&self) -> Result<NodeInfoResponse> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::GetNodeInfo).await?;

        match response {
            ClientRpcResponse::NodeInfo(info) => Ok(info),
            _ => anyhow::bail!("unexpected response type for GetNodeInfo"),
        }
    }

    /// Get cluster ticket for joining.
    pub async fn get_cluster_ticket(&self) -> Result<ClusterTicketResponse> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::GetClusterTicket).await?;

        match response {
            ClientRpcResponse::ClusterTicket(ticket) => Ok(ticket),
            _ => anyhow::bail!("unexpected response type for GetClusterTicket"),
        }
    }

    /// Initialize a new cluster.
    pub async fn init_cluster(&self) -> Result<InitResultResponse> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::InitCluster).await?;

        match response {
            ClientRpcResponse::InitResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for InitCluster"),
        }
    }

    /// Read a key from the key-value store.
    pub async fn read_key(&self, key: String) -> Result<ReadResultResponse> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::ReadKey { key }).await?;

        match response {
            ClientRpcResponse::ReadResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for ReadKey"),
        }
    }

    /// Write a key-value pair to the store.
    pub async fn write_key(&self, key: String, value: Vec<u8>) -> Result<WriteResultResponse> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::WriteKey { key, value }).await?;

        match response {
            ClientRpcResponse::WriteResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for WriteKey"),
        }
    }

    /// Trigger a Raft snapshot.
    pub async fn trigger_snapshot(&self) -> Result<SnapshotResultResponse> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::TriggerSnapshot).await?;

        match response {
            ClientRpcResponse::SnapshotResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for TriggerSnapshot"),
        }
    }

    /// Add a learner node to the cluster.
    pub async fn add_learner(&self, node_id: u64, addr: String) -> Result<AddLearnerResultResponse> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::AddLearner { node_id, addr }).await?;

        match response {
            ClientRpcResponse::AddLearnerResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for AddLearner"),
        }
    }

    /// Change cluster membership.
    pub async fn change_membership(&self, members: Vec<u64>) -> Result<ChangeMembershipResultResponse> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::ChangeMembership { members }).await?;

        match response {
            ClientRpcResponse::ChangeMembershipResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for ChangeMembership"),
        }
    }

    /// Send a ping to check connectivity.
    pub async fn ping(&self) -> Result<()> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::Ping).await?;

        match response {
            ClientRpcResponse::Pong => Ok(()),
            _ => anyhow::bail!("unexpected response type for Ping"),
        }
    }

    /// Get cluster state with all known nodes.
    pub async fn get_cluster_state(&self) -> Result<ClusterStateResponse> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::GetClusterState).await?;

        match response {
            ClientRpcResponse::ClusterState(state) => Ok(state),
            _ => anyhow::bail!("unexpected response type for GetClusterState"),
        }
    }

    /// Delete a key from the key-value store.
    pub async fn delete_key(&self, key: String) -> Result<DeleteResultResponse> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::DeleteKey { key }).await?;

        match response {
            ClientRpcResponse::DeleteResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for DeleteKey"),
        }
    }

    /// Scan keys with a prefix from the key-value store.
    pub async fn scan_keys(
        &self,
        prefix: String,
        limit: Option<u32>,
        continuation_token: Option<String>,
    ) -> Result<ScanResultResponse> {
        let response = self
            .send_rpc_with_retry(ClientRpcRequest::ScanKeys {
                prefix,
                limit,
                continuation_token,
            })
            .await?;

        match response {
            ClientRpcResponse::ScanResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for ScanKeys"),
        }
    }

    /// Get Prometheus-format metrics from the node.
    pub async fn get_metrics(&self) -> Result<MetricsResponse> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::GetMetrics).await?;

        match response {
            ClientRpcResponse::Metrics(metrics) => Ok(metrics),
            _ => anyhow::bail!("unexpected response type for GetMetrics"),
        }
    }

    /// Promote a learner node to voter.
    pub async fn promote_learner(
        &self,
        learner_id: u64,
        replace_node: Option<u64>,
        force: bool,
    ) -> Result<PromoteLearnerResultResponse> {
        let response = self
            .send_rpc_with_retry(ClientRpcRequest::PromoteLearner {
                learner_id,
                replace_node,
                force,
            })
            .await?;

        match response {
            ClientRpcResponse::PromoteLearnerResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for PromoteLearner"),
        }
    }

    /// Force a SQLite WAL checkpoint.
    pub async fn checkpoint_wal(&self) -> Result<CheckpointWalResultResponse> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::CheckpointWal).await?;

        match response {
            ClientRpcResponse::CheckpointWalResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for CheckpointWal"),
        }
    }

    /// Execute a SQL query against the cluster.
    pub async fn execute_sql(
        &self,
        query: String,
        consistency: String,
        limit: Option<u32>,
        timeout_ms: Option<u32>,
    ) -> Result<SqlResultResponse> {
        let response = self
            .send_rpc_with_retry(ClientRpcRequest::ExecuteSql {
                query,
                params: "[]".to_string(), // Empty params array
                consistency,
                limit,
                timeout_ms,
            })
            .await?;

        match response {
            ClientRpcResponse::SqlResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for ExecuteSql"),
        }
    }

    /// List jobs with optional status filter.
    pub async fn list_jobs(&self, status: Option<String>, limit: Option<u32>) -> Result<Vec<JobInfo>> {
        let response = self
            .send_rpc_with_retry(ClientRpcRequest::JobList {
                status,
                job_type: None,
                tags: vec![],
                limit,
                continuation_token: None,
            })
            .await?;

        match response {
            ClientRpcResponse::JobListResult(result) => {
                if let Some(error) = result.error {
                    anyhow::bail!("Failed to list jobs: {}", error);
                }
                Ok(result
                    .jobs
                    .into_iter()
                    .map(|j| JobInfo {
                        job_id: j.job_id,
                        job_type: j.job_type,
                        status: j.status,
                        priority: j.priority,
                        progress: j.progress,
                        progress_message: j.progress_message,
                        tags: j.tags,
                        submitted_at: j.submitted_at,
                        started_at: j.started_at,
                        completed_at: j.completed_at,
                        worker_id: j.worker_id,
                        attempts: j.attempts,
                        error_message: j.error_message,
                    })
                    .collect())
            }
            _ => anyhow::bail!("unexpected response type for JobList"),
        }
    }

    /// Get job queue statistics.
    pub async fn get_queue_stats(&self) -> Result<QueueStats> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::JobQueueStats).await?;

        match response {
            ClientRpcResponse::JobQueueStatsResult(result) => {
                if let Some(error) = result.error {
                    anyhow::bail!("Failed to get queue stats: {}", error);
                }
                Ok(QueueStats {
                    pending_count: result.pending_count,
                    scheduled_count: result.scheduled_count,
                    running_count: result.running_count,
                    completed_count: result.completed_count,
                    failed_count: result.failed_count,
                    cancelled_count: result.cancelled_count,
                    priority_counts: result.priority_counts.into_iter().map(|pc| (pc.priority, pc.count)).collect(),
                    type_counts: result.type_counts.into_iter().map(|tc| (tc.job_type, tc.count)).collect(),
                })
            }
            _ => anyhow::bail!("unexpected response type for JobQueueStats"),
        }
    }

    /// Get worker pool status.
    pub async fn get_worker_status(&self) -> Result<WorkerPoolInfo> {
        let response = self.send_rpc_with_retry(ClientRpcRequest::WorkerStatus).await?;

        match response {
            ClientRpcResponse::WorkerStatusResult(result) => {
                if let Some(error) = result.error {
                    anyhow::bail!("Failed to get worker status: {}", error);
                }
                Ok(WorkerPoolInfo {
                    workers: result
                        .workers
                        .into_iter()
                        .map(|w| WorkerInfo {
                            worker_id: w.worker_id,
                            status: w.status,
                            capabilities: w.capabilities,
                            capacity: w.capacity,
                            active_jobs: w.active_jobs,
                            active_job_ids: w.active_job_ids,
                            last_heartbeat: w.last_heartbeat,
                            total_processed: w.total_processed,
                            total_failed: w.total_failed,
                        })
                        .collect(),
                    total_workers: result.total_workers,
                    idle_workers: result.idle_workers,
                    busy_workers: result.busy_workers,
                    offline_workers: result.offline_workers,
                    total_capacity: result.total_capacity,
                    used_capacity: result.used_capacity,
                })
            }
            _ => anyhow::bail!("unexpected response type for WorkerStatus"),
        }
    }

    /// Cancel a job.
    pub async fn cancel_job(&self, job_id: &str, reason: Option<String>) -> Result<()> {
        let response = self
            .send_rpc_with_retry(ClientRpcRequest::JobCancel {
                job_id: job_id.to_string(),
                reason,
            })
            .await?;

        match response {
            ClientRpcResponse::JobCancelResult(result) => {
                if result.success {
                    Ok(())
                } else {
                    anyhow::bail!(
                        "Failed to cancel job: {}",
                        result.error.unwrap_or_else(|| "Unknown error".to_string())
                    )
                }
            }
            _ => anyhow::bail!("unexpected response type for JobCancel"),
        }
    }

    /// Shutdown the client and close all connections.
    pub async fn shutdown(self) -> Result<()> {
        self.endpoint.close().await;
        Ok(())
    }
}

/// Parse an Aspen cluster ticket into a list of EndpointAddrs.
///
/// Tickets have the format: "aspen{base32-encoded-data}"
/// Returns all bootstrap peers in the ticket.
pub fn parse_cluster_ticket(ticket: &str) -> Result<Vec<EndpointAddr>> {
    // AspenClusterTicket is imported at the top of the module

    let ticket = AspenClusterTicket::deserialize(ticket)?;

    // Extract ALL bootstrap peer endpoint IDs
    let endpoints: Vec<EndpointAddr> = ticket.bootstrap.iter().map(|peer_id| EndpointAddr::new(*peer_id)).collect();

    if endpoints.is_empty() {
        anyhow::bail!("no bootstrap peers in ticket");
    }

    Ok(endpoints)
}

/// Maximum number of nodes to track in multi-node client.
///
/// Tiger Style: Bounded to prevent unbounded resource growth.
const MAX_TRACKED_NODES: usize = 16;

/// Connection state for a single node.
#[derive(Debug)]
pub struct NodeConnection {
    /// Node identifier.
    pub node_id: u64,
    /// Target endpoint address.
    pub endpoint_addr: EndpointAddr,
    /// Whether the node is currently reachable.
    pub is_reachable: bool,
    /// Whether this node is the leader.
    pub is_leader: bool,
    /// Whether this node is a voter.
    pub is_voter: bool,
}

/// Multi-node client for connecting to and managing multiple cluster nodes.
///
/// Tiger Style:
/// - Bounded node count (MAX_TRACKED_NODES)
/// - Shared Iroh endpoint across all connections
/// - Automatic discovery via GetClusterState RPC
pub struct MultiNodeClient {
    /// Shared Iroh endpoint for all connections.
    endpoint: Arc<Endpoint>,
    /// Known nodes and their connection state.
    nodes: RwLock<std::collections::HashMap<u64, NodeConnection>>,
    /// Primary target node for discovery queries.
    primary_target: RwLock<EndpointAddr>,
    /// Current leader node ID, if known.
    leader_id: RwLock<Option<u64>>,
}

impl MultiNodeClient {
    /// Create a new multi-node client.
    ///
    /// # Arguments
    /// * `initial_targets` - Bootstrap endpoint addresses to connect to
    ///
    /// # Returns
    /// A new MultiNodeClient instance.
    pub async fn new(initial_targets: Vec<EndpointAddr>) -> Result<Self> {
        if initial_targets.is_empty() {
            anyhow::bail!("at least one bootstrap target is required");
        }

        // Generate a new secret key for the TUI client
        use rand::RngCore;
        let mut key_bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut key_bytes);
        let secret_key = SecretKey::from(key_bytes);

        // Build the shared Iroh endpoint with TUI ALPN
        // Add a timeout to prevent indefinite blocking on bind()
        let bind_timeout = Duration::from_secs(10);
        let endpoint = tokio::time::timeout(
            bind_timeout,
            Endpoint::builder().secret_key(secret_key).alpns(vec![CLIENT_ALPN.to_vec()]).bind(),
        )
        .await
        .context("timeout waiting for Iroh endpoint to bind")?
        .context("failed to bind Iroh endpoint")?;

        info!(
            node_id = %endpoint.id(),
            bootstrap_count = initial_targets.len(),
            "Multi-node TUI client endpoint created"
        );

        let primary_target = initial_targets[0].clone();

        Ok(Self {
            endpoint: Arc::new(endpoint),
            nodes: RwLock::new(std::collections::HashMap::new()),
            primary_target: RwLock::new(primary_target),
            leader_id: RwLock::new(None),
        })
    }

    /// Send an RPC request to a specific node address.
    async fn send_rpc_to(&self, target: &EndpointAddr, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        debug!(
            target_node_id = %target.id,
            request_type = ?request,
            "sending TUI RPC request"
        );

        // Connect to the target node
        let connection =
            self.endpoint.connect(target.clone(), CLIENT_ALPN).await.context("failed to connect to node")?;

        // Open a bidirectional stream for the RPC
        let (mut send, mut recv) = connection.open_bi().await.context("failed to open stream")?;

        // Serialize and send the request
        let request_bytes = postcard::to_stdvec(&request).context("failed to serialize request")?;

        send.write_all(&request_bytes).await.context("failed to send request")?;
        send.finish().context("failed to finish send stream")?;

        // Read the response
        let response_bytes = recv.read_to_end(MAX_CLIENT_MESSAGE_SIZE).await.context("failed to read response")?;

        // Deserialize the response
        let response: ClientRpcResponse =
            postcard::from_bytes(&response_bytes).context("failed to deserialize response")?;

        // Check for error response
        if let ClientRpcResponse::Error(err) = &response {
            anyhow::bail!("RPC error {}: {}", err.code, err.message);
        }

        Ok(response)
    }

    /// Send an RPC to the primary target with retry.
    async fn send_rpc_primary(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        let target = self.primary_target.read().await.clone();
        let mut retries = 0;

        loop {
            match tokio::time::timeout(RPC_TIMEOUT, self.send_rpc_to(&target, request.clone())).await {
                Ok(Ok(response)) => return Ok(response),
                Ok(Err(err)) => {
                    warn!(
                        error = %err,
                        retries,
                        max_retries = MAX_RETRIES,
                        "RPC request failed"
                    );

                    if retries >= MAX_RETRIES {
                        return Err(err).context("max retries exceeded");
                    }

                    retries += 1;
                    tokio::time::sleep(RETRY_DELAY).await;
                }
                Err(_) => {
                    warn!(retries, max_retries = MAX_RETRIES, "RPC request timed out");

                    if retries >= MAX_RETRIES {
                        anyhow::bail!("RPC request timed out after {} retries", MAX_RETRIES);
                    }

                    retries += 1;
                    tokio::time::sleep(RETRY_DELAY).await;
                }
            }
        }
    }

    /// Discover peers by querying the primary node for cluster state.
    ///
    /// Updates internal node tracking with discovered peers.
    pub async fn discover_peers(&self) -> Result<Vec<NodeDescriptor>> {
        let response = self.send_rpc_primary(ClientRpcRequest::GetClusterState).await?;

        match response {
            ClientRpcResponse::ClusterState(state) => {
                // Update leader ID
                {
                    let mut leader = self.leader_id.write().await;
                    *leader = state.leader_id;
                }

                // Update node tracking
                let mut nodes = self.nodes.write().await;

                // Tiger Style: Bounded node count
                for descriptor in state.nodes.iter().take(MAX_TRACKED_NODES) {
                    // Parse the endpoint address from the descriptor
                    // The endpoint_addr is stored as a debug string, so we need to handle this
                    // For now, we'll just track what we know
                    nodes.entry(descriptor.node_id).and_modify(|conn| {
                        conn.is_leader = descriptor.is_leader;
                        conn.is_voter = descriptor.is_voter;
                        conn.is_reachable = true;
                    });
                }

                info!(
                    discovered_nodes = state.nodes.len(),
                    leader_id = ?state.leader_id,
                    "discovered cluster peers"
                );

                Ok(state.nodes)
            }
            _ => anyhow::bail!("unexpected response type for GetClusterState"),
        }
    }

    /// Add a new node to track.
    ///
    /// # Arguments
    /// * `node_id` - Node identifier
    /// * `endpoint_addr` - Endpoint address for the node
    pub async fn add_node(&self, node_id: u64, endpoint_addr: EndpointAddr) {
        let mut nodes = self.nodes.write().await;

        // Tiger Style: Bounded node count
        if nodes.len() >= MAX_TRACKED_NODES && !nodes.contains_key(&node_id) {
            warn!(node_id, max_nodes = MAX_TRACKED_NODES, "node limit reached, not adding new node");
            return;
        }

        nodes.insert(
            node_id,
            NodeConnection {
                node_id,
                endpoint_addr,
                is_reachable: false,
                is_leader: false,
                is_voter: false,
            },
        );

        info!(node_id, "added node to tracking");
    }

    /// Remove a node from tracking.
    pub async fn remove_node(&self, node_id: u64) {
        let mut nodes = self.nodes.write().await;
        nodes.remove(&node_id);
        info!(node_id, "removed node from tracking");
    }

    /// Get all tracked nodes.
    pub async fn get_nodes(&self) -> Vec<NodeConnection> {
        let nodes = self.nodes.read().await;
        nodes
            .values()
            .map(|conn| NodeConnection {
                node_id: conn.node_id,
                endpoint_addr: conn.endpoint_addr.clone(),
                is_reachable: conn.is_reachable,
                is_leader: conn.is_leader,
                is_voter: conn.is_voter,
            })
            .collect()
    }

    /// Get the current leader ID.
    pub async fn get_leader_id(&self) -> Option<u64> {
        *self.leader_id.read().await
    }

    /// Set the primary target for discovery queries.
    pub async fn set_primary_target(&self, addr: EndpointAddr) {
        let mut target = self.primary_target.write().await;
        *target = addr;
    }

    /// Get node info from the primary target.
    pub async fn get_node_info(&self) -> Result<NodeInfoResponse> {
        let response = self.send_rpc_primary(ClientRpcRequest::GetNodeInfo).await?;

        match response {
            ClientRpcResponse::NodeInfo(info) => Ok(info),
            _ => anyhow::bail!("unexpected response type for GetNodeInfo"),
        }
    }

    /// Get health from the primary target.
    pub async fn get_health(&self) -> Result<HealthResponse> {
        let response = self.send_rpc_primary(ClientRpcRequest::GetHealth).await?;

        match response {
            ClientRpcResponse::Health(health) => Ok(health),
            _ => anyhow::bail!("unexpected response type for GetHealth"),
        }
    }

    /// Get Raft metrics from the primary target.
    pub async fn get_raft_metrics(&self) -> Result<RaftMetricsResponse> {
        let response = self.send_rpc_primary(ClientRpcRequest::GetRaftMetrics).await?;

        match response {
            ClientRpcResponse::RaftMetrics(metrics) => Ok(metrics),
            _ => anyhow::bail!("unexpected response type for GetRaftMetrics"),
        }
    }

    /// Initialize the cluster via the primary target.
    pub async fn init_cluster(&self) -> Result<InitResultResponse> {
        let response = self.send_rpc_primary(ClientRpcRequest::InitCluster).await?;

        match response {
            ClientRpcResponse::InitResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for InitCluster"),
        }
    }

    /// Read a key from the cluster.
    pub async fn read_key(&self, key: String) -> Result<ReadResultResponse> {
        let response = self.send_rpc_primary(ClientRpcRequest::ReadKey { key }).await?;

        match response {
            ClientRpcResponse::ReadResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for ReadKey"),
        }
    }

    /// Write a key-value pair to the cluster.
    pub async fn write_key(&self, key: String, value: Vec<u8>) -> Result<WriteResultResponse> {
        let response = self.send_rpc_primary(ClientRpcRequest::WriteKey { key, value }).await?;

        match response {
            ClientRpcResponse::WriteResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for WriteKey"),
        }
    }

    /// Trigger a Raft snapshot.
    pub async fn trigger_snapshot(&self) -> Result<SnapshotResultResponse> {
        let response = self.send_rpc_primary(ClientRpcRequest::TriggerSnapshot).await?;

        match response {
            ClientRpcResponse::SnapshotResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for TriggerSnapshot"),
        }
    }

    /// Add a learner node to the cluster.
    pub async fn add_learner(&self, node_id: u64, addr: String) -> Result<AddLearnerResultResponse> {
        let response = self.send_rpc_primary(ClientRpcRequest::AddLearner { node_id, addr }).await?;

        match response {
            ClientRpcResponse::AddLearnerResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for AddLearner"),
        }
    }

    /// Change cluster membership.
    pub async fn change_membership(&self, members: Vec<u64>) -> Result<ChangeMembershipResultResponse> {
        let response = self.send_rpc_primary(ClientRpcRequest::ChangeMembership { members }).await?;

        match response {
            ClientRpcResponse::ChangeMembershipResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for ChangeMembership"),
        }
    }

    /// Delete a key from the cluster.
    pub async fn delete_key(&self, key: String) -> Result<DeleteResultResponse> {
        let response = self.send_rpc_primary(ClientRpcRequest::DeleteKey { key }).await?;

        match response {
            ClientRpcResponse::DeleteResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for DeleteKey"),
        }
    }

    /// Scan keys with a prefix from the cluster.
    pub async fn scan_keys(
        &self,
        prefix: String,
        limit: Option<u32>,
        continuation_token: Option<String>,
    ) -> Result<ScanResultResponse> {
        let response = self
            .send_rpc_primary(ClientRpcRequest::ScanKeys {
                prefix,
                limit,
                continuation_token,
            })
            .await?;

        match response {
            ClientRpcResponse::ScanResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for ScanKeys"),
        }
    }

    /// Get Prometheus-format metrics from the primary target.
    pub async fn get_metrics(&self) -> Result<MetricsResponse> {
        let response = self.send_rpc_primary(ClientRpcRequest::GetMetrics).await?;

        match response {
            ClientRpcResponse::Metrics(metrics) => Ok(metrics),
            _ => anyhow::bail!("unexpected response type for GetMetrics"),
        }
    }

    /// Promote a learner node to voter.
    pub async fn promote_learner(
        &self,
        learner_id: u64,
        replace_node: Option<u64>,
        force: bool,
    ) -> Result<PromoteLearnerResultResponse> {
        let response = self
            .send_rpc_primary(ClientRpcRequest::PromoteLearner {
                learner_id,
                replace_node,
                force,
            })
            .await?;

        match response {
            ClientRpcResponse::PromoteLearnerResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for PromoteLearner"),
        }
    }

    /// Force a SQLite WAL checkpoint.
    pub async fn checkpoint_wal(&self) -> Result<CheckpointWalResultResponse> {
        let response = self.send_rpc_primary(ClientRpcRequest::CheckpointWal).await?;

        match response {
            ClientRpcResponse::CheckpointWalResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for CheckpointWal"),
        }
    }

    /// Execute a SQL query against the cluster.
    pub async fn execute_sql(
        &self,
        query: String,
        consistency: String,
        limit: Option<u32>,
        timeout_ms: Option<u32>,
    ) -> Result<SqlResultResponse> {
        let response = self
            .send_rpc_primary(ClientRpcRequest::ExecuteSql {
                query,
                params: "[]".to_string(), // Empty params array
                consistency,
                limit,
                timeout_ms,
            })
            .await?;

        match response {
            ClientRpcResponse::SqlResult(result) => Ok(result),
            _ => anyhow::bail!("unexpected response type for ExecuteSql"),
        }
    }

    /// List jobs with optional status filter.
    pub async fn list_jobs(&self, status: Option<String>, limit: Option<u32>) -> Result<Vec<JobInfo>> {
        let response = self
            .send_rpc_primary(ClientRpcRequest::JobList {
                status,
                job_type: None,
                tags: vec![],
                limit,
                continuation_token: None,
            })
            .await?;

        match response {
            ClientRpcResponse::JobListResult(result) => {
                if let Some(error) = result.error {
                    anyhow::bail!("Failed to list jobs: {}", error);
                }
                Ok(result
                    .jobs
                    .into_iter()
                    .map(|j| JobInfo {
                        job_id: j.job_id,
                        job_type: j.job_type,
                        status: j.status,
                        priority: j.priority,
                        progress: j.progress,
                        progress_message: j.progress_message,
                        tags: j.tags,
                        submitted_at: j.submitted_at,
                        started_at: j.started_at,
                        completed_at: j.completed_at,
                        worker_id: j.worker_id,
                        attempts: j.attempts,
                        error_message: j.error_message,
                    })
                    .collect())
            }
            _ => anyhow::bail!("unexpected response type for JobList"),
        }
    }

    /// Get job queue statistics.
    pub async fn get_queue_stats(&self) -> Result<QueueStats> {
        let response = self.send_rpc_primary(ClientRpcRequest::JobQueueStats).await?;

        match response {
            ClientRpcResponse::JobQueueStatsResult(result) => {
                if let Some(error) = result.error {
                    anyhow::bail!("Failed to get queue stats: {}", error);
                }
                Ok(QueueStats {
                    pending_count: result.pending_count,
                    scheduled_count: result.scheduled_count,
                    running_count: result.running_count,
                    completed_count: result.completed_count,
                    failed_count: result.failed_count,
                    cancelled_count: result.cancelled_count,
                    priority_counts: result.priority_counts.into_iter().map(|pc| (pc.priority, pc.count)).collect(),
                    type_counts: result.type_counts.into_iter().map(|tc| (tc.job_type, tc.count)).collect(),
                })
            }
            _ => anyhow::bail!("unexpected response type for JobQueueStats"),
        }
    }

    /// Get worker pool status.
    pub async fn get_worker_status(&self) -> Result<WorkerPoolInfo> {
        let response = self.send_rpc_primary(ClientRpcRequest::WorkerStatus).await?;

        match response {
            ClientRpcResponse::WorkerStatusResult(result) => {
                if let Some(error) = result.error {
                    anyhow::bail!("Failed to get worker status: {}", error);
                }
                Ok(WorkerPoolInfo {
                    workers: result
                        .workers
                        .into_iter()
                        .map(|w| WorkerInfo {
                            worker_id: w.worker_id,
                            status: w.status,
                            capabilities: w.capabilities,
                            capacity: w.capacity,
                            active_jobs: w.active_jobs,
                            active_job_ids: w.active_job_ids,
                            last_heartbeat: w.last_heartbeat,
                            total_processed: w.total_processed,
                            total_failed: w.total_failed,
                        })
                        .collect(),
                    total_workers: result.total_workers,
                    idle_workers: result.idle_workers,
                    busy_workers: result.busy_workers,
                    offline_workers: result.offline_workers,
                    total_capacity: result.total_capacity,
                    used_capacity: result.used_capacity,
                })
            }
            _ => anyhow::bail!("unexpected response type for WorkerStatus"),
        }
    }

    /// Cancel a job.
    pub async fn cancel_job(&self, job_id: &str, reason: Option<String>) -> Result<()> {
        let response = self
            .send_rpc_primary(ClientRpcRequest::JobCancel {
                job_id: job_id.to_string(),
                reason,
            })
            .await?;

        match response {
            ClientRpcResponse::JobCancelResult(result) => {
                if result.success {
                    Ok(())
                } else {
                    anyhow::bail!(
                        "Failed to cancel job: {}",
                        result.error.unwrap_or_else(|| "Unknown error".to_string())
                    )
                }
            }
            _ => anyhow::bail!("unexpected response type for JobCancel"),
        }
    }

    /// Shutdown the client.
    pub async fn shutdown(self) -> Result<()> {
        // Arc::try_unwrap will only succeed if we're the only reference holder
        match Arc::try_unwrap(self.endpoint) {
            Ok(endpoint) => {
                endpoint.close().await;
            }
            Err(_) => {
                warn!("endpoint has other references, not closing");
            }
        }
        Ok(())
    }
}
