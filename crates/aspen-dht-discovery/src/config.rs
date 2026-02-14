//! Configuration for the content discovery service.

use serde::Deserialize;
use serde::Serialize;

/// Configuration for the content discovery service.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct ContentDiscoveryConfig {
    /// Enable global DHT-based content discovery.
    /// Default: false (opt-in for privacy)
    pub enabled: bool,

    /// Run DHT node in server mode (accept incoming requests).
    /// Default: false (client-only mode)
    pub server_mode: bool,

    /// Bootstrap nodes for initial DHT connection.
    /// Default: uses mainline's built-in bootstrap nodes
    pub bootstrap_nodes: Vec<String>,

    /// Port for DHT UDP socket.
    /// Default: 0 (random port)
    pub dht_port: u16,

    /// Automatically announce all local blobs to DHT.
    /// Default: false (manual announcement only)
    pub auto_announce: bool,

    /// Maximum concurrent DHT queries.
    /// Default: 8
    pub max_concurrent_queries: usize,
}

impl Default for ContentDiscoveryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            server_mode: false,
            bootstrap_nodes: Vec::new(),
            dht_port: 0,
            auto_announce: false,
            max_concurrent_queries: 8,
        }
    }
}
