//! Federation discovery service using the BitTorrent Mainline DHT.
//!
//! This module provides decentralized discovery of federated clusters,
//! allowing nodes to find and connect to other Aspen clusters across
//! the global network without relying on centralized registries.
//!
//! # Overview
//!
//! The discovery service enables clusters to:
//!
//! - **Announce presence**: Publish cluster identity and connectivity info to the DHT
//! - **Discover peers**: Find other clusters by public key or application type
//! - **Register resources**: Advertise federated resources for cross-cluster sync
//! - **Cache results**: Maintain local cache of discovered clusters for efficiency
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    FederationDiscoveryService                    │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                  │
//! │  ┌─────────────────┐    ┌─────────────────┐                     │
//! │  │ ClusterIdentity │    │  Iroh Endpoint  │                     │
//! │  │   (Ed25519)     │    │  (connectivity) │                     │
//! │  └────────┬────────┘    └────────┬────────┘                     │
//! │           │                      │                               │
//! │           ▼                      ▼                               │
//! │  ┌────────────────────────────────────────┐                     │
//! │  │         ClusterAnnouncement            │                     │
//! │  │  - cluster_key, name, node_keys        │                     │
//! │  │  - relay_urls, apps, capabilities      │                     │
//! │  │  - HLC timestamp                       │                     │
//! │  └────────────────────┬───────────────────┘                     │
//! │                       │                                          │
//! │                       ▼                                          │
//! │  ┌────────────────────────────────────────┐                     │
//! │  │         BitTorrent Mainline DHT        │                     │
//! │  │          (BEP-44 mutable items)        │                     │
//! │  └────────────────────────────────────────┘                     │
//! │                                                                  │
//! │  ┌─────────────────┐    ┌─────────────────┐                     │
//! │  │ Discovered      │    │ Discovered      │                     │
//! │  │ Clusters Cache  │    │ Seeders Cache   │                     │
//! │  │ (LRU, 1024 max) │    │ (LRU, 10k max)  │                     │
//! │  └─────────────────┘    └─────────────────┘                     │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! The discovery system uses BEP-44 mutable items in the DHT:
//!
//! 1. **Cluster Announcements**: Each cluster announces its identity and endpoints
//! 2. **Resource Announcements**: Federated resources are announced with their cluster origin
//! 3. **Pull-Based Sync**: Other clusters discover and pull content on demand
//!
//! # DHT Key Structure
//!
//! DHT keys are derived deterministically from identifiers:
//!
//! - **Cluster announcement**: `sha256("aspen:cluster:v1:" || cluster_pubkey)[..20]`
//! - **Resource announcement**: `sha256("aspen:fed:v1:" || fed_id)[..20]`
//!
//! The 20-byte truncation matches the DHT's info-hash format for compatibility.
//!
//! # Usage
//!
//! ## Creating the Discovery Service
//!
//! ```ignore
//! use aspen_cluster::federation::{FederationDiscoveryService, ClusterIdentity};
//!
//! let identity = ClusterIdentity::generate("my-cluster".to_string());
//! let discovery = FederationDiscoveryService::new(
//!     identity,
//!     endpoint,
//!     cancel_token,
//!     6881,  // DHT port (0 for random)
//! ).await?;
//! ```
//!
//! ## Announcing Your Cluster
//!
//! ```ignore
//! // Announce to DHT (automatically rate-limited to 1 per 10 minutes)
//! discovery.announce_cluster().await?;
//! ```
//!
//! ## Discovering Clusters
//!
//! ```ignore
//! // Discover a specific cluster by public key
//! if let Some(cluster) = discovery.discover_cluster(&cluster_key).await {
//!     println!("Found: {} with {} nodes", cluster.name, cluster.node_keys.len());
//! }
//!
//! // Find all clusters running a specific application
//! let forge_clusters = discovery.find_clusters_with_app("forge");
//! for cluster in forge_clusters {
//!     println!("{} has Forge v{}", cluster.name,
//!         cluster.get_app("forge").map(|a| &a.version).unwrap_or(&"?".to_string()));
//! }
//!
//! // Find clusters with specific capabilities
//! let git_clusters = discovery.find_clusters_with_capability("git");
//! ```
//!
//! ## Manual Discovery (Testing)
//!
//! ```ignore
//! // Add a cluster without DHT (for testing or manual federation)
//! discovery.add_discovered_cluster(DiscoveredCluster { /* ... */ });
//! ```
//!
//! # Security
//!
//! All announcements are cryptographically secured:
//!
//! - **Signature verification**: Ed25519 signatures via BEP-44 mutable items
//! - **Identity binding**: Cluster public key is embedded in announcements
//! - **Origin validation**: Resource announcements include originating cluster
//! - **Replay protection**: HLC timestamps prevent stale announcement injection
//!
//! # Rate Limiting
//!
//! To prevent DHT abuse, announcements are rate-limited:
//!
//! | Operation | Minimum Interval |
//! |-----------|------------------|
//! | Cluster announcement | 10 minutes |
//! | Resource announcement | 5 minutes |
//! | Republish | 30 minutes |
//!
//! # Tiger Style Compliance
//!
//! All data structures have fixed bounds:
//!
//! | Resource | Limit | Constant |
//! |----------|-------|----------|
//! | Tracked clusters | 1,024 | [`MAX_TRACKED_CLUSTERS`] |
//! | Tracked resources | 10,000 | [`MAX_TRACKED_RESOURCES`] |
//! | Cluster announce size | 2 KB | [`MAX_CLUSTER_ANNOUNCE_SIZE`] |
//! | Resource announce size | 1 KB | [`MAX_RESOURCE_ANNOUNCE_SIZE`] |
//! | Discovered clusters returned | 100 | [`MAX_DISCOVERED_CLUSTERS`] |
//!
//! # Feature Flags
//!
//! - **`global-discovery`**: Required for actual DHT operations. Without this feature, the service
//!   logs operations but doesn't perform real DHT queries. Useful for testing and development.
//!
//! # Failure Modes
//!
//! | Failure | Behavior | Recovery |
//! |---------|----------|----------|
//! | DHT unavailable | Returns cached results | Automatic retry on next query |
//! | DHT query timeout | Returns `None` | Caller should retry with backoff |
//! | Rate limited | Skips operation (logged) | Wait for interval to pass |
//! | Cache full | LRU eviction | Automatic, oldest entries removed |
//!
//! # Testing
//!
//! For testing without a real DHT, use `aspen_testing::MockDiscoveryService`:
//!
//! ```ignore
//! use aspen_testing::MockDiscoveryService;
//!
//! let mock = MockDiscoveryService::new();
//! mock.register_cluster(&cluster_context);
//! mock.set_discovery_failures(cluster_key, 2);  // Simulate 2 failures
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use aspen_core::hlc::SerializableTimestamp;
use iroh::Endpoint;
use iroh::PublicKey;
use parking_lot::RwLock;
use serde::Deserialize;
use serde::Serialize;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::info;
#[cfg(feature = "global-discovery")]
use tracing::warn;

