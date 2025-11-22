//! Hiqlite Service - Distributed SQLite with Raft Consensus
//!
//! This module provides a distributed state management layer using hiqlite,
//! a Raft-replicated SQLite database. It serves as the foundation for:
//! - Workflow state replication across nodes
//! - Strong consistency guarantees for critical operations
//! - Automatic leader failover and self-healing

use anyhow::Result;
use hiqlite::{Client, NodeConfig, Param};
use std::borrow::Cow;
use tokio::sync::OnceCell;

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
    /// let service = HiqliteService::new().await?;
    /// ```
    pub async fn new() -> Result<Self> {
        let config = Self::build_config().await?;

        tracing::info!(
            node_id = ?config.node_id,
            data_dir = ?config.data_dir,
            "Initializing hiqlite node"
        );

        let client = hiqlite::start_node(config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to start hiqlite node: {}", e))?;

        tracing::info!("Hiqlite node initialized successfully");

        Ok(Self { client })
    }

    /// Build NodeConfig from environment variables with sensible defaults
    async fn build_config() -> Result<NodeConfig> {
        // Try loading from TOML file first, fall back to environment
        if let Ok(config) = NodeConfig::from_toml("hiqlite.toml", None, None).await {
            return Ok(config);
        }

        // Otherwise build from environment with defaults
        let config = NodeConfig::from_env();
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

        let mut config = NodeConfig::default();
        config.node_id = node_id;
        config.nodes = nodes;
        config.data_dir = data_dir.to_string_lossy().into();
        config.secret_raft = "test-secret-raft-123456".to_string();
        config.secret_api = "test-secret-api-123456".to_string();
        config.log_statements = true;
        config
    }

    /// Initialize with custom configuration (for testing)
    pub async fn with_config(mut config: NodeConfig) -> Result<Self> {
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
        self.client
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
                data TEXT
            )",
            params!(),
        ).await?;

        // Create heartbeats table for node health tracking
        self.execute(
            "CREATE TABLE IF NOT EXISTS heartbeats (
                node_id TEXT PRIMARY KEY,
                last_seen INTEGER NOT NULL,
                status TEXT NOT NULL
            )",
            params!(),
        ).await?;

        tracing::info!("Schema initialization complete");
        Ok(())
    }

    /// Check cluster health and connectivity
    pub async fn health_check(&self) -> Result<ClusterHealth> {
        // TODO: Query hiqlite for cluster status
        // For now, return a simple health check
        Ok(ClusterHealth {
            is_healthy: true,
            node_count: 1,
            has_leader: true,
        })
    }
}

/// Cluster health status
#[derive(Debug, Clone, serde::Serialize)]
pub struct ClusterHealth {
    pub is_healthy: bool,
    pub node_count: usize,
    pub has_leader: bool,
}

/// Global hiqlite service instance
static HIQLITE: OnceCell<HiqliteService> = OnceCell::const_new();

/// Initialize the global hiqlite service
pub async fn init_hiqlite() -> Result<()> {
    let service = HiqliteService::new().await?;
    service.initialize_schema().await?;

    HIQLITE
        .set(service)
        .map_err(|_| anyhow::anyhow!("Hiqlite already initialized"))?;

    Ok(())
}

/// Get the global hiqlite service instance
pub fn hiqlite() -> &'static HiqliteService {
    HIQLITE
        .get()
        .expect("Hiqlite not initialized - call init_hiqlite() first")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_hiqlite_initialization() {
        let temp_dir = TempDir::new().unwrap();
        let config = HiqliteService::build_test_config(1, temp_dir.path().to_path_buf());

        let service = HiqliteService::with_config(config)
            .await
            .unwrap();

        service.initialize_schema().await.unwrap();
        service.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_workflow_storage() {
        let temp_dir = TempDir::new().unwrap();
        let config = HiqliteService::build_test_config(1, temp_dir.path().to_path_buf());

        let service = HiqliteService::with_config(config)
            .await
            .unwrap();

        service.initialize_schema().await.unwrap();

        // Insert a workflow
        service.execute(
            "INSERT INTO workflows (id, status, created_at, updated_at) VALUES ($1, $2, $3, $4)",
            params!("test-wf", "pending", 1234567890_i64, 1234567890_i64),
        ).await.unwrap();

        // Note: query_as requires implementing From<Row> for the type
        // For now we'll just verify the insert succeeded
        let rows_affected = service.execute(
            "SELECT COUNT(*) FROM workflows WHERE id = $1",
            params!("test-wf"),
        ).await;

        assert!(rows_affected.is_ok());
        service.shutdown().await.unwrap();
    }
}
