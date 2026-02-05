//! Federation testing infrastructure for multi-cluster madsim tests.
//!
//! This module provides a `FederationTester` that manages multiple simulated
//! Aspen clusters for testing federation scenarios. It follows the same patterns
//! as `AspenRaftTester` for single-cluster tests.
//!
//! # Architecture
//!
//! ```text
//! +-------------------+     +-------------------+
//! |   Cluster A       |     |   Cluster B       |
//! | (AspenRaftTester) |<--->| (AspenRaftTester) |
//! |   + AppRegistry   |     |   + AppRegistry   |
//! |   + TrustManager  |     |   + TrustManager  |
//! +-------------------+     +-------------------+
//!           \                     /
//!            \                   /
//!             v                 v
//!     +-----------------------------+
//!     |    FederationMessageRouter   |
//!     |  (Simulates cross-cluster    |
//!     |   QUIC connections)          |
//!     +-----------------------------+
//!                   |
//!                   v
//!     +-----------------------------+
//!     |     MockDiscoveryService     |
//!     |  (Simulates DHT discovery)   |
//!     +-----------------------------+
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use aspen_testing::FederationTester;
//!
//! #[madsim::test]
//! async fn test_two_cluster_federation() {
//!     let mut t = FederationTester::new("two_cluster_test").await;
//!
//!     // Create two clusters
//!     t.create_cluster("alice", 3).await;
//!     t.create_cluster("bob", 3).await;
//!
//!     // Establish trust
//!     t.add_trust("alice", "bob").await;
//!
//!     // Create a federated resource on alice
//!     let fed_id = t.create_federated_repo("alice", "my-repo").await;
//!
//!     // Bob discovers and syncs
//!     let discovered = t.discover_resource("bob", &fed_id).await;
//!     assert!(discovered.is_some());
//!
//!     t.end();
//! }
//! ```
//!
//! # Design Principles (Tiger Style)
//!
//! - **Bounded resources**: MAX_CLUSTERS_PER_TEST, MAX_FEDERATED_RESOURCES
//! - **Deterministic**: Madsim-based simulation for reproducibility
//! - **Explicit errors**: All operations return Result
//! - **Isolated clusters**: Each cluster has its own Raft consensus
//!
//! # Limitations
//!
//! - Real DHT operations are not supported (madsim can't do real network I/O)
//! - Uses MockDiscoveryService instead of actual BitTorrent Mainline DHT
//! - Cross-cluster communication is simulated via FederationMessageRouter

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use aspen_cluster::federation::AppManifest;
use aspen_cluster::federation::AppRegistry;
use aspen_cluster::federation::ClusterIdentity;
use aspen_cluster::federation::DiscoveredCluster;
use aspen_cluster::federation::FederatedId;
use aspen_cluster::federation::FederationMode;
use aspen_cluster::federation::FederationSettings;
use aspen_cluster::federation::TrustLevel;
use aspen_cluster::federation::TrustManager;
use aspen_core::hlc::HLC;
use aspen_core::hlc::SerializableTimestamp;
use aspen_core::simulation::SimulationArtifactBuilder;
use iroh::PublicKey;
use parking_lot::RwLock;
use serde::Deserialize;
use serde::Serialize;
use tracing::debug;
use tracing::info;
use tracing::warn;

// Tiger Style: Fixed limits
const MAX_CLUSTERS_PER_TEST: usize = 8;
const MAX_FEDERATED_RESOURCES: usize = 100;
const MAX_DISCOVERY_FAILURES: usize = 10;
const MAX_OBJECTS_PER_RESOURCE: usize = 1000;
const MAX_SYNC_BATCH_SIZE: usize = 100;

// ============================================================================
// Syncable Objects - Simulated data for federation sync testing
// ============================================================================

/// A syncable object with content-addressed storage and HLC timestamp.
///
/// Uses Last-Write-Wins (LWW) conflict resolution based on HLC timestamps.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncableObject {
    /// Content hash (Blake3) - serves as the object ID.
    pub hash: [u8; 32],
    /// Object data (limited to 1MB in real system, smaller for tests).
    pub data: Vec<u8>,
    /// HLC timestamp for LWW conflict resolution.
    pub timestamp: SerializableTimestamp,
    /// Origin cluster that created this object.
    pub origin_cluster: [u8; 32],
}

impl SyncableObject {
    /// Create a new syncable object.
    pub fn new(data: Vec<u8>, timestamp: SerializableTimestamp, origin_cluster: PublicKey) -> Self {
        let hash = *blake3::hash(&data).as_bytes();
        Self {
            hash,
            data,
            timestamp,
            origin_cluster: *origin_cluster.as_bytes(),
        }
    }

    /// Get the object's content hash as a hex string.
    pub fn hash_hex(&self) -> String {
        hex::encode(&self.hash[..8])
    }
}

/// Result of a sync operation.
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// Objects that were received.
    pub received: Vec<SyncableObject>,
    /// Objects that had conflicts (resolved by LWW).
    pub conflicts_resolved: usize,
    /// Whether the sync completed fully.
    pub complete: bool,
    /// Error message if sync failed.
    pub error: Option<String>,
}

impl SyncResult {
    /// Create a successful sync result.
    pub fn success(received: Vec<SyncableObject>, conflicts_resolved: usize) -> Self {
        Self {
            received,
            conflicts_resolved,
            complete: true,
            error: None,
        }
    }

    /// Create a failed sync result.
    pub fn failed(error: impl Into<String>) -> Self {
        Self {
            received: vec![],
            conflicts_resolved: 0,
            complete: false,
            error: Some(error.into()),
        }
    }

    /// Check if the sync was successful.
    pub fn is_success(&self) -> bool {
        self.error.is_none() && self.complete
    }
}

/// Per-resource data store within a cluster.
#[derive(Debug, Default)]
pub struct ResourceDataStore {
    /// Objects stored for this resource (hash -> object).
    objects: RwLock<HashMap<[u8; 32], SyncableObject>>,
    /// Ref heads (ref_name -> object_hash).
    refs: RwLock<HashMap<String, [u8; 32]>>,
}

