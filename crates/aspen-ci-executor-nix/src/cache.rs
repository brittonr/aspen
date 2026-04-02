//! Cache registration and NAR upload to blob store.

use std::sync::Arc;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use aspen_cache::CacheEntry;
use aspen_cache::CacheIndex;
use aspen_ci_core::CiCoreError;
use aspen_ci_core::Result;
#[cfg(feature = "snix")]
use nix_compat::store_path::StorePath;
use serde::Serialize;
#[cfg(feature = "nix-cli-fallback")]
use tokio::process::Command;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::config::MAX_NAR_UPLOAD_SIZE;
use crate::executor::NixBuildWorker;

/// Information from nix path-info.
pub(crate) struct PathInfo {
    /// Store paths this entry references.
    pub(crate) references: Vec<String>,
    /// Deriver store path.
    pub(crate) deriver: Option<String>,
}

/// Metadata returned by `pathinfo_lookup` from PathInfoService.
#[cfg(feature = "snix")]
pub(crate) struct PathInfoMetadata {
    /// NAR size in bytes.
    pub(crate) nar_size: u64,
    /// NAR SHA-256 hash (raw 32 bytes).
    /// Used by upstream cache client for hash verification.
    #[expect(dead_code)]
    pub(crate) nar_sha256: [u8; 32],
    /// Store paths this entry references (absolute paths).
    pub(crate) references: Vec<String>,
    /// Deriver store path (absolute), if any.
    pub(crate) deriver: Option<String>,
}

/// Look up store path metadata from PathInfoService without subprocess.
///
/// Parses the store path to extract its 20-byte digest, queries
/// PathInfoService, and returns NAR size, hash, references, and deriver.
/// Returns `None` if the path is not in PathInfoService or parsing fails.
#[cfg(feature = "snix")]
pub(crate) async fn pathinfo_lookup(
    pathinfo_service: &dyn snix_store::pathinfoservice::PathInfoService,
    store_path: &str,
) -> Option<PathInfoMetadata> {
    let parsed = StorePath::<String>::from_absolute_path(store_path.as_bytes())
        .inspect_err(|e| debug!(store_path, "failed to parse store path: {e}"))
        .ok()?;

    let path_info = pathinfo_service
        .get(*parsed.digest())
        .await
        .inspect_err(|e| debug!(store_path, "PathInfoService lookup failed: {e}"))
        .ok()??;

    let references: Vec<String> = path_info.references.iter().map(|r| r.to_absolute_path()).collect();

    let deriver = path_info.deriver.as_ref().map(|d| d.to_absolute_path());

    Some(PathInfoMetadata {
        nar_size: path_info.nar_size,
        nar_sha256: path_info.nar_sha256,
        references,
        deriver,
    })
}

/// Information about an uploaded store path.
#[derive(Debug, Clone, Serialize)]
pub struct UploadedStorePath {
    /// The Nix store path.
    pub store_path: String,
    /// Blob hash of the NAR archive (BLAKE3).
    pub blob_hash: String,
    /// Size of the NAR archive in bytes.
    pub nar_size: u64,
    /// SHA256 hash of the NAR archive (Nix's native format).
    pub nar_hash: String,
    /// Whether this path was registered in the cache.
    pub cache_registered: bool,
}

impl NixBuildWorker {
    /// Check the NAR size of a store path.
    ///
    /// Queries PathInfoService first (zero subprocess). Falls back to
    /// `nix path-info --json` when `nix-cli-fallback` is enabled.
    /// Returns `None` if the path can't be resolved.
    pub(crate) async fn check_store_path_size(&self, store_path: &str) -> Option<u64> {
        // Primary: PathInfoService lookup (zero subprocess)
        #[cfg(feature = "snix")]
        if let Some(ref ps) = self.config.snix_pathinfo_service
            && let Some(meta) = pathinfo_lookup(ps.as_ref(), store_path).await
        {
            return Some(meta.nar_size);
        }

        // Fallback: nix path-info subprocess
        #[cfg(feature = "nix-cli-fallback")]
        {
            return self.check_store_path_size_subprocess(store_path).await;
        }

        #[cfg(not(feature = "nix-cli-fallback"))]
        {
            debug!(store_path, "path not in PathInfoService and nix-cli-fallback disabled");
            None
        }
    }

