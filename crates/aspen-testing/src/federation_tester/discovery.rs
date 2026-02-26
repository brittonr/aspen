//! Mock discovery service for federation testing.

use std::collections::HashMap;

use aspen_cluster::federation::DiscoveredCluster;
use aspen_cluster::federation::FederatedId;
use aspen_core::hlc::SerializableTimestamp;
use iroh::PublicKey;
use parking_lot::RwLock;
use tracing::debug;

use super::MAX_DISCOVERY_FAILURES;
use super::cluster_context::ClusterContext;

/// Mock discovery service for federation testing.
///
/// Replaces real DHT operations with an in-memory registry.
#[derive(Debug, Default)]
pub struct MockDiscoveryService {
    /// Registered clusters (cluster_key -> context).
    clusters: RwLock<HashMap<PublicKey, DiscoveredCluster>>,
    /// Registered federated resources (fed_id -> cluster_keys).
    resources: RwLock<HashMap<FederatedId, Vec<PublicKey>>>,
    /// Simulated discovery failures (cluster_key -> fail_count).
    failures: RwLock<HashMap<PublicKey, usize>>,
}

impl MockDiscoveryService {
    /// Create a new mock discovery service.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a cluster for discovery.
    pub fn register_cluster(&self, context: &ClusterContext) {
        let discovered = DiscoveredCluster {
            cluster_key: context.public_key(),
            name: context.name.clone(),
            node_keys: vec![context.public_key()], // Use cluster key as node key in tests
            relay_urls: vec![],
            apps: context.app_registry.list_apps(),
            capabilities: context.app_registry.all_capabilities(),
            discovered_at: std::time::Instant::now(),
            announced_at_hlc: SerializableTimestamp::from_millis(0),
        };

        self.clusters.write().insert(context.public_key(), discovered);
        debug!(cluster = %context.name, "registered cluster in mock discovery");
    }

    /// Discover a cluster by public key.
    pub fn discover_cluster(&self, cluster_key: &PublicKey) -> Option<DiscoveredCluster> {
        // Check for simulated failures
        let mut failures = self.failures.write();
        if let Some(count) = failures.get_mut(cluster_key)
            && *count > 0
        {
            *count -= 1;
            debug!(cluster = %cluster_key, remaining = *count, "simulated discovery failure");
            return None;
        }

        self.clusters.read().get(cluster_key).cloned()
    }

    /// Find clusters running a specific application.
    pub fn find_clusters_with_app(&self, app_id: &str) -> Vec<DiscoveredCluster> {
        self.clusters.read().values().filter(|c: &&DiscoveredCluster| c.has_app(app_id)).cloned().collect()
    }

    /// Register a federated resource with a seeding cluster.
    pub fn register_resource(&self, fed_id: FederatedId, cluster_key: PublicKey) {
        let mut resources = self.resources.write();
        resources.entry(fed_id).or_default().push(cluster_key);
    }

    /// Get seeders for a federated resource.
    pub fn get_seeders(&self, fed_id: &FederatedId) -> Vec<PublicKey> {
        self.resources.read().get(fed_id).cloned().unwrap_or_default()
    }

    /// Simulate discovery failures for a cluster.
    pub fn set_discovery_failures(&self, cluster_key: PublicKey, count: usize) {
        let count = count.min(MAX_DISCOVERY_FAILURES);
        self.failures.write().insert(cluster_key, count);
    }
}
