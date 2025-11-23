//! Service trait abstractions for infrastructure services
//!
//! This module defines trait-based abstractions for infrastructure services,
//! enabling dependency injection, testing with mocks, and flexible implementations.
//!
//! # Design Philosophy
//!
//! Services are split into **multiple focused traits** rather than monolithic interfaces:
//! - Each trait represents a distinct capability
//! - Implementations can be composed as needed
//! - Mocks only need to implement what they use
//! - Clear separation of concerns
//!
//! # Trait Organization
//!
//! ## Iroh Networking Traits
//! - `EndpointInfo`: Node identity and addressing (always available)
//! - `BlobStorage`: Content-addressed blob storage (optional capability)
//! - `GossipNetwork`: Distributed pub/sub messaging (optional capability)
//! - `PeerConnection`: Peer connection management (optional capability)
//!
//! ## Hiqlite Database Traits
//! - `DatabaseQueries`: SQL query and command execution (core operations)
//! - `DatabaseHealth`: Cluster health monitoring
//! - `DatabaseSchema`: Schema initialization and migration
//! - `DatabaseLifecycle`: Graceful shutdown

use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use std::borrow::Cow;

// Re-export types used in trait signatures
pub use hiqlite::Param;
pub use iroh::{EndpointAddr, EndpointId};

// Re-export ClusterHealth from hiqlite_service
use crate::hiqlite_service::ClusterHealth;

// =============================================================================
// IROH NETWORKING TRAITS
// =============================================================================

/// Node identity and addressing information
///
/// This trait provides read-only access to the node's cryptographic identity
/// and network addressing information. It's always available and fully implemented.
///
/// # Examples
///
/// ```rust,no_run
/// # use mvm_ci::services::traits::EndpointInfo;
/// # async fn example(endpoint: &dyn EndpointInfo) {
/// let node_id = endpoint.endpoint_id();
/// println!("Node ID: {}", node_id);
/// # }
/// ```
pub trait EndpointInfo: Send + Sync {
    /// Get the node's unique identifier (public key)
    fn endpoint_id(&self) -> EndpointId;

    /// Get the full endpoint address (relay URLs + direct addresses)
    fn endpoint_addr(&self) -> EndpointAddr;

    /// Get local socket addresses where the endpoint is listening
    fn local_endpoints(&self) -> Vec<String>;
}

/// Content-addressed blob storage capability
///
/// Provides distributed storage and retrieval of immutable blobs using
/// cryptographic hashes (BLAKE3) as identifiers.
///
/// # Implementation Status
///
/// Current implementations may return placeholder errors. Check specific
/// implementations for availability.
#[async_trait]
pub trait BlobStorage: Send + Sync {
    /// Store a blob and return its content hash
    ///
    /// # Arguments
    /// * `data` - The blob data to store
    ///
    /// # Returns
    /// Hash identifier (typically BLAKE3 in base32 encoding)
    async fn store_blob(&self, data: Bytes) -> Result<String>;

    /// Retrieve a blob by its content hash
    ///
    /// # Arguments
    /// * `hash` - The content hash identifier
    ///
    /// # Returns
    /// The blob data as bytes
    async fn retrieve_blob(&self, hash: &str) -> Result<Bytes>;
}

/// Distributed pub/sub messaging via gossip protocol
///
/// Enables broadcasting messages to all peers subscribed to a topic,
/// with eventual consistency guarantees.
///
/// # Implementation Status
///
/// Current implementations may return placeholder errors. Check specific
/// implementations for availability.
#[async_trait]
pub trait GossipNetwork: Send + Sync {
    /// Subscribe to a gossip topic
    ///
    /// # Arguments
    /// * `topic_id` - The topic identifier to join
    async fn join_topic(&self, topic_id: &str) -> Result<()>;

    /// Broadcast a message to all peers on a topic
    ///
    /// # Arguments
    /// * `topic_id` - The topic to broadcast on
    /// * `message` - The message data to broadcast
    async fn broadcast_message(&self, topic_id: &str, message: Bytes) -> Result<()>;
}

/// Peer connection management
///
/// Provides explicit control over peer connections, allowing manual
/// connection establishment and peer discovery.
///
/// # Implementation Status
///
/// Current implementations may return placeholder errors. Check specific
/// implementations for availability.
#[async_trait]
pub trait PeerConnection: Send + Sync {
    /// Connect to a peer using an endpoint address or ticket
    ///
    /// # Arguments
    /// * `endpoint_addr_str` - The endpoint address string (e.g., iroh ticket)
    async fn connect_peer(&self, endpoint_addr_str: &str) -> Result<()>;
}

