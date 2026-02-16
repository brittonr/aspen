//! Real cluster testing infrastructure for integration tests with real Iroh networking.
//!
//! This module provides `RealClusterTester` for testing job submission and execution
//! with actual Iroh P2P networking (not madsim simulation).

#![allow(dead_code, unused_variables)]
//!
//! # Tiger Style
//!
//! - Bounded timeouts: All operations have explicit timeouts
//! - Resource cleanup: TempDir and node handles are properly cleaned up
//! - Explicit error handling: All errors are wrapped with context

use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use aspen::api::AddLearnerRequest;
use aspen::api::ChangeMembershipRequest;
use aspen::api::ClusterController;
use aspen::api::ClusterNode;
use aspen::api::InitRequest;
use aspen::node::Node;
use aspen::node::NodeBuilder;
use aspen::node::NodeId;
use aspen::raft::storage::StorageBackend;
use aspen_client::AspenClient;
use aspen_client::AspenClientJobExt;
use aspen_client::AspenClusterTicket;
use aspen_client::JobSubmitBuilder;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::JobDetails;
use aspen_client_api::JobQueueStatsResultResponse;
use iroh_gossip::proto::TopicId;
use tempfile::TempDir;
use tokio::time::sleep;
use tokio::time::timeout;
use tracing::info;
use tracing::warn;

/// Maximum time to wait for cluster formation.
const CLUSTER_FORMATION_TIMEOUT: Duration = Duration::from_secs(60);
/// Time to wait for leader election.
const LEADER_ELECTION_WAIT: Duration = Duration::from_millis(2000);
/// Time to wait for gossip discovery between nodes.
const GOSSIP_DISCOVERY_WAIT: Duration = Duration::from_secs(3);
/// RPC timeout for client operations.
const RPC_TIMEOUT: Duration = Duration::from_secs(10);
/// Poll interval for job completion.
const JOB_POLL_INTERVAL: Duration = Duration::from_millis(500);

/// Configuration for a real cluster test.
#[derive(Debug, Clone)]
pub struct RealClusterConfig {
    /// Number of nodes in the cluster.
    pub node_count: usize,
    /// Whether to enable job workers on nodes.
    pub enable_workers: bool,
    /// Number of workers per node.
    pub workers_per_node: usize,
    /// Overall test timeout.
    pub timeout: Duration,
    /// Cluster cookie for authentication.
    pub cookie: String,
    /// Whether to enable CI system on nodes.
    #[cfg(feature = "ci")]
    pub enable_ci: bool,
}

impl Default for RealClusterConfig {
    fn default() -> Self {
        Self {
            node_count: 3,
            enable_workers: true,
            workers_per_node: 2,
            timeout: Duration::from_secs(60),
            cookie: format!("test-cluster-{}", rand::random::<u32>()),
            #[cfg(feature = "ci")]
            enable_ci: false,
        }
    }
}

impl RealClusterConfig {
    /// Create a new config with the specified node count.
    pub fn with_node_count(mut self, count: usize) -> Self {
        self.node_count = count;
        self
    }

    /// Enable or disable workers.
    pub fn with_workers(mut self, enabled: bool) -> Self {
        self.enable_workers = enabled;
        self
    }

    /// Set workers per node.
    pub fn with_workers_per_node(mut self, count: usize) -> Self {
        self.workers_per_node = count;
        self
    }

    /// Set the test timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Enable or disable CI system.
    #[cfg(feature = "ci")]
    pub fn with_ci(mut self, enabled: bool) -> Self {
        self.enable_ci = enabled;
        self
    }
}

/// Real cluster tester for integration tests with actual Iroh networking.
pub struct RealClusterTester {
    /// Configuration.
    config: RealClusterConfig,
    /// Running nodes.
    nodes: Vec<Node>,
    /// Temporary directory for node data.
    _temp_dir: TempDir,
    /// Client for RPC operations.
    client: Option<AspenClient>,
    /// Cluster ticket for client connections.
    ticket: Option<AspenClusterTicket>,
}

