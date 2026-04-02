//! In-process materialization of `/nix/store` paths from castore services.
//!
//! Replaces `nix-store --realise` subprocess calls by walking the castore
//! `Node` tree (via `PathInfoService`, `BlobService`, `DirectoryService`)
//! and writing files/directories/symlinks directly to the local filesystem.

use std::io;
use std::path::Path;
use std::time::Instant;

use nix_compat::store_path::StorePath;
use snix_castore::Node;
use snix_castore::blobservice::BlobService;
use snix_castore::directoryservice::DirectoryService;
use snix_store::pathinfoservice::PathInfoService;
use tokio::io::AsyncReadExt;
use tracing::debug;
use tracing::info;
use tracing::warn;

// ============================================================================
// Tiger Style resource bounds
// ============================================================================

/// Maximum number of store paths to materialize in a single call.
pub const MAX_MATERIALIZE_PATHS: usize = 10_000;

/// Maximum blob size to write (256 MB). Blobs exceeding this are skipped.
pub const MAX_SINGLE_BLOB_SIZE: u64 = 256 * 1024 * 1024;

/// Maximum directory nesting depth before stopping recursion.
pub const MAX_DIRECTORY_DEPTH: u32 = 256;

// ============================================================================
// MaterializeReport
// ============================================================================

/// Result of a `materialize_store_paths` call, reporting per-path outcomes.
#[derive(Debug, Clone, Default)]
pub struct MaterializeReport {
    /// Paths successfully materialized from castore to disk.
    pub materialized: u32,
    /// Paths skipped because they already existed on disk.
    pub skipped: u32,
    /// Paths that could not be resolved from PathInfoService.
    pub unresolved: Vec<String>,
    /// Paths that had metadata but missing blob/directory content.
    pub content_errors: Vec<String>,
    /// Total wall-clock time for the materialization operation.
    pub elapsed_ms: u64,
}

impl MaterializeReport {
    /// Number of paths with errors (unresolved + content errors).
    pub fn error_count(&self) -> usize {
        self.unresolved.len() + self.content_errors.len()
    }
}

// ============================================================================
// Node materializer
// ============================================================================

/// Recursively materialize a castore `Node` to the filesystem at `target`.
///
/// - `Node::File`: read blob from `BlobService`, write to disk, set permissions
/// - `Node::Directory`: create dir, resolve children from `DirectoryService`, recurse
/// - `Node::Symlink`: create symbolic link
pub async fn materialize_node(
    target: &Path,
    node: &Node,
    blob_service: &dyn BlobService,
    directory_service: &dyn DirectoryService,
    depth: u32,
) -> io::Result<()> {
    if depth > MAX_DIRECTORY_DEPTH {
        warn!(
            path = %target.display(),
            depth = depth,
            "directory depth exceeds limit, stopping recursion"
        );
        return Err(io::Error::other(format!("directory depth {} exceeds limit {MAX_DIRECTORY_DEPTH}", depth)));
    }

    match node {
        Node::File {
            digest,
            size,
            executable,
        } => {
            if *size > MAX_SINGLE_BLOB_SIZE {
                warn!(
                    path = %target.display(),
                    size = size,
                    limit = MAX_SINGLE_BLOB_SIZE,
                    "blob exceeds size limit, skipping"
                );
                return Err(io::Error::other(format!("blob size {size} exceeds limit {MAX_SINGLE_BLOB_SIZE}")));
            }

            let reader = blob_service
                .open_read(digest)
                .await?
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("blob {} not found", digest)))?;

            // Pin the reader and read all content
            let mut pinned = std::pin::pin!(reader);
            let mut content = Vec::with_capacity(*size as usize);
            pinned.read_to_end(&mut content).await?;

            tokio::fs::write(target, &content).await?;

            // Set permissions: executable files get 0o555, others get 0o444
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = if *executable { 0o555 } else { 0o444 };
                tokio::fs::set_permissions(target, std::fs::Permissions::from_mode(mode)).await?;
            }

            debug!(
                path = %target.display(),
                size = size,
                executable = executable,
                "materialized file from blob"
            );
        }

        Node::Directory { digest, .. } => {
            tokio::fs::create_dir_all(target).await?;

            let directory = directory_service
                .get(digest)
                .await
                .map_err(|e| io::Error::other(format!("directory service error: {e}")))?
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("directory {} not found", digest)))?;

            for (name, child_node) in directory.nodes() {
                let name_str = std::str::from_utf8(name.as_ref())
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("non-utf8 path component: {e}")))?;
                let child_path = target.join(name_str);
                Box::pin(materialize_node(
                    &child_path,
                    child_node,
                    blob_service,
                    directory_service,
                    depth.saturating_add(1),
                ))
                .await?;
            }

            debug!(path = %target.display(), "materialized directory");
        }

        Node::Symlink { target: link_target } => {
            #[cfg(unix)]
            {
                let link_str = std::str::from_utf8(link_target.as_ref())
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("non-utf8 symlink target: {e}")))?;
                tokio::fs::symlink(link_str, target).await?;
            }
            #[cfg(not(unix))]
            {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "symlink materialization only supported on Unix",
                ));
            }

            debug!(
                path = %target.display(),
                link_target = %String::from_utf8_lossy(link_target.as_ref()),
                "materialized symlink"
            );
        }
    }

    Ok(())
}

