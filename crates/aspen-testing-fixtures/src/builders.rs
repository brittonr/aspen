//! Builder patterns for test configurations.
//!
//! This module provides fluent builders for creating test setups
//! with sensible defaults that can be customized as needed.

use std::sync::Arc;

use aspen_testing_core::DeterministicClusterController;
use aspen_testing_core::DeterministicKeyValueStore;

/// Builder for configuring test cluster setups.
///
/// Provides a fluent API for creating cluster configurations
/// with customizable node counts, timeouts, and other parameters.
///
/// # Example
///
/// ```ignore
/// let builder = ClusterBuilder::new()
///     .with_nodes(3)
///     .with_timeout_ms(5000)
///     .with_heartbeat_interval_ms(100);
///
/// let (cluster_controller, initial_state) = builder.build();
/// ```
#[derive(Debug, Clone)]
pub struct ClusterBuilder {
    /// Number of nodes in the cluster.
    node_count: usize,
    /// Operation timeout in milliseconds.
    timeout_ms: u64,
    /// Heartbeat interval in milliseconds.
    heartbeat_interval_ms: u64,
    /// Election timeout in milliseconds.
    election_timeout_ms: u64,
    /// Whether to auto-initialize the cluster.
    auto_init: bool,
}

impl Default for ClusterBuilder {
    fn default() -> Self {
        Self {
            node_count: 3,
            timeout_ms: 5000,
            heartbeat_interval_ms: 100,
            election_timeout_ms: 1000,
            auto_init: true,
        }
    }
}

impl ClusterBuilder {
    /// Create a new cluster builder with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the number of nodes in the cluster.
    pub fn with_nodes(mut self, count: usize) -> Self {
        self.node_count = count;
        self
    }

    /// Set the operation timeout in milliseconds.
    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Set the heartbeat interval in milliseconds.
    pub fn with_heartbeat_interval_ms(mut self, interval_ms: u64) -> Self {
        self.heartbeat_interval_ms = interval_ms;
        self
    }

    /// Set the election timeout in milliseconds.
    pub fn with_election_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.election_timeout_ms = timeout_ms;
        self
    }

    /// Set whether to auto-initialize the cluster.
    pub fn with_auto_init(mut self, auto_init: bool) -> Self {
        self.auto_init = auto_init;
        self
    }

    /// Build the cluster configuration.
    ///
    /// Returns the cluster controller and initial state if auto_init is enabled.
    pub fn build(self) -> ClusterConfig {
        ClusterConfig {
            node_count: self.node_count,
            timeout_ms: self.timeout_ms,
            heartbeat_interval_ms: self.heartbeat_interval_ms,
            election_timeout_ms: self.election_timeout_ms,
            controller: DeterministicClusterController::new(),
        }
    }
}

/// Configuration for a test cluster.
#[derive(Clone)]
pub struct ClusterConfig {
    /// Number of nodes in the cluster.
    pub node_count: usize,
    /// Operation timeout in milliseconds.
    pub timeout_ms: u64,
    /// Heartbeat interval in milliseconds.
    pub heartbeat_interval_ms: u64,
    /// Election timeout in milliseconds.
    pub election_timeout_ms: u64,
    /// The cluster controller.
    pub controller: Arc<DeterministicClusterController>,
}

impl ClusterConfig {
    /// Get the timeout as a Duration.
    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.timeout_ms)
    }

    /// Get the heartbeat interval as a Duration.
    pub fn heartbeat_interval(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.heartbeat_interval_ms)
    }

    /// Get the election timeout as a Duration.
    pub fn election_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.election_timeout_ms)
    }
}

/// Builder for creating configured KV stores for testing.
///
/// Provides a fluent API for creating KV store instances
/// with optional prefixes and pre-populated data.
///
/// # Example
///
/// ```ignore
/// let kv = KvStoreBuilder::new()
///     .with_prefix("test/")
///     .with_entry("key1", "value1")
///     .with_entry("key2", "value2")
///     .build()
///     .await;
/// ```
#[derive(Debug, Clone, Default)]
pub struct KvStoreBuilder {
    /// Key prefix to apply to all operations.
    prefix: Option<String>,
    /// Initial entries to populate.
    initial_entries: Vec<(String, String)>,
}

