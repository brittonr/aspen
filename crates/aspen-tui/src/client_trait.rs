//! Common client trait for Iroh connections.

use aspen_client::NodeDescriptor;
use aspen_client::SqlResultResponse;
use async_trait::async_trait;
use color_eyre::Result;
use color_eyre::eyre::eyre;

use crate::types::CiPipelineDetail;
use crate::types::CiPipelineRunInfo;
use crate::types::ClusterMetrics;
use crate::types::JobInfo;
use crate::types::NodeInfo;
use crate::types::NodeStatus;
use crate::types::QueueStats;
use crate::types::WorkerPoolInfo;

/// Trait for Aspen cluster clients.
///
/// Implemented by both HTTP and Iroh clients for unified access.
#[async_trait]
pub trait ClusterClient: Send + Sync {
    /// Get node information.
    async fn get_node_info(&self) -> Result<NodeInfo>;

    /// Get cluster metrics.
    async fn get_metrics(&self) -> Result<ClusterMetrics>;

    /// Get all discovered cluster nodes (for multi-node clients).
    /// Returns empty vec for single-node clients.
    async fn get_discovered_nodes(&self) -> Result<Vec<NodeDescriptor>> {
        Ok(vec![])
    }

    /// Check if the node is healthy.
    async fn is_healthy(&self) -> Result<bool>;

    /// Initialize the cluster.
    async fn init_cluster(&self) -> Result<()>;

    /// Add a learner node.
    async fn add_learner(&self, node_id: u64, addr: String) -> Result<()>;

    /// Change cluster membership.
    async fn change_membership(&self, members: Vec<u64>) -> Result<()>;

    /// Write a key-value pair.
    async fn write(&self, key: String, value: Vec<u8>) -> Result<()>;

    /// Read a value by key.
    async fn read(&self, key: String) -> Result<Option<Vec<u8>>>;

    /// Trigger a snapshot.
    async fn trigger_snapshot(&self) -> Result<()>;

    /// Execute a SQL query against the cluster.
    ///
    /// # Arguments
    /// * `query` - SQL query string (SELECT only)
    /// * `consistency` - "linearizable" or "stale"
    /// * `limit` - Maximum rows to return
    /// * `timeout_ms` - Query timeout in milliseconds
    async fn execute_sql(
        &self,
        query: String,
        consistency: String,
        limit: Option<u32>,
        timeout_ms: Option<u32>,
    ) -> Result<SqlResultResponse> {
        let _ = (query, consistency, limit, timeout_ms);
        Err(eyre!("SQL queries not supported"))
    }

    /// List all vaults (keys with "vault:" prefix).
    ///
    /// Returns a list of vault names and their key counts.
    async fn list_vaults(&self) -> Result<Vec<VaultSummary>> {
        // Default implementation: scan keys with vault prefix
        // This works for HTTP clients; Iroh clients may override
        Ok(vec![])
    }

    /// List all keys in a specific vault.
    async fn list_vault_keys(&self, vault: &str) -> Result<Vec<VaultKeyEntry>> {
        let _ = vault;
        Ok(vec![])
    }

    /// List jobs with optional status filter.
    async fn list_jobs(&self, status: Option<String>, limit: Option<u32>) -> Result<Vec<JobInfo>> {
        let _ = (status, limit);
        Err(eyre!("Job listing not supported"))
    }

    /// Get job queue statistics.
    async fn get_queue_stats(&self) -> Result<QueueStats> {
        Err(eyre!("Queue stats not supported"))
    }

    /// Get worker pool status.
    async fn get_worker_status(&self) -> Result<WorkerPoolInfo> {
        Err(eyre!("Worker status not supported"))
    }

    /// Cancel a job.
    async fn cancel_job(&self, job_id: &str, reason: Option<String>) -> Result<()> {
        let _ = (job_id, reason);
        Err(eyre!("Job cancellation not supported"))
    }

    // =========================================================================
    // CI Pipeline Operations
    // =========================================================================