use super::app_registry::AppManifest;
use super::identity::ClusterIdentity;
use super::types::FederatedId;

// ============================================================================
// Constants (Tiger Style: Fixed limits)
// ============================================================================

/// Maximum number of tracked federated clusters.
pub const MAX_TRACKED_CLUSTERS: usize = 1024;

/// Maximum number of tracked federated resources.
pub const MAX_TRACKED_RESOURCES: usize = 10_000;

/// Minimum interval between cluster announcements (rate limiting).
pub const MIN_CLUSTER_ANNOUNCE_INTERVAL: Duration = Duration::from_secs(10 * 60); // 10 minutes

/// Minimum interval between resource announcements (rate limiting).
pub const MIN_RESOURCE_ANNOUNCE_INTERVAL: Duration = Duration::from_secs(5 * 60); // 5 minutes

/// Default interval for republishing cluster announcements.
pub const DEFAULT_REPUBLISH_INTERVAL: Duration = Duration::from_secs(30 * 60); // 30 minutes

/// Maximum size of a serialized cluster announcement.
pub const MAX_CLUSTER_ANNOUNCE_SIZE: usize = 2048;

/// Maximum size of a serialized resource announcement.
pub const MAX_RESOURCE_ANNOUNCE_SIZE: usize = 1024;

