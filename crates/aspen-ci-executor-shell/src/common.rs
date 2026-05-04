//! Common utilities for CI workers.
//!
//! This module provides shared functionality that doesn't require HTTP dependencies:
//! - Artifact collection from workspace directories
//! - Workspace seeding from blob store
//! - Source archive creation for VM jobs
//! - Log bridge for real-time CI log streaming to KV store

use std::io::Cursor;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use aspen_blob::BlobRef;
use aspen_blob::prelude::*;
use aspen_ci_core::log_writer::CI_LOG_COMPLETE_STATUS;
use aspen_ci_core::log_writer::CiLogChunk;
use aspen_ci_core::log_writer::CiLogCompleteMarker;
use aspen_core::KeyValueStore;
use aspen_core::WriteRequest;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use glob::glob;
use iroh_blobs::Hash;
use snafu::ResultExt;
use snafu::Snafu;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

// Tiger Style: Bounded resources
/// Maximum size of a single artifact file (100 MB).
const MAX_ARTIFACT_SIZE: u64 = 100_u64 << 20;
/// Maximum total size of all artifacts (500 MB).
const MAX_TOTAL_ARTIFACT_SIZE: u64 = 500_u64 << 20;
/// Maximum number of artifact files to collect.
const MAX_ARTIFACT_COUNT: usize = 1000;
/// Maximum source archive size (1 GB).
const MAX_SOURCE_ARCHIVE_SIZE: u64 = 1_u64 << 30;
/// Maximum files in source archive.
/// Maximum files in source archive.
#[allow(dead_code)]
const MAX_SOURCE_FILES: usize = 100_000;

/// Errors from common CI worker utilities.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
#[allow(missing_docs)] // Snafu errors are documented via display attributes
pub enum WorkerUtilError {
    /// Glob pattern error.
    #[snafu(display("invalid glob pattern '{pattern}': {source}"))]
    GlobPattern {
        pattern: String,
        source: glob::PatternError,
    },

    /// Failed to read artifact file.
    #[snafu(display("failed to read artifact {}: {source}", path.display()))]
    ReadArtifact { path: PathBuf, source: std::io::Error },

    /// Artifact path escapes workspace.
    #[snafu(display("artifact path {} escapes workspace", path.display()))]
    ArtifactEscapesWorkspace { path: PathBuf },

    /// Failed to seed workspace with source.
    #[snafu(display("failed to seed workspace: {reason}"))]
    WorkspaceSeed { reason: String },

    /// Failed to create source archive.
    #[snafu(display("failed to create source archive: {reason}"))]
    SourceArchive { reason: String },

    /// Failed to upload to blob store.
    #[snafu(display("failed to upload to blob store: {reason}"))]
    BlobUpload { reason: String },
}

/// Result type for common CI worker utilities.
pub type Result<T> = std::result::Result<T, WorkerUtilError>;

#[derive(Copy, Clone)]
struct LogChunkTarget<'a> {
    run_id: &'a str,
    job_id: &'a str,
}

fn len_u64(len: usize) -> u64 {
    u64::try_from(len).unwrap_or(u64::MAX)
}

#[allow(unknown_lints)]
#[allow(
    ambient_clock,
    reason = "CI log timestamps must reflect wall-clock time for client streaming"
)]
fn current_time_ms() -> u64 {
    let epoch_elapsed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    u64::try_from(epoch_elapsed.as_millis()).unwrap_or(u64::MAX)
}

fn canonicalize_workspace(workspace_dir: &Path) -> Result<PathBuf> {
    workspace_dir.canonicalize().map_err(|error| WorkerUtilError::WorkspaceSeed {
        reason: format!("failed to canonicalize workspace {}: {}", workspace_dir.display(), error),
    })
}

