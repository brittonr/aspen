//! Nanvix micro-VM worker implementation.
//!
//! Executes JavaScript, Python, and native binary workloads inside
//! hardware-isolated micro-VMs via hyperlight-nanvix. The Nanvix
//! microkernel provides a POSIX-compatible interface to guest code.
//! Guest I/O is captured from the console log file written by the
//! Nanvix microkernel.
//!
//! # Storage
//!
//! Workload files are sourced from Aspen's iroh-blobs store. Guest
//! workspace files are materialized from the Aspen KV store before
//! execution and synced back after execution.
//!
//! # Workspace Materialization
//!
//! Hyperlight-nanvix does not support VirtioFS. Its filesystem mechanism
//! is `SyscallTable<T>` for syscall interception, but `RuntimeConfig`
//! hardcodes `T = ()`, preventing custom state in handlers.
//!
//! Instead, this worker uses workspace materialization: files are read from
//! KV into a temp directory before VM execution, and new/modified files are
//! written back to KV after execution completes. This mirrors the pattern
//! used by CI systems and is sufficient for short-lived JS/Python jobs.
//!
//! KV key convention: `nanvix/workspaces/{job_id}/{relative_path}`
//! File content is base64-encoded since KV values are `String`.

use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use aspen_blob::prelude::*;
use aspen_kv_types::ScanRequest;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
use async_trait::async_trait;
use base64::Engine;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::error::JobError;
use crate::error::Result;
use crate::job::Job;
use crate::job::JobResult;
use crate::vm_executor::types::JobPayload;
use crate::worker::Worker;

/// Maximum size for a workload file (50MB).
const MAX_WORKLOAD_SIZE: usize = 50 * 1024 * 1024;

/// Valid workload types accepted by the NanvixWorker.
const VALID_WORKLOAD_TYPES: &[&str] = &["javascript", "python", "binary"];

/// Maximum number of files in a workspace (Tiger Style bounded scan).
const MAX_WORKSPACE_FILES: u32 = 1_000;

/// Maximum size for a single workspace file (1MB, matches `MAX_VALUE_SIZE`).
const MAX_WORKSPACE_FILE_SIZE_BYTES: u64 = 1_024 * 1_024;

/// KV key prefix for Nanvix workspaces.
const WORKSPACE_PREFIX: &str = "nanvix/workspaces/";

/// Maximum directory entries to read per directory during walk (prevents
/// runaway traversal on adversarial directory structures).
const MAX_DIR_ENTRIES: u32 = 10_000;

/// Worker that executes workloads in Nanvix micro-VMs via hyperlight-nanvix.
///
/// Supports JavaScript (QuickJS), Python (CPython 3.12), and native ELF
/// binaries. Requires KVM at runtime for hardware isolation.
///
/// Workspace files are materialized from and synced back to the Aspen KV
/// store under `nanvix/workspaces/{job_id}/`.
pub struct NanvixWorker {
    /// Blob store for retrieving workload files.
    blob_store: Arc<dyn BlobStore>,
    /// KV store for workspace file materialization and sync.
    kv_store: Arc<dyn KeyValueStore>,
}

impl NanvixWorker {
    /// Create a new Nanvix worker.
    /// Requires a blob store for retrieving workload files and a KV store
    /// for workspace materialization.
    pub fn new(blob_store: Arc<dyn BlobStore>, kv_store: Arc<dyn KeyValueStore>) -> Result<Self> {
        Ok(Self { blob_store, kv_store })
    }

    /// Build the KV key prefix for a job's workspace.
    fn workspace_prefix(job_id: &str) -> String {
        format!("{}{}/", WORKSPACE_PREFIX, job_id)
    }

