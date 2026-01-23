//! Repository checkout for CI jobs.
//!
//! This module provides functionality to checkout a repository from Forge
//! to a local directory for CI job execution.
//!
//! # Architecture
//!
//! ```text
//! ForgeNode.git                   checkout_repository()
//! ┌─────────────┐                 ┌────────────────────┐
//! │ get_commit  │────────────────►│ 1. Get commit      │
//! │ get_tree    │                 │ 2. Get root tree   │
//! │ get_blob    │                 │ 3. Recurse entries │
//! └─────────────┘                 │ 4. Write files     │
//!                                 └────────────────────┘
//!                                          │
//!                                          ▼
//!                                 /tmp/ci-run-{id}/
//!                                 ├── Cargo.toml
//!                                 ├── src/
//!                                 │   └── main.rs
//!                                 └── ...
//! ```

use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use aspen_blob::BlobStore;
use aspen_core::KeyValueStore;
use aspen_forge::ForgeNode;
use tokio::fs;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::error::CiError;
use crate::error::Result;

// Tiger Style: Bounded resource limits
/// Maximum total checkout size (500 MB).
const MAX_CHECKOUT_SIZE_BYTES: u64 = 500 * 1024 * 1024;
/// Maximum number of files in checkout.
const MAX_CHECKOUT_FILES: u32 = 50_000;
/// Maximum file path length.
const MAX_PATH_LENGTH: usize = 4096;
/// Maximum tree recursion depth.
const MAX_TREE_DEPTH: u32 = 100;

/// Checkout a repository at a specific commit to a directory.
///
/// This function recursively extracts all files from a commit's tree
/// to the specified directory, preserving the directory structure.
///
/// # Arguments
///
/// * `forge` - ForgeNode for accessing git objects
/// * `commit_hash` - The commit to checkout
/// * `target_dir` - Directory to write files to
///
/// # Returns
///
/// The path to the checkout directory on success.
///
/// # Errors
///
/// - `CiError::ForgeOperation` if git objects cannot be read
/// - `CiError::Checkout` if checkout exceeds resource limits
pub async fn checkout_repository<B: BlobStore, K: KeyValueStore + ?Sized>(
    forge: &Arc<ForgeNode<B, K>>,
    commit_hash: &[u8; 32],
    target_dir: &Path,
) -> Result<PathBuf> {
    let blake3_hash = blake3::Hash::from_bytes(*commit_hash);

    info!(
        commit = %hex::encode(commit_hash),
        target = %target_dir.display(),
        "Starting repository checkout"
    );

    // Get the commit object
    let commit = forge.git.get_commit(&blake3_hash).await.map_err(|e| CiError::ForgeOperation {
        reason: format!("Failed to load commit for checkout: {}", e),
    })?;

    // Get the root tree
    let tree_hash = blake3::Hash::from_bytes(commit.tree);

    // Create target directory
    fs::create_dir_all(target_dir).await.map_err(|e| CiError::Checkout {
        reason: format!("Failed to create checkout directory: {}", e),
    })?;

    // Track checkout statistics
    let mut stats = CheckoutStats::default();

    // Recursively checkout the tree
    checkout_tree(forge, &tree_hash, target_dir, "", &mut stats, 0).await?;

    info!(
        commit = %hex::encode(commit_hash),
        files = stats.files_written,
        bytes = stats.bytes_written,
        "Repository checkout complete"
    );

    Ok(target_dir.to_path_buf())
}

/// Statistics tracked during checkout.
#[derive(Debug, Default)]
struct CheckoutStats {
    files_written: u32,
    bytes_written: u64,
}

/// Recursively checkout a tree to a directory.
async fn checkout_tree<B: BlobStore, K: KeyValueStore + ?Sized>(
    forge: &Arc<ForgeNode<B, K>>,
    tree_hash: &blake3::Hash,
    base_dir: &Path,
    prefix: &str,
    stats: &mut CheckoutStats,
    depth: u32,
) -> Result<()> {
    // Tiger Style: Bounded recursion
    if depth > MAX_TREE_DEPTH {
        return Err(CiError::Checkout {
            reason: format!("Tree recursion depth exceeds limit of {}", MAX_TREE_DEPTH),
        });
    }

    let tree = forge.git.get_tree(tree_hash).await.map_err(|e| CiError::ForgeOperation {
        reason: format!("Failed to load tree: {}", e),
    })?;

    for entry in &tree.entries {
        // Build the full path
        let entry_path = if prefix.is_empty() {
            entry.name.clone()
        } else {
            format!("{}/{}", prefix, entry.name)
        };

        // Tiger Style: Bounded path length
        if entry_path.len() > MAX_PATH_LENGTH {
            warn!(
                path = %entry_path,
                "Skipping file with path exceeding {} bytes",
                MAX_PATH_LENGTH
            );
            continue;
        }

        let full_path = base_dir.join(&entry_path);
        let entry_hash = blake3::Hash::from_bytes(entry.hash);

        if entry.is_directory() {
            // Create directory and recurse
            fs::create_dir_all(&full_path).await.map_err(|e| CiError::Checkout {
                reason: format!("Failed to create directory {}: {}", full_path.display(), e),
            })?;

            Box::pin(checkout_tree(forge, &entry_hash, base_dir, &entry_path, stats, depth + 1)).await?;
        } else if entry.is_file() {
            // Tiger Style: Check limits before writing
            if stats.files_written >= MAX_CHECKOUT_FILES {
                return Err(CiError::Checkout {
                    reason: format!("Checkout exceeds maximum file count of {}", MAX_CHECKOUT_FILES),
                });
            }

            // Get blob content
            let content = forge.git.get_blob(&entry_hash).await.map_err(|e| CiError::ForgeOperation {
                reason: format!("Failed to load blob {}: {}", entry_path, e),
            })?;

            // Tiger Style: Check total size limit
            let new_total = stats.bytes_written.saturating_add(content.len() as u64);
            if new_total > MAX_CHECKOUT_SIZE_BYTES {
                return Err(CiError::Checkout {
                    reason: format!("Checkout exceeds maximum size of {} bytes", MAX_CHECKOUT_SIZE_BYTES),
                });
            }

            // Ensure parent directory exists
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent).await.map_err(|e| CiError::Checkout {
                    reason: format!("Failed to create parent directory: {}", e),
                })?;
            }

            // Write file
            fs::write(&full_path, &content).await.map_err(|e| CiError::Checkout {
                reason: format!("Failed to write file {}: {}", full_path.display(), e),
            })?;

            // Set executable permission if needed
            if entry.is_executable() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let perms = std::fs::Permissions::from_mode(0o755);
                    fs::set_permissions(&full_path, perms).await.map_err(|e| CiError::Checkout {
                        reason: format!("Failed to set permissions on {}: {}", full_path.display(), e),
                    })?;
                }
            }

            stats.files_written += 1;
            stats.bytes_written = new_total;

            debug!(
                path = %entry_path,
                size = content.len(),
                "Checked out file"
            );
        }
        // Skip symlinks for now (they're rarely used in CI contexts)
    }

    Ok(())
}

