//! Hiqlite Service - Distributed SQLite with Raft Consensus
//!
//! This module provides a distributed state management layer using hiqlite,
//! a Raft-replicated SQLite database. It serves as the foundation for:
//! - Workflow state replication across nodes
//! - Strong consistency guarantees for critical operations
//! - Automatic leader failover and self-healing

use anyhow::Result;
use async_trait::async_trait;
use hiqlite::{Client, NodeConfig, Param};
use std::borrow::Cow;

use crate::services::traits::{DatabaseHealth, DatabaseLifecycle, DatabaseQueries, DatabaseSchema};

/// Macro to create hiqlite parameters
#[macro_export]
macro_rules! params {
    ( $( $param:expr ),* $(,)? ) => {
        {
            #[allow(unused_mut)]
            let mut params = Vec::new();
            $(
                params.push(hiqlite::Param::from($param));
            )*
            params
        }
    };
}

/// HiqliteService wraps the hiqlite client and provides initialization
/// and management capabilities for the distributed database cluster.
#[derive(Clone)]
pub struct HiqliteService {
    client: Client,
}

impl std::fmt::Debug for HiqliteService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HiqliteService")
            .field("client", &"<hiqlite::Client>")
            .finish()
    }
}

impl HiqliteService {
    /// Create a placeholder service (for when hiqlite is disabled)
    pub fn placeholder() -> Self {
        // This is a temporary workaround - creates a non-functional client
        // TODO: Remove once hiqlite configuration is fixed
        unimplemented!("Hiqlite placeholder - real implementation needed")
    }