impl RealClusterTester {
    /// Create a new cluster tester and start all nodes.
    ///
    /// This initializes a cluster with the specified configuration,
    /// forming a Raft cluster with all nodes as voters.
    pub async fn new(config: RealClusterConfig) -> Result<Self> {
        let temp_dir = TempDir::new().context("failed to create temp directory")?;
        let mut nodes = Vec::with_capacity(config.node_count);

        // Create all nodes
        info!(node_count = config.node_count, "creating cluster nodes");
        for id in 1..=config.node_count as u64 {
            let node = Self::create_node(id, &temp_dir, &config).await?;
            nodes.push(node);
        }

        // Initialize cluster on node 1
        let node1 = &nodes[0];
        let init_request = InitRequest {
            initial_members: vec![ClusterNode::with_iroh_addr(1, node1.endpoint_addr())],
        };

        timeout(CLUSTER_FORMATION_TIMEOUT, node1.raft_node().init(init_request))
            .await
            .context("cluster init timeout")?
            .context("cluster init failed")?;

        // Wait for leader election
        sleep(LEADER_ELECTION_WAIT).await;

        // Wait for gossip discovery
        sleep(GOSSIP_DISCOVERY_WAIT).await;

        // Add other nodes as learners if more than 1 node
        if config.node_count > 1 {
            for id in 2..=config.node_count as u64 {
                let node = &nodes[(id - 1) as usize];
                let learner = ClusterNode::with_iroh_addr(id, node.endpoint_addr());
                let request = AddLearnerRequest { learner };

                match timeout(CLUSTER_FORMATION_TIMEOUT, node1.raft_node().add_learner(request)).await {
                    Ok(Ok(_)) => info!(node_id = id, "added learner"),
                    Ok(Err(e)) => warn!(node_id = id, error = %e, "failed to add learner"),
                    Err(_) => warn!(node_id = id, "add_learner timeout"),
                }
                sleep(Duration::from_millis(500)).await;
            }

            // Wait for learners to catch up
            sleep(Duration::from_secs(2)).await;

            // Promote all nodes to voters
            let members: Vec<u64> = (1..=config.node_count as u64).collect();
            let request = ChangeMembershipRequest { members };

            timeout(CLUSTER_FORMATION_TIMEOUT, node1.raft_node().change_membership(request))
                .await
                .context("change_membership timeout")?
                .context("change_membership failed")?;

            info!(node_count = config.node_count, "all nodes promoted to voters");
        }

        // Create cluster ticket for client connections
        let hash = blake3::hash(config.cookie.as_bytes());
        let topic_id = TopicId::from_bytes(*hash.as_bytes());
        let ticket = AspenClusterTicket::with_bootstrap_addr(topic_id, config.cookie.clone(), &node1.endpoint_addr());

        // Create client for RPC operations
        let client = AspenClient::connect_with_ticket(ticket.clone(), RPC_TIMEOUT, None)
            .await
            .context("failed to create client")?;

        Ok(Self {
            config,
            nodes,
            _temp_dir: temp_dir,
            client: Some(client),
            ticket: Some(ticket),
        })
    }

    /// Create a single node.
    async fn create_node(node_id: u64, temp_dir: &TempDir, config: &RealClusterConfig) -> Result<Node> {
        let data_dir = temp_dir.path().join(format!("node-{}", node_id));
        let secret_key = format!("{:064x}", 2000 + node_id);

        #[allow(unused_mut)]
        let mut builder = NodeBuilder::new(NodeId(node_id), &data_dir)
            .with_storage(StorageBackend::InMemory)
            .with_cookie(&config.cookie)
            .with_gossip(true)
            .with_mdns(false) // Disable mDNS for CI compatibility
            .with_iroh_secret_key(&secret_key)
            .with_heartbeat_interval_ms(500)
            .with_election_timeout_ms(1500, 3000);

        // Enable secrets if the feature is available
        #[cfg(feature = "secrets")]
        {
            builder = builder.with_secrets(true);
        }

        let mut node = builder.start().await.context("failed to start node")?;

        // Spawn the router to enable inter-node communication
        node.spawn_router();

        info!(
            node_id = node_id,
            endpoint = %node.endpoint_addr().id.fmt_short(),
            "node created"
        );

        Ok(node)
    }

