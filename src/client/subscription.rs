//! Cluster subscription types for client overlay.
//!
//! Defines the subscription model where clients can connect to multiple
//! clusters with priority ordering for reads (first match wins).
//!
//! # Subscription Filtering
//!
//! Subscriptions can optionally filter which keys are synced:
//! - `FullReplication`: Sync all keys (default)
//! - `PrefixFilter`: Only sync keys matching specified prefixes
//! - `PrefixExclude`: Sync all keys EXCEPT those matching specified prefixes

use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::constants::DEFAULT_CACHE_TTL;

/// Access level for a cluster subscription.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AccessLevel {
    /// Read-only access - can query data but not write.
    #[default]
    ReadOnly,
    /// Read-write access - can both query and modify data.
    ReadWrite,
}

impl AccessLevel {
    /// Returns true if this access level allows writes.
    pub fn can_write(&self) -> bool {
        matches!(self, AccessLevel::ReadWrite)
    }
}

/// Configuration for local caching of subscription data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Whether caching is enabled for this subscription.
    pub enabled: bool,
    /// TTL for cached entries.
    pub ttl: Duration,
    /// Maximum number of entries to cache for this subscription.
    pub max_entries: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ttl: DEFAULT_CACHE_TTL,
            max_entries: 10_000,
        }
    }
}

/// Filter for subscription data synchronization.
///
/// Controls which keys are synced from a peer cluster.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum SubscriptionFilter {
    /// Replicate all keys from this cluster (default).
    #[default]
    FullReplication,
    /// Only replicate keys matching these prefixes.
    PrefixFilter(Vec<String>),
    /// Replicate all keys EXCEPT those matching these prefixes.
    PrefixExclude(Vec<String>),
}

impl SubscriptionFilter {
    /// Check if a key should be included based on this filter.
    pub fn should_include(&self, key: &str) -> bool {
        match self {
            SubscriptionFilter::FullReplication => true,
            SubscriptionFilter::PrefixFilter(prefixes) => {
                prefixes.iter().any(|prefix| key.starts_with(prefix))
            }
            SubscriptionFilter::PrefixExclude(prefixes) => {
                !prefixes.iter().any(|prefix| key.starts_with(prefix))
            }
        }
    }

    /// Create a prefix filter from a list of prefixes.
    pub fn include_prefixes(prefixes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::PrefixFilter(prefixes.into_iter().map(Into::into).collect())
    }

    /// Create a prefix exclusion filter from a list of prefixes.
    pub fn exclude_prefixes(prefixes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::PrefixExclude(prefixes.into_iter().map(Into::into).collect())
    }
}

/// A subscription to an Aspen cluster.
///
/// Clients can have multiple subscriptions with priority ordering.
/// Reads are served from subscriptions in priority order (first match wins).
/// Writes go to the first read-write subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterSubscription {
    /// Unique identifier for this subscription.
    pub id: String,
    /// Human-readable name for the cluster.
    pub name: String,
    /// Priority for read ordering (0 = highest priority).
    /// Lower numbers are checked first when looking up keys.
    pub priority: u32,
    /// Cluster ID (typically hash of cluster cookie).
    pub cluster_id: String,
    /// Access level (read-only or read-write).
    pub access: AccessLevel,
    /// Cache configuration for this subscription.
    pub cache_config: CacheConfig,
    /// Filter for which keys to sync from this cluster.
    pub filter: SubscriptionFilter,
    /// Whether this subscription is currently enabled.
    /// Disabled subscriptions are skipped during sync and lookups.
    pub enabled: bool,
}

impl ClusterSubscription {
    /// Create a new subscription with default settings.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        cluster_id: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            priority: 0,
            cluster_id: cluster_id.into(),
            access: AccessLevel::ReadOnly,
            cache_config: CacheConfig::default(),
            filter: SubscriptionFilter::default(),
            enabled: true,
        }
    }

    /// Set the priority for this subscription.
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// Set the access level for this subscription.
    pub fn with_access(mut self, access: AccessLevel) -> Self {
        self.access = access;
        self
    }

    /// Set the cache configuration for this subscription.
    pub fn with_cache(mut self, config: CacheConfig) -> Self {
        self.cache_config = config;
        self
    }

    /// Set the filter for this subscription.
    pub fn with_filter(mut self, filter: SubscriptionFilter) -> Self {
        self.filter = filter;
        self
    }

    /// Set whether this subscription is enabled.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Check if a key should be synced based on this subscription's filter.
    pub fn should_sync_key(&self, key: &str) -> bool {
        self.enabled && self.filter.should_include(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_access_level() {
        assert!(!AccessLevel::ReadOnly.can_write());
        assert!(AccessLevel::ReadWrite.can_write());
    }

    #[test]
    fn test_subscription_builder() {
        let sub = ClusterSubscription::new("sub-1", "Production", "cluster-abc")
            .with_priority(1)
            .with_access(AccessLevel::ReadWrite);

        assert_eq!(sub.id, "sub-1");
        assert_eq!(sub.name, "Production");
        assert_eq!(sub.priority, 1);
        assert!(sub.access.can_write());
        assert!(sub.enabled);
        assert_eq!(sub.filter, SubscriptionFilter::FullReplication);
    }

    #[test]
    fn test_subscription_filter_full() {
        let filter = SubscriptionFilter::FullReplication;
        assert!(filter.should_include("any/key"));
        assert!(filter.should_include("__internal"));
        assert!(filter.should_include(""));
    }

    #[test]
    fn test_subscription_filter_prefix_include() {
        let filter = SubscriptionFilter::include_prefixes(["config/", "users/"]);
        assert!(filter.should_include("config/db"));
        assert!(filter.should_include("users/123"));
        assert!(!filter.should_include("logs/app"));
        assert!(!filter.should_include("other"));
    }

    #[test]
    fn test_subscription_filter_prefix_exclude() {
        let filter = SubscriptionFilter::exclude_prefixes(["__internal/", "temp/"]);
        assert!(filter.should_include("config/db"));
        assert!(filter.should_include("users/123"));
        assert!(!filter.should_include("__internal/cache"));
        assert!(!filter.should_include("temp/file"));
    }

    #[test]
    fn test_subscription_should_sync_key() {
        let sub = ClusterSubscription::new("sub-1", "Test", "cluster-a")
            .with_filter(SubscriptionFilter::include_prefixes(["data/"]));

        assert!(sub.should_sync_key("data/file"));
        assert!(!sub.should_sync_key("other/file"));

        // Disabled subscription should not sync anything
        let disabled = sub.with_enabled(false);
        assert!(!disabled.should_sync_key("data/file"));
    }
}