    /// Initialize a new hiqlite node from environment variables or configuration file
    ///
    /// # Parameters
    /// - `data_dir`: Optional data directory override (uses HQL_DATA_DIR env var if None)
    ///
    /// # Environment Variables
    /// - `HQL_NODE_ID`: Unique node identifier (required)
    /// - `HQL_DATA_DIR`: Data directory (default: ./data/hiqlite)
    /// - `HQL_SECRET_RAFT`: Raft cluster authentication secret (required)
    /// - `HQL_SECRET_API`: API authentication secret (required)
    /// - `HQL_NODES`: Comma-separated list of nodes (format: "id:raft_addr:api_addr,...")
    /// - `HQL_LISTEN_ADDR_RAFT`: Raft listen address (default: 0.0.0.0:8200)
    /// - `HQL_LISTEN_ADDR_API`: API listen address (default: 0.0.0.0:8201)
    ///
    /// # Example
    /// ```no_run
    /// use hiqlite_service::HiqliteService;
    ///
    /// let service = HiqliteService::new(Some("./data/custom".into())).await?;
    /// ```
    pub async fn new(data_dir: impl Into<Option<std::path::PathBuf>>) -> Result<Self> {
        let config = Self::build_config(data_dir.into()).await?;
        let node_id = config.node_id;
        let expected_nodes = config.nodes.len();

        tracing::info!(
            node_id = ?node_id,
            data_dir = ?config.data_dir,
            expected_cluster_size = expected_nodes,
            "Initializing hiqlite node"
        );

        let client = hiqlite::start_node(config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to start hiqlite node: {}", e))?;

        // Give hiqlite time to start up (use hardcoded value here as timing config not available)
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        // Wait for cluster to form (only if we have multiple nodes configured)
        if expected_nodes > 1 {
            tracing::info!(
                node_id = ?node_id,
                expected_nodes = expected_nodes,
                "Waiting for Raft cluster to form..."
            );

            let start_time = std::time::Instant::now();
            let mut last_log_time = std::time::Instant::now();

            loop {
                let elapsed = start_time.elapsed();

                // Warn if cluster formation is taking too long
                if elapsed > std::time::Duration::from_secs(60) {
                    tracing::warn!(
                        node_id = ?node_id,
                        elapsed_secs = elapsed.as_secs(),
                        "Cluster formation is taking longer than expected - check network connectivity and hiqlite configuration"
                    );
                }

                // Check if cluster is healthy
                match client.is_healthy_db().await {
                    Ok(_) => {
                        // Now check if all nodes are members
                        if let Ok(metrics) = client.metrics_db().await {
                            let membership = metrics.membership_config.membership();
                            let online_nodes = membership.nodes().count();
                            let voter_nodes = membership.voter_ids().count();
                            let learner_nodes = membership.learner_ids().count();

                            // Log member node IDs for debugging
                            let member_ids: Vec<_> = membership.nodes().map(|(id, _)| *id).collect();
                            let voter_ids: Vec<_> = membership.voter_ids().collect();

                            if online_nodes == expected_nodes && voter_nodes == expected_nodes {
                                tracing::info!(
                                    node_id = ?node_id,
                                    online_nodes = online_nodes,
                                    voter_nodes = voter_nodes,
                                    member_ids = ?member_ids,
                                    elapsed_secs = elapsed.as_secs(),
                                    "Raft cluster fully formed - all nodes are voting members"
                                );
                                break;
                            } else {
                                // Log progress every 5 seconds to avoid spam
                                if last_log_time.elapsed() > std::time::Duration::from_secs(5) {
                                    tracing::info!(
                                        node_id = ?node_id,
                                        online_nodes = online_nodes,
                                        voter_nodes = voter_nodes,
                                        learner_nodes = learner_nodes,
                                        expected_nodes = expected_nodes,
                                        member_ids = ?member_ids,
                                        voter_ids = ?voter_ids,
                                        "Waiting for all nodes to join cluster..."
                                    );
                                    last_log_time = std::time::Instant::now();
                                }
                            }
                        }
                    }
                    Err(e) => {
                        // Log at INFO level during cluster formation instead of DEBUG
                        if last_log_time.elapsed() > std::time::Duration::from_secs(5) {
                            tracing::info!(
                                node_id = ?node_id,
                                error = %e,
                                elapsed_secs = elapsed.as_secs(),
                                "Cluster not yet healthy, waiting..."
                            );
                            last_log_time = std::time::Instant::now();
                        }
                    }
                }

                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }

        tracing::info!(node_id = ?node_id, "Hiqlite node initialized successfully");

        Ok(Self { client })
    }

    /// Build NodeConfig from environment variables with sensible defaults
    ///
    /// # Parameters
    /// - `data_dir_override`: Optional data directory to override default/env/TOML values
    async fn build_config(data_dir_override: Option<std::path::PathBuf>) -> Result<NodeConfig> {
        // Try loading from TOML file first, fall back to environment
        if let Ok(mut config) = NodeConfig::from_toml("hiqlite.toml", None, None).await {
            // Apply data_dir override if provided
            if let Some(dir) = data_dir_override {
                let dir_str = dir.to_string_lossy().to_string();
                config.data_dir = Cow::Owned(dir_str.clone());
                tracing::info!(data_dir = ?dir_str, "Overriding hiqlite data_dir from configuration");
            }

            tracing::info!(
                node_id = ?config.node_id,
                data_dir = ?config.data_dir,
                nodes_count = config.nodes.len(),
                "Loaded hiqlite config from hiqlite.toml"
            );

            // Log detailed node configuration for debugging
            for node in &config.nodes {
                tracing::debug!(
                    node_id = node.id,
                    addr_raft = %node.addr_raft,
                    addr_api = %node.addr_api,
                    "Configured hiqlite node"
                );
            }

            return Ok(config);
        }

        // Otherwise build from environment with defaults
        let mut config = NodeConfig::from_env();

        // Apply data_dir override if provided
        if let Some(dir) = data_dir_override {
            let dir_str = dir.to_string_lossy().to_string();
            config.data_dir = Cow::Owned(dir_str.clone());
            tracing::info!(data_dir = ?dir_str, "Overriding hiqlite data_dir from configuration");
        }

        tracing::info!(
            node_id = ?config.node_id,
            data_dir = ?config.data_dir,
            nodes_count = config.nodes.len(),
            "Loaded hiqlite config from environment variables"
        );

        // Log detailed node configuration for debugging
        for node in &config.nodes {
            tracing::debug!(
                node_id = node.id,
                addr_raft = %node.addr_raft,
                addr_api = %node.addr_api,
                "Configured hiqlite node"
            );
        }

        Ok(config)
    }

    /// Create a test config builder for single-node clusters
    #[cfg(test)]
    fn build_test_config(node_id: u64, data_dir: std::path::PathBuf) -> NodeConfig {
        use hiqlite::Node;
        let nodes = vec![
            Node {
                id: 1,
                addr_raft: "127.0.0.1:38001".to_string(),
                addr_api: "127.0.0.1:37001".to_string(),
            },
        ];

        let data_dir_str = data_dir.to_string_lossy().to_string();

        let mut config = NodeConfig::default();
        config.node_id = node_id;
        config.nodes = nodes;
        config.data_dir = data_dir_str.into();
        config.secret_raft = "test-secret-raft-123456".to_string();
        config.secret_api = "test-secret-api-123456".to_string();
        config.log_statements = true;
        config
    }

    /// Initialize with custom configuration (for testing)
    pub async fn with_config(config: NodeConfig) -> Result<Self> {
        tracing::info!(
            node_id = ?config.node_id,
            "Starting hiqlite with custom config"
        );

        let client = hiqlite::start_node(config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to start hiqlite node: {}", e))?;

        Ok(Self { client })
    }

    /// Execute a SQL statement with parameters
    ///
    /// This operation is strongly consistent - it will be replicated
    /// to the Raft cluster before returning.
    ///
    /// Returns the number of rows affected.
    ///
    /// # Example
    /// ```no_run
    /// use hiqlite::params;
    /// service.execute(
    ///     "INSERT INTO workflows (id, status) VALUES ($1, $2)",
    ///     params!("wf123", "pending")
    /// ).await?;
    /// ```
    pub async fn execute(&self, query: impl Into<Cow<'static, str>>, params: Vec<Param>) -> Result<usize> {
        self.client
            .execute(query, params)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to execute query: {}", e))
    }

    /// Execute a SQL query and return results as typed structs
    ///
    /// # Example
    /// ```no_run
    /// use hiqlite::params;
    /// let results: Vec<MyStruct> = service
    ///     .query_as("SELECT id, status FROM workflows WHERE status = $1", params!("pending"))
    ///     .await?;
    /// ```
    pub async fn query_as<T>(&self, query: impl Into<Cow<'static, str>>, params: Vec<Param>) -> Result<Vec<T>>
    where
        T: serde::de::DeserializeOwned + From<hiqlite::Row<'static>> + Send + 'static,
    {
        self.client
            .query_as(query, params)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to query: {}", e))
    }

    /// Execute a SQL query and return a single result
    pub async fn query_as_one<T>(&self, query: impl Into<Cow<'static, str>>, params: Vec<Param>) -> Result<T>
    where
        T: serde::de::DeserializeOwned + From<hiqlite::Row<'static>> + Send + 'static,
    {
        self.client
            .query_as_one(query, params)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to query: {}", e))
    }

    /// Get the underlying hiqlite client for advanced operations
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Shutdown the hiqlite node gracefully
    ///
    /// IMPORTANT: Always call this before process exit to avoid
    /// full database rebuilds on restart.
    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("Shutting down hiqlite node gracefully");
        let _ = self.client
            .shutdown()
            .await;
        Ok(())
    }

    /// Initialize database schema for workflow state
    ///
    /// This should be called once during application startup to ensure
    /// the necessary tables exist.
    pub async fn initialize_schema(&self) -> Result<()> {
        tracing::info!("Initializing hiqlite schema");

        // Create workflows table
        self.execute(
            "CREATE TABLE IF NOT EXISTS workflows (
                id TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                claimed_by TEXT,
                completed_by TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                started_at INTEGER,
                error_message TEXT,
                retry_count INTEGER NOT NULL DEFAULT 0,
                data TEXT
            )",
            params!(),
        ).await?;

        // Add new columns to existing workflows table (migrations for existing databases)
        // These will fail silently if columns already exist
        let _ = self.execute(
            "ALTER TABLE workflows ADD COLUMN started_at INTEGER",
            params!(),
        ).await;

        let _ = self.execute(
            "ALTER TABLE workflows ADD COLUMN error_message TEXT",
            params!(),
        ).await;

        let _ = self.execute(
            "ALTER TABLE workflows ADD COLUMN retry_count INTEGER NOT NULL DEFAULT 0",
            params!(),
        ).await;

        let _ = self.execute(
            "ALTER TABLE workflows ADD COLUMN compatible_worker_types TEXT",
            params!(),
        ).await;

        let _ = self.execute(
            "ALTER TABLE workflows ADD COLUMN assigned_worker_id TEXT",
            params!(),
        ).await;

        // Create heartbeats table for node health tracking
        self.execute(
            "CREATE TABLE IF NOT EXISTS heartbeats (
                node_id TEXT PRIMARY KEY,
                last_seen INTEGER NOT NULL,
                status TEXT NOT NULL
            )",
            params!(),
        ).await?;

        // Create workers table for worker registry
        self.execute(
            "CREATE TABLE IF NOT EXISTS workers (
                id TEXT PRIMARY KEY,
                worker_type TEXT NOT NULL,
                status TEXT NOT NULL,
                endpoint_id TEXT NOT NULL,
                registered_at INTEGER NOT NULL,
                last_heartbeat INTEGER NOT NULL,
                cpu_cores INTEGER,
                memory_mb INTEGER,
                active_jobs INTEGER NOT NULL DEFAULT 0,
                total_jobs_completed INTEGER NOT NULL DEFAULT 0,
                metadata TEXT
            )",
            params!(),
        ).await?;

        // Create indices for efficient worker queries
        let _ = self.execute(
            "CREATE INDEX IF NOT EXISTS idx_workers_status ON workers(status)",
            params!(),
        ).await;

        let _ = self.execute(
            "CREATE INDEX IF NOT EXISTS idx_workers_type ON workers(worker_type)",
            params!(),
        ).await;

        let _ = self.execute(
            "CREATE INDEX IF NOT EXISTS idx_workers_heartbeat ON workers(last_heartbeat)",
            params!(),
        ).await;

        // Create VM registry table for distributed VM lifecycle management
        self.execute(
            "CREATE TABLE IF NOT EXISTS vms (
                id TEXT PRIMARY KEY,
                config TEXT NOT NULL,
                state TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                node_id TEXT NOT NULL,
                pid INTEGER,
                control_socket TEXT,
                job_dir TEXT,
                ip_address TEXT,
                metrics TEXT
            )",
            params!(),
        ).await?;

        // Create indices for VM queries
        let _ = self.execute(
            "CREATE INDEX IF NOT EXISTS idx_vms_state ON vms(state)",
            params!(),
        ).await;

        let _ = self.execute(
            "CREATE INDEX IF NOT EXISTS idx_vms_node_id ON vms(node_id)",
            params!(),
        ).await;

        // Create VM events table for audit trail
        self.execute(
            "CREATE TABLE IF NOT EXISTS vm_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                vm_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                event_data TEXT,
                timestamp INTEGER NOT NULL,
                node_id TEXT,
                FOREIGN KEY (vm_id) REFERENCES vms(id)
            )",
            params!(),
        ).await?;