    /// Extract the relative file path from a full KV key by stripping the
    /// workspace prefix. Returns `None` if the key doesn't start with the
    /// prefix or if the relative path is empty.
    fn relative_path_from_key<'a>(prefix: &str, key: &'a str) -> Option<&'a str> {
        let rel = key.strip_prefix(prefix)?;
        if rel.is_empty() {
            return None;
        }
        Some(rel)
    }

    /// Materialize workspace files from KV into the given directory.
    ///
    /// Scans `nanvix/workspaces/{job_id}/` and writes each entry as a file.
    /// Values are base64-decoded. Returns the number of files materialized.
    async fn materialize_workspace(&self, job_id: &str, workspace_dir: &Path) -> Result<u32> {
        let prefix = Self::workspace_prefix(job_id);

        let scan_result = self
            .kv_store
            .scan(ScanRequest {
                prefix: prefix.clone(),
                limit_results: Some(MAX_WORKSPACE_FILES),
                continuation_token: None,
            })
            .await
            .map_err(|e| JobError::VmExecutionFailed {
                reason: format!("failed to scan workspace KV prefix '{}': {}", prefix, e),
            })?;

        let mut count: u32 = 0;
        for entry in &scan_result.entries {
            let rel_path = match Self::relative_path_from_key(&prefix, &entry.key) {
                Some(p) => p,
                None => continue,
            };

            let decoded = base64::engine::general_purpose::STANDARD.decode(&entry.value).map_err(|e| {
                JobError::VmExecutionFailed {
                    reason: format!("failed to base64-decode workspace file '{}': {}", rel_path, e),
                }
            })?;

            let file_path = workspace_dir.join(rel_path);

            // Create parent directories if needed.
            if let Some(parent) = file_path.parent() {
                tokio::fs::create_dir_all(parent).await.map_err(|e| JobError::VmExecutionFailed {
                    reason: format!("failed to create directory '{}': {}", parent.display(), e),
                })?;
            }

            tokio::fs::write(&file_path, &decoded).await.map_err(|e| JobError::VmExecutionFailed {
                reason: format!("failed to write workspace file '{}': {}", file_path.display(), e),
            })?;

            debug!(path = rel_path, size = decoded.len(), "materialized workspace file");
            count = count.saturating_add(1);
        }

        info!(job_id, files = count, "materialized workspace from KV");
        Ok(count)
    }

    /// Sync workspace files back to KV after execution.
    ///
    /// Walks the workspace directory recursively and writes each file to KV
    /// under `nanvix/workspaces/{job_id}/`. File content is base64-encoded.
    /// Skips files larger than `MAX_WORKSPACE_FILE_SIZE_BYTES`. Returns the
    /// number of files synced.
    async fn sync_workspace(&self, job_id: &str, workspace_dir: &Path) -> Result<u32> {
        let prefix = Self::workspace_prefix(job_id);
        let files = walk_dir_recursive(workspace_dir).await?;

        let mut count: u32 = 0;
        for file_path in &files {
            if count >= MAX_WORKSPACE_FILES {
                warn!(job_id, limit = MAX_WORKSPACE_FILES, "workspace file limit reached, skipping remaining files");
                break;
            }

            let metadata = tokio::fs::metadata(file_path).await.map_err(|e| JobError::VmExecutionFailed {
                reason: format!("failed to stat '{}': {}", file_path.display(), e),
            })?;

            if metadata.len() > MAX_WORKSPACE_FILE_SIZE_BYTES {
                warn!(
                    path = %file_path.display(),
                    size = metadata.len(),
                    max = MAX_WORKSPACE_FILE_SIZE_BYTES,
                    "skipping oversized workspace file"
                );
                continue;
            }

            let bytes = tokio::fs::read(file_path).await.map_err(|e| JobError::VmExecutionFailed {
                reason: format!("failed to read '{}': {}", file_path.display(), e),
            })?;

            let rel_path = file_path.strip_prefix(workspace_dir).map_err(|e| JobError::VmExecutionFailed {
                reason: format!("failed to compute relative path for '{}': {}", file_path.display(), e),
            })?;

            let key = format!("{}{}", prefix, rel_path.display());
            let value = base64::engine::general_purpose::STANDARD.encode(&bytes);

            self.kv_store
                .write(WriteRequest::set(&key, &value))
                .await
                .map_err(|e| JobError::VmExecutionFailed {
                    reason: format!("failed to write workspace file '{}' to KV: {}", key, e),
                })?;

            debug!(key, size = bytes.len(), "synced workspace file to KV");
            count = count.saturating_add(1);
        }

        info!(job_id, files = count, "synced workspace to KV");
        Ok(count)
    }

    /// Map a workload type string to the file extension that nanvix
    /// `WorkloadType::from_path()` will recognize.
    fn workload_extension(workload_type: &str) -> Result<&'static str> {
        match workload_type {
            "javascript" => Ok(".js"),
            "python" => Ok(".py"),
            "binary" => Ok(".elf"),
            other => Err(JobError::VmExecutionFailed {
                reason: format!("unknown workload_type '{}': expected one of {:?}", other, VALID_WORKLOAD_TYPES),
            }),
        }
    }

    /// Retrieve workload bytes from the blob store.
    async fn retrieve_workload(&self, hash: &str, expected_size: u64) -> Result<Vec<u8>> {
        info!(hash, expected_size, "retrieving nanvix workload from blob store");

        let blob_hash = hash.parse::<iroh_blobs::Hash>().map_err(|e| JobError::VmExecutionFailed {
            reason: format!("invalid blob hash '{}': {}", hash, e),
        })?;

        let bytes = self
            .blob_store
            .get_bytes(&blob_hash)
            .await
            .map_err(|e| JobError::VmExecutionFailed {
                reason: format!("failed to retrieve blob: {}", e),
            })?
            .ok_or_else(|| JobError::VmExecutionFailed {
                reason: format!("blob not found: {}", hash),
            })?;

        // Validate size if provided (0 means skip validation).
        if expected_size > 0 && bytes.len() as u64 != expected_size {
            return Err(JobError::VmExecutionFailed {
                reason: format!("blob size mismatch: expected {} bytes, got {} bytes", expected_size, bytes.len()),
            });
        }

        if bytes.len() > MAX_WORKLOAD_SIZE {
            return Err(JobError::BinaryTooLarge {
                size_bytes: bytes.len() as u64,
                max_bytes: MAX_WORKLOAD_SIZE as u64,
            });
        }

        Ok(bytes.to_vec())
    }

    /// Execute a workload in a Nanvix sandbox with workspace materialization.
    ///
    /// 1. Creates a workspace temp directory
    /// 2. Materializes files from KV into the workspace
    /// 3. Writes the workload as `__workload{ext}` in the workspace
    /// 4. Runs the sandbox
    /// 5. Reads console output
    /// 6. Removes the workload file and syncs new/modified files back to KV
    async fn execute_workload(&self, workload_bytes: Vec<u8>, workload_type: &str, job_id: &str) -> Result<JobResult> {
        let extension = Self::workload_extension(workload_type)?;

        info!(workload_type, size = workload_bytes.len(), job_id, "executing nanvix workload");

        // Create a workspace directory (not just a single temp file).
        let workspace = tempfile::TempDir::new().map_err(|e| JobError::VmExecutionFailed {
            reason: format!("failed to create workspace temp dir: {}", e),
        })?;

        // Materialize existing workspace files from KV.
        let files_materialized = self.materialize_workspace(job_id, workspace.path()).await?;

        // Write workload as __workload{ext} to avoid collision with user files.
        let workload_filename = format!("__workload{}", extension);
        let workload_path = workspace.path().join(&workload_filename);

        tokio::fs::write(&workload_path, &workload_bytes).await.map_err(|e| JobError::VmExecutionFailed {
            reason: format!("failed to write workload to workspace: {}", e),
        })?;

        // Create RuntimeConfig with unique temp directories.
        let config = hyperlight_nanvix::RuntimeConfig::default();
        let log_directory = config.log_directory.clone();

        debug!(
            log_directory = %log_directory,
            workload_path = %workload_path.display(),
            workspace = %workspace.path().display(),
            "creating nanvix sandbox"
        );

        // Create and run sandbox (requires KVM at runtime).
        let mut sandbox = hyperlight_nanvix::Sandbox::new(config).map_err(|e| JobError::VmExecutionFailed {
            reason: format!("failed to create nanvix sandbox: {}", e),
        })?;

        sandbox.run(&workload_path).await.map_err(|e| JobError::VmExecutionFailed {
            reason: format!("nanvix workload execution failed: {}", e),
        })?;

        // Read console output from the guest.
        let console_log_path = format!("{}/guest-console.log", log_directory);
        let console_output = match tokio::fs::read_to_string(&console_log_path).await {
            Ok(content) => {
                debug!(output_len = content.len(), "read guest console output");
                content
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                debug!("no guest console output (file not found)");
                String::new()
            }
            Err(e) => {
                warn!(error = %e, path = %console_log_path, "failed to read guest console log");
                String::new()
            }
        };

        // Remove workload file before sync so it doesn't get written to KV.
        let _ = tokio::fs::remove_file(&workload_path).await;

        // Sync new/modified files back to KV.
        let files_written = self.sync_workspace(job_id, workspace.path()).await?;

        // Clean up temp directories (best-effort).
        let _ = tokio::fs::remove_dir_all(&log_directory).await;

        Ok(JobResult::success(serde_json::json!({
            "console_output": console_output,
            "workload_type": workload_type,
            "files_materialized": files_materialized,
            "files_written": files_written,
        })))
    }
}

