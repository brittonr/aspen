//! Cache registration and NAR upload to blob store.

use std::sync::Arc;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use aspen_cache::CacheEntry;
use aspen_cache::CacheIndex;
use aspen_ci_core::CiCoreError;
use aspen_ci_core::Result;
use serde::Serialize;
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
    /// Check the NAR size of a store path using `nix path-info --json`.
    ///
    /// Returns `None` if the command fails or the output can't be parsed.
    pub(crate) async fn check_store_path_size(&self, store_path: &str) -> Option<u64> {
        let output = Command::new(&self.config.nix_binary)
            .args(["path-info", "--json", store_path])
            .output()
            .await
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let parsed: serde_json::Value = serde_json::from_str(&json_str).ok()?;
        let entry = parsed.as_array()?.first()?;
        entry.get("narSize")?.as_u64()
    }

    /// Upload store paths to the blob store as NAR archives.
    ///
    /// Uses `nix nar dump-path` to create a NAR archive of each store path,
    /// then uploads the archive to the blob store. If a cache index is configured,
    /// also registers the store path in the distributed Nix binary cache.
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

            // Use nix nar dump-path to create a NAR archive
            let output =
                match Command::new(&self.config.nix_binary).args(["nar", "dump-path", store_path]).output().await {
                    Ok(output) => output,
                    Err(e) => {
                        warn!(store_path = %store_path, error = %e, "Failed to create NAR archive");
                        continue;
                    }
                };

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!(
                    store_path = %store_path,
                    stderr = %stderr,
                    "nix nar dump-path failed"
                );
                continue;
            }

            let nar_data = output.stdout;
            let nar_size = nar_data.len() as u64;

            // Compute SHA256 hash of NAR (Nix's native format)
            use sha2::Digest;
            use sha2::Sha256;
            let nar_hash = {
                let mut hasher = Sha256::new();
                hasher.update(&nar_data);
                let hash = hasher.finalize();
                format!("sha256:{}", hex::encode(hash))
            };

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

    /// Query nix path-info for a store path to get references and deriver.
    pub(crate) async fn query_path_info(&self, store_path: &str) -> Option<PathInfo> {
        let output = Command::new(&self.config.nix_binary)
            .args(["path-info", "--json", store_path])
            .output()
            .await
            .ok()?;

        if !output.status.success() {
            debug!(
                store_path = %store_path,
                stderr = %String::from_utf8_lossy(&output.stderr),
                "nix path-info failed"
            );
            return None;
        }

        // Parse JSON output - nix path-info --json returns an array
        let json_str = String::from_utf8_lossy(&output.stdout);
        let parsed: serde_json::Value = serde_json::from_str(&json_str).ok()?;

        // Get the first (and typically only) entry
        let entry = parsed.as_array()?.first()?;

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