        let _ = self.execute(
            "CREATE INDEX IF NOT EXISTS idx_vm_events_vm_id ON vm_events(vm_id)",
            params!(),
        ).await;

        let _ = self.execute(
            "CREATE INDEX IF NOT EXISTS idx_vm_events_timestamp ON vm_events(timestamp)",
            params!(),
        ).await;

        // Create execution_instances table for adapter-based execution tracking
        self.execute(
            "CREATE TABLE IF NOT EXISTS execution_instances (
                id TEXT PRIMARY KEY,
                backend_type TEXT NOT NULL,
                execution_handle TEXT NOT NULL,
                job_id TEXT,
                status TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                started_at INTEGER,
                completed_at INTEGER,
                node_id TEXT,
                metadata TEXT,
                FOREIGN KEY (job_id) REFERENCES workflows(id)
            )",
            params!(),
        ).await?;

        // Create indices for execution_instances queries
        let _ = self.execute(
            "CREATE INDEX IF NOT EXISTS idx_execution_instances_backend_type ON execution_instances(backend_type)",
            params!(),
        ).await;

        let _ = self.execute(
            "CREATE INDEX IF NOT EXISTS idx_execution_instances_job_id ON execution_instances(job_id)",
            params!(),
        ).await;

        let _ = self.execute(
            "CREATE INDEX IF NOT EXISTS idx_execution_instances_status ON execution_instances(status)",
            params!(),
        ).await;

        let _ = self.execute(
            "CREATE INDEX IF NOT EXISTS idx_execution_instances_created_at ON execution_instances(created_at)",
            params!(),
        ).await;

        // Create OpenTofu state backend tables
        self.execute(
            "CREATE TABLE IF NOT EXISTS tofu_workspaces (
                name TEXT PRIMARY KEY,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                current_state TEXT,
                state_version INTEGER NOT NULL DEFAULT 0,
                state_lineage TEXT,
                lock_id TEXT,
                lock_acquired_at INTEGER,
                lock_holder TEXT,
                lock_info TEXT
            )",
            params!(),
        ).await?;

        // Create index for lock operations
        let _ = self.execute(
            "CREATE INDEX IF NOT EXISTS idx_tofu_workspaces_lock_id ON tofu_workspaces(lock_id)",
            params!(),
        ).await;

        // Create table for OpenTofu plan storage
        self.execute(
            "CREATE TABLE IF NOT EXISTS tofu_plans (
                id TEXT PRIMARY KEY,
                workspace TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                plan_data BLOB NOT NULL,
                plan_json TEXT,
                status TEXT NOT NULL,
                approved_by TEXT,
                executed_at INTEGER,
                FOREIGN KEY (workspace) REFERENCES tofu_workspaces(name)
            )",
            params!(),
        ).await?;

        let _ = self.execute(
            "CREATE INDEX IF NOT EXISTS idx_tofu_plans_workspace ON tofu_plans(workspace)",
            params!(),
        ).await;

        let _ = self.execute(
            "CREATE INDEX IF NOT EXISTS idx_tofu_plans_status ON tofu_plans(status)",
            params!(),
        ).await;

        // Create table for OpenTofu state history (for rollback)
        self.execute(
            "CREATE TABLE IF NOT EXISTS tofu_state_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                workspace TEXT NOT NULL,
                state_data TEXT NOT NULL,
                state_version INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                created_by TEXT,
                FOREIGN KEY (workspace) REFERENCES tofu_workspaces(name)
            )",
            params!(),
        ).await?;

        let _ = self.execute(
            "CREATE INDEX IF NOT EXISTS idx_tofu_state_history_workspace ON tofu_state_history(workspace)",
            params!(),
        ).await;

        let _ = self.execute(
            "CREATE INDEX IF NOT EXISTS idx_tofu_state_history_created_at ON tofu_state_history(created_at DESC)",
            params!(),
        ).await;

        tracing::info!("Schema initialization complete");
        Ok(())
    }

    /// Check cluster health and connectivity
    pub async fn health_check(&self) -> Result<crate::domain::types::HealthStatus> {
        // Query hiqlite for actual cluster metrics
        match self.client.metrics_db().await {
            Ok(metrics) => {
                let membership = metrics.membership_config.membership();
                let node_count = membership.nodes().count();
                let has_leader = metrics.current_leader.is_some();
                let is_healthy = self.client.is_healthy_db().await.is_ok();

                Ok(crate::domain::types::HealthStatus {
                    is_healthy,
                    node_count,
                    has_leader,
                })
            }
            Err(e) => {
                tracing::warn!("Failed to get cluster metrics: {}", e);
                Ok(crate::domain::types::HealthStatus {
                    is_healthy: false,
                    node_count: 0,
                    has_leader: false,
                })
            }
        }
    }
}

