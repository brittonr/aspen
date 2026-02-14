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

use aspen_blob::prelude::*;
use aspen_core::KeyValueStore;
use aspen_forge::ForgeNode;
use snafu::ResultExt;
use tokio::fs;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::error::CheckoutLimitExceededSnafu;
use crate::error::CiError;
use crate::error::CleanupCheckoutSnafu;
use crate::error::CreateCheckoutDirSnafu;
use crate::error::Result;
use crate::error::SetCheckoutPermissionsSnafu;
use crate::error::WriteCheckoutFileSnafu;

// Tiger Style: Bounded resource limits
/// Maximum total checkout size (500 MB).
const MAX_CHECKOUT_SIZE_BYTES: u64 = 500 * 1024 * 1024;
/// Maximum number of files in checkout.
const MAX_CHECKOUT_FILES: u32 = 50_000;
/// Maximum file path length.
const MAX_PATH_LENGTH: usize = 4096;
/// Maximum tree recursion depth.
const MAX_TREE_DEPTH: u32 = 100;

// Retry configuration for handling replication latency
/// Maximum number of retries for fetching git objects.
const MAX_CHECKOUT_OBJECT_RETRIES: u32 = 5;
/// Initial backoff delay in milliseconds.
const CHECKOUT_RETRY_INITIAL_BACKOFF_MS: u64 = 100;
/// Maximum backoff delay in milliseconds.
const CHECKOUT_RETRY_MAX_BACKOFF_MS: u64 = 2000;

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

    // Get the commit object with retry (handles replication latency)
    let commit_hash_str = hex::encode(commit_hash);
    let commit = with_retry("commit", &commit_hash_str, || {
        let forge = forge.clone();
        async move { forge.git.get_commit(&blake3_hash).await }
    })
    .await?;

    // Get the root tree
    let tree_hash = blake3::Hash::from_bytes(commit.tree);

    // Create target directory
    fs::create_dir_all(target_dir).await.context(CreateCheckoutDirSnafu {
        path: target_dir.to_path_buf(),
    })?;

    // Track checkout statistics
    let mut stats = CheckoutStats::default();

    // Recursively checkout the tree
    checkout_tree(forge, &tree_hash, target_dir, "", &mut stats, 0).await?;

    if stats.symlinks_skipped > 0 {
        info!(
            commit = %hex::encode(commit_hash),
            files = stats.files_written,
            bytes = stats.bytes_written,
            symlinks_skipped = stats.symlinks_skipped,
            "Repository checkout complete (some symlinks were skipped)"
        );
    } else {
        info!(
            commit = %hex::encode(commit_hash),
            files = stats.files_written,
            bytes = stats.bytes_written,
            "Repository checkout complete"
        );
    }

    Ok(target_dir.to_path_buf())
}

/// Statistics tracked during checkout.
#[derive(Debug, Default)]
struct CheckoutStats {
    files_written: u32,
    bytes_written: u64,
    symlinks_skipped: u32,
}

/// Classify a Forge error to determine if it's retryable.
///
/// Returns `Some(reason)` if the error is transient and should be retried,
/// `None` if the error is permanent.
fn classify_forge_error(err: &aspen_forge::ForgeError) -> Option<&'static str> {
    use aspen_forge::ForgeError;
    match err {
        ForgeError::ObjectNotFound { .. } => Some("object not yet replicated"),
        ForgeError::BlobsNotAvailable { .. } => Some("blobs not yet available"),
        // All other errors are considered permanent
        _ => None,
    }
}