fn should_skip_artifact(result: &ArtifactCollectionResult, path: &Path, size_bytes: u64) -> bool {
    debug_assert!(path.is_absolute());
    if size_bytes > MAX_ARTIFACT_SIZE {
        return true;
    }
    if result.total_size_bytes.saturating_add(size_bytes) > MAX_TOTAL_ARTIFACT_SIZE {
        return true;
    }
    result.artifacts.len() >= MAX_ARTIFACT_COUNT
}

// ============================================================================
// Artifact Collection
// ============================================================================

/// A collected artifact from the workspace.
#[derive(Debug, Clone)]
pub struct CollectedArtifact {
    /// Relative path within the workspace.
    pub relative_path: PathBuf,

    /// Absolute path on the host filesystem.
    pub absolute_path: PathBuf,

    /// File size in bytes.
    pub size_bytes: u64,
}

/// Result of artifact collection.
#[derive(Debug, Default)]
pub struct ArtifactCollectionResult {
    /// Successfully collected artifacts.
    pub artifacts: Vec<CollectedArtifact>,

    /// Patterns that didn't match any files.
    pub unmatched_patterns: Vec<String>,

    /// Files skipped due to size limits.
    pub skipped_files: Vec<(PathBuf, u64)>,

    /// Total size of collected artifacts (bytes).
    pub total_size_bytes: u64,
}

/// An artifact that has been uploaded to the blob store.
#[derive(Debug, Clone)]
pub struct UploadedArtifact {
    /// Relative path within the workspace.
    pub relative_path: PathBuf,

    /// Reference to the uploaded blob.
    pub blob_ref: BlobRef,
}

/// Result of artifact upload.
#[derive(Debug, Default)]
pub struct ArtifactUploadResult {
    /// Successfully uploaded artifacts.
    pub uploaded: Vec<UploadedArtifact>,

    /// Artifacts that failed to upload.
    pub failed: Vec<(PathBuf, String)>,

    /// Total bytes uploaded.
    pub total_bytes: u64,
}

/// Collect artifacts from a workspace directory.
///
/// Matches files against glob patterns and collects metadata.
/// Files exceeding size limits are skipped.
pub async fn collect_artifacts(workspace_dir: &Path, patterns: &[String]) -> Result<ArtifactCollectionResult> {
    let mut result = ArtifactCollectionResult::default();
    let workspace_canonical = canonicalize_workspace(workspace_dir)?;

    for pattern in patterns {
        let full_pattern = workspace_dir.join(pattern);
        let pattern_str = full_pattern.to_string_lossy();
        let entries = glob(&pattern_str).context(GlobPatternSnafu {
            pattern: pattern.clone(),
        })?;

        let mut has_pattern_match = false;
        for entry in entries.flatten() {
            has_pattern_match = true;
            let canonical = match entry.canonicalize() {
                Ok(path) => path,
                Err(error) => {
                    warn!(path = ?entry, error = ?error, "failed to canonicalize artifact path");
                    continue;
                }
            };
            if !canonical.starts_with(&workspace_canonical) {
                return ArtifactEscapesWorkspaceSnafu { path: entry }.fail();
            }

            let metadata = match tokio::fs::metadata(&canonical).await {
                Ok(metadata) => metadata,
                Err(error) => {
                    warn!(path = ?canonical, error = ?error, "failed to stat artifact");
                    continue;
                }
            };
            if !metadata.is_file() {
                continue;
            }

            let size_bytes = metadata.len();
            if should_skip_artifact(&result, &canonical, size_bytes) {
                result.skipped_files.push((canonical.clone(), size_bytes));
                continue;
            }

            let relative_path = canonical.strip_prefix(&workspace_canonical).unwrap_or(&canonical).to_path_buf();
            result.artifacts.push(CollectedArtifact {
                relative_path,
                absolute_path: canonical,
                size_bytes,
            });
            result.total_size_bytes = result.total_size_bytes.saturating_add(size_bytes);
        }

        if !has_pattern_match {
            result.unmatched_patterns.push(pattern.clone());
        }
    }

    debug_assert!(result.total_size_bytes <= MAX_TOTAL_ARTIFACT_SIZE);
    debug_assert!(result.artifacts.len() <= MAX_ARTIFACT_COUNT);
    debug!(
        artifacts = result.artifacts.len(),
        total_size_bytes = result.total_size_bytes,
        skipped = result.skipped_files.len(),
        unmatched = result.unmatched_patterns.len(),
        "artifact collection complete"
    );
    Ok(result)
}

