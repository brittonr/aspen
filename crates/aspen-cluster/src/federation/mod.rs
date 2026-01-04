//! Federation support for cross-cluster communication.
//!
//! This module provides infrastructure for federating independent Aspen clusters,
//! enabling them to discover each other, share content, and synchronize resources
//! across organizational boundaries.
//!
//! # Architecture
//!
//! Federation is built on three core concepts:
//!
//! 1. **Cluster Identity**: Each cluster has a stable Ed25519 keypair that persists across node
//!    changes. This identity is used to sign federation announcements and establish trust
//!    relationships.
//!
//! 2. **Federated IDs**: Resources (like Forge repositories) gain global uniqueness through
//!    origin-prefixed identifiers: `origin_cluster_key:local_id`. This ties authority to the
//!    originating cluster while allowing content to flow freely.
//!
//! 3. **Pull-Based Sync**: Cross-cluster synchronization is pull-based with eventual consistency.
//!    Clusters discover each other via DHT, then fetch missing content on demand with signature
//!    verification.
//!
//! # Design Principles
//!
//! - **No HTTP/DNS required**: Pure P2P via iroh (unlike Forgejo/Tangled)
//! - **Self-sovereign identity**: Ed25519 keys, no external provider
//! - **Strong consistency within cluster**: Raft for authoritative state
//! - **Eventual consistency across clusters**: Pull-based sync with verification
//! - **DHT-based discovery**: No central relay needed
//!
//! # Trust Model
//!
//! Federation supports two modes per-resource:
//!
//! - **Public**: Anyone can discover and sync (requires robust spam protection)
//! - **AllowList**: Only explicitly trusted clusters can sync
//!
//! All cross-cluster data is verified:
//! 1. Cluster signatures on announcements
//! 2. Delegate signatures on canonical refs (for Forge)
//! 3. Content-addressed hashes on all objects
//!
//! # Example
//!
//! ```ignore
//! use aspen::cluster::federation::{ClusterIdentity, FederatedId};
//!
//! // Create cluster identity (usually loaded from config)
//! let identity = ClusterIdentity::generate("my-org".to_string());
//!
//! // Create a federated resource ID
//! let local_id = [0u8; 32]; // e.g., RepoId bytes
//! let fed_id = FederatedId::new(identity.public_key, local_id);
//!
//! // The federated ID is globally unique
//! println!("Federated ID: {}", fed_id.to_string());
//! ```

pub mod discovery;
pub mod gossip;
pub mod identity;
pub mod sync;
pub mod trust;
pub mod types;

pub use discovery::ClusterAnnouncement;
pub use discovery::DiscoveredCluster;
pub use discovery::DiscoveredSeeder;
pub use discovery::FederationDiscoveryService;
pub use discovery::ResourceAnnouncement;
pub use gossip::FederationEvent;
pub use gossip::FederationGossipMessage;
pub use gossip::FederationGossipService;
pub use gossip::SignedFederationMessage;
pub use identity::ClusterIdentity;
pub use identity::SignedClusterIdentity;
pub use sync::FEDERATION_ALPN;
pub use sync::FederationProtocolHandler;
pub use sync::FederationRequest;
pub use sync::FederationResponse;
pub use trust::TrustLevel;
pub use trust::TrustManager;
pub use trust::TrustRequest;
pub use types::FederatedId;
pub use types::FederationMode;
pub use types::FederationSettings;