/// Execute an async operation with exponential backoff retry.
///
/// Used for fetching git objects that may not yet be replicated.
async fn with_retry<T, F, Fut>(object_type: &str, object_hash: &str, operation: F) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = std::result::Result<T, aspen_forge::ForgeError>>,
{
    let mut attempt = 0u32;
    let mut backoff_ms = CHECKOUT_RETRY_INITIAL_BACKOFF_MS;

    loop {
        attempt += 1;

        match operation().await {
            Ok(value) => return Ok(value),
            Err(e) => {
                // Check if error is retryable
                if let Some(reason) = classify_forge_error(&e) {
                    if attempt < MAX_CHECKOUT_OBJECT_RETRIES {
                        debug!(
                            object_type = object_type,
                            hash = object_hash,
                            attempt = attempt,
                            max_attempts = MAX_CHECKOUT_OBJECT_RETRIES,
                            backoff_ms = backoff_ms,
                            reason = reason,
                            "Retrying git object fetch"
                        );

                        // Wait with exponential backoff
                        tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;

                        // Double backoff for next attempt, capped at max
                        backoff_ms = (backoff_ms * 2).min(CHECKOUT_RETRY_MAX_BACKOFF_MS);
                        continue;
                    }

                    // All retries exhausted
                    return Err(CiError::ObjectPermanentlyMissing {
                        object_type: object_type.to_string(),
                        hash: object_hash.to_string(),
                        attempts: attempt,
                    });
                }

                // Non-retryable error
                return Err(CiError::ForgeOperation {
                    reason: format!("Failed to load {}: {}", object_type, e),
                });
            }
        }
    }
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
        return CheckoutLimitExceededSnafu {
            reason: format!("Tree recursion depth exceeds limit of {}", MAX_TREE_DEPTH),
        }
        .fail();
    }

    // Fetch tree with retry (handles replication latency)
    let tree_hash_str = tree_hash.to_hex().to_string();
    let tree = with_retry("tree", &tree_hash_str, || {
        let forge = forge.clone();
        let tree_hash = *tree_hash;
        async move { forge.git.get_tree(&tree_hash).await }
    })
    .await?;

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
            fs::create_dir_all(&full_path).await.context(CreateCheckoutDirSnafu {
                path: full_path.clone(),
            })?;

            Box::pin(checkout_tree(forge, &entry_hash, base_dir, &entry_path, stats, depth + 1)).await?;
        } else if entry.is_file() {
            // Tiger Style: Check limits before writing
            if stats.files_written >= MAX_CHECKOUT_FILES {
                return CheckoutLimitExceededSnafu {
                    reason: format!("Checkout exceeds maximum file count of {}", MAX_CHECKOUT_FILES),
                }
                .fail();
            }

            // Get blob content with retry (handles replication latency)
            let blob_hash_str = entry_hash.to_hex().to_string();
            let content = with_retry("blob", &blob_hash_str, || {
                let forge = forge.clone();
                async move { forge.git.get_blob(&entry_hash).await }
            })
            .await?;

            // Tiger Style: Check total size limit
            let new_total = stats.bytes_written.saturating_add(content.len() as u64);
            if new_total > MAX_CHECKOUT_SIZE_BYTES {
                return CheckoutLimitExceededSnafu {
                    reason: format!("Checkout exceeds maximum size of {} bytes", MAX_CHECKOUT_SIZE_BYTES),
                }
                .fail();
            }

            // Ensure parent directory exists
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent).await.context(CreateCheckoutDirSnafu {
                    path: parent.to_path_buf(),
                })?;
            }

            // Write file
            fs::write(&full_path, &content).await.context(WriteCheckoutFileSnafu {
                path: full_path.clone(),
            })?;

            // Set executable permission if needed
            if entry.is_executable() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let perms = std::fs::Permissions::from_mode(0o755);
                    fs::set_permissions(&full_path, perms).await.context(SetCheckoutPermissionsSnafu {
                        path: full_path.clone(),
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
        } else if entry.mode == 0o120000 {
            // Symlink mode (0o120000)
            // Warn about skipped symlinks instead of silently ignoring them.
            // Symlinks are intentionally not supported in CI checkouts for security reasons:
            // - Prevents symlink attacks that could escape the checkout directory
            // - Ensures deterministic builds (symlink targets may not exist)
            // - Avoids issues with relative vs absolute paths in different environments
            warn!(
                path = %entry_path,
                mode = entry.mode,
                "Skipping symlink during checkout (symlinks are not supported in CI builds)"
            );
            stats.symlinks_skipped += 1;
        }
        // Note: Other entry types (submodules, etc.) are silently skipped
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
        fs::remove_dir_all(checkout_dir).await.context(CleanupCheckoutSnafu {
            path: checkout_dir.to_path_buf(),
        })?;
        debug!(
            path = %checkout_dir.display(),
            "Cleaned up checkout directory"
        );
    }
    Ok(())
}

/// Git source line for snix packages.
/// This must match the revision in Cargo.toml and flake.nix snix-src input.
const SNIX_GIT_SOURCE: &str = "git+https://git.snix.dev/snix/snix.git?rev=8fe3bade2013befd5ca98aa42224fa2a23551559#8fe3bade2013befd5ca98aa42224fa2a23551559";

/// Snix package names that need git source lines added to Cargo.lock.
const SNIX_PACKAGES: &[&str] = &[
    "nix-compat",
    "nix-compat-derive",
    "snix-castore",
    "snix-cli",
    "snix-store",
    "snix-tracing",
];