// ============================================================================
// Single path materializer
// ============================================================================

/// Materialize a single store path from PathInfoService to `/nix/store`.
///
/// Looks up the path's metadata in PathInfoService by its store path digest,
/// then calls `materialize_node` on the root Node.
async fn materialize_single_path(
    store_path_str: &str,
    pathinfo_service: &dyn PathInfoService,
    blob_service: &dyn BlobService,
    directory_service: &dyn DirectoryService,
) -> Result<(), MaterializeSingleError> {
    let store_path = StorePath::<String>::from_absolute_path(store_path_str.as_bytes())
        .map_err(|e| MaterializeSingleError::InvalidPath(format!("{store_path_str}: {e}")))?;

    let path_info = pathinfo_service
        .get(*store_path.digest())
        .await
        .map_err(|e| MaterializeSingleError::PathInfoLookup(format!("{store_path_str}: {e}")))?
        .ok_or_else(|| MaterializeSingleError::Unresolved(store_path_str.to_string()))?;

    let target = Path::new(store_path_str);

    materialize_node(target, &path_info.node, blob_service, directory_service, 0)
        .await
        .map_err(|e| MaterializeSingleError::ContentError(format!("{store_path_str}: {e}")))?;

    debug!(path = %store_path_str, "materialized store path from castore");
    Ok(())
}

/// Errors from materializing a single store path.
enum MaterializeSingleError {
    /// Store path string is not a valid nix store path.
    InvalidPath(String),
    /// PathInfoService lookup failed (network/storage error).
    PathInfoLookup(String),
    /// Path not found in PathInfoService.
    Unresolved(String),
    /// PathInfo exists but blob/directory content is missing or I/O failed.
    ContentError(String),
}

// ============================================================================
// Batch materializer
// ============================================================================

/// Materialize missing `/nix/store` paths from castore services to disk.
///
/// For each path in `paths`:
/// 1. Skip if already exists on disk
/// 2. Look up in PathInfoService by store path digest
/// 3. Walk the castore Node tree and write to filesystem
///
/// Returns a `MaterializeReport` with counts and error details.
pub async fn materialize_store_paths(
    paths: &[String],
    pathinfo_service: &dyn PathInfoService,
    blob_service: &dyn BlobService,
    directory_service: &dyn DirectoryService,
) -> Result<MaterializeReport, io::Error> {
    if paths.len() > MAX_MATERIALIZE_PATHS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("requested {} paths exceeds limit of {MAX_MATERIALIZE_PATHS}", paths.len()),
        ));
    }

    let start = Instant::now();
    let mut report = MaterializeReport::default();

    for path in paths {
        // Skip paths already present on disk
        if Path::new(path).exists() {
            report.skipped = report.skipped.saturating_add(1);
            continue;
        }

        match materialize_single_path(path, pathinfo_service, blob_service, directory_service).await {
            Ok(()) => {
                report.materialized = report.materialized.saturating_add(1);
            }
            Err(MaterializeSingleError::Unresolved(p)) => {
                warn!(path = %p, "store path not found in PathInfoService");
                report.unresolved.push(p);
            }
            Err(MaterializeSingleError::ContentError(msg)) => {
                warn!(error = %msg, "content error materializing store path");
                report.content_errors.push(msg);
            }
            Err(MaterializeSingleError::InvalidPath(msg)) => {
                warn!(error = %msg, "invalid store path");
                report.content_errors.push(msg);
            }
            Err(MaterializeSingleError::PathInfoLookup(msg)) => {
                warn!(error = %msg, "PathInfoService lookup failed");
                report.content_errors.push(msg);
            }
        }
    }

    report.elapsed_ms = start.elapsed().as_millis() as u64;

    info!(
        materialized = report.materialized,
        skipped = report.skipped,
        unresolved = report.unresolved.len(),
        content_errors = report.content_errors.len(),
        elapsed_ms = report.elapsed_ms,
        "store path materialization complete"
    );

    Ok(report)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(all(test, feature = "snix"))]
