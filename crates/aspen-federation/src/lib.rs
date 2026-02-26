//! Federation support for cross-cluster communication.

#![allow(
    dead_code,
    unused_imports,
    clippy::useless_conversion,
    clippy::await_holding_lock,
    clippy::collapsible_if
)]
//!
//! This crate provides infrastructure for federating independent Aspen clusters,
//! enabling them to discover each other, share content, and synchronize resources
//! across organizational boundaries.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                         Federation Layer                                 │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                          │
//! │   ┌────────────────┐         ┌────────────────┐                         │
//! │   │  Cluster A     │  gossip │  Cluster B     │                         │
//! │   │ ┌────────────┐ │◄───────►│ ┌────────────┐ │                         │
//! │   │ │ Identity   │ │         │ │ Identity   │ │                         │
//! │   │ │ (Ed25519)  │ │         │ │ (Ed25519)  │ │                         │
//! │   │ └────────────┘ │         │ └────────────┘ │                         │
//! │   │ ┌────────────┐ │         │ ┌────────────┐ │                         │
//! │   │ │ AppRegistry│ │         │ │ AppRegistry│ │                         │
//! │   │ └────────────┘ │         │ └────────────┘ │                         │
//! │   │ ┌────────────┐ │   sync  │ ┌────────────┐ │                         │
//! │   │ │ Resources  │ │◄───────►│ │ Resources  │ │                         │
//! │   │ └────────────┘ │         │ └────────────┘ │                         │
//! │   └───────┬────────┘         └───────┬────────┘                         │
//! │           │                          │                                   │
//! │   ┌───────▼──────────────────────────▼───────┐                          │
//! │   │              DHT Discovery               │                          │
//! │   │      (BitTorrent Mainline BEP-44)        │                          │
//! │   └──────────────────────────────────────────┘                          │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
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
//! # Key Components
//!
//! | Component | Purpose | Module |
//! |-----------|---------|--------|
//! | [`ClusterIdentity`] | Ed25519 keypair for signing | [`identity`] |
//! | [`FederatedId`] | Globally unique resource identifier | [`types`] |
//! | [`AppRegistry`] | Tracks installed applications | [`app_registry`] |
//! | [`TrustManager`] | Per-cluster trust relationships | [`trust`] |
//! | [`FederationDiscoveryService`] | DHT-based peer discovery | [`discovery`] |
//! | [`FederationGossipService`] | Real-time announcements | [`gossip`] |
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
//! # Tiger Style Resource Bounds
//!
//! All federation operations have fixed limits to prevent resource exhaustion:
//!
//! | Resource | Limit | Constant |
//! |----------|-------|----------|
//! | Apps per cluster | 32 | [`MAX_APPS_PER_CLUSTER`] |
//! | Capabilities per app | 16 | [`MAX_CAPABILITIES_PER_APP`] |
//! | Tracked clusters | 1024 | `discovery::MAX_TRACKED_CLUSTERS` |
//! | Gossip rate per cluster | 12/min | `gossip::CLUSTER_RATE_PER_MINUTE` |
//! | Global gossip rate | 600/min | `gossip::GLOBAL_RATE_PER_MINUTE` |
//!
//! # Feature Flags
//!
//! - **`global-discovery`**: Enables actual DHT operations. Without this, discovery operations are
//!   logged but don't perform real DHT queries (useful for testing).

pub mod app_registry;
pub mod discovery;
pub mod gossip;
pub mod identity;
pub mod resolver;
pub mod sync;
pub mod trust;
pub mod types;

// App registry types
pub use app_registry::AppManifest;
pub use app_registry::AppRegistry;
pub use app_registry::MAX_APPS_PER_CLUSTER;
pub use app_registry::MAX_CAPABILITIES_PER_APP;
pub use app_registry::SharedAppRegistry;
pub use app_registry::shared_registry;
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
pub use resolver::DirectResourceResolver;
pub use resolver::FederationResourceError;
pub use resolver::FederationResourceResolver;
pub use resolver::FederationResourceState;
pub use resolver::ShardedResourceResolver;
pub use sync::FEDERATION_ALPN;
pub use sync::FederationProtocolContext;
pub use sync::FederationProtocolHandler;
pub use sync::FederationRequest;
pub use sync::FederationResponse;
pub use trust::TrustLevel;
pub use trust::TrustManager;
pub use trust::TrustRequest;
pub use types::FederatedId;
pub use types::FederationMode;
pub use types::FederationSettings;