impl KvStoreBuilder {
    /// Create a new KV store builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a prefix for all keys.
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Add an initial entry to populate.
    pub fn with_entry(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.initial_entries.push((key.into(), value.into()));
        self
    }

    /// Add multiple initial entries.
    pub fn with_entries(mut self, entries: impl IntoIterator<Item = (String, String)>) -> Self {
        self.initial_entries.extend(entries);
        self
    }

    /// Build the KV store asynchronously.
    ///
    /// This populates the store with any initial entries that were configured.
    pub async fn build(self) -> Arc<DeterministicKeyValueStore> {
        use aspen_kv_types::WriteRequest;
        use aspen_traits::KeyValueStore;

        let store = DeterministicKeyValueStore::new();

        for (key, value) in self.initial_entries {
            let full_key = match &self.prefix {
                Some(prefix) => format!("{}{}", prefix, key),
                None => key,
            };
            let _ = store.write(WriteRequest::set(full_key, value)).await;
        }

        store
    }

    /// Build the KV store synchronously (no initial population).
    ///
    /// Use this when you don't need initial entries or will populate later.
    pub fn build_empty(self) -> Arc<DeterministicKeyValueStore> {
        DeterministicKeyValueStore::new()
    }

    /// Get the configured prefix.
    pub fn prefix(&self) -> Option<&str> {
        self.prefix.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_builder_defaults() {
        let builder = ClusterBuilder::new();
        assert_eq!(builder.node_count, 3);
        assert_eq!(builder.timeout_ms, 5000);
        assert_eq!(builder.heartbeat_interval_ms, 100);
        assert_eq!(builder.election_timeout_ms, 1000);
        assert!(builder.auto_init);
    }

    #[test]
    fn test_cluster_builder_customization() {
        let config = ClusterBuilder::new()
            .with_nodes(5)
            .with_timeout_ms(10000)
            .with_heartbeat_interval_ms(50)
            .with_election_timeout_ms(500)
            .with_auto_init(false)
            .build();

        assert_eq!(config.node_count, 5);
        assert_eq!(config.timeout_ms, 10000);
        assert_eq!(config.heartbeat_interval_ms, 50);
        assert_eq!(config.election_timeout_ms, 500);
    }

    #[test]
    fn test_cluster_config_durations() {
        let config = ClusterBuilder::new()
            .with_timeout_ms(5000)
            .with_heartbeat_interval_ms(100)
            .with_election_timeout_ms(1000)
            .build();

        assert_eq!(config.timeout(), std::time::Duration::from_millis(5000));
        assert_eq!(config.heartbeat_interval(), std::time::Duration::from_millis(100));
        assert_eq!(config.election_timeout(), std::time::Duration::from_millis(1000));
    }

    #[test]
    fn test_kv_store_builder_defaults() {
        let builder = KvStoreBuilder::new();
        assert!(builder.prefix().is_none());
    }

    #[test]
    fn test_kv_store_builder_with_prefix() {
        let builder = KvStoreBuilder::new().with_prefix("test/");
        assert_eq!(builder.prefix(), Some("test/"));
    }

    #[tokio::test]
    async fn test_kv_store_builder_with_entries() {
        use aspen_kv_types::ReadRequest;
        use aspen_traits::KeyValueStore;

        let store = KvStoreBuilder::new().with_entry("key1", "value1").with_entry("key2", "value2").build().await;

        let result = store.read(ReadRequest::new("key1")).await.unwrap();
        assert_eq!(result.kv.unwrap().value, "value1");

        let result = store.read(ReadRequest::new("key2")).await.unwrap();
        assert_eq!(result.kv.unwrap().value, "value2");
    }

    #[tokio::test]
    async fn test_kv_store_builder_with_prefix_and_entries() {
        use aspen_kv_types::ReadRequest;
        use aspen_traits::KeyValueStore;

        let store = KvStoreBuilder::new().with_prefix("test/").with_entry("key1", "value1").build().await;

        // Key should be prefixed
        let result = store.read(ReadRequest::new("test/key1")).await.unwrap();
        assert_eq!(result.kv.unwrap().value, "value1");

        // Unprefixed key should not exist
        let result = store.read(ReadRequest::new("key1")).await;
        assert!(result.is_err());
    }
}