impl ResourceDataStore {
    /// Create a new empty data store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Store an object, using LWW for conflicts.
    ///
    /// Returns true if the object was stored (new or newer timestamp).
    pub fn store_object(&self, obj: SyncableObject) -> bool {
        let mut objects = self.objects.write();

        // Check for existing object with same hash
        if let Some(existing) = objects.get(&obj.hash) {
            // LWW: keep the one with later timestamp
            if obj.timestamp <= existing.timestamp {
                debug!(
                    hash = %hex::encode(&obj.hash[..8]),
                    "object already exists with same or later timestamp"
                );
                return false;
            }
        }

        // Check capacity
        if objects.len() >= MAX_OBJECTS_PER_RESOURCE && !objects.contains_key(&obj.hash) {
            warn!(max = MAX_OBJECTS_PER_RESOURCE, "resource data store at capacity, rejecting object");
            return false;
        }

        objects.insert(obj.hash, obj);
        true
    }

    /// Get an object by hash.
    pub fn get_object(&self, hash: &[u8; 32]) -> Option<SyncableObject> {
        self.objects.read().get(hash).cloned()
    }

    /// Get all objects.
    pub fn all_objects(&self) -> Vec<SyncableObject> {
        self.objects.read().values().cloned().collect()
    }

    /// Get object count.
    pub fn object_count(&self) -> usize {
        self.objects.read().len()
    }

    /// Set a ref to point to an object hash.
    pub fn set_ref(&self, name: impl Into<String>, hash: [u8; 32]) {
        self.refs.write().insert(name.into(), hash);
    }

    /// Get a ref's target hash.
    pub fn get_ref(&self, name: &str) -> Option<[u8; 32]> {
        self.refs.read().get(name).copied()
    }

    /// Get all refs.
    pub fn all_refs(&self) -> HashMap<String, [u8; 32]> {
        self.refs.read().clone()
    }

    /// Get objects that the other side is missing.
    ///
    /// `their_hashes` is the set of hashes the remote side already has.
    pub fn objects_missing_from(&self, their_hashes: &HashSet<[u8; 32]>) -> Vec<SyncableObject> {
        self.objects
            .read()
            .values()
            .filter(|obj| !their_hashes.contains(&obj.hash))
            .take(MAX_SYNC_BATCH_SIZE)
            .cloned()
            .collect()
    }
}

/// Network partition state for simulating cluster isolation.
#[derive(Debug, Default)]
pub struct NetworkPartitions {
    /// Set of partitioned cluster pairs (a, b) where a < b lexicographically.
    partitions: RwLock<HashSet<(String, String)>>,
}

impl NetworkPartitions {
    /// Create a new partition tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Partition two clusters (bidirectional).
    pub fn partition(&self, cluster_a: &str, cluster_b: &str) {
        let pair = Self::ordered_pair(cluster_a, cluster_b);
        self.partitions.write().insert(pair);
        info!(a = %cluster_a, b = %cluster_b, "network partition created");
    }

    /// Heal a partition between two clusters.
    pub fn heal(&self, cluster_a: &str, cluster_b: &str) {
        let pair = Self::ordered_pair(cluster_a, cluster_b);
        self.partitions.write().remove(&pair);
        info!(a = %cluster_a, b = %cluster_b, "network partition healed");
    }

    /// Check if two clusters can communicate.
    pub fn can_communicate(&self, cluster_a: &str, cluster_b: &str) -> bool {
        let pair = Self::ordered_pair(cluster_a, cluster_b);
        !self.partitions.read().contains(&pair)
    }

    /// Heal all partitions.
    pub fn heal_all(&self) {
        let count = self.partitions.read().len();
        self.partitions.write().clear();
        info!(count, "healed all network partitions");
    }

    fn ordered_pair(a: &str, b: &str) -> (String, String) {
        if a < b {
            (a.to_string(), b.to_string())
        } else {
            (b.to_string(), a.to_string())
        }
    }
}

/// Configuration for creating a FederationTester.
#[derive(Debug, Clone)]
pub struct FederationTesterConfig {
    /// Test name for artifact identification.
    pub test_name: String,
    /// Explicit seed for determinism.
    pub seed: Option<u64>,
}

impl FederationTesterConfig {
    /// Create a new config with the given test name.
    pub fn new(test_name: impl Into<String>) -> Self {
        Self {
            test_name: test_name.into(),
            seed: None,
        }
    }

    /// Set an explicit seed for deterministic testing.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }
}

/// Context for a single simulated cluster.
pub struct ClusterContext {
    /// Cluster name (for identification in tests).
    pub name: String,
    /// Cluster identity (Ed25519 keypair).
    pub identity: ClusterIdentity,
    /// Trust manager for this cluster (not Debug).
    pub trust_manager: Arc<TrustManager>,
    /// App registry for this cluster.
    pub app_registry: Arc<AppRegistry>,
    /// Federation settings for resources (fed_id -> settings).
    pub resource_settings: RwLock<HashMap<FederatedId, FederationSettings>>,
    /// Data stores for federated resources (fed_id -> store).
    pub data_stores: RwLock<HashMap<FederatedId, Arc<ResourceDataStore>>>,
    /// HLC for timestamp generation.
    pub hlc: Arc<HLC>,
    /// When this cluster was created.
    pub created_at: Instant,
}

impl std::fmt::Debug for ClusterContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClusterContext")
            .field("name", &self.name)
            .field("public_key", &self.identity.public_key())
            .field("apps", &self.app_registry.list_apps().len())
            .field("resources", &self.data_stores.read().len())
            .finish()
    }
}

impl ClusterContext {
    /// Create a new cluster context.
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let identity = ClusterIdentity::generate(name.clone());
        let trust_manager = Arc::new(TrustManager::new());
        let app_registry = Arc::new(AppRegistry::new());
        let hlc = Arc::new(aspen_core::hlc::create_hlc(&name));

