//! Artifact collection from VM workspace.
//!
//! After a CI job completes, this module collects artifacts matching
//! user-specified glob patterns from the workspace directory.

#![allow(dead_code)] // API surface for artifact handling

use std::path::{Path, PathBuf};

use glob::glob;
use snafu::ResultExt;
use tracing::{debug, info, warn};

use super::error::{self, Result};

/// Maximum size of a single artifact file (100 MB).
const MAX_ARTIFACT_SIZE: u64 = 100 * 1024 * 1024;

/// Maximum total size of all artifacts (500 MB).
const MAX_TOTAL_ARTIFACT_SIZE: u64 = 500 * 1024 * 1024;

/// Maximum number of artifact files to collect.
const MAX_ARTIFACT_COUNT: usize = 1000;

/// A collected artifact from the workspace.
#[derive(Debug, Clone)]
pub struct CollectedArtifact {
    /// Relative path within the workspace.
    pub relative_path: PathBuf,

    /// Absolute path on the host filesystem.
    pub absolute_path: PathBuf,

    /// File size in bytes.
    pub size: u64,
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

    /// Total size of collected artifacts.
    pub total_size: u64,
}

/// Collect artifacts matching glob patterns from a workspace directory.
///
/// # Arguments
/// * `workspace_dir` - The host-side workspace directory
/// * `patterns` - Glob patterns relative to the workspace root
///
/// # Returns
/// Result containing collected artifacts and any issues encountered
pub async fn collect_artifacts(workspace_dir: &Path, patterns: &[String]) -> Result<ArtifactCollectionResult> {
    let mut result = ArtifactCollectionResult::default();

    if patterns.is_empty() {
        return Ok(result);
    }

    let workspace_canonical = workspace_dir.canonicalize().unwrap_or_else(|_| workspace_dir.to_path_buf());

    for pattern in patterns {
        let full_pattern = workspace_dir.join(pattern);
        let pattern_str = full_pattern.to_string_lossy();

        let matches = glob(&pattern_str).context(error::GlobPatternSnafu {
            pattern: pattern.clone(),
        })?;

        let mut pattern_matched = false;

        for entry in matches.flatten() {
            pattern_matched = true;

            // Security: ensure the path is within the workspace
            let canonical = match entry.canonicalize() {
                Ok(p) => p,
                Err(e) => {
                    warn!(path = ?entry, error = ?e, "failed to canonicalize artifact path");
                    continue;
                }
            };

            if !canonical.starts_with(&workspace_canonical) {
                warn!(path = ?entry, "artifact path escapes workspace, skipping");
                continue;
            }

            // Skip directories
            if canonical.is_dir() {
                continue;
            }

            // Check file size
            let metadata = match tokio::fs::metadata(&canonical).await {
                Ok(m) => m,
                Err(e) => {
                    warn!(path = ?entry, error = ?e, "failed to read artifact metadata");
                    continue;
                }
            };

            let size = metadata.len();

            // Enforce size limits
            if size > MAX_ARTIFACT_SIZE {
                warn!(
                    path = ?entry,
                    size = size,
                    max = MAX_ARTIFACT_SIZE,
                    "artifact too large, skipping"
                );
                result.skipped_files.push((entry.clone(), size));
                continue;
            }

            if result.total_size + size > MAX_TOTAL_ARTIFACT_SIZE {
                warn!(
                    path = ?entry,
                    current_total = result.total_size,
                    would_exceed = MAX_TOTAL_ARTIFACT_SIZE,
                    "total artifact size limit reached, skipping remaining"
                );
                result.skipped_files.push((entry.clone(), size));
                continue;
            }

            if result.artifacts.len() >= MAX_ARTIFACT_COUNT {
                warn!(max = MAX_ARTIFACT_COUNT, "maximum artifact count reached, skipping remaining");
                result.skipped_files.push((entry.clone(), size));
                continue;
            }

            // Compute relative path
            let relative_path = canonical.strip_prefix(&workspace_canonical).unwrap_or(&canonical).to_path_buf();

            debug!(
                relative = ?relative_path,
                size = size,
                "collected artifact"
            );

            result.artifacts.push(CollectedArtifact {
                relative_path,
                absolute_path: canonical,
                size,
            });
            result.total_size += size;
        }

        if !pattern_matched {
            result.unmatched_patterns.push(pattern.clone());
        }
    }

    info!(
        artifacts_count = result.artifacts.len(),
        total_size = result.total_size,
        unmatched_patterns = result.unmatched_patterns.len(),
        skipped_files = result.skipped_files.len(),
        "artifact collection complete"
    );

    Ok(result)
}

/// Read artifact content into memory.
///
/// Only use this for small artifacts. For larger artifacts,
/// stream directly to blob storage.
pub async fn read_artifact(artifact: &CollectedArtifact) -> Result<Vec<u8>> {
    tokio::fs::read(&artifact.absolute_path).await.context(error::ReadArtifactSnafu {
        path: artifact.absolute_path.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_collect_artifacts_empty_patterns() {
        let temp_dir = TempDir::new().unwrap();
        let result = collect_artifacts(temp_dir.path(), &[]).await.unwrap();

        assert!(result.artifacts.is_empty());
        assert!(result.unmatched_patterns.is_empty());
    }

    #[tokio::test]
    async fn test_collect_artifacts_basic() {
        let temp_dir = TempDir::new().unwrap();

        // Create test files
        let file1 = temp_dir.path().join("output.txt");
        tokio::fs::write(&file1, "hello").await.unwrap();

        let file2 = temp_dir.path().join("result.json");
        tokio::fs::write(&file2, r#"{"status": "ok"}"#).await.unwrap();

        let result = collect_artifacts(temp_dir.path(), &["*.txt".to_string(), "*.json".to_string()]).await.unwrap();

        assert_eq!(result.artifacts.len(), 2);
        assert!(result.unmatched_patterns.is_empty());
    }

    #[tokio::test]
    async fn test_collect_artifacts_unmatched_pattern() {
        let temp_dir = TempDir::new().unwrap();

        let result = collect_artifacts(temp_dir.path(), &["*.nonexistent".to_string()]).await.unwrap();

        assert!(result.artifacts.is_empty());
        assert_eq!(result.unmatched_patterns.len(), 1);
        assert_eq!(result.unmatched_patterns[0], "*.nonexistent");
    }

    #[tokio::test]
    async fn test_collect_artifacts_nested() {
        let temp_dir = TempDir::new().unwrap();

        // Create nested structure
        let sub_dir = temp_dir.path().join("build").join("output");
        tokio::fs::create_dir_all(&sub_dir).await.unwrap();

        let file = sub_dir.join("artifact.bin");
        tokio::fs::write(&file, vec![0u8; 100]).await.unwrap();

        let result = collect_artifacts(temp_dir.path(), &["**/artifact.bin".to_string()]).await.unwrap();

        assert_eq!(result.artifacts.len(), 1);
        assert_eq!(result.artifacts[0].relative_path, PathBuf::from("build/output/artifact.bin"));
    }

    #[tokio::test]
    async fn test_read_artifact() {
        let temp_dir = TempDir::new().unwrap();
        let content = b"artifact content";

        let file = temp_dir.path().join("test.bin");
        tokio::fs::write(&file, content).await.unwrap();

        let result = collect_artifacts(temp_dir.path(), &["test.bin".to_string()]).await.unwrap();

        let artifact = &result.artifacts[0];
        let read_content = read_artifact(artifact).await.unwrap();

        assert_eq!(read_content, content);
    }
}