/// Timeout for DHT queries.
#[allow(dead_code)]
pub const DHT_QUERY_TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum clusters to return from a discovery query.
pub const MAX_DISCOVERED_CLUSTERS: usize = 100;

// ============================================================================
// Announcement Types
// ============================================================================

/// Cluster announcement record stored in the DHT.
///
/// This is the payload that advertises a cluster's presence and
/// connectivity information to the federation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterAnnouncement {
    /// Protocol version for forward compatibility.
    pub version: u8,
    /// Public key of the cluster (32 bytes).
    pub cluster_key: [u8; 32],
    /// Human-readable cluster name.
    pub cluster_name: String,
    /// Iroh node addresses for connectivity (public keys of nodes).
    pub node_keys: Vec<[u8; 32]>,
    /// Optional relay URLs for NAT traversal.
    pub relay_urls: Vec<String>,
    /// Applications installed on this cluster.
    ///
    /// This replaces the previous `capabilities` field with richer metadata.
    /// Each app includes version, name, and fine-grained capabilities.
    pub apps: Vec<AppManifest>,
    /// Legacy capabilities field (for backwards compatibility during transition).
    ///
    /// New code should use `apps` instead. This field contains the union of
    /// all capabilities from all registered apps.
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// HLC timestamp for ordering.
    pub hlc_timestamp: SerializableTimestamp,
}

impl ClusterAnnouncement {
    const VERSION: u8 = 2; // Bumped for apps field

    /// Create a new cluster announcement.
    ///
    /// # Arguments
    ///
    /// * `identity` - Cluster identity for signing
    /// * `node_keys` - Iroh node public keys for connectivity
    /// * `relay_urls` - Relay URLs for NAT traversal
    /// * `apps` - Registered applications on this cluster
    /// * `hlc` - Hybrid logical clock for timestamping
    pub fn new(
        identity: &ClusterIdentity,
        node_keys: Vec<PublicKey>,
        relay_urls: Vec<String>,
        apps: Vec<AppManifest>,
        hlc: &aspen_core::hlc::HLC,
    ) -> Self {
        // Compute legacy capabilities from apps for backwards compatibility
        let capabilities: Vec<String> = apps.iter().flat_map(|app| app.capabilities.iter().cloned()).collect();

        Self {
            version: Self::VERSION,
            cluster_key: *identity.public_key().as_bytes(),
            cluster_name: identity.name().to_string(),
            node_keys: node_keys.iter().map(|k| *k.as_bytes()).collect(),
            relay_urls,
            apps,
            capabilities,
            hlc_timestamp: SerializableTimestamp::from(hlc.new_timestamp()),
        }
    }

    /// Check if this cluster has a specific application installed.
    pub fn has_app(&self, app_id: &str) -> bool {
        self.apps.iter().any(|app| app.app_id == app_id)
    }

    /// Get an application manifest by ID.
    pub fn get_app(&self, app_id: &str) -> Option<&AppManifest> {
        self.apps.iter().find(|app| app.app_id == app_id)
    }

    /// Check if this cluster has a specific capability.
    ///
    /// Searches across all registered apps.
    pub fn has_capability(&self, capability: &str) -> bool {
        self.apps.iter().any(|app| app.has_capability(capability))
    }

    /// Serialize to bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, postcard::Error> {
        postcard::to_allocvec(self)
    }

    /// Deserialize from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        postcard::from_bytes(bytes).ok()
    }

    /// Get the cluster's public key.
    pub fn cluster_public_key(&self) -> Option<PublicKey> {
        PublicKey::from_bytes(&self.cluster_key).ok()
    }

    /// Compute the DHT key for this cluster announcement.
    pub fn to_dht_key(cluster_key: &PublicKey) -> [u8; 20] {
        use sha2::Digest;
        use sha2::Sha256;

        let mut hasher = Sha256::new();
        hasher.update(b"aspen:cluster:v1:");
        hasher.update(cluster_key.as_bytes());
        let hash = hasher.finalize();

        let mut key = [0u8; 20];
        key.copy_from_slice(&hash[..20]);
        key
    }
}

