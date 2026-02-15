//! Federation configuration for cross-cluster communication.
//!
//! Enables independent Aspen clusters to discover each other, share content,
//! and synchronize resources across organizational boundaries.

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// Federation configuration for cross-cluster communication.
///
/// Enables independent Aspen clusters to discover each other, share content,
/// and synchronize resources across organizational boundaries.
///
/// # Architecture
///
/// - **Cluster Identity**: Each cluster has a stable Ed25519 keypair
/// - **DHT Discovery**: Clusters discover each other via BitTorrent Mainline DHT
/// - **Pull-Based Sync**: Cross-cluster sync is eventual consistent
///
/// # Example TOML
///
/// ```toml
/// [federation]
/// enabled = true
/// cluster_name = "my-organization"
/// # cluster_key is auto-generated if not specified
///
/// # Optional: explicit trusted clusters
/// trusted_clusters = [
///     "abc123def456...",  # public key of trusted cluster
/// ]
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FederationConfig {
    /// Enable federation support.
    ///
    /// When enabled, this cluster can participate in cross-cluster
    /// discovery and synchronization.
    ///
    /// Default: false (federation disabled).
    #[serde(default, rename = "enabled")]
    pub is_enabled: bool,

    /// Human-readable cluster name.
    ///
    /// Used in announcements and for identification in logs.
    /// Tiger Style: Max 128 characters.
    ///
    /// Default: hostname or "aspen-cluster"
    #[serde(default = "default_federation_cluster_name")]
    pub cluster_name: String,

    /// Hex-encoded cluster secret key (64 hex chars = 32 bytes).
    ///
    /// If not provided, a new key is generated on first start and
    /// stored in data_dir/federation/cluster_key.
    ///
    /// The cluster key should be the same across all nodes in the cluster.
    pub cluster_key: Option<String>,

    /// Path to file containing the cluster secret key.
    ///
    /// Alternative to inline cluster_key. Key is read from this file.
    /// File should contain 64 hex characters (32 bytes).
    pub cluster_key_path: Option<std::path::PathBuf>,

    /// Enable DHT-based cluster discovery.
    ///
    /// When enabled, the cluster announces itself to the BitTorrent
    /// Mainline DHT and discovers other federated clusters.
    ///
    /// Default: true (when federation is enabled)
    #[serde(default = "default_federation_dht_discovery")]
    pub enable_dht_discovery: bool,

    /// Enable gossip-based federation announcements.
    ///
    /// When enabled, the cluster participates in a global gossip topic
    /// for faster discovery of nearby federated clusters.
    ///
    /// Default: true (when federation is enabled)
    #[serde(default = "default_federation_gossip")]
    pub enable_gossip: bool,

    /// List of trusted cluster public keys.
    ///
    /// For resources with AllowList federation mode, only clusters
    /// in this list can sync. For resources with Public mode, this
    /// list is informational only.
    ///
    /// Tiger Style: Max 256 entries.
    #[serde(default)]
    pub trusted_clusters: Vec<String>,

    /// Interval for DHT announcements in seconds.
    ///
    /// How often to re-announce the cluster to the DHT to maintain
    /// discoverability.
    ///
    /// Default: 1800 seconds (30 minutes)
    #[serde(default = "default_federation_announce_interval_secs")]
    pub announce_interval_secs: u64,

    /// Maximum number of federated peers to track.
    ///
    /// Tiger Style: Bounded to prevent resource exhaustion.
    ///
    /// Default: 256
    #[serde(default = "default_federation_max_peers")]
    pub max_peers: u32,
}

impl Default for FederationConfig {
    fn default() -> Self {
        Self {
            is_enabled: false,
            cluster_name: default_federation_cluster_name(),
            cluster_key: None,
            cluster_key_path: None,
            enable_dht_discovery: default_federation_dht_discovery(),
            enable_gossip: default_federation_gossip(),
            trusted_clusters: Vec::new(),
            announce_interval_secs: default_federation_announce_interval_secs(),
            max_peers: default_federation_max_peers(),
        }
    }
}

pub(crate) fn default_federation_cluster_name() -> String {
    "aspen-cluster".to_string()
}

pub(crate) fn default_federation_dht_discovery() -> bool {
    true
}

pub(crate) fn default_federation_gossip() -> bool {
    true
}

pub(crate) fn default_federation_announce_interval_secs() -> u64 {
    1800 // 30 minutes
}

pub(crate) fn default_federation_max_peers() -> u32 {
    256
}