    /// Get the client for RPC operations.
    pub fn client(&self) -> &AspenClient {
        self.client.as_ref().expect("client not initialized")
    }

    /// Get a reference to a specific node.
    pub fn node(&self, idx: usize) -> Option<&Node> {
        self.nodes.get(idx)
    }

    /// Submit a job to a specific node via RPC.
    pub async fn submit_job(&self, node_idx: usize, job_type: &str, payload: serde_json::Value) -> Result<String> {
        let node = self.nodes.get(node_idx).context("invalid node index")?;

        // Create a ticket for this specific node
        let hash = blake3::hash(self.config.cookie.as_bytes());
        let topic_id = TopicId::from_bytes(*hash.as_bytes());
        let node_ticket =
            AspenClusterTicket::with_bootstrap_addr(topic_id, self.config.cookie.clone(), &node.endpoint_addr());

        // Connect to the specific node
        let node_client = AspenClient::connect_with_ticket(node_ticket, RPC_TIMEOUT, None)
            .await
            .context("failed to connect to node")?;

        // Submit the job
        let builder = JobSubmitBuilder::new(job_type, payload);
        let job_id = node_client.jobs().submit_job(builder).await.context("failed to submit job")?;

        node_client.shutdown().await;

        Ok(job_id)
    }

    /// Wait for a job to complete.
    pub async fn wait_for_job(&self, job_id: &str, timeout_duration: Duration) -> Result<JobDetails> {
        let client = self.client();
        let job_client = client.jobs();

        job_client
            .wait_for_completion(job_id, JOB_POLL_INTERVAL, Some(timeout_duration))
            .await
            .context("job wait failed")
    }

    /// Get job queue statistics from a specific node.
    pub async fn get_queue_stats(&self, node_idx: usize) -> Result<JobQueueStatsResultResponse> {
        let node = self.nodes.get(node_idx).context("invalid node index")?;

        // Create a ticket for this specific node
        let hash = blake3::hash(self.config.cookie.as_bytes());
        let topic_id = TopicId::from_bytes(*hash.as_bytes());
        let node_ticket =
            AspenClusterTicket::with_bootstrap_addr(topic_id, self.config.cookie.clone(), &node.endpoint_addr());

        // Connect to the specific node
        let node_client = AspenClient::connect_with_ticket(node_ticket, RPC_TIMEOUT, None)
            .await
            .context("failed to connect to node")?;

        // Get queue stats
        let response = node_client.send(ClientRpcRequest::JobQueueStats).await.context("failed to get queue stats")?;

        node_client.shutdown().await;

        match response {
            ClientRpcResponse::JobQueueStatsResult(stats) => {
                if let Some(error) = stats.error.as_ref() {
                    anyhow::bail!("queue stats error: {}", error);
                }
                Ok(stats)
            }
            _ => anyhow::bail!("unexpected response type"),
        }
    }

    /// Get job details by ID.
    pub async fn get_job(&self, job_id: &str) -> Result<Option<JobDetails>> {
        let client = self.client();
        client.jobs().get(job_id).await.context("failed to get job")
    }

    /// Shutdown the cluster gracefully.
    pub async fn shutdown(mut self) -> Result<()> {
        // Shutdown client first
        if let Some(client) = self.client.take() {
            client.shutdown().await;
        }

        // Shutdown all nodes
        for node in self.nodes.drain(..) {
            if let Err(e) = node.shutdown().await {
                warn!(error = %e, "node shutdown error");
            }
        }

        info!("cluster shutdown complete");
        Ok(())
    }

    /// Get the number of nodes in the cluster.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the cluster ticket.
    pub fn ticket(&self) -> Option<&AspenClusterTicket> {
        self.ticket.as_ref()
    }

    // =========================================================================
    // CI/CD Operations
    // =========================================================================