    /// List CI pipeline runs with optional filtering.
    async fn ci_list_runs(
        &self,
        repo_id: Option<String>,
        status: Option<String>,
        limit: Option<u32>,
    ) -> Result<Vec<CiPipelineRunInfo>> {
        let _ = (repo_id, status, limit);
        Err(eyre!("CI not supported"))
    }

    /// Get CI pipeline run status and details.
    async fn ci_get_status(&self, run_id: &str) -> Result<CiPipelineDetail> {
        let _ = run_id;
        Err(eyre!("CI not supported"))
    }

    /// Trigger a CI pipeline run.
    async fn ci_trigger_pipeline(&self, repo_id: &str, ref_name: &str, commit_hash: Option<&str>) -> Result<String> {
        let _ = (repo_id, ref_name, commit_hash);
        Err(eyre!("CI not supported"))
    }

    /// Cancel a CI pipeline run.
    async fn ci_cancel_run(&self, run_id: &str, reason: Option<String>) -> Result<()> {
        let _ = (run_id, reason);
        Err(eyre!("CI not supported"))
    }

    /// Get historical logs for a CI job.
    ///
    /// Returns log chunks starting from a specific index.
    async fn ci_get_job_logs(
        &self,
        run_id: &str,
        job_id: &str,
        start_index: u32,
        limit: Option<u32>,
    ) -> Result<CiJobLogsResult> {
        let _ = (run_id, job_id, start_index, limit);
        Err(eyre!("CI logs not supported"))
    }
}

/// Result from fetching CI job logs.
#[derive(Debug, Clone)]
pub struct CiJobLogsResult {
    /// Whether the job was found.
    pub was_found: bool,
    /// Log chunks in order.
    pub chunks: Vec<CiLogChunkResult>,
    /// Index of the last chunk returned.
    pub last_index: u32,
    /// Whether there are more chunks available.
    pub has_more: bool,
    /// Whether the log stream is complete (job finished).
    pub is_complete: bool,
}

/// A single CI log chunk.
#[derive(Debug, Clone)]
pub struct CiLogChunkResult {
    /// Chunk index.
    pub index: u32,
    /// Log content.
    pub content: String,
    /// Timestamp (ms since epoch).
    pub timestamp_ms: u64,
}

/// Summary information about a vault.
#[derive(Debug, Clone)]
pub struct VaultSummary {
    /// Name of the vault.
    pub name: String,
    /// Number of keys in the vault.
    pub key_count: u64,
}

/// A key-value entry within a vault.
#[derive(Debug, Clone)]
pub struct VaultKeyEntry {
    /// Key name (without vault prefix).
    pub key: String,
    /// Value (as string).
    pub value: String,
}

/// Wrapper enum for different client types.
pub enum ClientImpl {
    Iroh(crate::iroh_client::IrohClient),
    MultiNode(crate::iroh_client::MultiNodeClient),
    Disconnected(DisconnectedClient),
}

/// A client that's not connected to any nodes.
/// Returns errors for all operations until replaced with a real client.
pub struct DisconnectedClient;

#[async_trait]
impl ClusterClient for DisconnectedClient {
    async fn get_node_info(&self) -> Result<NodeInfo> {
        Err(eyre!("Not connected to any cluster"))
    }

    async fn get_metrics(&self) -> Result<ClusterMetrics> {
        Err(eyre!("Not connected to any cluster"))
    }

    async fn get_discovered_nodes(&self) -> Result<Vec<NodeDescriptor>> {
        Ok(vec![])
    }

    async fn is_healthy(&self) -> Result<bool> {
        Ok(false)
    }

    async fn init_cluster(&self) -> Result<()> {
        Err(eyre!("Not connected to any cluster"))
    }

    async fn add_learner(&self, _node_id: u64, _addr: String) -> Result<()> {
        Err(eyre!("Not connected to any cluster"))
    }