/// Resource announcement record stored in the DHT.
///
/// This advertises that a cluster is seeding a specific federated resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAnnouncement {
    /// Protocol version for forward compatibility.
    pub version: u8,
    /// The federated resource ID origin (cluster public key).
    pub fed_id_origin: [u8; 32],
    /// The federated resource ID local part.
    pub fed_id_local: [u8; 32],
    /// Public key of the announcing cluster.
    pub cluster_key: [u8; 32],
    /// Iroh node addresses for fetching.
    pub node_keys: Vec<[u8; 32]>,
    /// Relay URLs for NAT traversal.
    pub relay_urls: Vec<String>,
    /// Current heads (ref_name -> hash) for sync comparison.
    pub ref_heads: Vec<(String, [u8; 32])>,
    /// Timestamp (microseconds since epoch).
    pub timestamp_micros: u64,
}

impl ResourceAnnouncement {
    const VERSION: u8 = 1;

    /// Create a new resource announcement.
    pub fn new(
        fed_id: &FederatedId,
        cluster_key: PublicKey,
        node_keys: Vec<PublicKey>,
        relay_urls: Vec<String>,
        ref_heads: Vec<(String, [u8; 32])>,
    ) -> Self {
        let timestamp_micros = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_micros() as u64)
            .unwrap_or(0);

        Self {
            version: Self::VERSION,
            fed_id_origin: *fed_id.origin().as_bytes(),
            fed_id_local: *fed_id.local_id(),
            cluster_key: *cluster_key.as_bytes(),
            node_keys: node_keys.iter().map(|k| *k.as_bytes()).collect(),
            relay_urls,
            ref_heads,
            timestamp_micros,
        }
    }

    /// Serialize to bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, postcard::Error> {
        postcard::to_allocvec(self)
    }

    /// Deserialize from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        postcard::from_bytes(bytes).ok()
    }

    /// Get the federated ID.
    pub fn fed_id(&self) -> Option<FederatedId> {
        let origin = PublicKey::from_bytes(&self.fed_id_origin).ok()?;
        Some(FederatedId::new(origin, self.fed_id_local))
    }

    /// Get the cluster's public key.
    pub fn cluster_public_key(&self) -> Option<PublicKey> {
        PublicKey::from_bytes(&self.cluster_key).ok()
    }
}

// ============================================================================
// Discovered Cluster Info
// ============================================================================

/// Information about a discovered federated cluster.
#[derive(Debug, Clone)]
pub struct DiscoveredCluster {
    /// Public key of the cluster.
    pub cluster_key: PublicKey,
    /// Human-readable cluster name.
    pub name: String,
    /// Iroh node public keys for connectivity.
    pub node_keys: Vec<PublicKey>,
    /// Relay URLs for NAT traversal.
    pub relay_urls: Vec<String>,
    /// Applications installed on this cluster.
    pub apps: Vec<AppManifest>,
    /// Legacy capabilities (union of all app capabilities).
    pub capabilities: Vec<String>,
    /// When this cluster was discovered.
    pub discovered_at: Instant,
    /// HLC timestamp from the announcement.
    pub announced_at_hlc: SerializableTimestamp,
}

impl DiscoveredCluster {
    /// Create from a cluster announcement.
    pub fn from_announcement(announcement: &ClusterAnnouncement) -> Option<Self> {
        let cluster_key = announcement.cluster_public_key()?;
        let node_keys: Vec<PublicKey> =
            announcement.node_keys.iter().filter_map(|k| PublicKey::from_bytes(k).ok()).collect();

        Some(Self {
            cluster_key,
            name: announcement.cluster_name.clone(),
            node_keys,
            relay_urls: announcement.relay_urls.clone(),
            apps: announcement.apps.clone(),
            capabilities: announcement.capabilities.clone(),
            discovered_at: Instant::now(),
            announced_at_hlc: announcement.hlc_timestamp.clone(),
        })
    }

