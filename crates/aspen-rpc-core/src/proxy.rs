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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_proxy_config_is_disabled() {
        let config = ProxyConfig::default();
        assert!(!config.enabled, "proxy must be opt-in");
    }

    #[test]
    fn default_proxy_config_matches_constants() {
        use aspen_constants::proxy::*;
        let config = ProxyConfig::default();
        assert_eq!(config.max_hops, MAX_PROXY_HOPS);
        assert_eq!(config.timeout, std::time::Duration::from_secs(PROXY_TIMEOUT_SECS));
        assert_eq!(config.max_targets, MAX_PROXY_TARGETS);
    }

    #[test]
    fn proxy_config_json_roundtrip() {
        let config = ProxyConfig {
            enabled: true,
            max_hops: 5,
            timeout: std::time::Duration::from_secs(30),
            max_targets: 10,
        };
        let json = serde_json::to_string(&config).expect("serialize");
        let deserialized: ProxyConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.enabled, config.enabled);
        assert_eq!(deserialized.max_hops, config.max_hops);
        assert_eq!(deserialized.timeout, config.timeout);
        assert_eq!(deserialized.max_targets, config.max_targets);
    }
}