    /// Subprocess fallback for `check_store_path_size` via `nix path-info --json`.
    #[cfg(feature = "nix-cli-fallback")]
    async fn check_store_path_size_subprocess(&self, store_path: &str) -> Option<u64> {
        let output = Command::new(&self.config.nix_binary)
            .args(["path-info", "--json", store_path])
            .output()
            .await
            .inspect_err(|e| debug!(store_path, "nix path-info command failed: {e}"))
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let parsed: serde_json::Value = serde_json::from_str(&json_str)
            .inspect_err(|e| debug!(store_path, "failed to parse nix path-info JSON: {e}"))
            .ok()?;
        let entry = path_info_entry(&parsed, store_path)?;
        entry.get("narSize")?.as_u64()
    }

    /// Upload store paths to the blob store as NAR archives.
    ///
    /// When snix services are configured, uses castore ingestion
    /// (`ingest_nar_and_hash`) to decompose the NAR into the cluster's
    /// BlobService/DirectoryService and computes the NAR hash in-process.
    ///
    /// Falls back to `aspen_cache::nar::dump_path_nar_async` (reads from
    /// local filesystem) when snix services are not available.
    pub(crate) async fn upload_store_paths(
        &self,
        output_paths: &[String],
        ci_job_id: Option<&str>,
    ) -> Vec<UploadedStorePath> {
        let mut uploaded = Vec::new();

        let Some(ref blob_store) = self.config.blob_store else {
            debug!("No blob store configured, skipping store path upload");
            return uploaded;
        };

        for store_path in output_paths {
            info!(
                cluster_id = %self.config.cluster_id,
                node_id = self.config.node_id,
                store_path = %store_path,
                "Uploading store path as NAR"
            );

            // Check NAR size before dumping to avoid OOM on large paths
            if let Some(nar_size) = self.check_store_path_size(store_path).await
                && nar_size > MAX_NAR_UPLOAD_SIZE
            {
                warn!(
                    store_path = %store_path,
                    nar_size_bytes = nar_size,
                    max_size_bytes = MAX_NAR_UPLOAD_SIZE,
                    "Skipping store path upload: NAR size exceeds maximum"
                );
                continue;
            }

            // Create NAR archive in-process using nix-compat writer.
            // SHA-256 is computed in the same write pass via HashingWriter.
            let (nar_data, nar_sha256) = match aspen_cache::nar::dump_path_nar_async(store_path.into()).await {
                Ok(result) => result,
                Err(e) => {
                    warn!(store_path = %store_path, error = %e, "Failed to create NAR archive");
                    continue;
                }
            };

            let nar_size = nar_data.len() as u64;
            let nar_hash = format!("sha256:{}", aspen_cache::nixbase32::encode(&nar_sha256));

            // When snix services are available, also ingest the NAR into
            // castore so PathInfoService has the decomposed representation.
            // This replaces the need for `nix nar dump-path` in future
            // lookups — SimpleRenderer can reconstruct the NAR from castore.
            #[cfg(feature = "snix")]
            if let (Some(bs), Some(ds), Some(ps)) = (
                &self.config.snix_blob_service,
                &self.config.snix_directory_service,
                &self.config.snix_pathinfo_service,
            ) {
                let mut cursor = std::io::Cursor::new(&nar_data);
                match snix_store::nar::ingest_nar_and_hash(Arc::clone(bs), Arc::clone(ds), &mut cursor, &None).await {
                    Ok((root_node, ingested_sha256, ingested_size)) => {
                        if let Ok(sp) = StorePath::<String>::from_absolute_path(store_path.as_bytes()) {
                            let path_info = snix_store::path_info::PathInfo {
                                store_path: sp,
                                node: root_node,
                                references: self
                                    .query_path_info(store_path)
                                    .await
                                    .map(|info| {
                                        info.references
                                            .iter()
                                            .filter_map(|r| StorePath::from_absolute_path(r.as_bytes()).ok())
                                            .collect()
                                    })
                                    .unwrap_or_default(),
                                nar_size: ingested_size,
                                nar_sha256: ingested_sha256,
                                signatures: vec![],
                                deriver: None,
                                ca: None,
                            };
                            if let Err(e) = ps.put(path_info).await {
                                warn!(store_path = %store_path, error = %e, "failed to store PathInfo during upload");
                            } else {
                                debug!(store_path = %store_path, "ingested NAR into castore during upload");
                            }
                        }
                    }
                    Err(e) => {
                        warn!(store_path = %store_path, error = %e, "failed to ingest NAR into castore");
                    }
                }
            }

            // Upload to blob store
            let blob_hash = match blob_store.add_bytes(&nar_data).await {
                Ok(result) => {
                    let blob_hash = result.blob_ref.hash.to_string();
                    info!(
                        cluster_id = %self.config.cluster_id,
                        node_id = self.config.node_id,
                        store_path = %store_path,
                        blob_hash = %blob_hash,
                        nar_size = nar_size,
                        nar_hash = %nar_hash,
                        "Store path uploaded as NAR"
                    );
                    blob_hash
                }
                Err(e) => {
                    warn!(
                        store_path = %store_path,
                        error = %e,
                        "Failed to upload NAR to blob store"
                    );
                    continue;
                }
            };

            // Register in cache index if configured
            let cache_registered = if let Some(ref cache_index) = self.config.cache_index {
                match self.register_in_cache(cache_index, store_path, &blob_hash, nar_size, &nar_hash, ci_job_id).await
                {
                    Ok(()) => {
                        info!(
                            store_path = %store_path,
                            "Store path registered in cache"
                        );
                        true
                    }
                    Err(e) => {
                        warn!(
                            store_path = %store_path,
                            error = %e,
                            "Failed to register store path in cache"
                        );
                        false
                    }
                }
            } else {
                false
            };

            uploaded.push(UploadedStorePath {
                store_path: store_path.clone(),
                blob_hash,
                nar_size,
                nar_hash,
                cache_registered,
            });
        }

        uploaded
    }

