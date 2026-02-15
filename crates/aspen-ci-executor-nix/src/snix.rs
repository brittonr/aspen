//! SNIX storage integration for decomposed content-addressed store paths.

use std::io::Cursor;
use std::sync::Arc;

use nix_compat::store_path::StorePath as SnixStorePath;
use serde::Serialize;
use snix_store::nar::ingest_nar_and_hash;
use snix_store::pathinfoservice::PathInfo as SnixPathInfo;
use tokio::process::Command;
use tracing::info;
use tracing::warn;

use crate::executor::NixBuildWorker;

/// Information about an uploaded store path to SNIX storage.
#[derive(Debug, Clone, Serialize)]
pub struct UploadedStorePathSnix {
    /// The Nix store path.
    pub store_path: String,
    /// Size of the NAR archive in bytes.
    pub nar_size: u64,
    /// SHA256 hash of the NAR archive (Nix's native format).
    pub nar_sha256: String,
    /// Number of references this store path has.
    pub references_count: usize,
    /// Whether this store path has a deriver.
    pub has_deriver: bool,
}

impl NixBuildWorker {
    /// Upload store paths to SNIX storage as NAR archives.
    ///
    /// Uses `nix nar dump-path` to create a NAR archive of each store path,
    /// then ingests directly into SNIX storage using `ingest_nar_and_hash`.
    /// Creates PathInfo entries with proper metadata.
    pub(crate) async fn upload_store_paths_snix(&self, output_paths: &[String]) -> Vec<UploadedStorePathSnix> {
        let mut uploaded = Vec::new();

        // Check if all SNIX services are configured
        let (blob_service, directory_service, pathinfo_service) = match (
            &self.config.snix_blob_service,
            &self.config.snix_directory_service,
            &self.config.snix_pathinfo_service,
        ) {
            (Some(bs), Some(ds), Some(ps)) => (bs, ds, ps),
            _ => {
                tracing::debug!("SNIX services not fully configured, skipping SNIX upload");
                return uploaded;
            }
        };

        for store_path in output_paths {
            info!(
                cluster_id = %self.config.cluster_id,
                node_id = self.config.node_id,
                store_path = %store_path,
                "Uploading store path to SNIX storage"
            );

            // Use nix nar dump-path to create a NAR archive
            let output =
                match Command::new(&self.config.nix_binary).args(["nar", "dump-path", store_path]).output().await {
                    Ok(output) => output,
                    Err(e) => {
                        warn!(store_path = %store_path, error = %e, "Failed to create NAR archive for SNIX");
                        continue;
                    }
                };

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!(
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
                        cluster_id = %self.config.cluster_id,
                        node_id = self.config.node_id,
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
                        store_path = %store_path,
                        error = %e,
                        "Failed to store PathInfo in SNIX"
                    );
                }
            }
        }

        uploaded
    }
}
