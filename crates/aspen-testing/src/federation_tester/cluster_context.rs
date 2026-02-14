//! Cluster context for simulated federation clusters.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use aspen_cluster::federation::AppManifest;
use aspen_cluster::federation::AppRegistry;
use aspen_cluster::federation::ClusterIdentity;
use aspen_cluster::federation::FederatedId;
use aspen_cluster::federation::FederationSettings;
use aspen_core::hlc::HLC;
use aspen_core::hlc::SerializableTimestamp;
use iroh::PublicKey;
use parking_lot::RwLock;

use super::sync_types::ResourceDataStore;
use super::sync_types::SyncableObject;

/// Context for a single simulated cluster.
pub struct ClusterContext {
    /// Cluster name (for identification in tests).
    pub name: String,
    /// Cluster identity (Ed25519 keypair).
    pub identity: ClusterIdentity,
    /// Trust manager for this cluster (not Debug).
    pub trust_manager: Arc<aspen_cluster::federation::TrustManager>,
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
        let trust_manager = Arc::new(aspen_cluster::federation::TrustManager::new());
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