    async fn change_membership(&self, _members: Vec<u64>) -> Result<()> {
        Err(eyre!("Not connected to any cluster"))
    }

    async fn write(&self, _key: String, _value: Vec<u8>) -> Result<()> {
        Err(eyre!("Not connected to any cluster"))
    }

    async fn read(&self, _key: String) -> Result<Option<Vec<u8>>> {
        Err(eyre!("Not connected to any cluster"))
    }

    async fn trigger_snapshot(&self) -> Result<()> {
        Err(eyre!("Not connected to any cluster"))
    }

    async fn execute_sql(
        &self,
        _query: String,
        _consistency: String,
        _limit: Option<u32>,
        _timeout_ms: Option<u32>,
    ) -> Result<SqlResultResponse> {
        Err(eyre!("Not connected to any cluster"))
    }

    async fn list_vaults(&self) -> Result<Vec<VaultSummary>> {
        Err(eyre!("Not connected to any cluster"))
    }

    async fn list_vault_keys(&self, _vault: &str) -> Result<Vec<VaultKeyEntry>> {
        Err(eyre!("Not connected to any cluster"))
    }

    async fn list_jobs(&self, _status: Option<String>, _limit: Option<u32>) -> Result<Vec<JobInfo>> {
        Err(eyre!("Not connected to any cluster"))
    }

    async fn get_queue_stats(&self) -> Result<QueueStats> {
        Err(eyre!("Not connected to any cluster"))
    }

    async fn get_worker_status(&self) -> Result<WorkerPoolInfo> {
        Err(eyre!("Not connected to any cluster"))
    }

    async fn cancel_job(&self, _job_id: &str, _reason: Option<String>) -> Result<()> {
        Err(eyre!("Not connected to any cluster"))
    }

    async fn ci_list_runs(
        &self,
        _repo_id: Option<String>,
        _status: Option<String>,
        _limit: Option<u32>,
    ) -> Result<Vec<CiPipelineRunInfo>> {
        Err(eyre!("Not connected to any cluster"))
    }

    async fn ci_get_status(&self, _run_id: &str) -> Result<CiPipelineDetail> {
        Err(eyre!("Not connected to any cluster"))
    }

    async fn ci_trigger_pipeline(&self, _repo_id: &str, _ref_name: &str, _commit_hash: Option<&str>) -> Result<String> {
        Err(eyre!("Not connected to any cluster"))
    }

    async fn ci_cancel_run(&self, _run_id: &str, _reason: Option<String>) -> Result<()> {
        Err(eyre!("Not connected to any cluster"))
    }

    async fn ci_get_job_logs(
        &self,
        _run_id: &str,
        _job_id: &str,
        _start_index: u32,
        _limit: Option<u32>,
    ) -> Result<CiJobLogsResult> {
        Err(eyre!("Not connected to any cluster"))
    }
}

/// Helper to convert anyhow::Error to color_eyre::Report.
fn anyhow_to_eyre(err: anyhow::Error) -> color_eyre::Report {
    eyre!("{:#}", err)
}

#[async_trait]
impl ClusterClient for ClientImpl {
    async fn get_discovered_nodes(&self) -> Result<Vec<NodeDescriptor>> {
        match self {
            Self::Iroh(_) => Ok(vec![]), // Single Iroh client doesn't discover
            Self::MultiNode(client) => {
                // MultiNodeClient can discover peers
                client.discover_peers().await.map_err(anyhow_to_eyre)
            }
            Self::Disconnected(client) => client.get_discovered_nodes().await,
        }
    }

