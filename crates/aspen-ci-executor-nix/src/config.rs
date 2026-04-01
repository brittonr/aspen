//! Configuration for the Nix build worker.

use std::path::PathBuf;
use std::sync::Arc;

use aspen_blob::prelude::*;
use aspen_cache::CacheIndex;
use aspen_core::KeyValueStore;
use aspen_core::ReadRequest;
use iroh::Endpoint;
use iroh::PublicKey;
#[cfg(feature = "snix")]
use snix_castore::blobservice::BlobService;
#[cfg(feature = "snix")]
use snix_castore::directoryservice::DirectoryService;
#[cfg(feature = "snix")]
use snix_store::pathinfoservice::PathInfoService;
use tokio::sync::OnceCell;
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
#[cfg(feature = "nix-cli-fallback")]
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

    /// Optional KV store for streaming build logs.
    /// When set, build stderr is streamed to KV as log chunks in real-time,
    /// readable via `CiGetJobLogs` RPC.
    pub kv_store: Option<Arc<dyn KeyValueStore>>,

    /// Optional cache index for registering built store paths.
    /// When set, built store paths are automatically registered in the
    /// distributed Nix binary cache.
    pub cache_index: Option<Arc<dyn CacheIndex>>,

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

    /// URL of a running `aspen-nix-cache-gateway` HTTP server.
    /// When set (with `cache_public_key`), Nix builds use this as a substituter
    /// without needing the H3 proxy. Example: `http://127.0.0.1:8380`.
    pub gateway_url: Option<String>,

    /// Lazily resolved cache public key. On first build, if `cache_public_key`
    /// is None but `kv_store` is available, reads from KV. This handles the
    /// startup race where the signing key is created after cluster init but
    /// the worker config was built before init completed.
    #[doc(hidden)]
    pub resolved_public_key: OnceCell<Option<String>>,

    /// Upstream binary cache configuration for closure bootstrap.
    /// When set, the native build path can fetch missing store paths
    /// from upstream caches (e.g., cache.nixos.org) instead of falling
    /// back to `nix-store --realise`.
    #[cfg(feature = "snix-build")]
    pub upstream_cache_config: Option<crate::upstream_cache::UpstreamCacheConfig>,
}

impl Default for NixBuildWorkerConfig {
    fn default() -> Self {
        Self {
            node_id: 0,
            cluster_id: String::new(),
            blob_store: None,
            kv_store: None,
            cache_index: None,
            #[cfg(feature = "snix")]
            snix_blob_service: None,
            #[cfg(feature = "snix")]
            snix_directory_service: None,
            #[cfg(feature = "snix")]
            snix_pathinfo_service: None,
            output_dir: PathBuf::from("/tmp/aspen-ci/builds"),
            nix_binary: "nix".to_string(),
            is_verbose: false,
            // Cache proxy disabled by default until gateway is configured
            use_cluster_cache: false,
            iroh_endpoint: None,
            gateway_node: None,
            cache_public_key: None,
            gateway_url: None,
            resolved_public_key: OnceCell::new(),
            #[cfg(feature = "snix-build")]
            upstream_cache_config: Some(crate::upstream_cache::UpstreamCacheConfig::default()),
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
        #[cfg(feature = "snix")]
        {
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

    /// Check if cache proxy can be started (H3-based, requires iroh endpoint).
    pub fn can_use_cache_proxy(&self) -> bool {
        self.use_cluster_cache
            && self.iroh_endpoint.is_some()
            && self.gateway_node.is_some()
            && self.cache_public_key.is_some()
    }

    /// Check if a gateway URL substituter is available.
    ///
    /// This is the simpler path: point at a pre-running `aspen-nix-cache-gateway`
    /// HTTP server. No H3 proxy needed.
    pub fn can_use_gateway(&self) -> bool {
        self.gateway_url.is_some() && self.get_public_key().is_some()
    }

    /// Get extra args to inject into `nix build` for cache substitution.
    ///
    /// Returns `None` if no cache is configured.
    pub fn substituter_args(&self) -> Option<Vec<String>> {
        let key = self.get_public_key()?;
        let url = self.gateway_url.as_ref()?;
        // Use --extra-* variants to append to defaults rather than replace.
        // --trusted-public-keys would replace the default cache.nixos.org key,
        // causing nix to reject all substitutes from cache.nixos.org.
        Some(vec![
            "--extra-substituters".to_string(),
            url.clone(),
            "--extra-trusted-public-keys".to_string(),
            key,
        ])
    }

    /// Get the cache public key, using the eagerly-set value or the lazily
    /// resolved one from KV.
    pub(crate) fn get_public_key(&self) -> Option<String> {
        if let Some(ref key) = self.cache_public_key {
            return Some(key.clone());
        }
        self.resolved_public_key.get()?.clone()
    }

    /// Attempt to resolve the cache public key from KV store.
    ///
    /// Called before the first build to handle the startup race where
    /// `ensure_signing_key` runs after cluster init but the worker config
    /// was created before init completed.
    pub async fn resolve_cache_public_key(&self) {
        if self.cache_public_key.is_some() {
            return;
        }
        let kv = match self.kv_store.as_ref() {
            Some(kv) => kv,
            None => return,
        };
        let _ = self
            .resolved_public_key
            .get_or_init(|| async {
                let keys = [
                    aspen_cache::signing::CACHE_PUBLIC_KEY_KV,
                    "_system:nix-cache:public-key",
                ];
                for key in &keys {
                    if let Ok(result) = kv.read(ReadRequest::new(*key)).await
                        && let Some(kv_entry) = result.kv
                    {
                        info!(
                            public_key = %kv_entry.value,
                            kv_key = key,
                            "Lazily resolved Nix cache public key from KV"
                        );
                        return Some(kv_entry.value);
                    }
                }
                None
            })
            .await;
    }
}
