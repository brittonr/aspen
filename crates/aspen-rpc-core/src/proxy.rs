//! Proxy configuration for cross-cluster request forwarding.

use serde::Deserialize;
use serde::Serialize;

/// Configuration for cross-cluster request proxying.
///
/// When a request requires an app that isn't loaded locally, the proxy
/// can forward it to a discovered cluster that has the capability.
///
/// # Tiger Style
///
/// - Disabled by default (opt-in)
/// - Bounded hops, targets, and timeout
/// - Explicit configuration required
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Whether proxying is enabled.
    pub enabled: bool,

    /// Maximum proxy hops before rejecting (loop prevention).
    pub max_hops: u8,

    /// Timeout for each proxy attempt in seconds.
    pub timeout: std::time::Duration,

    /// Maximum number of target clusters to try.
    pub max_targets: usize,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        use aspen_constants::proxy::*;

        Self {
            enabled: false,
            max_hops: MAX_PROXY_HOPS,
            timeout: std::time::Duration::from_secs(PROXY_TIMEOUT_SECS),
            max_targets: MAX_PROXY_TARGETS,
        }
    }
}
