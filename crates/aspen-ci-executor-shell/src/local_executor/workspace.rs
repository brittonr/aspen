//! Workspace setup and cleanup for job execution.

use std::path::PathBuf;

use tracing::info;
use tracing::warn;

use super::LocalExecutorPayload;
use super::LocalExecutorWorker;
use super::nix::copy_directory_contents;
use super::nix::prefetch_and_rewrite_flake_lock;
use crate::common::seed_workspace_from_blob;

impl LocalExecutorWorker {
    /// Set up the job workspace directory.
    ///
    /// Creates a per-job directory, copies checkout contents if provided,
    /// pre-fetches flake inputs for nix commands, and seeds from blob store.
    ///
    /// Returns the workspace path and optionally the flake source store path
    /// (if a flake.nix was found and archived successfully).
    pub(super) async fn setup_job_workspace(
        &self,
        job_id: &str,
        payload: &LocalExecutorPayload,
    ) -> Result<(PathBuf, Option<PathBuf>), String> {
        let job_workspace = self.config.workspace_dir.join(job_id);

        // Create workspace directory
        tokio::fs::create_dir_all(&job_workspace)
            .await
            .map_err(|e| format!("failed to create job workspace: {}", e))?;

        info!(job_id = %job_id, workspace = %job_workspace.display(), "created job workspace");

        // Copy checkout directory if provided and get flake store path
        let flake_store_path = if let Some(ref checkout_dir) = payload.checkout_dir {
            self.copy_checkout_to_workspace(job_id, checkout_dir, &job_workspace, payload).await
        } else {
            None
        };

        // Seed from blob store if source_hash provided
        if let Some(ref source_hash) = payload.source_hash {
            self.seed_workspace_from_source(job_id, source_hash, &job_workspace).await;
        }

        Ok((job_workspace, flake_store_path))
    }

    /// Copy checkout directory contents to workspace and pre-fetch flake inputs.
    ///
    /// Returns the flake source store path if a flake was archived successfully.
    async fn copy_checkout_to_workspace(
        &self,
        job_id: &str,
        checkout_dir: &str,
        job_workspace: &std::path::Path,
        payload: &LocalExecutorPayload,
    ) -> Option<PathBuf> {
        let checkout_path = PathBuf::from(checkout_dir);
        if !checkout_path.exists() {
            return None;
        }

        match copy_directory_contents(&checkout_path, job_workspace).await {
            Ok(count) => {
                info!(
                    job_id = %job_id,
                    checkout_dir = %checkout_dir,
                    files_copied = count,
                    workspace = %job_workspace.display(),
                    "checkout copied to workspace"
                );
            }
            Err(e) => {
                warn!(job_id = %job_id, checkout_dir = %checkout_dir, error = ?e, "failed to copy checkout");
                return None;
            }
        }

        // Pre-fetch flake inputs for nix commands and get the flake store path
        if payload.command == "nix" && job_workspace.join("flake.nix").exists() {
            match prefetch_and_rewrite_flake_lock(job_workspace).await {
                Ok(store_path) => {
                    info!(job_id = %job_id, store_path = ?store_path, "pre-fetched flake inputs");
                    return store_path;
                }
                Err(e) => {
                    warn!(job_id = %job_id, error = ?e, "failed to pre-fetch flake");
                }
            }
        }

        None
    }

    /// Seed workspace from blob store if available.
    async fn seed_workspace_from_source(&self, job_id: &str, source_hash: &str, job_workspace: &std::path::Path) {
        let Some(ref blob_store) = self.blob_store else {
            return;
        };

        match seed_workspace_from_blob(blob_store, source_hash, job_workspace).await {
            Ok(bytes) => {
                info!(job_id = %job_id, source_hash = %source_hash, bytes = bytes, "workspace seeded");
            }
            Err(e) => {
                warn!(job_id = %job_id, source_hash = %source_hash, error = ?e, "workspace seeding failed");
            }
        }
    }

    /// Clean up the job workspace if configured.
    pub(super) async fn cleanup_workspace(&self, job_id: &str, job_workspace: &std::path::Path) {
        if !self.config.cleanup_workspaces {
            return;
        }

        if let Err(e) = tokio::fs::remove_dir_all(job_workspace).await {
            warn!(
                job_id = %job_id,
                workspace = %job_workspace.display(),
                error = ?e,
                "failed to clean up job workspace"
            );
        }
    }
}