        // Register default Forge app
        app_registry.register(
            AppManifest::new("forge", "1.0.0")
                .with_name("Aspen Forge")
                .with_capabilities(vec!["git", "issues", "patches"]),
        );

        Self {
            name,
            identity,
            trust_manager,
            app_registry,
            resource_settings: RwLock::new(HashMap::new()),
            data_stores: RwLock::new(HashMap::new()),
            hlc,
            created_at: Instant::now(),
        }
    }

    /// Get the cluster's public key.
    pub fn public_key(&self) -> PublicKey {
        self.identity.public_key()
    }

    /// Get or create a data store for a resource.
    pub fn get_or_create_store(&self, fed_id: FederatedId) -> Arc<ResourceDataStore> {
        let mut stores = self.data_stores.write();
        stores.entry(fed_id).or_insert_with(|| Arc::new(ResourceDataStore::new())).clone()
    }

    /// Get a data store if it exists.
    pub fn get_store(&self, fed_id: &FederatedId) -> Option<Arc<ResourceDataStore>> {
        self.data_stores.read().get(fed_id).cloned()
    }

    /// Generate a new HLC timestamp.
    pub fn new_timestamp(&self) -> SerializableTimestamp {
        SerializableTimestamp::from(self.hlc.new_timestamp())
    }

    /// Create an object in this cluster's context.
    ///
    /// Note: `fed_id` is currently unused but reserved for future per-resource HLC tracking.
    pub fn create_object(&self, _fed_id: FederatedId, data: Vec<u8>) -> SyncableObject {
        let timestamp = self.new_timestamp();
        SyncableObject::new(data, timestamp, self.public_key())
    }
}

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
            discovered_at: Instant::now(),
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

/// Federation tester for multi-cluster simulation tests.
pub struct FederationTester {
    /// Test name.
    test_name: String,
    /// Cluster contexts (cluster_name -> context).
    clusters: HashMap<String, ClusterContext>,
    /// Mock discovery service.
    discovery: Arc<MockDiscoveryService>,
    /// Network partition simulation.
    partitions: Arc<NetworkPartitions>,
    /// Simulation artifact builder for future deterministic replay support (not Debug).
    #[allow(dead_code)]
    artifact: SimulationArtifactBuilder,
    /// Federated resources created during the test.
    resources: Vec<FederatedId>,
    /// Sync statistics.
    sync_stats: RwLock<SyncStatistics>,
}

/// Statistics about sync operations during a test.
#[derive(Debug, Default, Clone)]
pub struct SyncStatistics {
    /// Total sync operations attempted.
    pub sync_attempts: usize,
    /// Successful sync operations.
    pub sync_successes: usize,
    /// Failed sync operations (network partition, trust issues, etc.).
    pub sync_failures: usize,
    /// Total objects transferred.
    pub objects_transferred: usize,
    /// Total conflicts resolved via LWW.
    pub conflicts_resolved: usize,
}

impl std::fmt::Debug for FederationTester {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FederationTester")
            .field("test_name", &self.test_name)
            .field("clusters", &self.clusters.keys().collect::<Vec<_>>())
            .field("resource_count", &self.resources.len())
            .finish()
    }
}

impl FederationTester {
    /// Create a new federation tester.
    pub async fn new(test_name: impl Into<String>) -> Self {
        let test_name = test_name.into();
        let artifact = SimulationArtifactBuilder::new(&test_name, 0);

        info!(test = %test_name, "created federation tester");

        Self {
            test_name,
            clusters: HashMap::new(),
            discovery: Arc::new(MockDiscoveryService::new()),
            partitions: Arc::new(NetworkPartitions::new()),
            artifact,
            resources: Vec::new(),
            sync_stats: RwLock::new(SyncStatistics::default()),
        }
    }

    /// Create a new federation tester with config.
    pub async fn with_config(config: FederationTesterConfig) -> Self {
        let seed = config.seed.unwrap_or(0);
        let artifact = SimulationArtifactBuilder::new(&config.test_name, seed);

        info!(test = %config.test_name, seed = seed, "created federation tester with config");

        Self {
            test_name: config.test_name,
            clusters: HashMap::new(),
            discovery: Arc::new(MockDiscoveryService::new()),
            partitions: Arc::new(NetworkPartitions::new()),
            artifact,
            resources: Vec::new(),
            sync_stats: RwLock::new(SyncStatistics::default()),
        }
    }

    /// Create a new cluster.
    ///
    /// Returns an error if MAX_CLUSTERS_PER_TEST would be exceeded.
    pub async fn create_cluster(&mut self, name: impl Into<String>) -> Result<PublicKey> {
        let name = name.into();

        if self.clusters.len() >= MAX_CLUSTERS_PER_TEST {
            anyhow::bail!("cannot create cluster: max {} clusters per test", MAX_CLUSTERS_PER_TEST);
        }

        if self.clusters.contains_key(&name) {
            anyhow::bail!("cluster '{}' already exists", name);
        }

        let context = ClusterContext::new(&name);
        let public_key = context.public_key();

        // Register with mock discovery
        self.discovery.register_cluster(&context);

        self.clusters.insert(name.clone(), context);

        info!(cluster = %name, key = %public_key, "created cluster");

        Ok(public_key)
    }

    /// Get a cluster context by name.
    pub fn get_cluster(&self, name: &str) -> Option<&ClusterContext> {
        self.clusters.get(name)
    }

    /// Get a cluster's public key by name.
    pub fn cluster_key(&self, name: &str) -> Option<PublicKey> {
        self.clusters.get(name).map(|c| c.public_key())
    }

    /// Add a trust relationship from one cluster to another.
    pub async fn add_trust(&mut self, from: &str, to: &str) -> Result<()> {
        let to_key = self.cluster_key(to).with_context(|| format!("cluster '{}' not found", to))?;

        let from_cluster = self.clusters.get(from).with_context(|| format!("cluster '{}' not found", from))?;

        from_cluster.trust_manager.add_trusted(to_key, to.to_string(), None);

        info!(from = %from, to = %to, "added trust relationship");

        Ok(())
    }