/// Recursively walk a directory, collecting all regular file paths.
///
/// Uses an explicit stack instead of recursion to avoid stack overflow
/// on deeply nested directories. Skips symlinks for security. Bounded
/// by `MAX_WORKSPACE_FILES` total files and `MAX_DIR_ENTRIES` per directory.
async fn walk_dir_recursive(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let mut entries = tokio::fs::read_dir(&dir).await.map_err(|e| JobError::VmExecutionFailed {
            reason: format!("failed to read directory '{}': {}", dir.display(), e),
        })?;

        let mut dir_count: u32 = 0;
        while let Some(entry) = entries.next_entry().await.map_err(|e| JobError::VmExecutionFailed {
            reason: format!("failed to read directory entry in '{}': {}", dir.display(), e),
        })? {
            dir_count = dir_count.saturating_add(1);
            if dir_count > MAX_DIR_ENTRIES {
                warn!(
                    dir = %dir.display(),
                    limit = MAX_DIR_ENTRIES,
                    "directory entry limit reached, skipping remaining entries"
                );
                break;
            }

            if files.len() as u32 >= MAX_WORKSPACE_FILES {
                return Ok(files);
            }

            let file_type = entry.file_type().await.map_err(|e| JobError::VmExecutionFailed {
                reason: format!("failed to get file type for '{}': {}", entry.path().display(), e),
            })?;

            // Skip symlinks for security.
            if file_type.is_symlink() {
                continue;
            }

            if file_type.is_dir() {
                stack.push(entry.path());
            } else if file_type.is_file() {
                files.push(entry.path());
            }
        }
    }

    Ok(files)
}

