//! Configuration for the Nix build worker.

use std::path::PathBuf;
use std::sync::Arc;

use aspen_blob::prelude::*;
use aspen_cache::CacheIndex;
use iroh::Endpoint;
use iroh::PublicKey;
use snix_castore::blobservice::BlobService;
use snix_castore::directoryservice::DirectoryService;
use snix_store::pathinfoservice::PathInfoService;
use tracing::info;
use tracing::warn;

// Tiger Style: All limits explicit and bounded
/// Maximum flake URL length.
pub(crate) const MAX_FLAKE_URL_LENGTH: usize = 4096;
/// Maximum attribute path length.
pub(crate) const MAX_ATTR_LENGTH: usize = 1024;
/// Maximum build log size to capture inline (64 KB).
pub(crate) const INLINE_LOG_THRESHOLD: usize = 64 * 1024;
/// Maximum total log size (10 MB).
pub(crate) const MAX_LOG_SIZE: usize = 10 * 1024 * 1024;
/// Default build timeout (30 minutes).
pub(crate) const DEFAULT_TIMEOUT_SECS: u64 = 1800;
/// Maximum build timeout (4 hours).
pub(crate) const MAX_TIMEOUT_SECS: u64 = 14400;
/// Maximum NAR size to upload (matches MAX_BLOB_SIZE from aspen-blob).
/// Store paths larger than this are skipped to avoid memory exhaustion.
pub(crate) const MAX_NAR_UPLOAD_SIZE: u64 = 1_073_741_824; // 1 GB

/// Configuration for NixBuildWorker.
pub struct NixBuildWorkerConfig {
    /// Node ID for logging/metrics.
    pub node_id: u64,

    /// Cluster ID (cookie) for identifying the cluster.
    pub cluster_id: String,

    /// Optional blob store for artifact storage.
    pub blob_store: Option<Arc<dyn BlobStore>>,

    /// Optional cache index for registering built store paths.
    /// When set, built store paths are automatically registered in the
    /// distributed Nix binary cache.
    pub cache_index: Option<Arc<dyn CacheIndex>>,

    /// SNIX blob service for decomposed content-addressed storage.
    /// When set along with directory and pathinfo services, built store paths
    /// are ingested as NAR archives directly into the SNIX storage layer.
    pub snix_blob_service: Option<Arc<dyn BlobService>>,
    /// SNIX directory service for storing directory metadata.
    pub snix_directory_service: Option<Arc<dyn DirectoryService>>,
    /// SNIX path info service for storing Nix store path metadata.
    pub snix_pathinfo_service: Option<Arc<dyn PathInfoService>>,

    /// Directory for build outputs.
    pub output_dir: PathBuf,

    /// Nix binary path (defaults to "nix").
    pub nix_binary: String,

    /// Whether to enable verbose logging.
    pub is_verbose: bool,

    // --- Cache Proxy Configuration (Phase 1) ---
    /// Whether to use the cluster's Nix binary cache as a substituter.
    /// When enabled, the worker starts a local HTTP proxy that bridges
    /// Nix's HTTP requests to the Aspen cache gateway over Iroh H3.
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

impl Default for NixBuildWorkerConfig {
    fn default() -> Self {
        Self {
            node_id: 0,
            cluster_id: String::new(),
            blob_store: None,
            cache_index: None,
            snix_blob_service: None,
            snix_directory_service: None,
            snix_pathinfo_service: None,
            output_dir: PathBuf::from("/tmp/aspen-ci/builds"),
            nix_binary: "nix".to_string(),
            is_verbose: false,
            // Cache proxy disabled by default until gateway is configured
            use_cluster_cache: false,
            iroh_endpoint: None,
            gateway_node: None,
            cache_public_key: None,
        }
    }
}

impl NixBuildWorkerConfig {
    /// Validate the configuration and log warnings for missing optional services.
    ///
    /// Returns `true` if the configuration is valid for full artifact storage,
    /// `false` if some services are missing (worker will still function but
    /// with reduced capabilities).
    ///
    /// # Warnings logged
    ///
    /// - Missing `blob_store`: Artifacts will not be uploaded to distributed storage
    /// - Missing `cache_index`: Store paths will not be registered in binary cache
    /// - Missing SNIX services: Store paths will not be ingested to SNIX storage
    pub fn validate(&self) -> bool {
        let mut all_services_available = true;

        if self.blob_store.is_none() {
            warn!(
                node_id = self.node_id,
                "NixBuildWorkerConfig: blob_store is None - CI artifacts will not be uploaded to distributed storage"
            );
            all_services_available = false;
        }

        if self.cache_index.is_none() {
            warn!(
                node_id = self.node_id,
                "NixBuildWorkerConfig: cache_index is None - store paths will not be registered in binary cache"
            );
            all_services_available = false;
        }

        // Check SNIX services (all three must be present for SNIX storage)
        let has_snix = self.snix_blob_service.is_some()
            && self.snix_directory_service.is_some()
            && self.snix_pathinfo_service.is_some();

        // Partial SNIX config is a misconfiguration (missing SNIX entirely is fine)
        if !has_snix
            && (self.snix_blob_service.is_some()
                || self.snix_directory_service.is_some()
                || self.snix_pathinfo_service.is_some())
        {
            warn!(
                node_id = self.node_id,
                has_blob_service = self.snix_blob_service.is_some(),
                has_directory_service = self.snix_directory_service.is_some(),
                has_pathinfo_service = self.snix_pathinfo_service.is_some(),
                "NixBuildWorkerConfig: partial SNIX services configured - all three services required for SNIX storage"
            );
        }

        // Check cache proxy config (all three must be present if enabled)
        if self.use_cluster_cache {
            let has_endpoint = self.iroh_endpoint.is_some();
            let has_gateway = self.gateway_node.is_some();
            let has_key = self.cache_public_key.is_some();

            if !has_endpoint || !has_gateway || !has_key {
                warn!(
                    node_id = self.node_id,
                    has_endpoint = has_endpoint,
                    has_gateway = has_gateway,
                    has_key = has_key,
                    "NixBuildWorkerConfig: use_cluster_cache enabled but missing required config. \
                     Ensure nix_cache.enabled is true and public key is stored at _system:nix-cache:public-key. \
                     Cache substituter will be disabled for builds."
                );
            } else {
                let gateway_short = self
                    .gateway_node
                    .as_ref()
                    .map(|g| g.fmt_short().to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                info!(
                    node_id = self.node_id,
                    gateway_node = %gateway_short,
                    "NixBuildWorkerConfig: cluster cache substituter enabled"
                );
            }
        }

        all_services_available
    }

    /// Check if cache proxy can be started.
    pub fn can_use_cache_proxy(&self) -> bool {
        self.use_cluster_cache
            && self.iroh_endpoint.is_some()
            && self.gateway_node.is_some()
            && self.cache_public_key.is_some()
    }
}
