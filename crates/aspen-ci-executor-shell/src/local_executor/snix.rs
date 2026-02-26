//! SNIX store path upload for Nix binary cache integration.
//!
//! This module handles uploading built Nix store paths to SNIX storage
//! as NAR archives, enabling the cluster's binary cache to serve them.

use std::io::Cursor;
use std::sync::Arc;

use nix_compat::store_path::StorePath as SnixStorePath;
use serde::Serialize;
use snix_store::nar::ingest_nar_and_hash;
use snix_store::pathinfoservice::PathInfo as SnixPathInfo;
use snix_store::pathinfoservice::PathInfoService;
use tracing::debug;
use tracing::info;
use tracing::warn;

use super::LocalExecutorWorker;

/// Information from nix path-info.
struct NixPathInfo {
    /// Store paths this entry references.
    references: Vec<String>,
    /// Deriver store path.
    deriver: Option<String>,
}

/// Information about an uploaded store path to SNIX storage.
#[derive(Debug, Clone, Serialize)]
pub(super) struct UploadedStorePathSnix {
    /// The Nix store path.
    store_path: String,
    /// Size of the NAR archive in bytes.
    nar_size: u64,
    /// SHA256 hash of the NAR archive (Nix's native format).
    nar_sha256: String,
    /// Number of references (dependencies).
    references_count: usize,
    /// Whether this path has a deriver.
    has_deriver: bool,
}

