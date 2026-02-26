//! Modular cluster node bootstrap system.
//!
//! This module provides the bootstrap orchestration for Aspen nodes using
//! direct async APIs. The bootstrap process creates and initializes all
//! necessary components for a functioning Raft cluster node.
//!
//! # Architecture
//!
//! The bootstrap system is organized into focused modules:
//!
//! - `resources`: Resource structs grouping related components
//! - `traits`: ResourceBuilder trait for modular initialization
//! - `node`: Main bootstrap functions and NodeHandle
//!
//! # Bootstrap Process
//!
//! The `bootstrap_node()` function orchestrates initialization in phases:
//!
//! 1. **Phase 1 (Parallel)**: Storage | Network
//! 2. **Phase 2 (Sequential)**: Discovery (needs Network)
//! 3. **Phase 3 (Parallel)**: Sync | Blob | Worker
//! 4. **Phase 4 (Sequential)**: Hooks (needs Sync, Blob)
//!
//! # Gossip Discovery
//!
//! When gossip is enabled (`config.iroh.enable_gossip = true`), the bootstrap
//! automatically spawns a `GossipPeerDiscovery` instance that:
//! 1. Derives a topic ID from the cluster cookie (or uses a ticket's topic ID)
//! 2. Broadcasts this node's ID and EndpointAddr every 10 seconds
//! 3. Receives peer announcements and adds them to the network factory
//!
//! # Example
//!
//! ```ignore
//! use aspen_cluster::bootstrap::bootstrap_node;
//!
//! let handle = bootstrap_node(config).await?;
//! // Node is now running, use handle.storage.raft_node for operations
//! handle.shutdown().await?;
//! ```

mod node;
pub mod resources;
pub mod traits;

// Re-export main types from resources for convenience
pub use node::BaseDiscoveryResources;
pub use node::BaseNodeResources;
pub use node::NodeHandle;
pub use node::ShardedNodeHandle;
pub use node::ShardingResources;
// Re-export main bootstrap functions and types from node module
pub use node::auto_announce_local_blobs;
pub use node::bootstrap_node;
pub use node::bootstrap_sharded_node;
pub use node::initialize_blob_replication;
pub use node::load_config;
pub use resources::BlobReplicationResources;
pub use resources::DiscoveryResources;
pub use resources::HookResources;
pub use resources::NetworkResources;
pub use resources::ShutdownCoordinator;
pub use resources::StorageResources;
pub use resources::SyncResources;
pub use resources::WorkerResources;
// Re-export traits
pub use traits::DiscoveryDependencies;
pub use traits::HookDependencies;
pub use traits::NetworkDependencies;
pub use traits::ResourceBuilder;
pub use traits::StorageDependencies;
pub use traits::SyncDependencies;
pub use traits::WorkerDependencies;