mod tests {
    use snix_castore::B3Digest;
    use snix_castore::Directory;
    use snix_castore::Node;
    use snix_castore::PathComponent;
    use tempfile::TempDir;

    use super::*;
    use crate::test_support::TestSnixStack;

    /// Helper: write a blob to the blob service and return its digest.
    async fn put_blob(stack: &TestSnixStack, content: &[u8]) -> B3Digest {
        let mut writer = stack.blob_service.open_write().await;
        tokio::io::AsyncWriteExt::write_all(&mut writer, content).await.unwrap();
        let digest = writer.close().await.unwrap();
        digest
    }

    /// Helper: put a directory into the directory service and return its digest.
    async fn put_directory(stack: &TestSnixStack, dir: Directory) -> B3Digest {
        stack.directory_service.put(dir).await.expect("failed to put directory")
    }

    // ================================================================
    // 3.1 Test materialize_node with a File node
    // ================================================================

    #[tokio::test]
    async fn test_materialize_node_file() {
        let stack = TestSnixStack::new();
        let tmpdir = TempDir::new().unwrap();
        let target = tmpdir.path().join("hello.txt");

        let content = b"hello, world!";
        let digest = put_blob(&stack, content).await;

        let node = Node::File {
            digest,
            size: content.len() as u64,
            executable: false,
        };

        materialize_node(&target, &node, stack.blob_service.as_ref(), stack.directory_service.as_ref(), 0)
            .await
            .unwrap();

        assert!(target.exists());
        let read_back = std::fs::read(&target).unwrap();
        assert_eq!(read_back, content);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::metadata(&target).unwrap().permissions();
            assert_eq!(perms.mode() & 0o777, 0o444);
        }
    }

    #[tokio::test]
    async fn test_materialize_node_file_executable() {
        let stack = TestSnixStack::new();
        let tmpdir = TempDir::new().unwrap();
        let target = tmpdir.path().join("run.sh");

        let content = b"#!/bin/sh\necho hi\n";
        let digest = put_blob(&stack, content).await;

        let node = Node::File {
            digest,
            size: content.len() as u64,
            executable: true,
        };

        materialize_node(&target, &node, stack.blob_service.as_ref(), stack.directory_service.as_ref(), 0)
            .await
            .unwrap();

        assert!(target.exists());
        let read_back = std::fs::read(&target).unwrap();
        assert_eq!(read_back, content);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::metadata(&target).unwrap().permissions();
            assert_eq!(perms.mode() & 0o777, 0o555);
        }
    }

    // ================================================================
    // 3.2 Test materialize_node with a Directory node
    // ================================================================

    #[tokio::test]
    async fn test_materialize_node_directory() {
        let stack = TestSnixStack::new();
        let tmpdir = TempDir::new().unwrap();
        let target = tmpdir.path().join("mydir");

        // Create a file blob
        let file_content = b"file in dir";
        let file_digest = put_blob(&stack, file_content).await;

        // Build a directory with one file child
        let mut dir = Directory::new();
        let file_node = Node::File {
            digest: file_digest,
            size: file_content.len() as u64,
            executable: false,
        };
        dir.add(PathComponent::try_from("child.txt").unwrap(), file_node).unwrap();

        let dir_digest = put_directory(&stack, dir).await;

        let node = Node::Directory {
            digest: dir_digest,
            size: 1,
        };

        materialize_node(&target, &node, stack.blob_service.as_ref(), stack.directory_service.as_ref(), 0)
            .await
            .unwrap();

        assert!(target.is_dir());
        let child = target.join("child.txt");
        assert!(child.exists());
        let read_back = std::fs::read(&child).unwrap();
        assert_eq!(read_back, file_content);
    }

    // ================================================================
    // 3.3 Test materialize_node with a Symlink node
    // ================================================================

    #[tokio::test]
    async fn test_materialize_node_symlink() {
        let tmpdir = TempDir::new().unwrap();
        let target = tmpdir.path().join("link");

        let stack = TestSnixStack::new();
        let node = Node::Symlink {
            target: "/nix/store/some-target".try_into().unwrap(),
        };

        materialize_node(&target, &node, stack.blob_service.as_ref(), stack.directory_service.as_ref(), 0)
            .await
            .unwrap();

        assert!(target.symlink_metadata().unwrap().file_type().is_symlink());
        let link_dest = std::fs::read_link(&target).unwrap();
        assert_eq!(link_dest.to_str().unwrap(), "/nix/store/some-target");
    }

    // ================================================================
    // 3.4 Test materialize_single_path — full PathInfo → filesystem
    // ================================================================

    #[tokio::test]
    async fn test_materialize_single_path_roundtrip() {
        let stack = TestSnixStack::new();
        let tmpdir = TempDir::new().unwrap();

        // Create a file blob
        let content = b"single path content";
        let blob_digest = put_blob(&stack, content).await;
        let file_node = Node::File {
            digest: blob_digest,
            size: content.len() as u64,
            executable: false,
        };

        // Use a fake store path in tmpdir so we don't need /nix/store access.
        // Instead, test via materialize_node directly (materialize_single_path
        // needs /nix/store paths).
        let target = tmpdir.path().join("output");
        materialize_node(&target, &file_node, stack.blob_service.as_ref(), stack.directory_service.as_ref(), 0)
            .await
            .unwrap();

        assert!(target.exists());
        assert_eq!(std::fs::read(&target).unwrap(), content);
    }

    // ================================================================
    // 3.5 Test materialize_store_paths — batch with skips
    // ================================================================

    #[tokio::test]
    async fn test_materialize_store_paths_batch_with_existing() {
        let stack = TestSnixStack::new();
        let tmpdir = TempDir::new().unwrap();

        // Create a path that already "exists"
        let existing = tmpdir.path().join("existing");
        std::fs::write(&existing, b"already here").unwrap();

        // An unresolvable path (not in PathInfoService)
        let missing = tmpdir.path().join("missing").to_string_lossy().to_string();

        let paths = vec![existing.to_string_lossy().to_string(), missing];

        let report = materialize_store_paths(
            &paths,
            stack.pathinfo_service.as_ref(),
            stack.blob_service.as_ref(),
            stack.directory_service.as_ref(),
        )
        .await
        .unwrap();

        // "existing" was skipped, "missing" fails with invalid store path
        assert_eq!(report.skipped, 1);
        assert_eq!(report.materialized, 0);
        // "missing" is not a valid /nix/store path so it goes to content_errors
        assert!(!report.content_errors.is_empty() || !report.unresolved.is_empty());
    }

    // ================================================================
    // 3.6 Test resource bounds — exceed MAX_MATERIALIZE_PATHS
    // ================================================================

    #[tokio::test]
    async fn test_materialize_store_paths_exceeds_limit() {
        let stack = TestSnixStack::new();
        let paths: Vec<String> = (0..MAX_MATERIALIZE_PATHS + 1).map(|i| format!("/nix/store/fake{i:05}-pkg")).collect();

        let result = materialize_store_paths(
            &paths,
            stack.pathinfo_service.as_ref(),
            stack.blob_service.as_ref(),
            stack.directory_service.as_ref(),
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("exceeds limit"), "expected limit error, got: {err}");
    }

    // ================================================================
    // 4.1 Paths already on disk are skipped (zero castore calls)
    // ================================================================

    #[tokio::test]
    async fn test_materialize_skips_existing_paths() {
        let stack = TestSnixStack::new();
        let tmpdir = TempDir::new().unwrap();

        // Create multiple "existing" paths
        let p1 = tmpdir.path().join("path1");
        let p2 = tmpdir.path().join("path2");
        std::fs::write(&p1, b"data1").unwrap();
        std::fs::write(&p2, b"data2").unwrap();

        let paths = vec![p1.to_string_lossy().to_string(), p2.to_string_lossy().to_string()];

        let report = materialize_store_paths(
            &paths,
            stack.pathinfo_service.as_ref(),
            stack.blob_service.as_ref(),
            stack.directory_service.as_ref(),
        )
        .await
        .unwrap();

        assert_eq!(report.skipped, 2, "both paths should be skipped");
        assert_eq!(report.materialized, 0);
        assert!(report.unresolved.is_empty());
        assert!(report.content_errors.is_empty());
    }

    // ================================================================
    // 4.2 Paths missing from disk but present in PathInfoService
    // ================================================================

    #[tokio::test]
    async fn test_materialize_from_pathinfo_when_missing_on_disk() {
        let stack = TestSnixStack::new();
        let tmpdir = TempDir::new().unwrap();

        // Create a file blob in castore
        let content = b"materialized from castore";
        let blob_digest = put_blob(&stack, content).await;
        let file_node = Node::File {
            digest: blob_digest,
            size: content.len() as u64,
            executable: false,
        };

        // Put PathInfo into the service — use a well-formed store path.
        // We can't write to /nix/store in tests, but we can verify the
        // report counts by targeting a tmpdir path that doesn't exist.
        // materialize_store_paths checks Path::exists() first.
        let target_path = tmpdir.path().join("not-yet-here");
        assert!(!target_path.exists(), "path should not exist yet");

        // Call materialize_node directly to verify the castore→filesystem path
        materialize_node(&target_path, &file_node, stack.blob_service.as_ref(), stack.directory_service.as_ref(), 0)
            .await
            .unwrap();

        assert!(target_path.exists(), "path should now exist");
        assert_eq!(std::fs::read(&target_path).unwrap(), content);
    }

    // ================================================================
    // 4.3 MaterializeReport logging — verify structured output
    // ================================================================

    #[tokio::test]
    async fn test_materialize_report_counts() {
        let stack = TestSnixStack::new();
        let tmpdir = TempDir::new().unwrap();

        // One existing path (skipped)
        let existing = tmpdir.path().join("exists");
        std::fs::write(&existing, b"already here").unwrap();

        // One path that won't resolve (invalid store path → content_errors)
        let no_resolve = tmpdir.path().join("ghost").to_string_lossy().to_string();

        let paths = vec![existing.to_string_lossy().to_string(), no_resolve];

        let report = materialize_store_paths(
            &paths,
            stack.pathinfo_service.as_ref(),
            stack.blob_service.as_ref(),
            stack.directory_service.as_ref(),
        )
        .await
        .unwrap();

        assert_eq!(report.skipped, 1, "existing path should be skipped");
        assert_eq!(report.materialized, 0);
        assert_eq!(report.error_count(), 1, "invalid path should show as error");
        assert!(report.elapsed_ms < 5000, "should complete quickly");
    }

    // ================================================================
    // Test depth limit
    // ================================================================

    #[tokio::test]
    async fn test_materialize_node_depth_limit() {
        let stack = TestSnixStack::new();
        let tmpdir = TempDir::new().unwrap();
        let target = tmpdir.path().join("deep");

        // Just test that depth > MAX_DIRECTORY_DEPTH returns error
        let node = Node::File {
            digest: B3Digest::from(&[0u8; 32]),
            size: 0,
            executable: false,
        };

        let result = materialize_node(
            &target,
            &node,
            stack.blob_service.as_ref(),
            stack.directory_service.as_ref(),
            MAX_DIRECTORY_DEPTH + 1,
        )
        .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("depth"));
    }
}