    /// Add bidirectional trust between two clusters.
    pub async fn add_mutual_trust(&mut self, cluster_a: &str, cluster_b: &str) -> Result<()> {
        self.add_trust(cluster_a, cluster_b).await?;
        self.add_trust(cluster_b, cluster_a).await?;
        Ok(())
    }

    /// Create a federated resource on a cluster.
    pub async fn create_federated_resource(
        &mut self,
        cluster_name: &str,
        local_id: [u8; 32],
        mode: FederationMode,
    ) -> Result<FederatedId> {
        if self.resources.len() >= MAX_FEDERATED_RESOURCES {
            anyhow::bail!("cannot create resource: max {} resources per test", MAX_FEDERATED_RESOURCES);
        }

        let cluster =
            self.clusters.get(cluster_name).with_context(|| format!("cluster '{}' not found", cluster_name))?;

        let fed_id = FederatedId::new(cluster.public_key(), local_id);

        // Set federation settings
        let settings = match mode {
            FederationMode::Disabled => FederationSettings::disabled(),
            FederationMode::Public => FederationSettings::public(),
            FederationMode::AllowList => FederationSettings::allowlist(vec![]),
        };

        cluster.resource_settings.write().insert(fed_id, settings);

        // Register with discovery if public
        if mode == FederationMode::Public {
            self.discovery.register_resource(fed_id, cluster.public_key());
        }

        self.resources.push(fed_id);

        info!(
            cluster = %cluster_name,
            fed_id = %fed_id.short(),
            mode = %mode,
            "created federated resource"
        );

        Ok(fed_id)
    }

    /// Discover clusters running a specific application.
    pub fn find_clusters_with_app(&self, app_id: &str) -> Vec<DiscoveredCluster> {
        self.discovery.find_clusters_with_app(app_id)
    }

    /// Get seeders for a federated resource.
    pub fn get_seeders(&self, fed_id: &FederatedId) -> Vec<PublicKey> {
        self.discovery.get_seeders(fed_id)
    }

    /// Simulate discovery failures for a cluster.
    pub fn set_discovery_failures(&self, cluster_name: &str, count: usize) -> Result<()> {
        let key = self.cluster_key(cluster_name).with_context(|| format!("cluster '{}' not found", cluster_name))?;

        self.discovery.set_discovery_failures(key, count);
        Ok(())
    }

    /// Check if a cluster trusts another.
    pub fn is_trusted(&self, from: &str, to: &str) -> bool {
        let Some(to_key) = self.cluster_key(to) else {
            return false;
        };

        let Some(from_cluster) = self.clusters.get(from) else {
            return false;
        };

        from_cluster.trust_manager.trust_level(&to_key) == TrustLevel::Trusted
    }

    /// Get the number of clusters.
    pub fn cluster_count(&self) -> usize {
        self.clusters.len()
    }

    /// Get the number of federated resources.
    pub fn resource_count(&self) -> usize {
        self.resources.len()
    }

    // ========================================================================
    // Network Partition Simulation
    // ========================================================================

    /// Create a network partition between two clusters.
    ///
    /// Partitioned clusters cannot sync data until the partition is healed.
    pub fn partition(&self, cluster_a: &str, cluster_b: &str) {
        self.partitions.partition(cluster_a, cluster_b);
    }

    /// Heal a network partition between two clusters.
    pub fn heal_partition(&self, cluster_a: &str, cluster_b: &str) {
        self.partitions.heal(cluster_a, cluster_b);
    }

    /// Heal all network partitions.
    pub fn heal_all_partitions(&self) {
        self.partitions.heal_all();
    }

    /// Check if two clusters can communicate.
    pub fn can_communicate(&self, cluster_a: &str, cluster_b: &str) -> bool {
        self.partitions.can_communicate(cluster_a, cluster_b)
    }

    // ========================================================================
    // Data Storage and Sync Operations
    // ========================================================================

    /// Store an object on a cluster for a federated resource.
    ///
    /// Returns the object's hash.
    pub fn store_object(&self, cluster_name: &str, fed_id: FederatedId, data: Vec<u8>) -> Result<[u8; 32]> {
        let cluster =
            self.clusters.get(cluster_name).with_context(|| format!("cluster '{}' not found", cluster_name))?;

        let obj = cluster.create_object(fed_id, data);
        let hash = obj.hash;

        let store = cluster.get_or_create_store(fed_id);
        store.store_object(obj);

        debug!(
            cluster = %cluster_name,
            fed_id = %fed_id.short(),
            hash = %hex::encode(&hash[..8]),
            "stored object"
        );

        Ok(hash)
    }

    /// Store an object with a specific timestamp (for conflict testing).
    pub fn store_object_with_timestamp(
        &self,
        cluster_name: &str,
        fed_id: FederatedId,
        data: Vec<u8>,
        timestamp: SerializableTimestamp,
    ) -> Result<[u8; 32]> {
        let cluster =
            self.clusters.get(cluster_name).with_context(|| format!("cluster '{}' not found", cluster_name))?;

        let obj = SyncableObject::new(data, timestamp, cluster.public_key());
        let hash = obj.hash;

        let store = cluster.get_or_create_store(fed_id);
        store.store_object(obj);

        Ok(hash)
    }

    /// Get an object from a cluster.
    pub fn get_object(&self, cluster_name: &str, fed_id: &FederatedId, hash: &[u8; 32]) -> Option<SyncableObject> {
        let cluster = self.clusters.get(cluster_name)?;
        let store = cluster.get_store(fed_id)?;
        store.get_object(hash)
    }