/// Post-process checkout to remove development-only configurations.
///
/// This performs two operations:
/// 1. Removes `[patch]` sections from `.cargo/config.toml` that reference local path dependencies.
/// 2. Adds git source lines to Cargo.lock for snix packages.
///
/// Both operations are necessary for Nix sandbox builds where:
/// - External paths (like `../snix/snix/*`) are not available
/// - Cargo.lock needs source entries for Crane to vendor git dependencies
///
/// # Arguments
///
/// * `checkout_dir` - The checkout directory to prepare
///
/// # Returns
///
/// Ok(()) on success, or an error if files cannot be processed.
pub async fn prepare_for_ci_build(checkout_dir: &Path) -> Result<()> {
    // Step 1: Remove [patch.*] sections from .cargo/config.toml
    remove_cargo_patches(checkout_dir).await?;

    // Step 2: Add git source lines to Cargo.lock for snix packages
    add_snix_git_sources(checkout_dir).await?;

    Ok(())
}

/// Remove [patch.*] sections from .cargo/config.toml.
async fn remove_cargo_patches(checkout_dir: &Path) -> Result<()> {
    let cargo_config = checkout_dir.join(".cargo/config.toml");

    if !cargo_config.exists() {
        debug!(
            path = %cargo_config.display(),
            "No .cargo/config.toml found, skipping patch removal"
        );
        return Ok(());
    }

    let content = fs::read_to_string(&cargo_config).await.map_err(|e| CiError::Checkout {
        reason: format!("Failed to read .cargo/config.toml: {}", e),
    })?;

    // Parse TOML and remove [patch.*] sections
    let mut doc: toml_edit::DocumentMut = content.parse().map_err(|e| CiError::Checkout {
        reason: format!("Failed to parse .cargo/config.toml: {}", e),
    })?;

    // Find all [patch.*] tables to remove
    let keys_to_remove: Vec<_> =
        doc.as_table().iter().filter(|(k, _)| k.starts_with("patch")).map(|(k, _)| k.to_string()).collect();

    if keys_to_remove.is_empty() {
        debug!(
            path = %cargo_config.display(),
            "No patch sections found in .cargo/config.toml"
        );
        return Ok(());
    }

    for key in &keys_to_remove {
        doc.remove(key);
    }

    fs::write(&cargo_config, doc.to_string()).await.map_err(|e| CiError::Checkout {
        reason: format!("Failed to write .cargo/config.toml: {}", e),
    })?;

    info!(
        path = %cargo_config.display(),
        patches_removed = keys_to_remove.len(),
        "Removed path patches from cargo config for CI build"
    );

    Ok(())
}

