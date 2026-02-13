//! Peer cluster synchronization configuration.
//!
//! Controls cluster-to-cluster data synchronization via iroh-docs.

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// Peer cluster synchronization configuration.
///
/// Controls cluster-to-cluster data synchronization via iroh-docs.
/// When enabled, this cluster can subscribe to other Aspen clusters
/// and receive their KV data with priority-based conflict resolution.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PeerSyncConfig {
    /// Enable peer cluster synchronization.
    ///
    /// When enabled, this node can subscribe to other Aspen clusters
    /// and import their KV data via iroh-docs CRDT sync.
    ///
    /// Default: false (peer sync disabled).
    #[serde(default)]
    pub enabled: bool,

    /// Default priority for imported peer data.
    ///
    /// Lower values have higher priority. Local cluster data has priority 0.
    /// When conflicts occur, the entry with lower priority wins.
    ///
    /// Default: 100 (lower priority than local data).
    #[serde(default = "default_peer_sync_priority")]
    pub default_priority: u32,

    /// Maximum number of peer cluster subscriptions.
    ///
    /// Tiger Style: Bounded to prevent resource exhaustion.
    ///
    /// Default: 32.
    #[serde(default = "default_max_peer_subscriptions")]
    pub max_subscriptions: u32,

    /// Reconnection interval in seconds when peer connection is lost.
    ///
    /// Default: 30 seconds.
    #[serde(default = "default_peer_reconnect_interval_secs")]
    pub reconnect_interval_secs: u64,

    /// Maximum reconnection attempts before giving up.
    ///
    /// Set to 0 for unlimited retries.
    ///
    /// Default: 0 (unlimited retries).
    #[serde(default)]
    pub max_reconnect_attempts: u32,
}

impl Default for PeerSyncConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_priority: default_peer_sync_priority(),
            max_subscriptions: default_max_peer_subscriptions(),
            reconnect_interval_secs: default_peer_reconnect_interval_secs(),
            max_reconnect_attempts: 0,
        }
    }
}

pub(crate) fn default_peer_sync_priority() -> u32 {
    100
}

pub(crate) fn default_max_peer_subscriptions() -> u32 {
    32
}

pub(crate) fn default_peer_reconnect_interval_secs() -> u64 {
    30
}
