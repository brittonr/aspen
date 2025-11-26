//! Security validation for OpenTofu/Terraform operations
//!
//! This module provides validation functions to prevent command injection
//! and path traversal attacks.

use anyhow::{Result, Context, bail};
use std::path::{Path, PathBuf};

/// Validate workspace name to prevent command injection
///
/// Workspace names must be alphanumeric with optional hyphens, underscores, and dots.
/// This prevents shell metacharacters and command injection attacks.
pub fn validate_workspace_name(workspace: &str) -> Result<()> {
    // Check length constraints
    if workspace.is_empty() {
        bail!("Workspace name cannot be empty");
    }
    if workspace.len() > 90 {
        bail!("Workspace name exceeds maximum length of 90 characters");
    }

    // Check for valid characters: alphanumeric, hyphen, underscore, and dot only
    // This prevents command injection via shell metacharacters
    if !workspace.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.') {
        bail!("Workspace name contains invalid characters. Only alphanumeric, hyphen, underscore, and dot are allowed");
    }

    // Prevent path traversal in workspace names
    if workspace.contains("..") || workspace.starts_with('/') || workspace.starts_with('\\') {
        bail!("Workspace name cannot contain path traversal sequences");
    }

    // Log security-relevant validation
    tracing::debug!(
        workspace = workspace,
        "Validated workspace name"
    );

    Ok(())
}

/// Validate and canonicalize a path to ensure it's within the allowed work directory
///
/// This prevents path traversal attacks by ensuring the path doesn't escape
/// the work directory, even through symlinks.
pub async fn validate_path_in_work_dir(work_dir: &Path, path: &Path) -> Result<PathBuf> {
    // Canonicalize both paths to resolve symlinks and relative components
    let canonical_path = tokio::fs::canonicalize(path)
        .await
        .context("Failed to canonicalize path")?;

    let canonical_work_dir = tokio::fs::canonicalize(work_dir)
        .await
        .context("Failed to canonicalize work directory")?;

    // Ensure the canonical path is within the work directory
    if !canonical_path.starts_with(&canonical_work_dir) {
        tracing::warn!(
            path = ?path,
            canonical_path = ?canonical_path,
            work_dir = ?canonical_work_dir,
            "Path traversal attempt detected"
        );
        bail!("Path traversal detected: path escapes work directory");
    }

    tracing::debug!(
        original_path = ?path,
        canonical_path = ?canonical_path,
        "Validated path within work directory"
    );

    Ok(canonical_path)
}

/// Validate a source path before copying to prevent path traversal
pub async fn validate_source_path(work_dir: &Path, src: &Path, dst: &Path) -> Result<()> {
    // Ensure source exists
    if !tokio::fs::try_exists(src).await? {
        bail!("Source path does not exist: {}", src.display());
    }

    // Get canonical paths
    let canonical_src = tokio::fs::canonicalize(src)
        .await
        .context("Failed to canonicalize source path")?;

    let canonical_dst = if tokio::fs::try_exists(dst).await? {
        tokio::fs::canonicalize(dst)
            .await
            .context("Failed to canonicalize destination path")?
    } else {
        // For non-existent destinations, canonicalize parent and append filename
        let parent = dst.parent().context("Destination has no parent directory")?;
        let canonical_parent = tokio::fs::canonicalize(parent)
            .await
            .context("Failed to canonicalize destination parent")?;
        canonical_parent.join(dst.file_name().context("Destination has no filename")?)
    };

    // Ensure destination is within work directory
    let canonical_work_dir = tokio::fs::canonicalize(work_dir)
        .await
        .context("Failed to canonicalize work directory")?;

    if !canonical_dst.starts_with(&canonical_work_dir) {
        tracing::warn!(
            src = ?src,
            dst = ?dst,
            canonical_dst = ?canonical_dst,
            work_dir = ?canonical_work_dir,
            "Path traversal attempt detected during file copy"
        );
        bail!("Path traversal detected: destination escapes work directory");
    }

    tracing::debug!(
        src = ?canonical_src,
        dst = ?canonical_dst,
        "Validated source and destination paths"
    );

    Ok(())
}