#[async_trait]
impl Worker for NanvixWorker {
    async fn execute(&self, job: Job) -> JobResult {
        let payload: JobPayload = match serde_json::from_value(job.spec.payload.clone()) {
            Ok(p) => p,
            Err(e) => {
                return JobResult::failure(format!("failed to parse job payload: {}", e));
            }
        };

        let result = match payload {
            JobPayload::NanvixWorkload {
                hash,
                size,
                workload_type,
            } => {
                let workload_bytes = match self.retrieve_workload(&hash, size).await {
                    Ok(bytes) => bytes,
                    Err(e) => return JobResult::failure(format!("failed to retrieve nanvix workload: {}", e)),
                };

                self.execute_workload(workload_bytes, &workload_type, &job.id.to_string()).await
            }

            other => {
                warn!(job_id = %job.id, payload_type = ?other, "NanvixWorker received non-Nanvix payload");
                Err(JobError::VmExecutionFailed {
                    reason: "NanvixWorker only handles NanvixWorkload payloads".to_string(),
                })
            }
        };

        match result {
            Ok(job_result) => job_result,
            Err(e) => JobResult::failure(format!("nanvix execution failed: {}", e)),
        }
    }

    fn job_types(&self) -> Vec<String> {
        vec!["nanvix_execute".to_string()]
    }
}

#[cfg(test)]
mod tests {
    use aspen_blob::InMemoryBlobStore;
    use aspen_testing::DeterministicKeyValueStore;

    use super::*;

    fn test_kv_store() -> Arc<DeterministicKeyValueStore> {
        DeterministicKeyValueStore::new()
    }

    fn test_blob_store() -> Arc<InMemoryBlobStore> {
        Arc::new(InMemoryBlobStore::new())
    }

    #[test]
    fn test_workspace_prefix() {
        let prefix = NanvixWorker::workspace_prefix("job-123");
        assert_eq!(prefix, "nanvix/workspaces/job-123/");
    }

    #[test]
    fn test_relative_path_from_key() {
        let prefix = "nanvix/workspaces/job-123/";

        // Normal file.
        assert_eq!(NanvixWorker::relative_path_from_key(prefix, "nanvix/workspaces/job-123/foo.txt"), Some("foo.txt"));

        // Nested path.
        assert_eq!(
            NanvixWorker::relative_path_from_key(prefix, "nanvix/workspaces/job-123/dir/sub/file.py"),
            Some("dir/sub/file.py")
        );

        // Empty relative path.
        assert_eq!(NanvixWorker::relative_path_from_key(prefix, "nanvix/workspaces/job-123/"), None);

        // Wrong prefix.
        assert_eq!(NanvixWorker::relative_path_from_key(prefix, "nanvix/workspaces/other-job/foo.txt"), None);
    }