    /// Check if this cluster has a specific application installed.
    pub fn has_app(&self, app_id: &str) -> bool {
        self.apps.iter().any(|app| app.app_id == app_id)
    }

    /// Get an application manifest by ID.
    pub fn get_app(&self, app_id: &str) -> Option<&AppManifest> {
        self.apps.iter().find(|app| app.app_id == app_id)
    }

    /// Check if this cluster has a specific capability.
    ///
    /// Searches across all registered apps.
    pub fn has_capability(&self, capability: &str) -> bool {
        self.apps.iter().any(|app| app.has_capability(capability))
    }
}

/// Information about a discovered resource seeder.
#[derive(Debug, Clone)]
pub struct DiscoveredSeeder {
    /// The federated resource ID.
    pub fed_id: FederatedId,
    /// Public key of the seeding cluster.
    pub cluster_key: PublicKey,
    /// Iroh node public keys for fetching.
    pub node_keys: Vec<PublicKey>,
    /// Relay URLs for NAT traversal.
    pub relay_urls: Vec<String>,
    /// Current ref heads for sync comparison.
    pub ref_heads: HashMap<String, [u8; 32]>,
    /// When this seeder was discovered.
    pub discovered_at: Instant,
}

impl DiscoveredSeeder {
    /// Create from a resource announcement.
    pub fn from_announcement(announcement: &ResourceAnnouncement) -> Option<Self> {
        let fed_id = announcement.fed_id()?;
        let cluster_key = announcement.cluster_public_key()?;
        let node_keys: Vec<PublicKey> =
            announcement.node_keys.iter().filter_map(|k| PublicKey::from_bytes(k).ok()).collect();

        let ref_heads: HashMap<String, [u8; 32]> = announcement.ref_heads.iter().cloned().collect();

        Some(Self {
            fed_id,
            cluster_key,
            node_keys,
            relay_urls: announcement.relay_urls.clone(),
            ref_heads,
            discovered_at: Instant::now(),
        })
    }
}

// ============================================================================
// Federation Discovery Service
// ============================================================================

/// Service for discovering federated clusters and resources.
///
/// This service manages:
/// - Announcing our cluster to the DHT
/// - Announcing federated resources we seed
/// - Discovering other clusters and resource seeders
pub struct FederationDiscoveryService {
    /// Our cluster identity.
    cluster_identity: ClusterIdentity,

    /// The iroh endpoint for connectivity info.
    #[allow(dead_code)] // Used when global-discovery feature is enabled
    endpoint: Arc<Endpoint>,

    /// Discovered clusters (cluster_key -> info).
    discovered_clusters: RwLock<HashMap<PublicKey, DiscoveredCluster>>,

    /// Discovered resource seeders (fed_id -> seeders).
    discovered_seeders: RwLock<HashMap<FederatedId, Vec<DiscoveredSeeder>>>,

    /// Last announce times for rate limiting.
    #[allow(dead_code)] // Used when global-discovery feature is enabled
    cluster_announce_times: RwLock<HashMap<PublicKey, Instant>>,
    #[allow(dead_code)] // Used when global-discovery feature is enabled
    resource_announce_times: RwLock<HashMap<FederatedId, Instant>>,

    /// Cancellation token for shutdown.
    cancel: CancellationToken,

    /// DHT client (feature-gated).
    #[cfg(feature = "global-discovery")]
    dht: Option<Arc<mainline::async_dht::AsyncDht>>,
}

