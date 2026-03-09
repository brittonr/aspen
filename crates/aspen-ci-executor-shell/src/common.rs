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
const MAX_ARTIFACT_SIZE: u64 = 100 * 1024 * 1024;
/// Maximum total size of all artifacts (500 MB).
const MAX_TOTAL_ARTIFACT_SIZE: u64 = 500 * 1024 * 1024;
/// Maximum number of artifact files to collect.
const MAX_ARTIFACT_COUNT: usize = 1000;
/// Maximum source archive size (1 GB).
const MAX_SOURCE_ARCHIVE_SIZE: u64 = 1024 * 1024 * 1024;
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

    for pattern in patterns {
        let full_pattern = workspace_dir.join(pattern);
        let pattern_str = full_pattern.to_string_lossy();

        let entries = glob(&pattern_str).context(GlobPatternSnafu {
            pattern: pattern.clone(),
        })?;

        let mut matched = false;
        for entry in entries.flatten() {
            matched = true;

            // Ensure path is within workspace (prevent path traversal)
            let canonical = match entry.canonicalize() {
                Ok(p) => p,
                Err(e) => {
                    warn!(path = ?entry, error = ?e, "failed to canonicalize artifact path");
                    continue;
                }
            };

            let workspace_canonical = match workspace_dir.canonicalize() {
                Ok(p) => p,
                Err(e) => {
                    warn!(workspace = ?workspace_dir, error = ?e, "failed to canonicalize workspace");
                    continue;
                }
            };

            if !canonical.starts_with(&workspace_canonical) {
                return ArtifactEscapesWorkspaceSnafu { path: entry }.fail();
            }

            // Get file metadata
            let metadata = match tokio::fs::metadata(&canonical).await {
                Ok(m) => m,
                Err(e) => {
                    warn!(path = ?canonical, error = ?e, "failed to stat artifact");
                    continue;
                }
            };

            if !metadata.is_file() {
                continue;
            }

            let size = metadata.len();

            // Check size limits
            if size > MAX_ARTIFACT_SIZE {
                result.skipped_files.push((canonical.clone(), size));
                continue;
            }

            if result.total_size_bytes + size > MAX_TOTAL_ARTIFACT_SIZE {
                result.skipped_files.push((canonical.clone(), size));
                continue;
            }

            if result.artifacts.len() >= MAX_ARTIFACT_COUNT {
                result.skipped_files.push((canonical.clone(), size));
                continue;
            }

            let relative_path = canonical.strip_prefix(&workspace_canonical).unwrap_or(&canonical).to_path_buf();

            result.artifacts.push(CollectedArtifact {
                relative_path,
                absolute_path: canonical,
                size_bytes: size,
            });
            result.total_size_bytes += size;
        }

        if !matched {
            result.unmatched_patterns.push(pattern.clone());
        }
    }

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
                    info!(
                        job_id = %job_id,
                        path = ?artifact.relative_path,
                        hash = %add_result.blob_ref.hash.to_hex(),
                        size = content.len(),
                        "uploaded artifact to blob store"
                    );
                    result.total_bytes += content.len() as u64;
                    result.uploaded.push(UploadedArtifact {
                        relative_path: artifact.relative_path.clone(),
                        blob_ref: add_result.blob_ref,
                    });
                }
                Err(e) => {
                    error!(
                        job_id = %job_id,
                        path = ?artifact.relative_path,
                        error = ?e,
                        "failed to upload artifact to blob store"
                    );
                    result.failed.push((artifact.relative_path.clone(), e.to_string()));
                }
            },
            Err(e) => {
                error!(
                    job_id = %job_id,
                    path = ?artifact.absolute_path,
                    error = ?e,
                    "failed to read artifact file"
                );
                result.failed.push((artifact.relative_path.clone(), e.to_string()));
            }
        }
    }

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
    // Parse the hash
    let hash = parse_hash(source_hash)?;

    info!(
        hash = %source_hash,
        workspace = ?workspace_dir,
        "seeding workspace from blob"
    );

    // Download the blob content
    let content = blob_store.get_bytes(&hash).await.map_err(|e| WorkerUtilError::WorkspaceSeed {
        reason: format!("failed to download blob {}: {}", source_hash, e),
    })?;

    let content = content.ok_or_else(|| WorkerUtilError::WorkspaceSeed {
        reason: format!("blob {} not found in store", source_hash),
    })?;

    let content_len = content.len() as u64;

    // Ensure workspace directory exists
    tokio::fs::create_dir_all(workspace_dir).await.map_err(|e| WorkerUtilError::WorkspaceSeed {
        reason: format!("failed to create workspace directory: {}", e),
    })?;

    // Decompress and extract the tar.gz archive
    // This runs in a blocking context since tar is synchronous
    let workspace_path = workspace_dir.to_path_buf();
    let bytes_extracted = tokio::task::spawn_blocking(move || {
        let cursor = Cursor::new(&content);
        let decoder = GzDecoder::new(cursor);
        let mut archive = tar::Archive::new(decoder);

        // Don't try to preserve ownership or permissions — the workspace
        // is on a virtiofs mount inside a VM where chown/chmod may fail.
        archive.set_preserve_permissions(false);
        archive.set_unpack_xattrs(false);
        archive.set_preserve_mtime(false);
        archive.set_overwrite(true);

        // Extract entry-by-entry to handle errors gracefully.
        // Some entries (device files, hard links across mounts) may fail
        // on virtiofs — skip those and continue with the rest.
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

            // Skip directories that aren't needed for the build and cause
            // issues in VMs:
            // - target/: cargo build artifacts with hard links that fail on virtiofs
            // - .git/: nix tries `git ls-files` when .git exists, which can fail if the checkout from Forge has
            //   an incomplete git repo. Without .git nix uses builtins.path which hashes the directory
            //   directly.
            let skip_prefixes: &[&str] = &["target", "./target", ".git", "./.git"];
            if skip_prefixes.iter().any(|p| path.starts_with(p)) {
                // Drain the entry so the reader advances
                let _ = std::io::copy(&mut entry, &mut std::io::sink());
                skipped += 1;
                continue;
            }

            if let Err(e) = entry.unpack_in(&workspace_path) {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    kind = ?e.kind(),
                    "skipping tar entry that failed to extract"
                );
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

    debug!(
        hash = %source_hash,
        bytes = bytes_extracted,
        "workspace seeded successfully"
    );

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
        if buffer.len() as u64 > MAX_SOURCE_ARCHIVE_SIZE {
            return Err(WorkerUtilError::SourceArchive {
                reason: format!(
                    "source archive too large: {} bytes (max: {} bytes)",
                    buffer.len(),
                    MAX_SOURCE_ARCHIVE_SIZE
                ),
            });
        }

        Ok(buffer)
    })
    .await
    .map_err(|e| WorkerUtilError::SourceArchive {
        reason: format!("archive task panicked: {}", e),
    })??;

    let archive_size = archive_bytes.len();

    // Upload to blob store
    let add_result = blob_store.add_bytes(&archive_bytes).await.map_err(|e| WorkerUtilError::BlobUpload {
        reason: format!("failed to upload source archive: {}", e),
    })?;

    let hash_hex = add_result.blob_ref.hash.to_hex().to_string();

    info!(
        source = ?source_dir,
        hash = %hash_hex,
        size = archive_size,
        "created and uploaded source archive"
    );

    Ok(hash_hex)
}