// =============================================================================
// HIQLITE DATABASE TRAITS
// =============================================================================

/// SQL query and command execution
///
/// Provides strongly consistent distributed SQL operations via Raft consensus.
/// All writes are replicated across the cluster before acknowledging.
///
/// # Examples
///
/// ```rust,no_run
/// # use mvm_ci::services::traits::DatabaseQueries;
/// # use mvm_ci::params;
/// # async fn example(db: &dyn DatabaseQueries) -> anyhow::Result<()> {
/// // Execute a command
/// let rows = db.execute(
///     "INSERT INTO workflows (id, status) VALUES ($1, $2)",
///     params!("job-123", "pending")
/// ).await?;
///
/// // Query with type mapping
/// #[derive(serde::Deserialize)]
/// struct WorkflowRow {
///     id: String,
///     status: String,
/// }
///
/// impl From<hiqlite::Row<'static>> for WorkflowRow {
///     fn from(mut row: hiqlite::Row<'static>) -> Self {
///         Self {
///             id: row.get("id"),
///             status: row.get("status"),
///         }
///     }
/// }
///
/// let rows: Vec<WorkflowRow> = db.query_as(
///     "SELECT id, status FROM workflows WHERE status = $1",
///     params!("pending")
/// ).await?;
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait DatabaseQueries: Send + Sync {
    /// Execute a SQL statement (INSERT, UPDATE, DELETE)
    ///
    /// # Arguments
    /// * `query` - SQL statement with $1, $2, etc. placeholders
    /// * `params` - Query parameters (use `params!` macro)
    ///
    /// # Returns
    /// Number of rows affected
    async fn execute(
        &self,
        query: impl Into<Cow<'static, str>> + Send,
        params: Vec<Param>,
    ) -> Result<usize>;

    /// Execute a query and deserialize results
    ///
    /// # Type Requirements
    /// The type `T` must implement:
    /// - `serde::de::DeserializeOwned`
    /// - `From<hiqlite::Row<'static>>`
    /// - `Send + 'static`
    ///
    /// # Arguments
    /// * `query` - SQL SELECT statement
    /// * `params` - Query parameters
    ///
    /// # Returns
    /// Vector of deserialized results
    async fn query_as<T>(
        &self,
        query: impl Into<Cow<'static, str>> + Send,
        params: Vec<Param>,
    ) -> Result<Vec<T>>
    where
        T: serde::de::DeserializeOwned + From<hiqlite::Row<'static>> + Send + 'static;

    /// Execute a query and return a single result
    ///
    /// # Arguments
    /// * `query` - SQL SELECT statement expected to return one row
    /// * `params` - Query parameters
    ///
    /// # Returns
    /// Single deserialized result (errors if zero or multiple rows)
    async fn query_as_one<T>(
        &self,
        query: impl Into<Cow<'static, str>> + Send,
        params: Vec<Param>,
    ) -> Result<T>
    where
        T: serde::de::DeserializeOwned + From<hiqlite::Row<'static>> + Send + 'static;
}

/// Database cluster health monitoring
///
/// Provides visibility into cluster state, leadership, and node membership.
#[async_trait]
pub trait DatabaseHealth: Send + Sync {
    /// Check cluster health status
    ///
    /// # Returns
    /// Cluster health information including node count and leader status
    async fn health_check(&self) -> Result<ClusterHealth>;
}

/// Database schema management
///
/// Handles schema initialization, migrations, and DDL operations.
#[async_trait]
pub trait DatabaseSchema: Send + Sync {
    /// Initialize database schema
    ///
    /// Creates tables and indexes if they don't exist. Should be called
    /// once at application startup.
    async fn initialize_schema(&self) -> Result<()>;
}

/// Database lifecycle management
///
/// Handles graceful shutdown and cleanup operations.
#[async_trait]
pub trait DatabaseLifecycle: Send + Sync {
    /// Gracefully shutdown the database
    ///
    /// **CRITICAL**: Must be called before process exit to prevent
    /// full database rebuilds on restart.
    async fn shutdown(&self) -> Result<()>;
}
