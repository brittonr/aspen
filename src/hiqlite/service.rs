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
    ///
    /// Creates a minimal HiqliteService for testing scenarios. This method can be called
    /// from any context (sync or async) and avoids the "cannot create runtime within runtime"
    /// error by spawning the initialization in a separate thread when necessary.
    ///
    /// # Implementation Details
    ///
    /// When called from within a tokio runtime context, this spawns a new thread and creates
    /// a fresh runtime in that thread to perform the async initialization. When called from
    /// outside a runtime, it creates a runtime in the current thread. This ensures that we
    /// never attempt to create a nested runtime, which would panic.
    ///
    /// # Panics
    /// Panics if HiqliteService::new fails to initialize.
    ///
    /// # Note
    /// This is primarily used in test contexts where TofuService requires a HiqliteService
    /// dependency but tests won't actually invoke operations on it.
    pub fn placeholder() -> Self {
        // Check if we're in a tokio runtime
        match tokio::runtime::Handle::try_current() {
            Ok(_) => {
                // We're in a runtime, but we need to use spawn_blocking to avoid
                // issues with block_on being called from an async context
                std::thread::scope(|s| {
                    let join_handle = s.spawn(|| {
                        // Create a new runtime in this thread for the async initialization
                        let rt = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .expect("Failed to create runtime for placeholder");

                        rt.block_on(async {
                            // Use unique directory for placeholder to avoid conflicts
                            let temp_dir = format!("/tmp/hiqlite-placeholder-{}", std::process::id());
                            Self::new(Some(std::path::PathBuf::from(temp_dir))).await
                                .expect("Failed to create placeholder HiqliteService")
                        })
                    });

                    join_handle.join().expect("Thread panic in placeholder creation")
                })
            }
            Err(_) => {
                // Not in a runtime, create one
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to create runtime for placeholder");

                rt.block_on(async {
                    // Use unique directory for placeholder to avoid conflicts
                    let temp_dir = format!("/tmp/hiqlite-placeholder-{}", std::process::id());
                    Self::new(Some(std::path::PathBuf::from(temp_dir))).await
                        .expect("Failed to create placeholder HiqliteService")
                })
            }
        }
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
    /// use hiqlite::HiqliteService;
    ///
    /// let service = HiqliteService::new(Some("./data/custom".into())).await?;
    /// ```
    pub async fn new(data_dir: impl Into<Option<std::path::PathBuf>>) -> Result<Self> {
        Self::new_with_operational_config(data_dir, None).await
    }

    /// Initialize a new hiqlite node with custom operational configuration
    ///
    /// # Parameters
    /// - `data_dir`: Optional data directory override
    /// - `operational_config`: Optional operational config for timing values (uses defaults if None)
    pub async fn new_with_operational_config(
        data_dir: impl Into<Option<std::path::PathBuf>>,
        operational_config: Option<&crate::config::OperationalConfig>,
    ) -> Result<Self> {
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

        // Use operational config for startup delay, or default to 3 seconds
        let startup_delay = operational_config
            .map(|c| std::time::Duration::from_secs(c.hiqlite_startup_delay_secs))
            .unwrap_or(std::time::Duration::from_secs(3));

        tokio::time::sleep(startup_delay).await;

        // Wait for cluster to form (only if we have multiple nodes configured)
        if expected_nodes > 1 {
            let cluster_config = operational_config
                .map(crate::hiqlite::ClusterFormationConfig::from_operational)
                .unwrap_or_default();

            let coordinator = crate::hiqlite::ClusterFormationCoordinator::new(cluster_config);
            coordinator.wait_for_cluster_formation(&client, node_id, expected_nodes).await?;
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
        if let Err(e) = self.client.shutdown().await {
            tracing::warn!("Error during hiqlite shutdown: {}", e);
        }
        Ok(())
    }

    // Schema initialization moved to schema.rs module

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
        if let Err(e) = self.client.shutdown().await {
            tracing::warn!("Error during hiqlite shutdown: {}", e);
        }
        Ok(())
    }
}