/// Upload collected artifacts to the blob store.
pub async fn upload_artifacts_to_blob_store(
    collected: &ArtifactCollectionResult,
    blob_store: &Arc<dyn BlobStore>,
    job_id: &str,
) -> ArtifactUploadResult {
    let mut result = ArtifactUploadResult::default();

    for artifact in &collected.artifacts {
        match tokio::fs::read(&artifact.absolute_path).await {
            Ok(content) => match blob_store.add_bytes(&content).await {
                Ok(add_result) => {
                    let content_size_bytes = len_u64(content.len());
                    info!(
                        job_id = %job_id,
                        path = ?artifact.relative_path,
                        hash = %add_result.blob_ref.hash.to_hex(),
                        size_bytes = content_size_bytes,
                        "uploaded artifact to blob store"
                    );
                    result.total_bytes = result.total_bytes.saturating_add(content_size_bytes);
                    result.uploaded.push(UploadedArtifact {
                        relative_path: artifact.relative_path.clone(),
                        blob_ref: add_result.blob_ref,
                    });
                }
                Err(error) => {
                    error!(
                        job_id = %job_id,
                        path = ?artifact.relative_path,
                        error = ?error,
                        "failed to upload artifact to blob store"
                    );
                    result.failed.push((artifact.relative_path.clone(), error.to_string()));
                }
            },
            Err(error) => {
                error!(
                    job_id = %job_id,
                    path = ?artifact.absolute_path,
                    error = ?error,
                    "failed to read artifact file"
                );
                result.failed.push((artifact.relative_path.clone(), error.to_string()));
            }
        }
    }

    debug_assert!(result.uploaded.len().saturating_add(result.failed.len()) <= collected.artifacts.len());
    debug_assert!(result.total_bytes <= MAX_TOTAL_ARTIFACT_SIZE);
    result
}

// ============================================================================
// Workspace Seeding
// ============================================================================