    async fn get_node_info(&self) -> Result<NodeInfo> {
        match self {
            Self::Iroh(client) => {
                let info = client.get_node_info().await.map_err(anyhow_to_eyre)?;
                let health = client.get_health().await.map_err(anyhow_to_eyre)?;
                let metrics = client.get_raft_metrics().await.ok();

                let is_leader =
                    metrics.as_ref().and_then(|m| m.current_leader).map(|l| l == info.node_id).unwrap_or(false);

                Ok(NodeInfo {
                    node_id: info.node_id,
                    status: NodeStatus::from_str(&health.status),
                    is_leader,
                    last_applied_index: metrics.as_ref().and_then(|m| m.last_applied_index),
                    current_term: metrics.as_ref().map(|m| m.current_term),
                    uptime_secs: Some(health.uptime_seconds),
                    addr: format!("iroh://{}", info.endpoint_addr),
                })
            }
            Self::MultiNode(client) => {
                let info = client.get_node_info().await.map_err(anyhow_to_eyre)?;
                let health = client.get_health().await.map_err(anyhow_to_eyre)?;
                let metrics = client.get_raft_metrics().await.ok();

                let is_leader =
                    metrics.as_ref().and_then(|m| m.current_leader).map(|l| l == info.node_id).unwrap_or(false);

                Ok(NodeInfo {
                    node_id: info.node_id,
                    status: NodeStatus::from_str(&health.status),
                    is_leader,
                    last_applied_index: metrics.as_ref().and_then(|m| m.last_applied_index),
                    current_term: metrics.as_ref().map(|m| m.current_term),
                    uptime_secs: Some(health.uptime_seconds),
                    addr: format!("iroh://{}", info.endpoint_addr),
                })
            }
            Self::Disconnected(client) => client.get_node_info().await,
        }
    }

    async fn get_metrics(&self) -> Result<ClusterMetrics> {
        match self {
            Self::Iroh(client) => {
                let metrics = client.get_raft_metrics().await.map_err(anyhow_to_eyre)?;
                Ok(ClusterMetrics {
                    leader: metrics.current_leader,
                    term: metrics.current_term,
                    node_count: 1, // Would need to query cluster for full count
                    last_log_index: metrics.last_log_index,
                    last_applied_index: metrics.last_applied_index,
                })
            }
            Self::MultiNode(client) => {
                let metrics = client.get_raft_metrics().await.map_err(anyhow_to_eyre)?;
                // Use discover_peers to get actual node count
                let node_count = client.discover_peers().await.map(|peers| peers.len() as u32).unwrap_or(1);
                Ok(ClusterMetrics {
                    leader: metrics.current_leader,
                    term: metrics.current_term,
                    node_count,
                    last_log_index: metrics.last_log_index,
                    last_applied_index: metrics.last_applied_index,
                })
            }
            Self::Disconnected(client) => client.get_metrics().await,
        }
    }

    async fn is_healthy(&self) -> Result<bool> {
        match self {
            Self::Iroh(client) => {
                let health = client.get_health().await.map_err(anyhow_to_eyre)?;
                Ok(health.status == "healthy")
            }
            Self::MultiNode(client) => {
                let health = client.get_health().await.map_err(anyhow_to_eyre)?;
                Ok(health.status == "healthy")
            }
            Self::Disconnected(client) => client.is_healthy().await,
        }
    }

    async fn init_cluster(&self) -> Result<()> {
        match self {
            Self::Iroh(client) => {
                let result = client.init_cluster().await.map_err(anyhow_to_eyre)?;
                if !result.is_success {
                    if let Some(error) = result.error {
                        return Err(eyre!("Failed to initialize cluster: {}", error));
                    } else {
                        return Err(eyre!("Failed to initialize cluster"));
                    }
                }
                Ok(())
            }
            Self::MultiNode(client) => {
                let result = client.init_cluster().await.map_err(anyhow_to_eyre)?;
                if !result.is_success {
                    if let Some(error) = result.error {
                        return Err(eyre!("Failed to initialize cluster: {}", error));
                    } else {
                        return Err(eyre!("Failed to initialize cluster"));
                    }
                }
                Ok(())
            }
            Self::Disconnected(client) => client.init_cluster().await,
        }
    }