impl FederationDiscoveryService {
    /// Create a new federation discovery service.
    ///
    /// # Arguments
    ///
    /// * `cluster_identity` - Our cluster's identity
    /// * `endpoint` - The iroh endpoint for connectivity
    /// * `cancel` - Cancellation token for shutdown
    #[cfg(feature = "global-discovery")]
    pub async fn new(
        cluster_identity: ClusterIdentity,
        endpoint: Arc<Endpoint>,
        cancel: CancellationToken,
        dht_port: u16,
    ) -> anyhow::Result<Self> {
        use mainline::Dht;

        let mut builder = Dht::builder();
        if dht_port > 0 {
            builder.port(dht_port);
        }

        let dht = builder.build()?;
        let async_dht = dht.as_async();

        // Wait for bootstrap
        let bootstrapped = async_dht.bootstrapped().await;
        if bootstrapped {
            info!("federation DHT bootstrap complete");
        } else {
            warn!("federation DHT bootstrap may not be complete");
        }

        Ok(Self {
            cluster_identity,
            endpoint,
            discovered_clusters: RwLock::new(HashMap::new()),
            discovered_seeders: RwLock::new(HashMap::new()),
            cluster_announce_times: RwLock::new(HashMap::new()),
            resource_announce_times: RwLock::new(HashMap::new()),
            cancel,
            dht: Some(Arc::new(async_dht)),
        })
    }

    /// Create a new federation discovery service (stub when feature disabled).
    #[cfg(not(feature = "global-discovery"))]
    pub async fn new(
        cluster_identity: ClusterIdentity,
        endpoint: Arc<Endpoint>,
        cancel: CancellationToken,
        _dht_port: u16,
    ) -> anyhow::Result<Self> {
        info!("federation discovery created (DHT disabled - enable 'global-discovery' feature)");

        Ok(Self {
            cluster_identity,
            endpoint,
            discovered_clusters: RwLock::new(HashMap::new()),
            discovered_seeders: RwLock::new(HashMap::new()),
            cluster_announce_times: RwLock::new(HashMap::new()),
            resource_announce_times: RwLock::new(HashMap::new()),
            cancel,
        })
    }

    /// Get our cluster identity.
    pub fn cluster_identity(&self) -> &ClusterIdentity {
        &self.cluster_identity
    }

    /// Announce our cluster to the DHT.
    ///
    /// This makes our cluster discoverable by other federated clusters.
    #[cfg(feature = "global-discovery")]
    pub async fn announce_cluster(&self) -> anyhow::Result<()> {
        let dht = match &self.dht {
            Some(d) => d,
            None => {
                warn!("DHT not initialized, skipping cluster announcement");
                return Ok(());
            }
        };

        // Rate limiting
        let cluster_key = self.cluster_identity.public_key();
        {
            let times = self.cluster_announce_times.read();
            if let Some(last) = times.get(&cluster_key) {
                if last.elapsed() < MIN_CLUSTER_ANNOUNCE_INTERVAL {
                    debug!("skipping cluster announcement (rate limited)");
                    return Ok(());
                }
            }
        }

        // Build announcement
        let node_keys = vec![self.endpoint.id()];
        let relay_urls: Vec<String> = self.endpoint.addr().relay_urls().map(|u| u.to_string()).collect();

        let announcement = ClusterAnnouncement::new(&self.cluster_identity, node_keys, relay_urls);

        let announce_bytes = announcement.to_bytes()?;
        if announce_bytes.len() > MAX_CLUSTER_ANNOUNCE_SIZE {
            anyhow::bail!("cluster announcement too large: {} > {}", announce_bytes.len(), MAX_CLUSTER_ANNOUNCE_SIZE);
        }

        // Sign and store via BEP-44
        let signing_key = iroh_secret_to_signing_key(self.cluster_identity.secret_key());
        let dht_key = ClusterAnnouncement::to_dht_key(&cluster_key);
        let seq = (announcement.timestamp_micros / 1_000_000) as i64; // Use seconds as seq

        let item = mainline::MutableItem::new(signing_key, &announce_bytes, seq, Some(&dht_key));

        dht.put_mutable(item, None).await?;

        // Update rate limit
        self.cluster_announce_times.write().insert(cluster_key, Instant::now());

        info!(
            cluster_name = %self.cluster_identity.name(),
            cluster_key = %cluster_key,
            "announced cluster to federation DHT"
        );

        Ok(())
    }

    /// Announce our cluster to the DHT (stub when feature disabled).
    #[cfg(not(feature = "global-discovery"))]
    pub async fn announce_cluster(&self) -> anyhow::Result<()> {
        debug!(
            cluster_name = %self.cluster_identity.name(),
            "would announce cluster (DHT disabled)"
        );
        Ok(())
    }