impl LocalExecutorWorker {
    /// Upload store paths to SNIX storage as NAR archives.
    ///
    /// Uses `nix nar dump-path` to create a NAR archive of each store path,
    /// then ingests directly into SNIX storage using `ingest_nar_and_hash`.
    /// Creates PathInfo entries with proper metadata.
    ///
    /// This enables the cluster's Nix binary cache to serve built paths to
    /// other jobs and developers.
    pub(super) async fn upload_store_paths_snix(
        &self,
        job_id: &str,
        output_paths: &[String],
    ) -> Vec<UploadedStorePathSnix> {
        use tokio::process::Command;

        let mut uploaded = Vec::new();

        info!(
            job_id = %job_id,
            paths_count = output_paths.len(),
            "starting SNIX upload for store paths"
        );

        // Check if all SNIX services are configured
        let has_blob = self.config.snix_blob_service.is_some();
        let has_dir = self.config.snix_directory_service.is_some();
        let has_pathinfo = self.config.snix_pathinfo_service.is_some();

        debug!(
            job_id = %job_id,
            has_blob_service = has_blob,
            has_directory_service = has_dir,
            has_pathinfo_service = has_pathinfo,
            "SNIX service configuration check"
        );

        let (blob_service, directory_service, pathinfo_service) = match (
            &self.config.snix_blob_service,
            &self.config.snix_directory_service,
            &self.config.snix_pathinfo_service,
        ) {
            (Some(bs), Some(ds), Some(ps)) => {
                info!(job_id = %job_id, "all SNIX services configured, proceeding with upload");
                (bs, ds, ps)
            }
            _ => {
                warn!(
                    job_id = %job_id,
                    has_blob_service = has_blob,
                    has_directory_service = has_dir,
                    has_pathinfo_service = has_pathinfo,
                    "SNIX services not fully configured, skipping SNIX upload"
                );
                return uploaded;
            }
        };

        for store_path in output_paths {
            info!(
                job_id = %job_id,
                store_path = %store_path,
                "Uploading store path to SNIX storage"
            );

            // Use nix nar dump-path to create a NAR archive
            let output = match Command::new("nix").args(["nar", "dump-path", store_path]).output().await {
                Ok(output) => output,
                Err(e) => {
                    warn!(job_id = %job_id, store_path = %store_path, error = %e, "Failed to create NAR archive for SNIX");
                    continue;
                }
            };

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!(
                    job_id = %job_id,
                    store_path = %store_path,
                    stderr = %stderr,
                    "nix nar dump-path failed for SNIX upload"
                );
                continue;
            }

            let nar_data = output.stdout;
            let nar_size = nar_data.len() as u64;

            // Ingest NAR into SNIX storage
            let mut nar_reader = Cursor::new(&nar_data);
            let (root_node, nar_sha256, actual_nar_size) = match ingest_nar_and_hash(
                Arc::clone(blob_service),
                Arc::clone(directory_service),
                &mut nar_reader,
                &None, // No expected CA hash
            )
            .await
            {
                Ok((node, hash, size)) => (node, hash, size),
                Err(e) => {
                    warn!(
                        job_id = %job_id,
                        store_path = %store_path,
                        error = %e,
                        "Failed to ingest NAR into SNIX storage"
                    );
                    continue;
                }
            };

            // Verify size matches
            if actual_nar_size != nar_size {
                warn!(
                    job_id = %job_id,
                    store_path = %store_path,
                    expected_size = nar_size,
                    actual_size = actual_nar_size,
                    "NAR size mismatch after SNIX ingestion"
                );
                continue;
            }

            // Parse the store path
            let store_path_parsed: SnixStorePath<String> = match SnixStorePath::from_bytes(store_path.as_bytes()) {
                Ok(sp) => sp,
                Err(e) => {
                    warn!(
                        job_id = %job_id,
                        store_path = %store_path,
                        error = %e,
                        "Failed to parse store path for SNIX"
                    );
                    continue;
                }
            };

            // Query nix path-info for references and deriver
            let path_info_extra = self.query_path_info(store_path).await;

            // Create PathInfo for SNIX
            let path_info = SnixPathInfo {
                store_path: store_path_parsed.to_owned(),
                node: root_node,
                references: path_info_extra
                    .as_ref()
                    .map(|info| {
                        info.references.iter().filter_map(|r| SnixStorePath::from_bytes(r.as_bytes()).ok()).collect()
                    })
                    .unwrap_or_default(),
                nar_size: actual_nar_size,
                nar_sha256,
                signatures: vec![], // No signatures for CI builds
                deriver: path_info_extra
                    .as_ref()
                    .and_then(|info| info.deriver.as_ref().and_then(|d| SnixStorePath::from_bytes(d.as_bytes()).ok())),
                ca: None, // No content addressing for CI builds
            };

            // Store PathInfo in SNIX
            match pathinfo_service.put(path_info).await {
                Ok(stored_path_info) => {
                    info!(
                        job_id = %job_id,
                        store_path = %store_path,
                        nar_size = actual_nar_size,
                        nar_sha256 = hex::encode(nar_sha256),
                        "Store path uploaded to SNIX storage successfully"
                    );

                    uploaded.push(UploadedStorePathSnix {
                        store_path: store_path.clone(),
                        nar_size: actual_nar_size,
                        nar_sha256: hex::encode(nar_sha256),
                        references_count: stored_path_info.references.len(),
                        has_deriver: stored_path_info.deriver.is_some(),
                    });
                }
                Err(e) => {
                    warn!(
                        job_id = %job_id,
                        store_path = %store_path,
                        error = %e,
                        "Failed to store PathInfo in SNIX"
                    );
                }
            }
        }

        uploaded
    }

    /// Query nix path-info for a store path to get references and deriver.
    async fn query_path_info(&self, store_path: &str) -> Option<NixPathInfo> {
        use tokio::process::Command;

        let output = Command::new("nix").args(["path-info", "--json", store_path]).output().await.ok()?;

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

        Some(NixPathInfo { references, deriver })
    }

    /// Parse nix build output to extract store paths.
    ///
    /// Nix build with --print-out-paths outputs one store path per line.
    /// This method extracts valid /nix/store/* paths from the output.
    pub(super) fn parse_nix_output_paths(&self, stdout: &str) -> Vec<String> {
        stdout
            .lines()
            .filter(|line| line.starts_with("/nix/store/"))
            .map(|line| line.trim().to_string())
            .collect()
    }
}
