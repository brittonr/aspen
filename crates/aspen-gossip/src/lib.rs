//! Gossip-based peer discovery for Aspen clusters.
//!
//! This crate provides automatic peer discovery using iroh-gossip, enabling
//! nodes to announce their presence and discover peers without manual configuration.
//!
//! # Architecture
//!
//! Each node:
//! 1. Subscribes to a cluster-wide gossip topic (derived from cluster cookie)
//! 2. Periodically broadcasts its EndpointAddr (every 10 seconds)
//! 3. Listens for peer announcements and adds them to the Iroh endpoint
//!
//! # Security
//!
//! Messages are signed with the node's SecretKey and verified on receipt.
//! Invalid signatures are rejected (fail-fast).
//!
//! # Example
//!
//! ```no_run
//! use aspen_gossip::GossipPeerDiscovery;
//! use iroh_gossip::proto::TopicId;
//!
//! # async fn example(
//! #     node_id: u64,
//! #     iroh_endpoint: &iroh::Endpoint,
//! #     gossip: std::sync::Arc<iroh_gossip::net::Gossip>,
//! #     endpoint_addr: iroh::EndpointAddr,
//! #     secret_key: iroh::SecretKey,
//! # ) -> anyhow::Result<()> {
//! let topic_id = TopicId::from_bytes([1u8; 32]);
//! let discovery = GossipPeerDiscovery::new(
//!     topic_id,
//!     node_id.into(),
//!     gossip,
//!     endpoint_addr,
//!     secret_key,
//! )?;
//!
//! // Discovery runs in background, automatically connecting to discovered peers...
//!
//! discovery.shutdown().await?;
//! # Ok(())
//! # }
//! ```

pub mod constants;
pub mod discovery;
pub mod rate_limiter;
pub mod types;

// Re-export main types for convenience
pub use discovery::{broadcast_blob_announcement, BlobAnnouncementParams, GossipPeerDiscovery};
pub use types::{BlobAnnouncement, GossipMessage, PeerAnnouncement, SignedBlobAnnouncement};

// Re-export from aspen-core for compatibility
pub use aspen_core::{DiscoveredPeer, DiscoveryHandle, PeerDiscoveredCallback, PeerDiscovery};