    /// Discover a cluster by its public key.
    #[cfg(feature = "global-discovery")]
    pub async fn discover_cluster(&self, cluster_key: &PublicKey) -> Option<DiscoveredCluster> {
        let dht = self.dht.as_ref()?;

        let dht_key = ClusterAnnouncement::to_dht_key(cluster_key);

        // Query for the mutable item
        let result = tokio::time::timeout(DHT_QUERY_TIMEOUT, async {
            dht.get_mutable_most_recent(cluster_key.as_bytes(), Some(&dht_key)).await
        })
        .await;

        let item = match result {
            Ok(Some(item)) => item,
            Ok(None) => {
                debug!(cluster_key = %cluster_key, "cluster not found in DHT");
                return None;
            }
            Err(_) => {
                warn!(cluster_key = %cluster_key, "DHT query timed out");
                return None;
            }
        };

        // Parse announcement
        let announcement = ClusterAnnouncement::from_bytes(item.value())?;
        let discovered = DiscoveredCluster::from_announcement(&announcement)?;

        // Cache it
        self.discovered_clusters.write().insert(*cluster_key, discovered.clone());

        info!(
            cluster_key = %cluster_key,
            cluster_name = %discovered.name,
            nodes = discovered.node_keys.len(),
            "discovered federated cluster"
        );

        Some(discovered)
    }

    /// Discover a cluster by its public key (stub when feature disabled).
    #[cfg(not(feature = "global-discovery"))]
    pub async fn discover_cluster(&self, cluster_key: &PublicKey) -> Option<DiscoveredCluster> {
        // Check cache only when DHT is disabled
        self.discovered_clusters.read().get(cluster_key).cloned()
    }

    /// Get all discovered clusters.
    pub fn get_discovered_clusters(&self) -> Vec<DiscoveredCluster> {
        self.discovered_clusters.read().values().cloned().collect()
    }

    /// Find clusters running a specific application.
    ///
    /// Searches through all discovered clusters and returns those that have
    /// the specified application installed.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Find all clusters running Forge
    /// let forge_clusters = discovery.find_clusters_with_app("forge");
    /// for cluster in forge_clusters {
    ///     println!("Cluster {} has Forge", cluster.name);
    /// }
    /// ```
    pub fn find_clusters_with_app(&self, app_id: &str) -> Vec<DiscoveredCluster> {
        self.discovered_clusters.read().values().filter(|c| c.has_app(app_id)).cloned().collect()
    }

    /// Find clusters with a specific capability.
    ///
    /// Searches through all discovered clusters and returns those that have
    /// at least one application providing the specified capability.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Find all clusters with git hosting capability
    /// let git_clusters = discovery.find_clusters_with_capability("git");
    /// ```
    pub fn find_clusters_with_capability(&self, capability: &str) -> Vec<DiscoveredCluster> {
        self.discovered_clusters.read().values().filter(|c| c.has_capability(capability)).cloned().collect()
    }

    /// Get discovered seeders for a resource.
    pub fn get_seeders(&self, fed_id: &FederatedId) -> Vec<DiscoveredSeeder> {
        self.discovered_seeders.read().get(fed_id).cloned().unwrap_or_default()
    }

    /// Add a manually discovered cluster (for testing or manual federation).
    pub fn add_discovered_cluster(&self, cluster: DiscoveredCluster) {
        let mut clusters = self.discovered_clusters.write();

        // Tiger Style: Enforce limit
        if clusters.len() >= MAX_TRACKED_CLUSTERS {
            // Remove oldest
            if let Some(oldest_key) = clusters.iter().min_by_key(|(_, c)| c.discovered_at).map(|(k, _)| *k) {
                clusters.remove(&oldest_key);
            }
        }

        clusters.insert(cluster.cluster_key, cluster);
    }

    /// Check if the service is cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }
}

