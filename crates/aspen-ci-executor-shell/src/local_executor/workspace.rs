//! Workspace setup and cleanup for job execution.

use std::path::PathBuf;

use tracing::debug;
use tracing::info;
use tracing::warn;

use super::LocalExecutorPayload;
use super::LocalExecutorWorker;
use super::nix::copy_directory_contents;
use super::nix::prefetch_and_rewrite_flake_lock;
use super::nix::rewrite_flake_lock_with_store_paths;
use crate::agent::protocol::LogMessage;
use crate::common::seed_workspace_from_blob_for_job;

const MAX_WORKSPACE_ROOT_ENTRIES: usize = 16;
const SOURCE_MATERIALIZATION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);
const CI_PROGRESS_MARKER: &str = "ASPEN_CI_COMMAND_PROGRESS";

fn workspace_setup_marker(job_id: &str, phase: &'static str) -> String {
    format!("{CI_PROGRESS_MARKER} phase={phase} job_id={job_id}")
}

fn workspace_setup_marker_with_bool(job_id: &str, phase: &'static str, key: &str, value: bool) -> String {
    format!("{CI_PROGRESS_MARKER} phase={phase} job_id={job_id} {key}={value}")
}

fn workspace_setup_marker_with_reason(job_id: &str, phase: &'static str, reason: &'static str) -> String {
    format!("{CI_PROGRESS_MARKER} phase={phase} job_id={job_id} reason={reason}")
}

fn classify_workspace_seed_error(error: &str) -> &'static str {
    if error.contains("timed out") {
        "timeout"
    } else if error.contains("no blob store") {
        "missing_blob_store"
    } else if error.contains("not found") {
        "blob_missing"
    } else if error.contains("download") {
        "blob_download"
    } else if error.contains("extract") || error.contains("tar") || error.contains("archive") {
        "archive_unpack"
    } else {
        "workspace_seed"
    }
}

impl LocalExecutorWorker {
    fn try_emit_workspace_phase(&self, job_id: &str, marker: String) {
        if let Some(sink) = self.log_sink.as_ref() {
            let _ = sink.try_send(LogMessage::Stderr(format!("{marker}\n")));
        }
        info!(job_id = %job_id, marker = %marker, "local executor workspace phase");
    }

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
        self.try_emit_workspace_phase(job_id, workspace_setup_marker(job_id, "workspace_setup_enter"));
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
            self.try_emit_workspace_phase(job_id, workspace_setup_marker(job_id, "workspace_materialization_enter"));
            match self.seed_workspace_from_source(job_id, source_hash, &job_workspace).await {
                Ok(()) => {
                    self.try_emit_workspace_phase(
                        job_id,
                        workspace_setup_marker(job_id, "workspace_materialization_done"),
                    );
                }
                Err(e) => {
                    let reason = classify_workspace_seed_error(&e);
                    self.try_emit_workspace_phase(
                        job_id,
                        workspace_setup_marker_with_reason(job_id, "workspace_materialization_failed", reason),
                    );
                    return Err(e);
                }
            }
            self.require_workspace_flake_for_nix(job_id, payload, &job_workspace).await?;
            self.rewrite_prefetched_flake_inputs(job_id, payload, &job_workspace)?;