    /// Get all objects for a resource on a cluster.
    pub fn get_all_objects(&self, cluster_name: &str, fed_id: &FederatedId) -> Vec<SyncableObject> {
        let Some(cluster) = self.clusters.get(cluster_name) else {
            return vec![];
        };
        let Some(store) = cluster.get_store(fed_id) else {
            return vec![];
        };
        store.all_objects()
    }

    /// Get the object count for a resource on a cluster.
    pub fn object_count(&self, cluster_name: &str, fed_id: &FederatedId) -> usize {
        let Some(cluster) = self.clusters.get(cluster_name) else {
            return 0;
        };
        let Some(store) = cluster.get_store(fed_id) else {
            return 0;
        };
        store.object_count()
    }

    /// Sync a federated resource from one cluster to another.
    ///
    /// This simulates the federation sync protocol:
    /// 1. Check network connectivity
    /// 2. Check trust relationship
    /// 3. Exchange object hashes
    /// 4. Transfer missing objects
    /// 5. Apply LWW conflict resolution
    pub fn sync_resource(&mut self, from_cluster: &str, to_cluster: &str, fed_id: FederatedId) -> SyncResult {
        // Update stats
        self.sync_stats.write().sync_attempts += 1;

        // Check network partition
        if !self.can_communicate(from_cluster, to_cluster) {
            self.sync_stats.write().sync_failures += 1;
            return SyncResult::failed(format!("network partition between {} and {}", from_cluster, to_cluster));
        }

        // Check trust
        if !self.is_trusted(to_cluster, from_cluster) {
            self.sync_stats.write().sync_failures += 1;
            return SyncResult::failed(format!("{} does not trust {}", to_cluster, from_cluster));
        }

        // Get clusters
        let Some(from) = self.clusters.get(from_cluster) else {
            self.sync_stats.write().sync_failures += 1;
            return SyncResult::failed(format!("cluster '{}' not found", from_cluster));
        };
        let Some(to) = self.clusters.get(to_cluster) else {
            self.sync_stats.write().sync_failures += 1;
            return SyncResult::failed(format!("cluster '{}' not found", to_cluster));
        };

        // Get stores
        let from_store = from.get_or_create_store(fed_id);
        let to_store = to.get_or_create_store(fed_id);

        // Get hashes the receiver already has
        let their_hashes: HashSet<[u8; 32]> = to_store.all_objects().iter().map(|obj| obj.hash).collect();

        // Get objects missing from receiver
        let missing = from_store.objects_missing_from(&their_hashes);

        // Transfer objects and track conflicts
        let mut received = Vec::new();
        let mut conflicts = 0;

        for obj in missing {
            // Check if this is a conflict (same hash but different timestamp)
            if let Some(existing) = to_store.get_object(&obj.hash)
                && existing.timestamp != obj.timestamp
            {
                conflicts += 1;
            }

            if to_store.store_object(obj.clone()) {
                received.push(obj);
            }
        }

        // Update stats
        {
            let mut stats = self.sync_stats.write();
            stats.sync_successes += 1;
            stats.objects_transferred += received.len();
            stats.conflicts_resolved += conflicts;
        }

        info!(
            from = %from_cluster,
            to = %to_cluster,
            fed_id = %fed_id.short(),
            objects = received.len(),
            conflicts,
            "sync completed"
        );

        SyncResult::success(received, conflicts)
    }

    /// Sync a resource bidirectionally between two clusters.
    pub fn sync_bidirectional(
        &mut self,
        cluster_a: &str,
        cluster_b: &str,
        fed_id: FederatedId,
    ) -> (SyncResult, SyncResult) {
        let a_to_b = self.sync_resource(cluster_a, cluster_b, fed_id);
        let b_to_a = self.sync_resource(cluster_b, cluster_a, fed_id);
        (a_to_b, b_to_a)
    }

    /// Get sync statistics.
    pub fn sync_stats(&self) -> SyncStatistics {
        self.sync_stats.read().clone()
    }

    /// Set a ref on a cluster's resource.
    pub fn set_ref(
        &self,
        cluster_name: &str,
        fed_id: FederatedId,
        ref_name: impl Into<String>,
        hash: [u8; 32],
    ) -> Result<()> {
        let cluster =
            self.clusters.get(cluster_name).with_context(|| format!("cluster '{}' not found", cluster_name))?;

        let store = cluster.get_or_create_store(fed_id);
        store.set_ref(ref_name, hash);
        Ok(())
    }

    /// Get a ref from a cluster's resource.
    pub fn get_ref(&self, cluster_name: &str, fed_id: &FederatedId, ref_name: &str) -> Option<[u8; 32]> {
        let cluster = self.clusters.get(cluster_name)?;
        let store = cluster.get_store(fed_id)?;
        store.get_ref(ref_name)
    }