    #[tokio::test]
    async fn test_materialize_empty_workspace() {
        let blob_store = test_blob_store();
        let kv_store = test_kv_store();
        let worker = NanvixWorker::new(blob_store, kv_store).unwrap();

        let workspace = tempfile::TempDir::new().unwrap();
        let count = worker.materialize_workspace("empty-job", workspace.path()).await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_materialize_workspace_with_files() {
        let blob_store = test_blob_store();
        let kv_store = test_kv_store();

        // Pre-populate KV with workspace files.
        let content_a = b"hello world";
        let encoded_a = base64::engine::general_purpose::STANDARD.encode(content_a);
        kv_store.write(WriteRequest::set("nanvix/workspaces/job-1/greeting.txt", &encoded_a)).await.unwrap();

        let content_b = b"print('hi')";
        let encoded_b = base64::engine::general_purpose::STANDARD.encode(content_b);
        kv_store
            .write(WriteRequest::set("nanvix/workspaces/job-1/scripts/hello.py", &encoded_b))
            .await
            .unwrap();

        let worker = NanvixWorker::new(blob_store, kv_store).unwrap();

        let workspace = tempfile::TempDir::new().unwrap();
        let count = worker.materialize_workspace("job-1", workspace.path()).await.unwrap();
        assert_eq!(count, 2);

        // Verify file contents.
        let read_a = tokio::fs::read(workspace.path().join("greeting.txt")).await.unwrap();
        assert_eq!(read_a, content_a);

        let read_b = tokio::fs::read(workspace.path().join("scripts/hello.py")).await.unwrap();
        assert_eq!(read_b, content_b);
    }

    #[tokio::test]
    async fn test_sync_workspace() {
        let blob_store = test_blob_store();
        let kv_store = test_kv_store();
        let worker = NanvixWorker::new(blob_store, kv_store.clone()).unwrap();

        let workspace = tempfile::TempDir::new().unwrap();

        // Create some files in the workspace.
        let file_a = workspace.path().join("output.txt");
        tokio::fs::write(&file_a, b"result data").await.unwrap();

        let nested_dir = workspace.path().join("logs");
        tokio::fs::create_dir_all(&nested_dir).await.unwrap();
        let file_b = nested_dir.join("run.log");
        tokio::fs::write(&file_b, b"log line 1").await.unwrap();

        let count = worker.sync_workspace("job-2", workspace.path()).await.unwrap();
        assert_eq!(count, 2);

        // Verify KV contents.
        use aspen_kv_types::ReadRequest;
        let result = kv_store.read(ReadRequest::new("nanvix/workspaces/job-2/output.txt")).await.unwrap();
        let decoded = base64::engine::general_purpose::STANDARD.decode(&result.kv.unwrap().value).unwrap();
        assert_eq!(decoded, b"result data");

        let result = kv_store.read(ReadRequest::new("nanvix/workspaces/job-2/logs/run.log")).await.unwrap();
        let decoded = base64::engine::general_purpose::STANDARD.decode(&result.kv.unwrap().value).unwrap();
        assert_eq!(decoded, b"log line 1");
    }

    #[tokio::test]
    async fn test_roundtrip_materialize_sync() {
        let blob_store = test_blob_store();
        let kv_store = test_kv_store();

        // Materialize from KV.
        let original_content = b"original file content";
        let encoded = base64::engine::general_purpose::STANDARD.encode(original_content);
        kv_store.write(WriteRequest::set("nanvix/workspaces/job-rt/data.txt", &encoded)).await.unwrap();

        let worker = NanvixWorker::new(blob_store, kv_store.clone()).unwrap();

        let workspace = tempfile::TempDir::new().unwrap();
        let mat_count = worker.materialize_workspace("job-rt", workspace.path()).await.unwrap();
        assert_eq!(mat_count, 1);

        // Add a new file in the workspace.
        tokio::fs::write(workspace.path().join("new_file.txt"), b"brand new").await.unwrap();

        // Sync back.
        let sync_count = worker.sync_workspace("job-rt", workspace.path()).await.unwrap();
        assert_eq!(sync_count, 2); // data.txt + new_file.txt

        // Verify the new file made it to KV.
        use aspen_kv_types::ReadRequest;
        let result = kv_store.read(ReadRequest::new("nanvix/workspaces/job-rt/new_file.txt")).await.unwrap();
        let decoded = base64::engine::general_purpose::STANDARD.decode(&result.kv.unwrap().value).unwrap();
        assert_eq!(decoded, b"brand new");
    }

    #[tokio::test]
    async fn test_walk_dir_recursive() {
        let dir = tempfile::TempDir::new().unwrap();

        // Create nested structure.
        tokio::fs::write(dir.path().join("a.txt"), b"a").await.unwrap();
        let sub = dir.path().join("sub");
        tokio::fs::create_dir_all(&sub).await.unwrap();
        tokio::fs::write(sub.join("b.txt"), b"b").await.unwrap();
        let deep = sub.join("deep");
        tokio::fs::create_dir_all(&deep).await.unwrap();
        tokio::fs::write(deep.join("c.txt"), b"c").await.unwrap();

        let files = walk_dir_recursive(dir.path()).await.unwrap();
        assert_eq!(files.len(), 3);

        // All returned paths should be files that exist.
        for f in &files {
            assert!(f.is_file());
        }
    }
}