/// Add git source lines to Cargo.lock for snix packages.
///
/// When [patch] sections are active during local development, Cargo strips
/// the `source` field from Cargo.lock for patched packages. This function
/// adds them back so Nix/Crane can vendor the git dependencies.
async fn add_snix_git_sources(checkout_dir: &Path) -> Result<()> {
    let cargo_lock = checkout_dir.join("Cargo.lock");

    if !cargo_lock.exists() {
        debug!(
            path = %cargo_lock.display(),
            "No Cargo.lock found, skipping snix source patching"
        );
        return Ok(());
    }

    let content = fs::read_to_string(&cargo_lock).await.map_err(|e| CiError::Checkout {
        reason: format!("Failed to read Cargo.lock: {}", e),
    })?;

    let mut lines: Vec<String> = content.lines().map(String::from).collect();
    let source_line = format!("source = \"{}\"", SNIX_GIT_SOURCE);
    let mut packages_patched = 0u32;

    // Process line by line, looking for snix package entries
    // We collect insertion points first to avoid borrow issues
    let mut insertions: Vec<(usize, String)> = Vec::new();

    for i in 0..lines.len() {
        // Check if this line is a package name we care about
        let is_snix_package = lines[i]
            .strip_prefix("name = \"")
            .and_then(|s| s.strip_suffix('"'))
            .map(|name| SNIX_PACKAGES.contains(&name))
            .unwrap_or(false);

        if is_snix_package {
            // Check if next line is version = "0.1.0"
            if i + 1 < lines.len() && lines[i + 1].starts_with("version = \"0.1.0\"") {
                // Check if source line already exists
                let has_source = i + 2 < lines.len() && lines[i + 2].starts_with("source = ");

                if !has_source {
                    // Record insertion point (will insert after processing)
                    insertions.push((i + 2, source_line.clone()));
                }
            }
        }
    }

    // Apply insertions in reverse order to maintain correct indices
    for (index, line) in insertions.into_iter().rev() {
        lines.insert(index, line);
        packages_patched += 1;
    }

    if packages_patched > 0 {
        let new_content = lines.join("\n");
        fs::write(&cargo_lock, new_content).await.map_err(|e| CiError::Checkout {
            reason: format!("Failed to write Cargo.lock: {}", e),
        })?;

        info!(
            path = %cargo_lock.display(),
            packages_patched = packages_patched,
            "Added git source lines to Cargo.lock for snix packages"
        );
    } else {
        debug!(
            path = %cargo_lock.display(),
            "No snix packages needed source patching (already have sources or not found)"
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

    #[test]
    fn test_retry_constants() {
        // Verify retry constants are reasonable
        assert_eq!(MAX_CHECKOUT_OBJECT_RETRIES, 5);
        assert_eq!(CHECKOUT_RETRY_INITIAL_BACKOFF_MS, 100);
        assert_eq!(CHECKOUT_RETRY_MAX_BACKOFF_MS, 2000);
        // Total max wait: 100 + 200 + 400 + 800 + 1600 = 3100ms ~3s (reasonable)
    }

    #[test]
    fn test_classify_forge_error() {
        use aspen_forge::ForgeError;

        // Retryable errors
        let not_found = ForgeError::ObjectNotFound {
            hash: "abc123".to_string(),
        };
        assert!(classify_forge_error(&not_found).is_some());

        let blobs_unavailable = ForgeError::BlobsNotAvailable {
            count: 5,
            timeout_ms: 1000,
        };
        assert!(classify_forge_error(&blobs_unavailable).is_some());

        // Non-retryable errors
        let invalid = ForgeError::InvalidObject {
            message: "bad format".to_string(),
        };
        assert!(classify_forge_error(&invalid).is_none());
    }

    #[test]
    fn test_snix_constants() {
        // Verify SNIX_GIT_SOURCE format
        assert!(SNIX_GIT_SOURCE.starts_with("git+https://git.snix.dev/snix/snix.git?rev="));
        assert!(SNIX_GIT_SOURCE.contains("#")); // Has commit hash anchor

        // Verify all expected packages are listed
        assert!(SNIX_PACKAGES.contains(&"nix-compat"));
        assert!(SNIX_PACKAGES.contains(&"nix-compat-derive"));
        assert!(SNIX_PACKAGES.contains(&"snix-castore"));
        assert!(SNIX_PACKAGES.contains(&"snix-cli"));
        assert!(SNIX_PACKAGES.contains(&"snix-store"));
        assert!(SNIX_PACKAGES.contains(&"snix-tracing"));
        assert_eq!(SNIX_PACKAGES.len(), 6);
    }

    #[tokio::test]
    async fn test_add_snix_git_sources() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();

        // Create a minimal Cargo.lock without source lines
        let cargo_lock_content = r#"# This file is automatically @generated by Cargo.
version = 4

[[package]]
name = "snix-castore"
version = "0.1.0"
dependencies = [
 "tokio",
]

[[package]]
name = "tokio"
version = "1.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
"#;

        let cargo_lock_path = temp_dir.path().join("Cargo.lock");
        std::fs::write(&cargo_lock_path, cargo_lock_content).unwrap();

        // Run the function
        add_snix_git_sources(temp_dir.path()).await.unwrap();

        // Verify source line was added
        let result = std::fs::read_to_string(&cargo_lock_path).unwrap();
        assert!(result.contains("source = \"git+https://git.snix.dev/snix/snix.git"));

        // Verify it's in the right place (after version line for snix-castore)
        let lines: Vec<&str> = result.lines().collect();
        let name_line = lines.iter().position(|l| l.contains("name = \"snix-castore\"")).unwrap();
        let version_line = lines.iter().position(|l| l.contains("version = \"0.1.0\"")).unwrap();
        let source_line = lines.iter().position(|l| l.contains("source = \"git+https://git.snix.dev")).unwrap();

        assert_eq!(version_line, name_line + 1);
        assert_eq!(source_line, version_line + 1);
    }

    #[tokio::test]
    async fn test_add_snix_git_sources_idempotent() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();

        // Create Cargo.lock that already has source line
        let cargo_lock_content = format!(
            r#"[[package]]
name = "snix-castore"
version = "0.1.0"
source = "{}"
dependencies = []
"#,
            SNIX_GIT_SOURCE
        );

        let cargo_lock_path = temp_dir.path().join("Cargo.lock");
        std::fs::write(&cargo_lock_path, &cargo_lock_content).unwrap();

        // Run the function
        add_snix_git_sources(temp_dir.path()).await.unwrap();

        // Verify file wasn't changed (no duplicate source lines)
        let result = std::fs::read_to_string(&cargo_lock_path).unwrap();
        let source_count = result.matches("source = ").count();
        assert_eq!(source_count, 1);
    }
}
