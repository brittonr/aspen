//! iroh-blobs configuration for content-addressed storage.

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// iroh-blobs configuration for content-addressed storage.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BlobConfig {
    /// Enable iroh-blobs integration for content-addressed storage.
    ///
    /// When enabled, the node can store and retrieve blobs (large binary data)
    /// using content-addressed storage with automatic deduplication.
    ///
    /// Default: true (blobs enabled).
    #[serde(default = "default_blobs_enabled")]
    pub enabled: bool,

    /// Enable automatic offloading of large KV values to blobs.
    ///
    /// When enabled, KV values larger than offload_threshold_bytes are
    /// automatically stored in the blob store, with a reference saved
    /// in the KV store. The original value is reconstructed on read.
    ///
    /// Default: true (auto-offload enabled).
    #[serde(default = "default_auto_offload")]
    pub auto_offload: bool,

    /// Threshold in bytes for automatic blob offloading.
    ///
    /// KV values larger than this size are stored as blobs when auto_offload is enabled.
    ///
    /// Default: 1048576 (1 MB).
    #[serde(default = "default_offload_threshold_bytes")]
    pub offload_threshold_bytes: u32,

    /// Interval for blob garbage collection in seconds.
    ///
    /// Untagged blobs are collected after the GC grace period expires.
    ///
    /// Default: 60 seconds.
    #[serde(default = "default_gc_interval_secs")]
    pub gc_interval_secs: u64,

    /// Grace period for blob garbage collection in seconds.
    ///
    /// Blobs must remain untagged for this duration before being collected.
    /// This prevents race conditions during concurrent tag updates.
    ///
    /// Default: 300 seconds (5 minutes).
    #[serde(default = "default_gc_grace_period_secs")]
    pub gc_grace_period_secs: u64,

    // === Replication Configuration ===
    /// Default replication factor for new blobs.
    ///
    /// Specifies how many copies of each blob should be maintained across the cluster.
    /// A replication factor of 3 means the blob will be stored on 3 different nodes.
    ///
    /// Tiger Style: Bounded to MAX_REPLICATION_FACTOR (7).
    ///
    /// Default: 1 (no replication, single copy).
    #[serde(default = "default_replication_factor")]
    pub replication_factor: u32,

    /// Minimum replicas required before a write is considered durable.
    ///
    /// When `enable_quorum_writes` is true, blob writes block until at least
    /// this many replicas have confirmed storage. Must be <= replication_factor.
    ///
    /// For strong durability: min_replicas = (replication_factor / 2) + 1
    ///
    /// Default: 1.
    #[serde(default = "default_min_replicas")]
    pub min_replicas: u32,

    /// Maximum replication factor allowed.
    ///
    /// Tiger Style: Upper bound to prevent excessive replication.
    ///
    /// Default: 5.
    #[serde(default = "default_max_replicas")]
    pub max_replicas: u32,

    /// How often the repair service checks for under-replicated blobs (seconds).
    ///
    /// The repair service scans blob metadata periodically and triggers
    /// replication for blobs with fewer than min_replicas copies.
    ///
    /// Default: 60 seconds.
    #[serde(default = "default_repair_interval_secs")]
    pub repair_interval_secs: u64,

    /// Grace period before repairing under-replicated blobs (seconds).
    ///
    /// When a node goes offline, wait this long before triggering repair.
    /// This avoids unnecessary replication for transient failures.
    ///
    /// Default: 300 seconds (5 minutes).
    #[serde(default = "default_repair_delay_secs")]
    pub repair_delay_secs: u64,

    /// Enable quorum writes (synchronous replication).
    ///
    /// When enabled, blob writes block until `min_replicas` nodes have
    /// confirmed storage. This provides stronger durability guarantees
    /// at the cost of higher write latency.
    ///
    /// When disabled (default), replication happens asynchronously in the
    /// background. The write returns immediately after local storage.
    ///
    /// Default: false (async replication).
    #[serde(default)]
    pub enable_quorum_writes: bool,

    /// Node tag key for failure domain spreading.
    ///
    /// When set, the placement algorithm spreads replicas across different
    /// failure domains. For example, if set to "rack", replicas will be
    /// placed on nodes with different "rack" tags.
    ///
    /// Requires nodes to have the specified tag in their WorkerConfig.tags.
    ///
    /// Default: None (no failure domain awareness).
    #[serde(default)]
    pub failure_domain_key: Option<String>,

    /// Enable automatic replication of blobs.
    ///
    /// When enabled, the replication manager automatically replicates
    /// newly added blobs to achieve the configured replication factor.
    ///
    /// Default: false (manual replication only).
    #[serde(default)]
    pub enable_auto_replication: bool,
}

impl Default for BlobConfig {
    fn default() -> Self {
        Self {
            enabled: default_blobs_enabled(),
            auto_offload: default_auto_offload(),
            offload_threshold_bytes: default_offload_threshold_bytes(),
            gc_interval_secs: default_gc_interval_secs(),
            gc_grace_period_secs: default_gc_grace_period_secs(),
            // Replication defaults
            replication_factor: default_replication_factor(),
            min_replicas: default_min_replicas(),
            max_replicas: default_max_replicas(),
            repair_interval_secs: default_repair_interval_secs(),
            repair_delay_secs: default_repair_delay_secs(),
            enable_quorum_writes: false,
            failure_domain_key: None,
            enable_auto_replication: false,
        }
    }
}

pub(crate) fn default_blobs_enabled() -> bool {
    true
}

pub(crate) fn default_auto_offload() -> bool {
    true
}

pub(crate) fn default_offload_threshold_bytes() -> u32 {
    1_048_576 // 1 MB
}

pub(crate) fn default_gc_interval_secs() -> u64 {
    60
}

pub(crate) fn default_gc_grace_period_secs() -> u64 {
    300 // 5 minutes
}

pub(crate) fn default_replication_factor() -> u32 {
    1 // No replication by default (single copy)
}

pub(crate) fn default_min_replicas() -> u32 {
    1 // At least one replica (the local one)
}

pub(crate) fn default_max_replicas() -> u32 {
    5 // Tiger Style: Upper bound on replication
}

pub(crate) fn default_repair_interval_secs() -> u64 {
    60 // Check every minute
}

pub(crate) fn default_repair_delay_secs() -> u64 {
    300 // 5 minute grace period before repair
}