    /// Register a store path in the cache index.
    ///
    /// Queries `nix path-info --json` to get references and deriver, then
    /// creates a cache entry and stores it.
    pub(crate) async fn register_in_cache(
        &self,
        cache_index: &Arc<dyn CacheIndex>,
        store_path: &str,
        blob_hash: &str,
        nar_size: u64,
        nar_hash: &str,
        ci_job_id: Option<&str>,
    ) -> Result<()> {
        // Parse store path to extract hash
        let (store_hash, _name) =
            aspen_cache::parse_store_path(store_path).map_err(|e| CiCoreError::ArtifactStorage {
                reason: format!("Invalid store path: {e}"),
            })?;

        // Query nix path-info for references and deriver
        let path_info = self.query_path_info(store_path).await;

        let created_at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

        // Create cache entry
        let mut entry = CacheEntry::new(
            store_path.to_string(),
            store_hash,
            blob_hash.to_string(),
            nar_size,
            nar_hash.to_string(),
            created_at,
            self.config.node_id,
        );

        // Add references and deriver if available
        if let Some(info) = path_info {
            entry = entry.with_references(info.references).map_err(|e| CiCoreError::ArtifactStorage {
                reason: format!("Failed to set references: {e}"),
            })?;
            entry = entry.with_deriver(info.deriver).map_err(|e| CiCoreError::ArtifactStorage {
                reason: format!("Failed to set deriver: {e}"),
            })?;
        }

        // Add CI metadata if available
        if let Some(job_id) = ci_job_id {
            entry = entry.with_ci_metadata(Some(job_id.to_string()), None);
        }

        // Store in cache
        cache_index.put(entry).await.map_err(|e| CiCoreError::ArtifactStorage {
            reason: format!("Failed to store cache entry: {e}"),
        })?;

        Ok(())
    }