    async fn add_learner(&self, node_id: u64, addr: String) -> Result<()> {
        match self {
            Self::Iroh(client) => {
                let result = client.add_learner(node_id, addr).await.map_err(anyhow_to_eyre)?;
                if !result.is_success {
                    if let Some(error) = result.error {
                        return Err(eyre!("Failed to add learner: {}", error));
                    } else {
                        return Err(eyre!("Failed to add learner"));
                    }
                }
                Ok(())
            }
            Self::MultiNode(client) => {
                let result = client.add_learner(node_id, addr).await.map_err(anyhow_to_eyre)?;
                if !result.is_success {
                    if let Some(error) = result.error {
                        return Err(eyre!("Failed to add learner: {}", error));
                    } else {
                        return Err(eyre!("Failed to add learner"));
                    }
                }
                Ok(())
            }
            Self::Disconnected(client) => client.add_learner(node_id, addr).await,
        }
    }

    async fn change_membership(&self, members: Vec<u64>) -> Result<()> {
        match self {
            Self::Iroh(client) => {
                let result = client.change_membership(members).await.map_err(anyhow_to_eyre)?;
                if !result.is_success {
                    if let Some(error) = result.error {
                        return Err(eyre!("Failed to change membership: {}", error));
                    } else {
                        return Err(eyre!("Failed to change membership"));
                    }
                }
                Ok(())
            }
            Self::MultiNode(client) => {
                let result = client.change_membership(members).await.map_err(anyhow_to_eyre)?;
                if !result.is_success {
                    if let Some(error) = result.error {
                        return Err(eyre!("Failed to change membership: {}", error));
                    } else {
                        return Err(eyre!("Failed to change membership"));
                    }
                }
                Ok(())
            }
            Self::Disconnected(client) => client.change_membership(members).await,
        }
    }

    async fn write(&self, key: String, value: Vec<u8>) -> Result<()> {
        match self {
            Self::Iroh(client) => {
                let result = client.write_key(key, value).await.map_err(anyhow_to_eyre)?;
                if !result.is_success {
                    if let Some(error) = result.error {
                        return Err(eyre!("Failed to write key: {}", error));
                    } else {
                        return Err(eyre!("Failed to write key"));
                    }
                }
                Ok(())
            }
            Self::MultiNode(client) => {
                let result = client.write_key(key, value).await.map_err(anyhow_to_eyre)?;
                if !result.is_success {
                    if let Some(error) = result.error {
                        return Err(eyre!("Failed to write key: {}", error));
                    } else {
                        return Err(eyre!("Failed to write key"));
                    }
                }
                Ok(())
            }
            Self::Disconnected(client) => client.write(key, value).await,
        }
    }

    async fn read(&self, key: String) -> Result<Option<Vec<u8>>> {
        match self {
            Self::Iroh(client) => {
                let result = client.read_key(key).await.map_err(anyhow_to_eyre)?;
                if let Some(err) = result.error {
                    return Err(eyre!("Read failed: {}", err));
                }
                Ok(result.value)
            }
            Self::MultiNode(client) => {
                let result = client.read_key(key).await.map_err(anyhow_to_eyre)?;
                if let Some(err) = result.error {
                    return Err(eyre!("Read failed: {}", err));
                }
                Ok(result.value)
            }
            Self::Disconnected(client) => client.read(key).await,
        }
    }

    async fn trigger_snapshot(&self) -> Result<()> {
        match self {
            Self::Iroh(client) => {
                let result = client.trigger_snapshot().await.map_err(anyhow_to_eyre)?;
                if !result.is_success {
                    if let Some(error) = result.error {
                        return Err(eyre!("Failed to trigger snapshot: {}", error));
                    } else {
                        return Err(eyre!("Failed to trigger snapshot"));
                    }
                }
                Ok(())
            }
            Self::MultiNode(client) => {
                let result = client.trigger_snapshot().await.map_err(anyhow_to_eyre)?;
                if !result.is_success {
                    if let Some(error) = result.error {
                        return Err(eyre!("Failed to trigger snapshot: {}", error));
                    } else {
                        return Err(eyre!("Failed to trigger snapshot"));
                    }
                }
                Ok(())
            }
            Self::Disconnected(client) => client.trigger_snapshot().await,
        }
    }

