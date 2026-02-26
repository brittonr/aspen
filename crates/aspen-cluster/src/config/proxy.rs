//! HTTP proxy configuration for TCP/HTTP tunneling over iroh QUIC.
//!
//! Enables proxying TCP traffic through Aspen's P2P network. The upstream
//! proxy runs on the node and accepts authenticated connections from peers.
//! Downstream proxies run on clients and tunnel local TCP to a remote node.

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// HTTP proxy configuration.
///
/// When enabled, the node registers an upstream proxy protocol handler that
/// accepts TCP/HTTP tunneling requests from authenticated peers over iroh QUIC.
///
/// # Example TOML
///
/// ```toml
/// [proxy]
/// enabled = true
/// max_connections = 128
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProxyConfig {
    /// Enable the HTTP proxy protocol handler on this node.
    ///
    /// When enabled, the node accepts proxy requests from authenticated
    /// peers and forwards them to local or remote TCP services.
    ///
    /// Default: false
    #[serde(default, rename = "enabled")]
    pub is_enabled: bool,

    /// Maximum concurrent proxy connections.
    ///
    /// Tiger Style: Explicit upper bound prevents resource exhaustion.
    ///
    /// Default: 128
    #[serde(default = "default_proxy_max_connections")]
    pub max_connections: u32,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            is_enabled: false,
            max_connections: default_proxy_max_connections(),
        }
    }
}

/// Default maximum concurrent proxy connections.
///
/// Tiger Style: Fixed limit — 128 is generous for most deployments.
pub(crate) fn default_proxy_max_connections() -> u32 {
    128
}
