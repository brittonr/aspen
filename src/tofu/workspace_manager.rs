//! Workspace lifecycle management
//!
//! This module handles workspace operations including directory creation,
//! workspace selection, and cleanup.

use anyhow::{Result, bail};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

use crate::tofu::executor::TofuExecutor;

/// Manages workspace lifecycle and work directories
pub struct WorkspaceManager {
    executor: Arc<dyn TofuExecutor>,
    work_dir: PathBuf,
}

impl WorkspaceManager {
    /// Create a new workspace manager
    pub fn new(executor: Arc<dyn TofuExecutor>, work_dir: PathBuf) -> Self {
        Self { executor, work_dir }
    }

    /// Prepare a unique work directory for execution
    pub async fn prepare_work_directory(&self) -> Result<PathBuf> {
        let execution_id = Uuid::new_v4().to_string();
        let work_dir = self.work_dir.join(&execution_id);
        tokio::fs::create_dir_all(&work_dir).await?;
        Ok(work_dir)
    }

    /// Clean up work directory
    pub async fn cleanup_work_directory(&self, work_dir: &Path) {
        if let Err(e) = tokio::fs::remove_dir_all(work_dir).await {
            tracing::warn!(error = %e, work_dir = %work_dir.display(), "Failed to clean up work directory");
        }
    }

    /// Initialize OpenTofu in the work directory
    pub async fn run_tofu_init(&self, work_dir: &Path, workspace: &str) -> Result<()> {
        let init_output = self.executor.init(work_dir).await?;

        if !init_output.success {
            tracing::error!(workspace = workspace, error = %init_output.stderr, "OpenTofu init failed");
            bail!("OpenTofu init failed: {}", init_output.stderr);
        }

        Ok(())
    }

    /// Ensure workspace is selected, creating it if necessary
    pub async fn ensure_workspace_selected(&self, work_dir: &Path, workspace: &str) -> Result<()> {
        // Try to create workspace (ok if it fails - workspace may already exist)
        if let Err(e) = self.executor.workspace_new(work_dir, workspace).await {
            tracing::warn!(error = %e, workspace = workspace, "Failed to create workspace (may already exist)");
        }

        let select_output = self.executor.workspace_select(work_dir, workspace).await?;

        if !select_output.success {
            tracing::warn!(
                workspace = workspace,
                error = %select_output.stderr,
                "Workspace selection failed (may be expected if workspace doesn't exist)"
            );
        }

        Ok(())
    }

    /// Select workspace for tofu operation
    pub async fn select_workspace(&self, work_dir: &Path, workspace: &str) -> Result<()> {
        let select_output = self.executor.workspace_select(work_dir, workspace).await?;

        if !select_output.success {
            tracing::warn!(
                workspace = workspace,
                error = %select_output.stderr,
                "Workspace selection failed"
            );
        }

        Ok(())
    }

    /// Select workspace strictly (fail if workspace doesn't exist)
    pub async fn select_workspace_strict(&self, work_dir: &Path, workspace: &str) -> Result<()> {
        let select_output = self.executor.workspace_select(work_dir, workspace).await?;

        if !select_output.success {
            tracing::error!(error = %select_output.stderr, workspace = workspace, "Failed to select workspace");
            bail!("Failed to select workspace: {}", select_output.stderr);
        }

        Ok(())
    }
}