/// Seed a workspace directory from a blob.
///
/// The blob is expected to contain a tar.gz archive of the source tree.
/// The archive is extracted to the workspace directory.
///
/// # Arguments
/// * `blob_store` - The blob store to download from
/// * `source_hash` - The hex-encoded blake3 hash of the source blob
/// * `workspace_dir` - The directory to extract to
///
/// # Returns
/// Ok(bytes_written) on success, or an error if seeding failed
pub async fn seed_workspace_from_blob(
    blob_store: &Arc<dyn BlobStore>,
    source_hash: &str,
    workspace_dir: &Path,
) -> Result<u64> {
    let hash = parse_hash(source_hash)?;
    info!(hash = %source_hash, workspace = ?workspace_dir, "seeding workspace from blob");

    let content = blob_store.get_bytes(&hash).await.map_err(|e| WorkerUtilError::WorkspaceSeed {
        reason: format!("failed to download blob {}: {}", source_hash, e),
    })?;
    let content = content.ok_or_else(|| WorkerUtilError::WorkspaceSeed {
        reason: format!("blob {} not found in store", source_hash),
    })?;
    let content_len = len_u64(content.len());
    tokio::fs::create_dir_all(workspace_dir).await.map_err(|e| WorkerUtilError::WorkspaceSeed {
        reason: format!("failed to create workspace directory: {}", e),
    })?;

    let workspace_path = workspace_dir.to_path_buf();
    let bytes_extracted = tokio::task::spawn_blocking(move || {
        let cursor = Cursor::new(&content);
        let decoder = GzDecoder::new(cursor);
        let mut archive = tar::Archive::new(decoder);
        archive.set_preserve_permissions(false);
        archive.set_unpack_xattrs(false);
        archive.set_preserve_mtime(false);
        archive.set_overwrite(true);

        let mut extracted = 0u64;
        let mut skipped = 0u64;
        for entry_result in archive.entries().map_err(|e| WorkerUtilError::WorkspaceSeed {
            reason: format!("failed to read tar entries: {}", e),
        })? {
            let mut entry = match entry_result {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!(error = %e, "skipping unreadable tar entry");
                    skipped += 1;
                    continue;
                }
            };

            let path = entry.path().map(|p| p.to_path_buf()).unwrap_or_default();
            let skip_prefixes: &[&str] = &["target", "./target", ".git", "./.git"];
            if skip_prefixes.iter().any(|p| path.starts_with(p)) {
                if let Err(error) = std::io::copy(&mut entry, &mut std::io::sink()) {
                    tracing::warn!(path = %path.display(), error = %error, "failed to drain skipped tar entry");
                }
                skipped += 1;
                continue;
            }

            if let Err(e) = entry.unpack_in(&workspace_path) {
                tracing::warn!(path = %path.display(), error = %e, kind = ?e.kind(), "skipping tar entry that failed to extract");
                skipped += 1;
                continue;
            }
            extracted += 1;
        }

        tracing::info!(extracted, skipped, "tar extraction complete");
        Ok::<u64, WorkerUtilError>(content_len)
    })
    .await
    .map_err(|e| WorkerUtilError::WorkspaceSeed {
        reason: format!("extract task panicked: {}", e),
    })??;

    debug_assert!(bytes_extracted <= content_len);
    debug_assert!(workspace_dir.exists());
    debug!(hash = %source_hash, bytes = bytes_extracted, "workspace seeded successfully");
    Ok(bytes_extracted)
}

/// Parse a hex-encoded blake3 hash.
fn parse_hash(hex_hash: &str) -> Result<Hash> {
    let bytes: [u8; 32] = hex::decode(hex_hash)
        .map_err(|e| WorkerUtilError::WorkspaceSeed {
            reason: format!("invalid hash hex: {}", e),
        })?
        .try_into()
        .map_err(|v: Vec<u8>| WorkerUtilError::WorkspaceSeed {
            reason: format!("hash must be 32 bytes, got {} bytes", v.len()),
        })?;

    Ok(Hash::from_bytes(bytes))
}

// ============================================================================
// Source Archive Creation
// ============================================================================