    async fn execute_sql(
        &self,
        query: String,
        consistency: String,
        limit: Option<u32>,
        timeout_ms: Option<u32>,
    ) -> Result<SqlResultResponse> {
        match self {
            Self::Iroh(client) => {
                client.execute_sql(query, consistency, limit, timeout_ms).await.map_err(anyhow_to_eyre)
            }
            Self::MultiNode(client) => {
                client.execute_sql(query, consistency, limit, timeout_ms).await.map_err(anyhow_to_eyre)
            }
            Self::Disconnected(client) => client.execute_sql(query, consistency, limit, timeout_ms).await,
        }
    }

    async fn list_vaults(&self) -> Result<Vec<VaultSummary>> {
        match self {
            Self::Iroh(client) => {
                // Scan for all vault: prefixed keys
                let result = client.scan_keys("vault:".to_string(), Some(1000), None).await.map_err(anyhow_to_eyre)?;

                // Group by vault name
                let mut vault_counts: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
                for entry in result.entries {
                    // Key format: vault:<vault-name>:<key>
                    if let Some(rest) = entry.key.strip_prefix("vault:") {
                        if let Some(vault_name) = rest.split(':').next() {
                            *vault_counts.entry(vault_name.to_string()).or_default() += 1;
                        }
                    }
                }

                Ok(vault_counts.into_iter().map(|(name, key_count)| VaultSummary { name, key_count }).collect())
            }
            Self::MultiNode(client) => {
                let result = client.scan_keys("vault:".to_string(), Some(1000), None).await.map_err(anyhow_to_eyre)?;

                let mut vault_counts: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
                for entry in result.entries {
                    if let Some(rest) = entry.key.strip_prefix("vault:") {
                        if let Some(vault_name) = rest.split(':').next() {
                            *vault_counts.entry(vault_name.to_string()).or_default() += 1;
                        }
                    }
                }

                Ok(vault_counts.into_iter().map(|(name, key_count)| VaultSummary { name, key_count }).collect())
            }
            Self::Disconnected(client) => client.list_vaults().await,
        }
    }

    async fn list_vault_keys(&self, vault: &str) -> Result<Vec<VaultKeyEntry>> {
        match self {
            Self::Iroh(client) => {
                let prefix = format!("vault:{}:", vault);
                let result = client.scan_keys(prefix.clone(), Some(1000), None).await.map_err(anyhow_to_eyre)?;

                Ok(result
                    .entries
                    .into_iter()
                    .filter_map(|entry| {
                        let key = entry.key.strip_prefix(&prefix)?.to_string();
                        Some(VaultKeyEntry {
                            key,
                            value: entry.value,
                        })
                    })
                    .collect())
            }
            Self::MultiNode(client) => {
                let prefix = format!("vault:{}:", vault);
                let result = client.scan_keys(prefix.clone(), Some(1000), None).await.map_err(anyhow_to_eyre)?;

                Ok(result
                    .entries
                    .into_iter()
                    .filter_map(|entry| {
                        let key = entry.key.strip_prefix(&prefix)?.to_string();
                        Some(VaultKeyEntry {
                            key,
                            value: entry.value,
                        })
                    })
                    .collect())
            }
            Self::Disconnected(client) => client.list_vault_keys(vault).await,
        }
    }

    async fn list_jobs(&self, status: Option<String>, limit: Option<u32>) -> Result<Vec<JobInfo>> {
        match self {
            Self::Iroh(client) => client.list_jobs(status, limit).await.map_err(anyhow_to_eyre),
            Self::MultiNode(client) => client.list_jobs(status, limit).await.map_err(anyhow_to_eyre),
            Self::Disconnected(client) => client.list_jobs(status, limit).await,
        }
    }