            // Virtiofs mounts have I/O issues that cause nix to fail when importing
            // the workspace to the nix store. If the workspace is on virtiofs (i.e.,
            // under /workspace), copy the seeded files to a tmpfs-backed directory
            // so nix can read them reliably.
            if job_workspace.starts_with("/workspace") {
                let tmpfs_workspace = PathBuf::from(format!("/tmp/ci-workspace-{}", job_id));
                match Self::copy_to_tmpfs(job_id, &job_workspace, &tmpfs_workspace).await {
                    Ok(()) => {
                        info!(
                            job_id = %job_id,
                            from = %job_workspace.display(),
                            to = %tmpfs_workspace.display(),
                            "copied workspace from virtiofs to tmpfs for nix compatibility"
                        );
                        return Ok((tmpfs_workspace, flake_store_path));
                    }
                    Err(e) => {
                        warn!(
                            job_id = %job_id,
                            error = %e,
                            "failed to copy workspace to tmpfs, using virtiofs directly"
                        );
                    }
                }
            }
        }

        self.try_emit_workspace_phase(job_id, workspace_setup_marker(job_id, "workspace_setup_done"));
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

    /// Copy workspace from virtiofs to a tmpfs-backed directory.
    ///
    /// Nix's file import mechanism fails on virtiofs mounts (files disappear
    /// or return I/O errors when copied to the nix store). Moving the workspace
    /// to tmpfs (/tmp) works around this by ensuring all reads come from a
    /// standard filesystem.
    async fn copy_to_tmpfs(
        job_id: &str,
        src: &std::path::Path,
        dst: &std::path::Path,
    ) -> std::result::Result<(), String> {
        // Use cp -a for a reliable deep copy
        let status = tokio::process::Command::new("cp")
            .args(["-a", &src.display().to_string(), &dst.display().to_string()])
            .status()
            .await
            .map_err(|e| format!("failed to spawn cp: {}", e))?;

        if !status.success() {
            return Err(format!("cp exited with status: {}", status));
        }

        debug!(job_id = %job_id, "workspace copied to tmpfs");
        Ok(())
    }

    /// Seed workspace from blob store if available.
    async fn seed_workspace_from_source(
        &self,
        job_id: &str,
        source_hash: &str,
        job_workspace: &std::path::Path,
    ) -> Result<(), String> {
        let Some(ref blob_store) = self.blob_store else {
            let message = format!("source hash {source_hash} provided but no blob store is configured");
            warn!(job_id = %job_id, source_hash = %source_hash, "workspace seeding failed: no blob store configured");
            return Err(message);
        };

        match tokio::time::timeout(
            SOURCE_MATERIALIZATION_TIMEOUT,
            seed_workspace_from_blob_for_job(blob_store, source_hash, job_workspace, job_id),
        )
        .await
        {
            Ok(Ok(bytes)) => {
                info!(job_id = %job_id, source_hash = %source_hash, bytes = bytes, "workspace seeded");
                Ok(())
            }
            Ok(Err(e)) => {
                warn!(job_id = %job_id, source_hash = %source_hash, error = ?e, "workspace seeding failed");
                Err(format!("workspace seeding failed for source hash {source_hash}: {e}"))
            }
            Err(_) => {
                let timeout_secs = SOURCE_MATERIALIZATION_TIMEOUT.as_secs();
                warn!(
                    job_id = %job_id,
                    source_hash = %source_hash,
                    timeout_secs,
                    marker = %format!(
                        "{CI_PROGRESS_MARKER} phase=workspace_materialization_timeout job_id={job_id} timeout_secs={timeout_secs}"
                    ),
                    "workspace source materialization timed out"
                );
                Err(format!(
                    "workspace source materialization timed out after {timeout_secs} seconds for source hash {source_hash}"
                ))
            }
        }
    }

    async fn require_workspace_flake_for_nix(
        &self,
        job_id: &str,
        payload: &LocalExecutorPayload,
        job_workspace: &std::path::Path,
    ) -> Result<(), String> {
        if payload.command != "nix" {
            return Ok(());
        }
        let flake_path = job_workspace.join("flake.nix");
        info!(
            job_id = %job_id,
            marker = %workspace_setup_marker(job_id, "workspace_preflight_enter"),
            "workspace materialization preflight starting"
        );
        let root_flake_present = flake_path.exists();
        let root_entries = bounded_workspace_root_entries(job_workspace).await;
        info!(
            job_id = %job_id,
            workspace = %job_workspace.display(),
            source_hash_present = payload.source_hash.is_some(),
            materialization_attempted = payload.source_hash.is_some(),
            root_flake_present,
            root_entries = ?root_entries,
            marker = %workspace_setup_marker_with_bool(
                job_id,
                "workspace_preflight_done",
                "had_flake_nix",
                root_flake_present,
            ),
            "workspace materialization preflight"
        );

        if root_flake_present {
            return Ok(());
        }

        Err(format!(
            "workspace source materialization failed: root flake.nix missing after seeding workspace {}; root_entries={:?}",
            job_workspace.display(),
            root_entries
        ))
    }

    fn rewrite_prefetched_flake_inputs(
        &self,
        job_id: &str,
        payload: &LocalExecutorPayload,
        job_workspace: &std::path::Path,
    ) -> Result<(), String> {
        if payload.command != "nix" || payload.flake_input_paths.is_empty() {
            return Ok(());
        }

        rewrite_flake_lock_with_store_paths(job_workspace, &payload.flake_input_paths).map_err(|e| {
            format!("failed to rewrite flake.lock with {} prefetched input paths: {e}", payload.flake_input_paths.len())
        })?;
        info!(
            job_id = %job_id,
            inputs = payload.flake_input_paths.len(),
            "rewrote flake.lock to host-prefetched store paths"
        );
        Ok(())
    }

    /// Clean up the job workspace if configured.
    pub(super) async fn cleanup_workspace(&self, job_id: &str, job_workspace: &std::path::Path) {
        if !self.config.should_cleanup_workspaces {
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

async fn bounded_workspace_root_entries(path: &std::path::Path) -> Vec<String> {
    let Ok(mut entries) = tokio::fs::read_dir(path).await else {
        return Vec::new();
    };
    let mut names = Vec::new();
    while names.len() < MAX_WORKSPACE_ROOT_ENTRIES {
        let Ok(Some(entry)) = entries.next_entry().await else {
            break;
        };
        names.push(entry.file_name().to_string_lossy().into_owned());
    }
    names.sort();
    names
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use aspen_blob::InMemoryBlobStore;
    use aspen_blob::prelude::BlobStore;

    use super::*;
    use crate::common::create_source_archive;
    use crate::local_executor::LocalExecutorWorkerConfig;

    fn nix_payload(source_hash: Option<String>) -> LocalExecutorPayload {
        LocalExecutorPayload {
            job_name: Some("workspace-test".to_string()),
            command: "nix".to_string(),
            args: vec!["build".to_string(), ".#default".to_string()],
            working_dir: ".".to_string(),
            env: HashMap::new(),
            timeout_secs: 60,
            artifacts: vec![],
            source_hash,
            checkout_dir: None,
            flake_attr: None,
            flake_input_paths: std::collections::BTreeMap::new(),
            run_id: None,
            cached_execution: false,
        }
    }

    #[tokio::test]
    async fn source_hash_without_blob_store_fails_workspace_setup() {
        let workspace_root = tempfile::tempdir().expect("workspace root");
        let config = LocalExecutorWorkerConfig {
            workspace_dir: workspace_root.path().to_path_buf(),
            ..LocalExecutorWorkerConfig::default()
        };
        let worker = LocalExecutorWorker::new(config);
        let payload = nix_payload(Some("0".repeat(64)));

        let error = worker
            .setup_job_workspace("job-no-blob", &payload)
            .await
            .expect_err("source_hash without blob store must fail before command execution");

        assert!(error.contains("no blob store is configured"));
    }

    #[test]
    fn workspace_timeout_marker_is_bounded_and_redacted() {
        let job_id = "job-timeout";
        let timeout_secs = SOURCE_MATERIALIZATION_TIMEOUT.as_secs();
        let marker = format!(
            "{CI_PROGRESS_MARKER} phase=workspace_materialization_timeout job_id={job_id} timeout_secs={timeout_secs}"
        );

        assert_eq!(
            marker,
            "ASPEN_CI_COMMAND_PROGRESS phase=workspace_materialization_timeout job_id=job-timeout timeout_secs=120"
        );
        assert!(!marker.contains("source_hash"));
        assert!(!marker.contains("ticket"));
        assert!(!marker.contains("password"));
        assert!(!marker.contains("/tmp/"));
    }

    #[tokio::test]
    async fn seeded_nix_workspace_without_root_flake_fails_preflight() {
        let source_root = tempfile::tempdir().expect("source root");
        tokio::fs::write(source_root.path().join("README.md"), "not a flake").await.expect("source fixture");
        let blob_store: Arc<dyn BlobStore> = Arc::new(InMemoryBlobStore::new());
        let source_hash = create_source_archive(source_root.path(), &blob_store).await.expect("source archive");

        let workspace_root = tempfile::tempdir().expect("workspace root");
        let config = LocalExecutorWorkerConfig {
            workspace_dir: workspace_root.path().to_path_buf(),
            ..LocalExecutorWorkerConfig::default()
        };
        let worker = LocalExecutorWorker::with_blob_store(config, blob_store);
        let payload = nix_payload(Some(source_hash));

        let error = worker
            .setup_job_workspace("job-no-flake", &payload)
            .await
            .expect_err("missing root flake must fail before nix build");

        assert!(error.contains("workspace source materialization failed"));
        assert!(error.contains("root flake.nix missing"));
        assert!(error.contains("README.md"));
    }
}