/// Create a unique checkout directory for a pipeline run.
///
/// Returns a path like `/tmp/ci-checkout-{run_id}/`.
pub fn checkout_dir_for_run(run_id: &str) -> PathBuf {
    PathBuf::from(format!("/tmp/ci-checkout-{}", run_id))
}

/// Clean up a checkout directory after pipeline completion.
///
/// This should be called when a pipeline finishes (success or failure)
/// to free up disk space.
pub async fn cleanup_checkout(checkout_dir: &Path) -> Result<()> {
    if checkout_dir.exists() {
        fs::remove_dir_all(checkout_dir).await.map_err(|e| CiError::Checkout {
            reason: format!("Failed to clean up checkout directory: {}", e),
        })?;
        debug!(
            path = %checkout_dir.display(),
            "Cleaned up checkout directory"
        );
    }
    Ok(())
}

/// External dependency configuration for CI checkout.
///
/// Defines external git repositories that need to be cloned
/// alongside the main repository (e.g., for path dependencies).
#[derive(Debug, Clone)]
pub struct ExternalDependency {
    /// Git URL to clone from.
    pub git_url: String,
    /// Relative path from checkout root (e.g., "../snix" becomes sibling dir).
    pub relative_path: String,
    /// Optional git ref (branch, tag, or commit hash). Defaults to HEAD.
    pub git_ref: Option<String>,
}

// Known external dependencies for the Aspen project.
// This is hardcoded for now but could be made configurable via CI config.
const KNOWN_EXTERNAL_DEPS: &[(&str, &str, Option<&str>)] = &[
    // Snix - Nix store implementation used by aspen-ci and aspen-snix
    ("https://git.snix.dev/snix/snix.git", "../snix", None),
];

/// Clone external dependencies needed by the project.
///
/// This function clones external git repositories that are referenced
/// as path dependencies in Cargo.toml. The dependencies are cloned
/// as sibling directories relative to the checkout root.
///
/// # Arguments
///
/// * `checkout_dir` - The main checkout directory
///
/// # Returns
///
/// Ok(()) on success, or an error if cloning fails.
pub async fn clone_external_dependencies(checkout_dir: &Path) -> Result<()> {
    use tokio::process::Command;

    for (git_url, relative_path, git_ref) in KNOWN_EXTERNAL_DEPS {
        // Resolve the target path relative to checkout_dir
        let target_path = checkout_dir.join(relative_path);

        // Skip if already exists
        if target_path.exists() {
            debug!(
                path = %target_path.display(),
                "External dependency already exists, skipping"
            );
            continue;
        }

        // Ensure parent directory exists
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| CiError::Checkout {
                reason: format!("Failed to create parent directory for {}: {}", target_path.display(), e),
            })?;
        }

        info!(
            url = git_url,
            target = %target_path.display(),
            "Cloning external dependency"
        );

        // Clone the repository with shallow depth for speed
        let mut cmd = Command::new("git");
        cmd.arg("clone").arg("--depth").arg("1");

        // Add branch/ref if specified
        if let Some(ref_name) = git_ref {
            cmd.arg("--branch").arg(ref_name);
        }

        cmd.arg(git_url).arg(&target_path);

        let output = cmd.output().await.map_err(|e| CiError::Checkout {
            reason: format!("Failed to execute git clone: {}", e),
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CiError::Checkout {
                reason: format!("git clone failed for {}: {}", git_url, stderr),
            });
        }

        info!(
            url = git_url,
            target = %target_path.display(),
            "Successfully cloned external dependency"
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkout_dir_for_run() {
        let dir = checkout_dir_for_run("abc123");
        assert_eq!(dir.to_str().unwrap(), "/tmp/ci-checkout-abc123");
    }

    #[test]
    fn test_max_checkout_limits() {
        // Verify limits are reasonable
        assert_eq!(MAX_CHECKOUT_SIZE_BYTES, 500 * 1024 * 1024);
        assert_eq!(MAX_CHECKOUT_FILES, 50_000);
        assert_eq!(MAX_TREE_DEPTH, 100);
    }
}