    async fn get_queue_stats(&self) -> Result<QueueStats> {
        match self {
            Self::Iroh(client) => client.get_queue_stats().await.map_err(anyhow_to_eyre),
            Self::MultiNode(client) => client.get_queue_stats().await.map_err(anyhow_to_eyre),
            Self::Disconnected(client) => client.get_queue_stats().await,
        }
    }

    async fn get_worker_status(&self) -> Result<WorkerPoolInfo> {
        match self {
            Self::Iroh(client) => client.get_worker_status().await.map_err(anyhow_to_eyre),
            Self::MultiNode(client) => client.get_worker_status().await.map_err(anyhow_to_eyre),
            Self::Disconnected(client) => client.get_worker_status().await,
        }
    }

    async fn cancel_job(&self, job_id: &str, reason: Option<String>) -> Result<()> {
        match self {
            Self::Iroh(client) => client.cancel_job(job_id, reason).await.map_err(anyhow_to_eyre),
            Self::MultiNode(client) => client.cancel_job(job_id, reason).await.map_err(anyhow_to_eyre),
            Self::Disconnected(client) => client.cancel_job(job_id, reason).await,
        }
    }

    async fn ci_list_runs(
        &self,
        repo_id: Option<String>,
        status: Option<String>,
        limit: Option<u32>,
    ) -> Result<Vec<CiPipelineRunInfo>> {
        match self {
            Self::Iroh(client) => client.ci_list_runs(repo_id, status, limit).await.map_err(anyhow_to_eyre),
            Self::MultiNode(client) => client.ci_list_runs(repo_id, status, limit).await.map_err(anyhow_to_eyre),
            Self::Disconnected(client) => client.ci_list_runs(repo_id, status, limit).await,
        }
    }

    async fn ci_get_status(&self, run_id: &str) -> Result<CiPipelineDetail> {
        match self {
            Self::Iroh(client) => client.ci_get_status(run_id).await.map_err(anyhow_to_eyre),
            Self::MultiNode(client) => client.ci_get_status(run_id).await.map_err(anyhow_to_eyre),
            Self::Disconnected(client) => client.ci_get_status(run_id).await,
        }
    }

    async fn ci_trigger_pipeline(&self, repo_id: &str, ref_name: &str, commit_hash: Option<&str>) -> Result<String> {
        match self {
            Self::Iroh(client) => {
                client.ci_trigger_pipeline(repo_id, ref_name, commit_hash).await.map_err(anyhow_to_eyre)
            }
            Self::MultiNode(client) => {
                client.ci_trigger_pipeline(repo_id, ref_name, commit_hash).await.map_err(anyhow_to_eyre)
            }
            Self::Disconnected(client) => client.ci_trigger_pipeline(repo_id, ref_name, commit_hash).await,
        }
    }

    async fn ci_cancel_run(&self, run_id: &str, reason: Option<String>) -> Result<()> {
        match self {
            Self::Iroh(client) => client.ci_cancel_run(run_id, reason).await.map_err(anyhow_to_eyre),
            Self::MultiNode(client) => client.ci_cancel_run(run_id, reason).await.map_err(anyhow_to_eyre),
            Self::Disconnected(client) => client.ci_cancel_run(run_id, reason).await,
        }
    }

    async fn ci_get_job_logs(
        &self,
        run_id: &str,
        job_id: &str,
        start_index: u32,
        limit: Option<u32>,
    ) -> Result<CiJobLogsResult> {
        match self {
            Self::Iroh(client) => {
                client.ci_get_job_logs(run_id, job_id, start_index, limit).await.map_err(anyhow_to_eyre)
            }
            Self::MultiNode(client) => {
                client.ci_get_job_logs(run_id, job_id, start_index, limit).await.map_err(anyhow_to_eyre)
            }
            Self::Disconnected(client) => client.ci_get_job_logs(run_id, job_id, start_index, limit).await,
        }
    }
}
