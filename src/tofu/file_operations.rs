//! File operations for tofu execution
//!
//! This module handles file copying and reading for tofu operations,
//! with security validation to prevent path traversal attacks.

use anyhow::{Result, Context};
use std::path::{Path, PathBuf};

use crate::tofu::validation;

/// Handles file operations with security validation
pub struct FileOperations {
    work_dir: PathBuf,
}

impl FileOperations {
    /// Create a new file operations handler
    pub fn new(work_dir: PathBuf) -> Self {
        Self { work_dir }
    }

    /// Copy configuration files to work directory
    ///
    /// Only copies .tf and .tfvars files, skips symlinks for security.
    pub async fn copy_config_to_work_dir(&self, src: &Path, dst: &Path) -> Result<()> {
        // Validate source and destination paths before copying
        validation::validate_source_path(&self.work_dir, src, dst).await
            .context("Path validation failed for config copy")?;

        tracing::debug!(
            src = ?src,
            dst = ?dst,
            "Copying configuration files to work directory"
        );

        let mut entries = tokio::fs::read_dir(src).await
            .context("Failed to read source directory")?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let file_name = entry.file_name();
            let dst_path = dst.join(&file_name);

            // Validate that the destination path is still within work directory
            // This prevents symlink attacks where a symlink could point outside the work dir
            validation::validate_source_path(&self.work_dir, &path, &dst_path).await
                .context("Path validation failed during recursive copy")?;

            // Get metadata without following symlinks to detect symlink attacks
            let metadata = tokio::fs::symlink_metadata(&path).await?;

            if metadata.is_symlink() {
                // Log and skip symlinks to prevent path traversal attacks
                tracing::warn!(
                    path = ?path,
                    "Skipping symlink in configuration directory (security policy)"
                );
                continue;
            }

            if metadata.is_dir() {
                tokio::fs::create_dir_all(&dst_path).await?;
                Box::pin(self.copy_config_to_work_dir(&path, &dst_path)).await?;
            } else if metadata.is_file() {
                // Only copy Terraform files
                if path.extension().map_or(false, |ext| ext == "tf" || ext == "tfvars") {
                    tokio::fs::copy(&path, &dst_path).await
                        .context("Failed to copy configuration file")?;

                    tracing::debug!(
                        src = ?path,
                        dst = ?dst_path,
                        "Copied configuration file"
                    );
                }
            }
        }

        Ok(())
    }

    /// Read the plan file from work directory
    pub async fn read_plan_file(&self, work_dir: &Path) -> Result<Vec<u8>> {
        let plan_file = work_dir.join("tfplan");
        tokio::fs::read(&plan_file).await.map_err(Into::into)
    }
}