/// Create a tar.gz archive of a source directory and upload to blob store.
///
/// This is used to package the checkout directory for VM jobs that need
/// to download the source from the blob store (since VMs can't access
/// host paths directly).
///
/// # Arguments
/// * `source_dir` - The directory to archive
/// * `blob_store` - The blob store to upload to
///
/// # Returns
/// The hex-encoded hash of the uploaded archive
pub async fn create_source_archive(source_dir: &Path, blob_store: &Arc<dyn BlobStore>) -> Result<String> {
    info!(source = ?source_dir, "creating source archive for VM jobs");

    // Create archive in a blocking task since tar is synchronous
    let source_path = source_dir.to_path_buf();
    let archive_bytes = tokio::task::spawn_blocking(move || {
        let mut buffer = Vec::new();
        {
            let encoder = GzEncoder::new(&mut buffer, Compression::fast());
            let mut archive = tar::Builder::new(encoder);

            // Add files from source directory, skipping directories that aren't
            // needed for CI builds and cause problems on virtiofs mounts:
            // - .git/: corrupted git index causes nix to fail
            // - target/: cargo build artifacts with hard links
            // - build-plan*.json: large cargo artifacts that cause I/O errors
            for entry in walkdir::WalkDir::new(&source_path).into_iter().filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                // Skip .git and target directories entirely
                !(name == ".git" || name == "target")
            }) {
                let entry = entry.map_err(|e| WorkerUtilError::SourceArchive {
                    reason: format!("failed to walk source directory: {}", e),
                })?;

                let name = entry.file_name().to_string_lossy();
                // Skip build-plan JSON files (cargo artifacts)
                if name.starts_with("build-plan") && name.ends_with(".json") {
                    continue;
                }

                let rel_path = entry.path().strip_prefix(&source_path).unwrap_or(entry.path());
                // Skip the root directory entry itself
                if rel_path == std::path::Path::new("") {
                    continue;
                }

                let rel_str = format!("./{}", rel_path.display());
                archive.append_path_with_name(entry.path(), &rel_str).map_err(|e| WorkerUtilError::SourceArchive {
                    reason: format!("failed to add {} to archive: {}", rel_str, e),
                })?;
            }

            archive
                .into_inner()
                .map_err(|e| WorkerUtilError::SourceArchive {
                    reason: format!("failed to finalize archive: {}", e),
                })?
                .finish()
                .map_err(|e| WorkerUtilError::SourceArchive {
                    reason: format!("failed to finish gzip: {}", e),
                })?;
        }

        // Check size limit
        let archive_size_bytes = len_u64(buffer.len());
        if archive_size_bytes > MAX_SOURCE_ARCHIVE_SIZE {
            return Err(WorkerUtilError::SourceArchive {
                reason: format!(
                    "source archive too large: {} bytes (max: {} bytes)",
                    archive_size_bytes, MAX_SOURCE_ARCHIVE_SIZE
                ),
            });
        }

        debug_assert!(archive_size_bytes <= MAX_SOURCE_ARCHIVE_SIZE);
        debug_assert!(!buffer.is_empty() || source_path.exists());
        Ok(buffer)
    })
    .await
    .map_err(|e| WorkerUtilError::SourceArchive {
        reason: format!("archive task panicked: {}", e),
    })??;

    let archive_size_bytes = archive_bytes.len();

    // Upload to blob store
    let add_result = blob_store.add_bytes(&archive_bytes).await.map_err(|e| WorkerUtilError::BlobUpload {
        reason: format!("failed to upload source archive: {}", e),
    })?;

    let hash_hex = add_result.blob_ref.hash.to_hex().to_string();

    info!(
        source = ?source_dir,
        hash = %hash_hex,
        size_bytes = archive_size_bytes,
        "created and uploaded source archive"
    );

    Ok(hash_hex)
}

// ============================================================================
// Log Bridge — shared CI log streaming for all executor workers
// ============================================================================

/// Completion marker suffix.
pub use aspen_core::CI_LOG_COMPLETE_MARKER;
/// KV key prefix for CI log chunks.
pub use aspen_core::CI_LOG_KV_PREFIX;
/// Maximum bytes buffered before flushing a chunk.
pub const LOG_FLUSH_THRESHOLD: usize = 8 * 1024;
/// Periodic flush interval in milliseconds.
/// Ensures partial buffers are written to KV for real-time streaming,
/// even when output is sparse and doesn't fill a full chunk.
pub const LOG_FLUSH_INTERVAL_MS: u64 = aspen_core::CI_LOG_FLUSH_INTERVAL_MS;

