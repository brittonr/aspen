//! Bootstrap traits for modular resource building.
//!
//! This module defines the `ResourceBuilder` trait pattern used for creating
//! and initializing node resources in a modular, testable way.
//!
//! # ResourceBuilder Pattern
//!
//! Each resource type (storage, network, discovery, etc.) has a corresponding
//! builder that implements the `ResourceBuilder` trait. Builders:
//!
//! - Take configuration and dependencies as input
//! - Create and initialize the resource
//! - Return the fully configured resource or an error
//! - Can be unit tested in isolation
//!
//! # Example
//!
//! ```ignore
//! use aspen_cluster::bootstrap::traits::ResourceBuilder;
//!
//! let storage = StorageBuilder::build(&config, deps, shutdown).await?;
//! let network = NetworkBuilder::build(&config, deps, shutdown).await?;
//! ```

use std::sync::Arc;

use anyhow::Result;
use tokio_util::sync::CancellationToken;

use crate::config::NodeConfig;

/// Trait for building node resources in a modular way.
///
/// Each resource type has a corresponding builder that implements this trait.
/// Builders encapsulate the initialization logic for a specific resource,
/// making it easier to test and maintain.
///
/// # Type Parameters
///
/// - `Resource`: The type of resource this builder creates
/// - `Config`: Additional configuration beyond `NodeConfig` (often `()`)
/// - `Dependencies`: Other resources this builder depends on
///
/// # Lifecycle
///
/// 1. `should_build()` checks if the resource should be created
/// 2. `build()` creates and initializes the resource
/// 3. The returned resource is stored in the appropriate `*Resources` struct
#[async_trait::async_trait]
pub trait ResourceBuilder: Send + Sync {
    /// The type of resource this builder creates.
    type Resource: Send + Sync;

    /// Additional configuration beyond the base `NodeConfig`.
    type Config: Send + Sync;

    /// Other resources this builder depends on.
    type Dependencies: Send + Sync;

    /// Build and initialize the resource.
    ///
    /// # Arguments
    ///
    /// * `config` - Base node configuration
    /// * `extra_config` - Additional configuration specific to this resource
    /// * `deps` - Dependencies required by this resource
    /// * `shutdown` - Cancellation token for graceful shutdown
    ///
    /// # Returns
    ///
    /// The fully initialized resource or an error.
    async fn build(
        config: &NodeConfig,
        extra_config: Self::Config,
        deps: Self::Dependencies,
        shutdown: CancellationToken,
    ) -> Result<Self::Resource>;

    /// Check if this resource should be built based on configuration.
    ///
    /// Default implementation returns `true`. Override for resources
    /// that are conditionally enabled (e.g., blob store, gossip discovery).
    fn should_build(config: &NodeConfig, _extra_config: &Self::Config) -> bool {
        let _ = config;
        true
    }
}

/// Dependencies for storage resource building.
pub struct StorageDependencies {
    /// Path to the data directory.
    pub data_dir: std::path::PathBuf,
    /// Log broadcast sender for log entries.
    pub log_broadcast: Option<tokio::sync::broadcast::Sender<aspen_raft::log_subscriber::LogEntryPayload>>,
    /// Snapshot event sender for snapshot operations.
    pub snapshot_broadcast: Option<tokio::sync::broadcast::Sender<aspen_raft::storage_shared::SnapshotEvent>>,
}

/// Dependencies for network resource building.
pub struct NetworkDependencies {
    /// Peer addresses for initial connections.
    pub peer_addrs: std::collections::HashMap<aspen_raft::types::NodeId, iroh::EndpointAddr>,
}

/// Dependencies for discovery resource building.
pub struct DiscoveryDependencies {
    /// Iroh endpoint manager for P2P connections.
    pub iroh_manager: Arc<crate::IrohEndpointManager>,
    /// Network factory for registering discovered peers.
    pub network_factory: Arc<crate::IrpcRaftNetworkFactory>,
    /// Gossip topic ID for peer discovery.
    pub gossip_topic_id: iroh_gossip::proto::TopicId,
    /// Shutdown token for graceful cleanup.
    pub shutdown: CancellationToken,
}

/// Dependencies for sync resource building.
pub struct SyncDependencies {
    /// Blob store for content-addressed storage.
    pub blob_store: Option<Arc<aspen_blob::IrohBlobStore>>,
    /// Log broadcast receiver for observing Raft log entries.
    pub log_broadcast: Option<tokio::sync::broadcast::Sender<aspen_raft::log_subscriber::LogEntryPayload>>,
    /// Docs event broadcaster for hook integration.
    pub docs_broadcaster: Option<Arc<aspen_docs::DocsEventBroadcaster>>,
}

/// Dependencies for worker resource building.
pub struct WorkerDependencies {
    /// Job manager for distributed job execution.
    pub job_manager: Option<Arc<aspen_jobs::JobManager<dyn aspen_core::KeyValueStore>>>,
    /// KV store for worker coordination.
    pub kv_store: Arc<dyn aspen_core::KeyValueStore>,
}

/// Dependencies for hook resource building.
pub struct HookDependencies {
    /// Log broadcast receiver for observing Raft log entries.
    pub log_broadcast: Option<tokio::sync::broadcast::Sender<aspen_raft::log_subscriber::LogEntryPayload>>,
    /// Snapshot event receiver for snapshot operations.
    pub snapshot_broadcast: Option<tokio::sync::broadcast::Sender<aspen_raft::storage_shared::SnapshotEvent>>,
    /// Blob event sender for blob operations.
    pub blob_event_sender: Option<tokio::sync::broadcast::Sender<aspen_blob::BlobEvent>>,
    /// Docs event sender for docs operations.
    pub docs_event_sender: Option<tokio::sync::broadcast::Sender<aspen_docs::DocsEvent>>,
    /// Raft node for metrics access.
    pub raft_node: Arc<aspen_raft::node::RaftNode>,
    /// State machine for TTL events.
    pub state_machine: aspen_raft::StateMachineVariant,
}