// ============================================================================
// Log Bridge — shared CI log streaming for all executor workers
// ============================================================================

/// KV key prefix for CI log chunks.
pub const CI_LOG_KV_PREFIX: &str = "_ci:logs:";
/// Completion marker suffix.
pub const CI_LOG_COMPLETE_MARKER: &str = "__complete__";
/// Maximum bytes buffered before flushing a chunk.
pub const LOG_FLUSH_THRESHOLD: usize = 8 * 1024;
/// Periodic flush interval in milliseconds.
/// Ensures partial buffers are written to KV for real-time streaming,
/// even when output is sparse and doesn't fill a full chunk.
pub const LOG_FLUSH_INTERVAL_MS: u64 = 500;

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
    let mut flush_interval = interval(Duration::from_millis(LOG_FLUSH_INTERVAL_MS));

    // Skip the immediate first tick
    flush_interval.tick().await;

    loop {
        tokio::select! {
            biased;

            msg = rx.recv() => {
                match msg {
                    Some(line) => {
                        buffer.push_str(&line);

                        // Flush when buffer exceeds threshold
                        if buffer.len() >= LOG_FLUSH_THRESHOLD {
                            log_bridge_flush_chunk(&kv_store, &run_id, &job_id, &mut chunk_index, &mut buffer).await;
                        }
                    }
                    None => {
                        // Channel closed — job finished
                        break;
                    }
                }
            }
            _ = flush_interval.tick() => {
                // Periodic flush: write partial buffer to KV so streaming
                // clients see output in real-time, not just at 8KB boundaries.
                if !buffer.is_empty() {
                    log_bridge_flush_chunk(&kv_store, &run_id, &job_id, &mut chunk_index, &mut buffer).await;
                }
            }
        }
    }

    // Flush remaining buffer
    if !buffer.is_empty() {
        log_bridge_flush_chunk(&kv_store, &run_id, &job_id, &mut chunk_index, &mut buffer).await;
    }

    // Write completion marker
    let marker_key = format!("{CI_LOG_KV_PREFIX}{run_id}:{job_id}:{CI_LOG_COMPLETE_MARKER}");
    let now_ms =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
    let marker = CiLogCompleteMarker {
        total_chunks: chunk_index,
        timestamp_ms: now_ms,
        status: "done".to_string(),
    };
    if let Ok(json) = serde_json::to_string(&marker) {
        let _ = kv_store.write(WriteRequest::set(marker_key, json)).await;
    }
}

/// Flush the current buffer as a log chunk to KV.
pub async fn log_bridge_flush_chunk(
    kv_store: &Arc<dyn KeyValueStore>,
    run_id: &str,
    job_id: &str,
    chunk_index: &mut u32,
    buffer: &mut String,
) {
    let now_ms =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;

    let chunk = CiLogChunk {
        index: *chunk_index,
        content: buffer.clone(),
        timestamp_ms: now_ms,
    };

    if let Ok(json) = serde_json::to_string(&chunk) {
        let key = format!("{CI_LOG_KV_PREFIX}{run_id}:{job_id}:{:010}", *chunk_index);
        if let Err(e) = kv_store.write(WriteRequest::set(key, json)).await {
            tracing::warn!(run_id = %run_id, job_id = %job_id, chunk = *chunk_index, error = %e, "Failed to write log chunk");
        }
    }

    *chunk_index += 1;
    buffer.clear();
}