    /// Trigger a CI pipeline run for a repository.
    #[cfg(feature = "ci")]
    pub async fn ci_trigger_pipeline(
        &self,
        repo_id: &str,
        ref_name: &str,
        commit_hash: Option<String>,
    ) -> Result<aspen_client_api::CiTriggerPipelineResponse> {
        let client = self.client();
        let response = client
            .send(ClientRpcRequest::CiTriggerPipeline {
                repo_id: repo_id.to_string(),
                ref_name: ref_name.to_string(),
                commit_hash,
            })
            .await
            .context("failed to trigger CI pipeline")?;

        match response {
            ClientRpcResponse::CiTriggerPipelineResult(result) => Ok(result),
            ClientRpcResponse::Error(e) => anyhow::bail!("CI trigger error: {}: {}", e.code, e.message),
            _ => anyhow::bail!("unexpected response type"),
        }
    }

    /// Get CI pipeline status.
    #[cfg(feature = "ci")]
    pub async fn ci_get_status(&self, run_id: &str) -> Result<aspen_client_api::CiGetStatusResponse> {
        let client = self.client();
        let response = client
            .send(ClientRpcRequest::CiGetStatus {
                run_id: run_id.to_string(),
            })
            .await
            .context("failed to get CI status")?;

        match response {
            ClientRpcResponse::CiGetStatusResult(result) => Ok(result),
            ClientRpcResponse::Error(e) => anyhow::bail!("CI status error: {}: {}", e.code, e.message),
            _ => anyhow::bail!("unexpected response type"),
        }
    }

    /// List CI pipeline runs.
    #[cfg(feature = "ci")]
    pub async fn ci_list_runs(
        &self,
        repo_id: Option<&str>,
        status: Option<&str>,
        limit: Option<u32>,
    ) -> Result<aspen_client_api::CiListRunsResponse> {
        let client = self.client();
        let response = client
            .send(ClientRpcRequest::CiListRuns {
                repo_id: repo_id.map(|s| s.to_string()),
                status: status.map(|s| s.to_string()),
                limit,
            })
            .await
            .context("failed to list CI runs")?;

        match response {
            ClientRpcResponse::CiListRunsResult(result) => Ok(result),
            ClientRpcResponse::Error(e) => anyhow::bail!("CI list error: {}: {}", e.code, e.message),
            _ => anyhow::bail!("unexpected response type"),
        }
    }

    /// Cancel a CI pipeline run.
    #[cfg(feature = "ci")]
    pub async fn ci_cancel_run(
        &self,
        run_id: &str,
        reason: Option<&str>,
    ) -> Result<aspen_client_api::CiCancelRunResponse> {
        let client = self.client();
        let response = client
            .send(ClientRpcRequest::CiCancelRun {
                run_id: run_id.to_string(),
                reason: reason.map(|s| s.to_string()),
            })
            .await
            .context("failed to cancel CI run")?;

        match response {
            ClientRpcResponse::CiCancelRunResult(result) => Ok(result),
            ClientRpcResponse::Error(e) => anyhow::bail!("CI cancel error: {}: {}", e.code, e.message),
            _ => anyhow::bail!("unexpected response type"),
        }
    }

    /// Watch a repository for CI triggers.
    #[cfg(feature = "ci")]
    pub async fn ci_watch_repo(&self, repo_id: &str) -> Result<aspen_client_api::CiWatchRepoResponse> {
        let client = self.client();
        let response = client
            .send(ClientRpcRequest::CiWatchRepo {
                repo_id: repo_id.to_string(),
            })
            .await
            .context("failed to watch repo for CI")?;

        match response {
            ClientRpcResponse::CiWatchRepoResult(result) => Ok(result),
            ClientRpcResponse::Error(e) => anyhow::bail!("CI watch error: {}: {}", e.code, e.message),
            _ => anyhow::bail!("unexpected response type"),
        }
    }

    /// Unwatch a repository.
    #[cfg(feature = "ci")]
    pub async fn ci_unwatch_repo(&self, repo_id: &str) -> Result<aspen_client_api::CiUnwatchRepoResponse> {
        let client = self.client();
        let response = client
            .send(ClientRpcRequest::CiUnwatchRepo {
                repo_id: repo_id.to_string(),
            })
            .await
            .context("failed to unwatch repo")?;

        match response {
            ClientRpcResponse::CiUnwatchRepoResult(result) => Ok(result),
            ClientRpcResponse::Error(e) => anyhow::bail!("CI unwatch error: {}: {}", e.code, e.message),
            _ => anyhow::bail!("unexpected response type"),
        }
    }
}
