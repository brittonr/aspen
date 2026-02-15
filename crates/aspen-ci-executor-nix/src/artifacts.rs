//! Artifact collection from Nix build outputs.

use std::path::PathBuf;

use aspen_ci_core::Result;
use tracing::debug;
use tracing::warn;

use crate::executor::NixBuildWorker;

/// A collected artifact.
#[derive(Debug, Clone)]
pub(crate) struct CollectedArtifact {
    /// Full path to the artifact.
    pub(crate) path: PathBuf,
    /// Path relative to output directory.
    pub(crate) relative_path: PathBuf,
    /// Blob hash if uploaded.
    pub(crate) blob_hash: Option<String>,
}

impl NixBuildWorker {
    /// Collect artifacts matching the specified patterns.
    pub(crate) async fn collect_artifacts(
        &self,
        output_paths: &[String],
        patterns: &[String],
    ) -> Result<Vec<CollectedArtifact>> {
        let mut artifacts = Vec::new();

        for output_path in output_paths {
            let path = PathBuf::from(output_path);

            for pattern in patterns {
                // Use glob to match files
                let glob_pattern = if pattern.starts_with('/') {
                    pattern.clone()
                } else {
                    format!("{}/{}", output_path, pattern)
                };

                match glob::glob(&glob_pattern) {
                    Ok(entries) => {
                        for entry in entries.flatten() {
                            if entry.is_file() {
                                let artifact = CollectedArtifact {
                                    path: entry.clone(),
                                    relative_path: entry.strip_prefix(&path).unwrap_or(&entry).to_path_buf(),
                                    blob_hash: None, // Will be set after upload
                                };
                                artifacts.push(artifact);
                            }
                        }
                    }
                    Err(e) => {
                        warn!(pattern = %glob_pattern, error = %e, "Failed to glob artifacts");
                    }
                }
            }
        }

        // Upload to blob store if configured
        if let Some(ref blob_store) = self.config.blob_store {
            for artifact in &mut artifacts {
                match tokio::fs::read(&artifact.path).await {
                    Ok(data) => match blob_store.add_bytes(&data).await {
                        Ok(result) => {
                            artifact.blob_hash = Some(result.blob_ref.hash.to_string());
                            debug!(
                                path = ?artifact.path,
                                hash = ?artifact.blob_hash,
                                "Uploaded artifact to blob store"
                            );
                        }
                        Err(e) => {
                            warn!(path = ?artifact.path, error = %e, "Failed to upload artifact");
                        }
                    },
                    Err(e) => {
                        warn!(path = ?artifact.path, error = %e, "Failed to read artifact");
                    }
                }
            }
        }

        Ok(artifacts)
    }
}
