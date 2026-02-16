//! LocalExecutorWorkerConfig - configuration for the local executor worker.

use std::path::PathBuf;
use std::sync::Arc;

use aspen_cache::CacheIndex;
use aspen_core::KeyValueStore;
use iroh::Endpoint;
use iroh::PublicKey;
#[cfg(feature = "snix")]
use snix_castore::blobservice::BlobService;
#[cfg(feature = "snix")]
use snix_castore::directoryservice::DirectoryService;
#[cfg(feature = "snix")]
use snix_store::pathinfoservice::PathInfoService;

/// Configuration for LocalExecutorWorker.
pub struct LocalExecutorWorkerConfig {
    /// Base workspace directory where jobs run.
    /// Each job gets a subdirectory under this path.
    pub workspace_dir: PathBuf,

    /// Whether to clean up job workspaces after completion.
    pub should_cleanup_workspaces: bool,

    // --- SNIX services for Nix binary cache integration ---
    /// SNIX blob service for decomposed content-addressed storage.
    /// When set along with directory and pathinfo services, built store paths
    /// are ingested as NAR archives directly into the SNIX storage layer.
    #[cfg(feature = "snix")]
    pub snix_blob_service: Option<Arc<dyn BlobService>>,

    /// SNIX directory service for storing directory metadata.
    #[cfg(feature = "snix")]
    pub snix_directory_service: Option<Arc<dyn DirectoryService>>,

    /// SNIX path info service for storing Nix store path metadata.
    #[cfg(feature = "snix")]
    pub snix_pathinfo_service: Option<Arc<dyn PathInfoService>>,

    /// Optional cache index for registering built store paths.
    /// When set, built store paths are automatically registered in the
    /// distributed Nix binary cache (legacy format).
    pub cache_index: Option<Arc<dyn CacheIndex>>,

    // --- Cache/store access ---
    /// KV store for cache metadata (Cargo cache, etc).
    pub kv_store: Option<Arc<dyn KeyValueStore>>,

    // --- Nix cache substituter configuration ---
    /// Whether to use the cluster's Nix binary cache as a substituter.
    /// When enabled, nix commands will be configured to use the cluster cache.
    pub use_cluster_cache: bool,

    /// Iroh endpoint for connecting to the cache gateway.
    /// Required when `use_cluster_cache` is true.
    pub iroh_endpoint: Option<Arc<Endpoint>>,

    /// NodeId of the nix-cache-gateway service.
    /// Required when `use_cluster_cache` is true.
    pub gateway_node: Option<PublicKey>,

    /// Trusted public key for the cache (e.g., "aspen-cache:base64key").
    /// Required when `use_cluster_cache` is true to verify signed narinfo.
    pub cache_public_key: Option<String>,
}

impl Default for LocalExecutorWorkerConfig {
    fn default() -> Self {
        Self {
            workspace_dir: PathBuf::from("/workspace"),
            should_cleanup_workspaces: true,
            #[cfg(feature = "snix")]
            snix_blob_service: None,
            #[cfg(feature = "snix")]
            snix_directory_service: None,
            #[cfg(feature = "snix")]
            snix_pathinfo_service: None,
            cache_index: None,
            kv_store: None,
            use_cluster_cache: false,
            iroh_endpoint: None,
            gateway_node: None,
            cache_public_key: None,
        }
    }
}

impl LocalExecutorWorkerConfig {
    /// Check if cache proxy can be started.
    ///
    /// Returns true if all required components are available:
    /// - use_cluster_cache is enabled
    /// - iroh endpoint is configured
    /// - gateway node is known
    /// - cache public key is set
    ///
    /// Requires the `nix-cache-proxy` feature to be enabled.
    #[cfg(feature = "nix-cache-proxy")]
    pub fn can_use_cache_proxy(&self) -> bool {
        self.use_cluster_cache
            && self.iroh_endpoint.is_some()
            && self.gateway_node.is_some()
            && self.cache_public_key.is_some()
    }

    /// Cache proxy is not available without the `nix-cache-proxy` feature.
    #[cfg(not(feature = "nix-cache-proxy"))]
    pub fn can_use_cache_proxy(&self) -> bool {
        false
    }
}