    /// End the test and finalize artifacts.
    pub fn end(self) {
        let stats = self.sync_stats.read().clone();
        info!(
            test = %self.test_name,
            clusters = self.clusters.len(),
            resources = self.resources.len(),
            sync_attempts = stats.sync_attempts,
            sync_successes = stats.sync_successes,
            objects_transferred = stats.objects_transferred,
            conflicts_resolved = stats.conflicts_resolved,
            "federation test complete"
        );

        // Artifact is dropped here, which finalizes it
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_clusters() {
        let mut t = FederationTester::new("test_create_clusters").await;

        let alice_key = t.create_cluster("alice").await.unwrap();
        let bob_key = t.create_cluster("bob").await.unwrap();

        assert_ne!(alice_key, bob_key);
        assert_eq!(t.cluster_count(), 2);

        // Can't create duplicate
        assert!(t.create_cluster("alice").await.is_err());

        t.end();
    }

    #[tokio::test]
    async fn test_trust_relationships() {
        let mut t = FederationTester::new("test_trust").await;

        t.create_cluster("alice").await.unwrap();
        t.create_cluster("bob").await.unwrap();

        assert!(!t.is_trusted("alice", "bob"));
        assert!(!t.is_trusted("bob", "alice"));

        t.add_trust("alice", "bob").await.unwrap();

        assert!(t.is_trusted("alice", "bob"));
        assert!(!t.is_trusted("bob", "alice"));

        t.add_mutual_trust("alice", "bob").await.unwrap();

        assert!(t.is_trusted("alice", "bob"));
        assert!(t.is_trusted("bob", "alice"));

        t.end();
    }

    #[tokio::test]
    async fn test_app_discovery() {
        let mut t = FederationTester::new("test_app_discovery").await;

        t.create_cluster("alice").await.unwrap();
        t.create_cluster("bob").await.unwrap();

        // Both clusters have Forge by default
        let forge_clusters = t.find_clusters_with_app("forge");
        assert_eq!(forge_clusters.len(), 2);

        // No clusters have "unknown" app
        let unknown_clusters = t.find_clusters_with_app("unknown");
        assert!(unknown_clusters.is_empty());

        t.end();
    }

    #[tokio::test]
    async fn test_federated_resources() {
        let mut t = FederationTester::new("test_resources").await;

        t.create_cluster("alice").await.unwrap();

        let local_id = blake3::hash(b"my-repo").into();
        let fed_id = t.create_federated_resource("alice", local_id, FederationMode::Public).await.unwrap();

        assert_eq!(t.resource_count(), 1);

        // Resource is registered for discovery
        let seeders = t.get_seeders(&fed_id);
        assert_eq!(seeders.len(), 1);
        assert_eq!(seeders[0], t.cluster_key("alice").unwrap());

        t.end();
    }

    #[tokio::test]
    async fn test_discovery_failures() {
        let mut t = FederationTester::new("test_discovery_failures").await;

        t.create_cluster("alice").await.unwrap();
        let alice_key = t.cluster_key("alice").unwrap();

        // Initially discoverable
        assert!(t.discovery.discover_cluster(&alice_key).is_some());

        // Set 2 failures
        t.set_discovery_failures("alice", 2).unwrap();

        // First two attempts fail
        assert!(t.discovery.discover_cluster(&alice_key).is_none());
        assert!(t.discovery.discover_cluster(&alice_key).is_none());

        // Third attempt succeeds
        assert!(t.discovery.discover_cluster(&alice_key).is_some());

        t.end();
    }

    #[tokio::test]
    async fn test_max_clusters_limit() {
        let mut t = FederationTester::new("test_max_clusters").await;

        // Create maximum allowed clusters
        for i in 0..MAX_CLUSTERS_PER_TEST {
            t.create_cluster(format!("cluster{}", i)).await.unwrap();
        }

        // Next one should fail
        assert!(t.create_cluster("overflow").await.is_err());

        t.end();
    }

    // ========================================================================
    // Multi-Cluster Sync Tests
    // ========================================================================

    #[tokio::test]
    async fn test_basic_two_cluster_sync() {
        let mut t = FederationTester::new("test_basic_sync").await;

        // Create two clusters with mutual trust
        t.create_cluster("alice").await.unwrap();
        t.create_cluster("bob").await.unwrap();
        t.add_mutual_trust("alice", "bob").await.unwrap();

        // Create a federated resource on alice
        let local_id = blake3::hash(b"shared-repo").into();
        let fed_id = t.create_federated_resource("alice", local_id, FederationMode::Public).await.unwrap();

        // Store some objects on alice
        let obj1 = t.store_object("alice", fed_id, b"commit1".to_vec()).unwrap();
        let obj2 = t.store_object("alice", fed_id, b"commit2".to_vec()).unwrap();
        let obj3 = t.store_object("alice", fed_id, b"commit3".to_vec()).unwrap();

        assert_eq!(t.object_count("alice", &fed_id), 3);
        assert_eq!(t.object_count("bob", &fed_id), 0);

        // Sync from alice to bob
        let result = t.sync_resource("alice", "bob", fed_id);
        assert!(result.is_success());
        assert_eq!(result.received.len(), 3);

        // Bob should now have all objects
        assert_eq!(t.object_count("bob", &fed_id), 3);
        assert!(t.get_object("bob", &fed_id, &obj1).is_some());
        assert!(t.get_object("bob", &fed_id, &obj2).is_some());
        assert!(t.get_object("bob", &fed_id, &obj3).is_some());

        // Check stats
        let stats = t.sync_stats();
        assert_eq!(stats.sync_attempts, 1);
        assert_eq!(stats.sync_successes, 1);
        assert_eq!(stats.objects_transferred, 3);

        t.end();
    }

    #[tokio::test]
    async fn test_bidirectional_sync() {
        let mut t = FederationTester::new("test_bidirectional").await;

        t.create_cluster("alice").await.unwrap();
        t.create_cluster("bob").await.unwrap();
        t.add_mutual_trust("alice", "bob").await.unwrap();

        let local_id = blake3::hash(b"collab-repo").into();
        let fed_id = t.create_federated_resource("alice", local_id, FederationMode::Public).await.unwrap();

        // Alice creates some objects
        t.store_object("alice", fed_id, b"alice-obj1".to_vec()).unwrap();
        t.store_object("alice", fed_id, b"alice-obj2".to_vec()).unwrap();

        // Bob creates different objects
        t.store_object("bob", fed_id, b"bob-obj1".to_vec()).unwrap();
        t.store_object("bob", fed_id, b"bob-obj2".to_vec()).unwrap();
        t.store_object("bob", fed_id, b"bob-obj3".to_vec()).unwrap();

        assert_eq!(t.object_count("alice", &fed_id), 2);
        assert_eq!(t.object_count("bob", &fed_id), 3);

        // Bidirectional sync
        let (a_to_b, b_to_a) = t.sync_bidirectional("alice", "bob", fed_id);

        assert!(a_to_b.is_success());
        assert!(b_to_a.is_success());
        assert_eq!(a_to_b.received.len(), 2); // bob got alice's 2
        assert_eq!(b_to_a.received.len(), 3); // alice got bob's 3

        // Both should have all 5 objects
        assert_eq!(t.object_count("alice", &fed_id), 5);
        assert_eq!(t.object_count("bob", &fed_id), 5);

        t.end();
    }

    #[tokio::test]
    async fn test_three_way_federation() {
        let mut t = FederationTester::new("test_three_way").await;

        // Create three clusters in a chain: alice <-> bob <-> carol
        t.create_cluster("alice").await.unwrap();
        t.create_cluster("bob").await.unwrap();
        t.create_cluster("carol").await.unwrap();

        // Bob trusts both, alice and carol trust bob
        t.add_mutual_trust("alice", "bob").await.unwrap();
        t.add_mutual_trust("bob", "carol").await.unwrap();
        // Note: alice and carol don't directly trust each other

        let local_id = blake3::hash(b"shared-project").into();
        let fed_id = t.create_federated_resource("alice", local_id, FederationMode::Public).await.unwrap();

        // Alice creates the initial content
        let alice_obj = t.store_object("alice", fed_id, b"initial".to_vec()).unwrap();

        // Sync alice -> bob
        let r1 = t.sync_resource("alice", "bob", fed_id);
        assert!(r1.is_success());
        assert_eq!(t.object_count("bob", &fed_id), 1);

        // Sync bob -> carol
        let r2 = t.sync_resource("bob", "carol", fed_id);
        assert!(r2.is_success());
        assert_eq!(t.object_count("carol", &fed_id), 1);

        // Carol adds content
        let carol_obj = t.store_object("carol", fed_id, b"carol-contrib".to_vec()).unwrap();

        // Carol can't sync directly to alice (no trust)
        let r3 = t.sync_resource("carol", "alice", fed_id);
        assert!(!r3.is_success());
        assert!(r3.error.as_ref().unwrap().contains("does not trust"));

        // But carol -> bob -> alice works
        t.sync_resource("carol", "bob", fed_id);
        t.sync_resource("bob", "alice", fed_id);

        // Now all three have both objects
        assert_eq!(t.object_count("alice", &fed_id), 2);
        assert_eq!(t.object_count("bob", &fed_id), 2);
        assert_eq!(t.object_count("carol", &fed_id), 2);

        assert!(t.get_object("carol", &fed_id, &alice_obj).is_some());
        assert!(t.get_object("alice", &fed_id, &carol_obj).is_some());

        t.end();
    }

    #[tokio::test]
    async fn test_network_partition_blocks_sync() {
        let mut t = FederationTester::new("test_partition").await;

        t.create_cluster("alice").await.unwrap();
        t.create_cluster("bob").await.unwrap();
        t.add_mutual_trust("alice", "bob").await.unwrap();

        let local_id = blake3::hash(b"repo").into();
        let fed_id = t.create_federated_resource("alice", local_id, FederationMode::Public).await.unwrap();

        t.store_object("alice", fed_id, b"obj1".to_vec()).unwrap();

        // Initially can communicate
        assert!(t.can_communicate("alice", "bob"));
        let r1 = t.sync_resource("alice", "bob", fed_id);
        assert!(r1.is_success());
        assert_eq!(t.object_count("bob", &fed_id), 1);

        // Create partition
        t.partition("alice", "bob");
        assert!(!t.can_communicate("alice", "bob"));

        // Store more objects during partition
        t.store_object("alice", fed_id, b"obj2".to_vec()).unwrap();
        t.store_object("alice", fed_id, b"obj3".to_vec()).unwrap();

        // Sync fails during partition
        let r2 = t.sync_resource("alice", "bob", fed_id);
        assert!(!r2.is_success());
        assert!(r2.error.as_ref().unwrap().contains("partition"));

        // Bob still has only 1 object
        assert_eq!(t.object_count("bob", &fed_id), 1);

        // Heal partition
        t.heal_partition("alice", "bob");
        assert!(t.can_communicate("alice", "bob"));

        // Now sync works again
        let r3 = t.sync_resource("alice", "bob", fed_id);
        assert!(r3.is_success());
        assert_eq!(r3.received.len(), 2); // The 2 new objects

        assert_eq!(t.object_count("bob", &fed_id), 3);

        t.end();
    }

    #[tokio::test]
    async fn test_partition_recovery_with_concurrent_writes() {
        let mut t = FederationTester::new("test_partition_recovery").await;

        t.create_cluster("alice").await.unwrap();
        t.create_cluster("bob").await.unwrap();
        t.add_mutual_trust("alice", "bob").await.unwrap();

        let local_id = blake3::hash(b"repo").into();
        let fed_id = t.create_federated_resource("alice", local_id, FederationMode::Public).await.unwrap();

        // Initial sync
        t.store_object("alice", fed_id, b"initial".to_vec()).unwrap();
        t.sync_resource("alice", "bob", fed_id);

        // Partition
        t.partition("alice", "bob");

        // Both clusters write concurrently during partition
        t.store_object("alice", fed_id, b"alice-during-partition-1".to_vec()).unwrap();
        t.store_object("alice", fed_id, b"alice-during-partition-2".to_vec()).unwrap();
        t.store_object("bob", fed_id, b"bob-during-partition-1".to_vec()).unwrap();
        t.store_object("bob", fed_id, b"bob-during-partition-2".to_vec()).unwrap();
        t.store_object("bob", fed_id, b"bob-during-partition-3".to_vec()).unwrap();

        // Heal and reconcile
        t.heal_partition("alice", "bob");
        t.sync_bidirectional("alice", "bob", fed_id);

        // Both should have all 6 objects (1 initial + 2 alice + 3 bob)
        assert_eq!(t.object_count("alice", &fed_id), 6);
        assert_eq!(t.object_count("bob", &fed_id), 6);

        t.end();
    }

    #[tokio::test]
    async fn test_sync_without_trust_fails() {
        let mut t = FederationTester::new("test_no_trust").await;

        t.create_cluster("alice").await.unwrap();
        t.create_cluster("bob").await.unwrap();
        // No trust established

        let local_id = blake3::hash(b"repo").into();
        let fed_id = t.create_federated_resource("alice", local_id, FederationMode::Public).await.unwrap();

        t.store_object("alice", fed_id, b"data".to_vec()).unwrap();

        let result = t.sync_resource("alice", "bob", fed_id);
        assert!(!result.is_success());
        assert!(result.error.as_ref().unwrap().contains("does not trust"));

        // Bob has nothing
        assert_eq!(t.object_count("bob", &fed_id), 0);

        t.end();
    }

    #[tokio::test]
    async fn test_ref_sync() {
        let mut t = FederationTester::new("test_refs").await;

        t.create_cluster("alice").await.unwrap();
        t.create_cluster("bob").await.unwrap();
        t.add_mutual_trust("alice", "bob").await.unwrap();

        let local_id = blake3::hash(b"git-repo").into();
        let fed_id = t.create_federated_resource("alice", local_id, FederationMode::Public).await.unwrap();

        // Store commits and set refs
        let commit1 = t.store_object("alice", fed_id, b"commit-1".to_vec()).unwrap();
        let commit2 = t.store_object("alice", fed_id, b"commit-2".to_vec()).unwrap();

        t.set_ref("alice", fed_id, "refs/heads/main", commit2).unwrap();
        t.set_ref("alice", fed_id, "refs/heads/feature", commit1).unwrap();

        // Verify refs on alice
        assert_eq!(t.get_ref("alice", &fed_id, "refs/heads/main"), Some(commit2));
        assert_eq!(t.get_ref("alice", &fed_id, "refs/heads/feature"), Some(commit1));

        // Sync objects to bob
        t.sync_resource("alice", "bob", fed_id);

        // Bob has objects but not refs (refs are local)
        assert_eq!(t.object_count("bob", &fed_id), 2);
        assert!(t.get_ref("bob", &fed_id, "refs/heads/main").is_none());

        // Bob can set their own refs pointing to synced objects
        t.set_ref("bob", fed_id, "refs/heads/main", commit2).unwrap();
        assert_eq!(t.get_ref("bob", &fed_id, "refs/heads/main"), Some(commit2));

        t.end();
    }

    #[tokio::test]
    async fn test_incremental_sync() {
        let mut t = FederationTester::new("test_incremental").await;

        t.create_cluster("alice").await.unwrap();
        t.create_cluster("bob").await.unwrap();
        t.add_mutual_trust("alice", "bob").await.unwrap();

        let local_id = blake3::hash(b"repo").into();
        let fed_id = t.create_federated_resource("alice", local_id, FederationMode::Public).await.unwrap();

        // First batch
        for i in 0..5 {
            t.store_object("alice", fed_id, format!("obj-batch1-{}", i).into_bytes()).unwrap();
        }

        let r1 = t.sync_resource("alice", "bob", fed_id);
        assert!(r1.is_success());
        assert_eq!(r1.received.len(), 5);
        assert_eq!(t.object_count("bob", &fed_id), 5);

        // Second batch
        for i in 0..3 {
            t.store_object("alice", fed_id, format!("obj-batch2-{}", i).into_bytes()).unwrap();
        }

        // Incremental sync should only transfer new objects
        let r2 = t.sync_resource("alice", "bob", fed_id);
        assert!(r2.is_success());
        assert_eq!(r2.received.len(), 3); // Only the new ones
        assert_eq!(t.object_count("bob", &fed_id), 8);

        // Syncing again with no new objects
        let r3 = t.sync_resource("alice", "bob", fed_id);
        assert!(r3.is_success());
        assert_eq!(r3.received.len(), 0); // Nothing new

        t.end();
    }

    #[tokio::test]
    async fn test_sync_stats() {
        let mut t = FederationTester::new("test_stats").await;

        t.create_cluster("alice").await.unwrap();
        t.create_cluster("bob").await.unwrap();
        t.create_cluster("carol").await.unwrap();

        t.add_mutual_trust("alice", "bob").await.unwrap();
        // Carol has no trust with anyone

        let local_id = blake3::hash(b"repo").into();
        let fed_id = t.create_federated_resource("alice", local_id, FederationMode::Public).await.unwrap();

        t.store_object("alice", fed_id, b"obj1".to_vec()).unwrap();
        t.store_object("alice", fed_id, b"obj2".to_vec()).unwrap();

        // Successful sync
        t.sync_resource("alice", "bob", fed_id);

        // Failed sync (no trust)
        t.sync_resource("alice", "carol", fed_id);

        // Partition and fail
        t.partition("alice", "bob");
        t.sync_resource("alice", "bob", fed_id);

        let stats = t.sync_stats();
        assert_eq!(stats.sync_attempts, 3);
        assert_eq!(stats.sync_successes, 1);
        assert_eq!(stats.sync_failures, 2);
        assert_eq!(stats.objects_transferred, 2);

        t.end();
    }

    #[tokio::test]
    async fn test_large_scale_sync() {
        let mut t = FederationTester::new("test_large_scale").await;

        t.create_cluster("alice").await.unwrap();
        t.create_cluster("bob").await.unwrap();
        t.add_mutual_trust("alice", "bob").await.unwrap();

        let local_id = blake3::hash(b"big-repo").into();
        let fed_id = t.create_federated_resource("alice", local_id, FederationMode::Public).await.unwrap();

        // Store many objects (within batch limit)
        let object_count = 50;
        for i in 0..object_count {
            t.store_object("alice", fed_id, format!("object-{:04}", i).into_bytes()).unwrap();
        }

        assert_eq!(t.object_count("alice", &fed_id), object_count);

        // Sync all at once
        let result = t.sync_resource("alice", "bob", fed_id);
        assert!(result.is_success());
        assert_eq!(result.received.len(), object_count);
        assert_eq!(t.object_count("bob", &fed_id), object_count);

        // Verify stats
        let stats = t.sync_stats();
        assert_eq!(stats.objects_transferred, object_count);

        t.end();
    }
}