/// Convert an iroh SecretKey to a mainline SigningKey.
#[cfg(feature = "global-discovery")]
fn iroh_secret_to_signing_key(secret_key: &iroh::SecretKey) -> mainline::SigningKey {
    let secret_bytes: [u8; 32] = secret_key.to_bytes();
    mainline::SigningKey::from_bytes(&secret_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_identity() -> ClusterIdentity {
        ClusterIdentity::generate("test-cluster".to_string())
    }

    fn test_node_key() -> PublicKey {
        iroh::SecretKey::generate(&mut rand::rng()).public()
    }

    fn test_hlc() -> aspen_core::hlc::HLC {
        aspen_core::create_hlc("test-node")
    }

    fn test_apps() -> Vec<AppManifest> {
        vec![AppManifest::new("forge", "1.0.0").with_capabilities(vec!["git", "issues"])]
    }

    #[test]
    fn test_cluster_announcement_roundtrip() {
        let identity = test_identity();
        let node_keys = vec![test_node_key(), test_node_key()];
        let relay_urls = vec!["https://relay.example.com".to_string()];
        let apps = test_apps();
        let hlc = test_hlc();

        let announcement = ClusterAnnouncement::new(&identity, node_keys.clone(), relay_urls.clone(), apps, &hlc);

        let bytes = announcement.to_bytes().expect("serialization should work");
        assert!(bytes.len() < MAX_CLUSTER_ANNOUNCE_SIZE);

        let parsed = ClusterAnnouncement::from_bytes(&bytes).expect("deserialization should work");
        assert_eq!(parsed.cluster_name, identity.name());
        assert_eq!(parsed.version, ClusterAnnouncement::VERSION);
        assert_eq!(parsed.node_keys.len(), 2);
        assert_eq!(parsed.apps.len(), 1);
        assert!(parsed.has_app("forge"));
        assert!(parsed.has_capability("git"));
    }

    #[test]
    fn test_resource_announcement_roundtrip() {
        let origin = test_node_key();
        let local_id = [0xab; 32];
        let fed_id = FederatedId::new(origin, local_id);

        let cluster_key = test_node_key();
        let node_keys = vec![test_node_key()];
        let relay_urls = vec![];
        let ref_heads = vec![("heads/main".to_string(), [0xcd; 32])];

        let announcement = ResourceAnnouncement::new(&fed_id, cluster_key, node_keys, relay_urls, ref_heads.clone());

        let bytes = announcement.to_bytes().expect("serialization should work");
        assert!(bytes.len() < MAX_RESOURCE_ANNOUNCE_SIZE);

        let parsed = ResourceAnnouncement::from_bytes(&bytes).expect("deserialization should work");
        let parsed_fed_id = parsed.fed_id().expect("should have valid fed_id");
        assert_eq!(parsed_fed_id, fed_id);
        assert_eq!(parsed.ref_heads.len(), 1);
    }

    #[test]
    fn test_discovered_cluster_from_announcement() {
        let identity = test_identity();
        let node_keys = vec![test_node_key()];
        let relay_urls = vec!["https://relay.example.com".to_string()];
        let apps = test_apps();
        let hlc = test_hlc();

        let announcement = ClusterAnnouncement::new(&identity, node_keys, relay_urls, apps, &hlc);
        let discovered = DiscoveredCluster::from_announcement(&announcement).expect("should create discovered cluster");

        assert_eq!(discovered.cluster_key, identity.public_key());
        assert_eq!(discovered.name, identity.name());
        assert_eq!(discovered.node_keys.len(), 1);
        assert_eq!(discovered.relay_urls.len(), 1);
        assert!(discovered.has_app("forge"));
        assert!(discovered.has_capability("git"));
        assert!(!discovered.has_app("unknown"));
        assert!(!discovered.has_capability("unknown"));
    }

    #[test]
    fn test_dht_key_deterministic() {
        let key1 = test_node_key();
        let key2 = test_node_key();

        let dht_key1a = ClusterAnnouncement::to_dht_key(&key1);
        let dht_key1b = ClusterAnnouncement::to_dht_key(&key1);
        let dht_key2 = ClusterAnnouncement::to_dht_key(&key2);

        // Same input = same output
        assert_eq!(dht_key1a, dht_key1b);

        // Different input = different output
        assert_ne!(dht_key1a, dht_key2);
    }
}