// ClusterHealth type removed - use domain::types::HealthStatus instead

// =============================================================================
// TRAIT IMPLEMENTATIONS
// =============================================================================

#[async_trait]
impl DatabaseQueries for HiqliteService {
    async fn execute(
        &self,
        query: impl Into<Cow<'static, str>> + Send,
        params: Vec<Param>,
    ) -> Result<usize> {
        self.client
            .execute(query, params)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to execute query: {}", e))
    }

    async fn query_as<T>(
        &self,
        query: impl Into<Cow<'static, str>> + Send,
        params: Vec<Param>,
    ) -> Result<Vec<T>>
    where
        T: serde::de::DeserializeOwned + From<hiqlite::Row<'static>> + Send + 'static,
    {
        self.client
            .query_as(query, params)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to query: {}", e))
    }

    async fn query_as_one<T>(
        &self,
        query: impl Into<Cow<'static, str>> + Send,
        params: Vec<Param>,
    ) -> Result<T>
    where
        T: serde::de::DeserializeOwned + From<hiqlite::Row<'static>> + Send + 'static,
    {
        self.client
            .query_as_one(query, params)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to query: {}", e))
    }
}

#[async_trait]
impl DatabaseHealth for HiqliteService {
    async fn health_check(&self) -> Result<crate::domain::types::HealthStatus> {
        // Query hiqlite for actual cluster metrics
        match self.client.metrics_db().await {
            Ok(metrics) => {
                let membership = metrics.membership_config.membership();
                let node_count = membership.nodes().count();
                let has_leader = metrics.current_leader.is_some();
                let is_healthy = self.client.is_healthy_db().await.is_ok();

                Ok(crate::domain::types::HealthStatus {
                    is_healthy,
                    node_count,
                    has_leader,
                })
            }
            Err(e) => {
                tracing::warn!("Failed to get cluster metrics: {}", e);
                Ok(crate::domain::types::HealthStatus {
                    is_healthy: false,
                    node_count: 0,
                    has_leader: false,
                })
            }
        }
    }
}

#[async_trait]
impl DatabaseSchema for HiqliteService {
    async fn initialize_schema(&self) -> Result<()> {
        // Delegate to the public method
        Self::initialize_schema(self).await
    }
}

#[async_trait]
impl DatabaseLifecycle for HiqliteService {
    async fn shutdown(&self) -> Result<()> {
        tracing::info!("Shutting down hiqlite node gracefully");
        let _ = self.client
            .shutdown()
            .await;
        Ok(())
    }
}