    /// Query path-info for a store path to get references and deriver.
    ///
    /// Queries PathInfoService first (zero subprocess). Falls back to
    /// `nix path-info --json` when `nix-cli-fallback` is enabled.
    pub(crate) async fn query_path_info(&self, store_path: &str) -> Option<PathInfo> {
        // Primary: PathInfoService lookup (zero subprocess)
        #[cfg(feature = "snix")]
        if let Some(ref ps) = self.config.snix_pathinfo_service
            && let Some(meta) = pathinfo_lookup(ps.as_ref(), store_path).await
        {
            return Some(PathInfo {
                references: meta.references,
                deriver: meta.deriver,
            });
        }

        // Fallback: nix path-info subprocess
        #[cfg(feature = "nix-cli-fallback")]
        {
            return self.query_path_info_subprocess(store_path).await;
        }

        #[cfg(not(feature = "nix-cli-fallback"))]
        {
            debug!(store_path, "path not in PathInfoService and nix-cli-fallback disabled");
            None
        }
    }

    /// Subprocess fallback for `query_path_info` via `nix path-info --json`.
    #[cfg(feature = "nix-cli-fallback")]
    async fn query_path_info_subprocess(&self, store_path: &str) -> Option<PathInfo> {
        let output = Command::new(&self.config.nix_binary)
            .args(["path-info", "--json", store_path])
            .output()
            .await
            .inspect_err(|e| debug!(store_path, "nix path-info command failed: {e}"))
            .ok()?;

        if !output.status.success() {
            debug!(
                store_path = %store_path,
                stderr = %String::from_utf8_lossy(&output.stderr),
                "nix path-info failed"
            );
            return None;
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let parsed: serde_json::Value = serde_json::from_str(&json_str).ok()?;
        let entry = path_info_entry(&parsed, store_path)?;

        let references: Vec<String> = entry
            .get("references")?
            .as_array()?
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        let deriver = entry.get("deriver").and_then(|v| v.as_str()).map(|s| s.to_string());

        Some(PathInfo { references, deriver })
    }
}

/// Extract the path-info entry from `nix path-info --json` output.
///
/// Newer nix versions return an object keyed by store path:
///   `{"/nix/store/abc-foo": {"narSize": 123, ...}}`
/// Older versions return an array:
///   `[{"narSize": 123, ...}]`
#[cfg(feature = "nix-cli-fallback")]
fn path_info_entry<'a>(parsed: &'a serde_json::Value, store_path: &str) -> Option<&'a serde_json::Value> {
    // Try object format first (newer nix): keyed by store path
    if let Some(obj) = parsed.as_object() {
        if let Some(entry) = obj.get(store_path) {
            return Some(entry);
        }
        // Fall back to first value if store path key doesn't match exactly
        return obj.values().next();
    }
    // Try array format (older nix)
    parsed.as_array()?.first()
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[cfg(feature = "nix-cli-fallback")]
    #[test]
    fn path_info_entry_object_format() {
        let json: serde_json::Value =
            serde_json::from_str(r#"{"/nix/store/abc-foo": {"narSize": 123, "references": ["/nix/store/xyz-bar"]}}"#)
                .unwrap();
        let entry = path_info_entry(&json, "/nix/store/abc-foo").unwrap();
        assert_eq!(entry.get("narSize").unwrap().as_u64().unwrap(), 123);
        assert_eq!(entry.get("references").unwrap().as_array().unwrap().len(), 1);
    }

    #[cfg(feature = "nix-cli-fallback")]
    #[test]
    fn path_info_entry_array_format() {
        let json: serde_json::Value = serde_json::from_str(r#"[{"narSize": 456, "references": []}]"#).unwrap();
        let entry = path_info_entry(&json, "/nix/store/anything").unwrap();
        assert_eq!(entry.get("narSize").unwrap().as_u64().unwrap(), 456);
    }

    #[cfg(feature = "nix-cli-fallback")]
    #[test]
    fn path_info_entry_object_fallback_to_first_value() {
        let json: serde_json::Value =
            serde_json::from_str(r#"{"/nix/store/different-path": {"narSize": 789}}"#).unwrap();
        // Store path doesn't match key, but we still get the first value
        let entry = path_info_entry(&json, "/nix/store/other").unwrap();
        assert_eq!(entry.get("narSize").unwrap().as_u64().unwrap(), 789);
    }

    #[cfg(feature = "nix-cli-fallback")]
    #[test]
    fn path_info_entry_null_returns_none() {
        let json = serde_json::Value::Null;
        assert!(path_info_entry(&json, "/nix/store/foo").is_none());
    }

    // ================================================================
    // pathinfo_lookup tests (snix feature)
    // ================================================================

    #[cfg(feature = "snix")]
    mod pathinfo_lookup_tests {
        use std::num::NonZeroUsize;
        use std::sync::Arc;

        use nix_compat::store_path::StorePath;
        use snix_castore::Node;
        use snix_castore::fixtures::DUMMY_DIGEST;
        use snix_store::path_info::PathInfo as SnixPathInfo;
        use snix_store::pathinfoservice::LruPathInfoService;
        use snix_store::pathinfoservice::PathInfoService;

        use super::super::pathinfo_lookup;

        fn make_test_pathinfo_service() -> Arc<dyn PathInfoService> {
            Arc::new(LruPathInfoService::with_capacity("test".to_string(), NonZeroUsize::new(100).unwrap()))
        }

        #[tokio::test]
        async fn test_pathinfo_lookup_returns_metadata() {
            let ps = make_test_pathinfo_service();

            let sp =
                StorePath::<String>::from_absolute_path(b"/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-hello").unwrap();

            let ref_sp =
                StorePath::<String>::from_absolute_path(b"/nix/store/mp57d33657rf34lzvlbpfa1gjfv5gmpg-glibc").unwrap();

            let path_info = SnixPathInfo {
                store_path: sp.clone(),
                node: Node::File {
                    digest: DUMMY_DIGEST.clone(),
                    size: 42,
                    executable: false,
                },
                references: vec![ref_sp.clone()],
                nar_size: 1234,
                nar_sha256: [0xAB; 32],
                signatures: vec![],
                deriver: None,
                ca: None,
            };

            ps.put(path_info).await.unwrap();

            let meta = pathinfo_lookup(ps.as_ref(), "/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-hello")
                .await
                .expect("should resolve");

            assert_eq!(meta.nar_size, 1234);
            assert_eq!(meta.nar_sha256, [0xAB; 32]);
            assert_eq!(meta.references.len(), 1);
            assert!(meta.references[0].contains("glibc"));
            assert!(meta.deriver.is_none());
        }

        #[tokio::test]
        async fn test_pathinfo_lookup_missing_path() {
            let ps = make_test_pathinfo_service();
            let result = pathinfo_lookup(ps.as_ref(), "/nix/store/00bgd045z0d4icpbc2yyz4gx48ak44la-missing").await;
            assert!(result.is_none());
        }

        #[tokio::test]
        async fn test_pathinfo_lookup_invalid_path() {
            let ps = make_test_pathinfo_service();
            let result = pathinfo_lookup(ps.as_ref(), "not-a-store-path").await;
            assert!(result.is_none());
        }
    }
}