/// Bridge task: reads log lines from a channel, buffers them, and writes
/// log chunks to KV. Flushes on size threshold OR periodic timer to ensure
/// real-time streaming even with sparse output.
///
/// Used by both `NixBuildWorker` and `LocalExecutorWorker` for consistent
/// CI log streaming.
pub async fn log_bridge(
    mut rx: tokio::sync::mpsc::Receiver<String>,
    kv_store: Arc<dyn KeyValueStore>,
    run_id: String,
    job_id: String,
) {
    use std::time::Duration;

    use tokio::time::interval;

    let mut chunk_index: u32 = 0;
    let mut buffer = String::new();
    let mut flush_timer = interval(Duration::from_millis(LOG_FLUSH_INTERVAL_MS));
    let log_target = LogChunkTarget {
        run_id: &run_id,
        job_id: &job_id,
    };

    // Skip the immediate first tick
    flush_timer.tick().await;

    loop {
        tokio::select! {
            biased;

            msg = rx.recv() => {
                match msg {
                    Some(line) => {
                        buffer.push_str(&line);
                        if buffer.len() >= LOG_FLUSH_THRESHOLD {
                            flush_log_chunk(log_target, &kv_store, &mut chunk_index, &mut buffer).await;
                        }
                    }
                    None => break,
                }
            }
            _ = flush_timer.tick() => {
                if !buffer.is_empty() {
                    flush_log_chunk(log_target, &kv_store, &mut chunk_index, &mut buffer).await;
                }
            }
        }
    }

    if !buffer.is_empty() {
        flush_log_chunk(log_target, &kv_store, &mut chunk_index, &mut buffer).await;
    }

    let marker_key = format!("{CI_LOG_KV_PREFIX}{run_id}:{job_id}:{CI_LOG_COMPLETE_MARKER}");
    let marker = CiLogCompleteMarker {
        total_chunks: chunk_index,
        timestamp_ms: current_time_ms(),
        status: CI_LOG_COMPLETE_STATUS.to_string(),
    };
    if let Ok(json) = serde_json::to_string(&marker) {
        if let Err(error) = kv_store.write(WriteRequest::set(marker_key, json)).await {
            tracing::warn!(run_id = %run_id, job_id = %job_id, error = %error, "failed to write log completion marker");
        }
    }

    debug_assert!(buffer.is_empty());
    debug_assert!(chunk_index <= u32::MAX);
}

/// Flush the current buffer as a log chunk to KV.
pub async fn log_bridge_flush_chunk(
    kv_store: &Arc<dyn KeyValueStore>,
    run_id: &str,
    job_id_value: impl AsRef<str>,
    chunk_index: &mut u32,
    buffer: &mut String,
) {
    flush_log_chunk(
        LogChunkTarget {
            run_id,
            job_id: job_id_value.as_ref(),
        },
        kv_store,
        chunk_index,
        buffer,
    )
    .await;
}

async fn flush_log_chunk(
    target: LogChunkTarget<'_>,
    kv_store: &Arc<dyn KeyValueStore>,
    chunk_index: &mut u32,
    buffer: &mut String,
) {
    let chunk = CiLogChunk {
        index: *chunk_index,
        content: buffer.clone(),
        timestamp_ms: current_time_ms(),
    };

    if let Ok(json) = serde_json::to_string(&chunk) {
        let key = format!("{CI_LOG_KV_PREFIX}{}:{}:{:010}", target.run_id, target.job_id, *chunk_index);
        if let Err(error) = kv_store.write(WriteRequest::set(key, json)).await {
            tracing::warn!(run_id = %target.run_id, job_id = %target.job_id, chunk = *chunk_index, error = %error, "Failed to write log chunk");
        }
    }

    *chunk_index = chunk_index.saturating_add(1);
    buffer.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ci_log_constants_follow_core_source_of_truth() {
        assert_eq!(CI_LOG_KV_PREFIX, aspen_core::CI_LOG_KV_PREFIX);
        assert_eq!(CI_LOG_COMPLETE_MARKER, aspen_core::CI_LOG_COMPLETE_MARKER);
        assert_eq!(LOG_FLUSH_INTERVAL_MS, aspen_core::CI_LOG_FLUSH_INTERVAL_MS);
    }

    #[test]
    fn completion_marker_uses_log_stream_status_not_job_status() {
        let marker = CiLogCompleteMarker {
            total_chunks: 2,
            timestamp_ms: 42,
            status: CI_LOG_COMPLETE_STATUS.to_string(),
        };

        assert_eq!(marker.status, CI_LOG_COMPLETE_STATUS);
        assert_ne!(marker.status, "success");
        assert_ne!(marker.status, "failed");
        assert_ne!(marker.status, "cancelled");
    }
}
