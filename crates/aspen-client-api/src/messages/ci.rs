//! CI/CD operation types.
//!
//! Request/response types for CI pipeline management, execution, Nix binary cache,
//! SNIX storage, and cache migration operations.

use serde::Deserialize;
use serde::Serialize;

/// CI domain request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CiRequest {
    /// Trigger a CI pipeline run for a repository.
    CiTriggerPipeline {
        repo_id: String,
        ref_name: String,
        commit_hash: Option<String>,
    },
    /// Get pipeline run status and details.
    CiGetStatus { run_id: String },
    /// List pipeline runs with optional filtering.
    CiListRuns {
        repo_id: Option<String>,
        status: Option<String>,
        limit: Option<u32>,
    },
    /// Cancel a running pipeline.
    CiCancelRun { run_id: String, reason: Option<String> },
    /// Watch a repository for CI triggers.
    CiWatchRepo { repo_id: String },
    /// Unwatch a repository.
    CiUnwatchRepo { repo_id: String },
    /// List artifacts for a CI job.
    CiListArtifacts { job_id: String, run_id: Option<String> },
    /// Get artifact metadata and download ticket.
    CiGetArtifact { blob_hash: String },
    /// Get historical logs for a CI job.
    CiGetJobLogs {
        run_id: String,
        job_id: String,
        start_index: u32,
        limit: Option<u32>,
    },
    /// Subscribe to real-time logs for a CI job.
    CiSubscribeLogs {
        run_id: String,
        job_id: String,
        from_index: Option<u64>,
    },
    /// Get full job output (stdout/stderr).
    CiGetJobOutput { run_id: String, job_id: String },

    // Nix Binary Cache operations
    /// Query the Nix binary cache for a store path.
    CacheQuery { store_hash: String },
    /// Get cache statistics.
    CacheStats,
    /// Get a blob ticket for downloading a NAR.
    CacheDownload { store_hash: String },

    // SNIX operations (for remote workers)
    /// Get a directory from SNIX DirectoryService.
    SnixDirectoryGet { digest: String },
    /// Put a directory to SNIX DirectoryService.
    SnixDirectoryPut { directory_bytes: String },
    /// Get path info from SNIX PathInfoService.
    SnixPathInfoGet { digest: String },
    /// Put path info to SNIX PathInfoService.
    SnixPathInfoPut { pathinfo_bytes: String },

    // Cache Migration operations (feature-gated at variant level)
    /// Start cache migration from legacy to SNIX format.
    #[cfg(feature = "ci")]
    CacheMigrationStart {
        batch_size: Option<u32>,
        batch_delay_ms: Option<u64>,
        dry_run: bool,
    },
    /// Get cache migration status.
    #[cfg(feature = "ci")]
    CacheMigrationStatus,
    /// Cancel an in-progress cache migration.
    #[cfg(feature = "ci")]
    CacheMigrationCancel,
    /// Validate cache migration completeness.
    #[cfg(feature = "ci")]
    CacheMigrationValidate { max_report: Option<u32> },
}

impl CiRequest {
    /// Convert to an authorization operation.
    pub fn to_operation(&self) -> Option<aspen_auth::Operation> {
        use aspen_auth::Operation;
        match self {
            Self::CiGetStatus { run_id } => Some(Operation::Read {
                key: format!("_ci:runs:{}", run_id),
            }),
            Self::CiListRuns { repo_id, .. } => Some(Operation::Read {
                key: format!("_ci:runs:{}", repo_id.as_deref().unwrap_or("")),
            }),
            Self::CiTriggerPipeline { repo_id, .. }
            | Self::CiWatchRepo { repo_id }
            | Self::CiUnwatchRepo { repo_id } => Some(Operation::Write {
                key: format!("_ci:repos:{}", repo_id),
                value: vec![],
            }),
            Self::CiCancelRun { run_id, .. } => Some(Operation::Write {
                key: format!("_ci:runs:{}", run_id),
                value: vec![],
            }),
            Self::CiListArtifacts { job_id, run_id } => Some(Operation::Read {
                key: format!("_ci:artifacts:{}:{}", job_id, run_id.as_deref().unwrap_or("")),
            }),
            Self::CiGetArtifact { blob_hash } => Some(Operation::Read {
                key: format!("_ci:artifacts:{}", blob_hash),
            }),
            Self::CiGetJobLogs { run_id, job_id, .. } => Some(Operation::Read {
                key: format!("_ci:logs:{}:{}", run_id, job_id),
            }),
            Self::CiSubscribeLogs { run_id, job_id, .. } => Some(Operation::Read {
                key: format!("_ci:logs:{}:{}", run_id, job_id),
            }),
            Self::CiGetJobOutput { run_id, job_id } => Some(Operation::Read {
                key: format!("_ci:runs:{}:{}", run_id, job_id),
            }),

            // Cache operations
            Self::CacheQuery { store_hash } | Self::CacheDownload { store_hash } => Some(Operation::Read {
                key: format!("_cache:narinfo:{store_hash}"),
            }),
            Self::CacheStats => Some(Operation::Read {
                key: "_cache:stats".to_string(),
            }),

            // SNIX operations
            Self::SnixDirectoryGet { digest } => Some(Operation::Read {
                key: format!("snix:dir:{digest}"),
            }),
            Self::SnixDirectoryPut { .. } => Some(Operation::Write {
                key: "snix:dir:".to_string(),
                value: vec![],
            }),
            Self::SnixPathInfoGet { digest } => Some(Operation::Read {
                key: format!("snix:pathinfo:{digest}"),
            }),
            Self::SnixPathInfoPut { .. } => Some(Operation::Write {
                key: "snix:pathinfo:".to_string(),
                value: vec![],
            }),

            // Cache migration operations
            #[cfg(feature = "ci")]
            Self::CacheMigrationStart { .. } | Self::CacheMigrationCancel => Some(Operation::ClusterAdmin {
                action: "cache_migration".to_string(),
            }),
            #[cfg(feature = "ci")]
            Self::CacheMigrationStatus | Self::CacheMigrationValidate { .. } => Some(Operation::Read {
                key: "_cache:migration:".to_string(),
            }),
        }
    }
}

/// CI trigger pipeline response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiTriggerPipelineResponse {
    /// Whether the trigger was successful.
    pub is_success: bool,
    /// Pipeline run ID (if successful).
    pub run_id: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// CI pipeline stage information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiStageInfo {
    /// Stage name.
    pub name: String,
    /// Stage status: pending, running, succeeded, failed, cancelled.
    pub status: String,
    /// Jobs in this stage.
    pub jobs: Vec<CiJobInfo>,
}

/// CI pipeline job information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiJobInfo {
    /// Job ID.
    pub id: String,
    /// Job name.
    pub name: String,
    /// Job status: pending, running, succeeded, failed, cancelled.
    pub status: String,
    /// Job start time (Unix timestamp in milliseconds).
    pub started_at_ms: Option<u64>,
    /// Job end time (Unix timestamp in milliseconds).
    pub ended_at_ms: Option<u64>,
    /// Error message if job failed.
    pub error: Option<String>,
}

/// CI get status response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiGetStatusResponse {
    /// Whether the pipeline run was found.
    pub was_found: bool,
    /// Pipeline run ID.
    pub run_id: Option<String>,
    /// Repository ID.
    pub repo_id: Option<String>,
    /// Git reference.
    pub ref_name: Option<String>,
    /// Commit hash.
    pub commit_hash: Option<String>,
    /// Pipeline status: pending, running, succeeded, failed, cancelled.
    pub status: Option<String>,
    /// Stage information.
    pub stages: Vec<CiStageInfo>,
    /// Creation time (Unix timestamp in milliseconds).
    pub created_at_ms: Option<u64>,
    /// Completion time (Unix timestamp in milliseconds).
    pub completed_at_ms: Option<u64>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// CI pipeline run summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiRunInfo {
    /// Pipeline run ID.
    pub run_id: String,
    /// Repository ID.
    pub repo_id: String,
    /// Git reference.
    pub ref_name: String,
    /// Pipeline status.
    pub status: String,
    /// Creation time (Unix timestamp in milliseconds).
    pub created_at_ms: u64,
}

/// CI list runs response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiListRunsResponse {
    /// Pipeline runs.
    pub runs: Vec<CiRunInfo>,
}

/// CI cancel run response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiCancelRunResponse {
    /// Whether the cancel was successful.
    pub is_success: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// CI watch repo response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiWatchRepoResponse {
    /// Whether the watch was successful.
    pub is_success: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// CI unwatch repo response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiUnwatchRepoResponse {
    /// Whether the unwatch was successful.
    pub is_success: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Information about a CI artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiArtifactInfo {
    /// Blob hash in the distributed store.
    pub blob_hash: String,
    /// Artifact name (e.g., store path for Nix builds).
    pub name: String,
    /// Size in bytes.
    pub size_bytes: u64,
    /// Content type (e.g., "application/x-nix-nar").
    pub content_type: String,
    /// When the artifact was created.
    pub created_at: String,
    /// Additional metadata.
    pub metadata: std::collections::HashMap<String, String>,
}

/// CI list artifacts response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiListArtifactsResponse {
    /// Whether the operation was successful.
    pub is_success: bool,
    /// List of artifacts.
    pub artifacts: Vec<CiArtifactInfo>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// CI get artifact response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiGetArtifactResponse {
    /// Whether the operation was successful.
    pub is_success: bool,
    /// Artifact metadata.
    pub artifact: Option<CiArtifactInfo>,
    /// Blob ticket for downloading (base32 encoded).
    pub blob_ticket: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// A single CI log chunk from the KV store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiLogChunkInfo {
    /// Chunk index within the job's log stream.
    pub index: u32,
    /// Log content (may contain multiple lines with stream prefixes).
    pub content: String,
    /// Timestamp when this chunk was written (ms since epoch).
    pub timestamp_ms: u64,
}

/// CI get job logs response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiGetJobLogsResponse {
    /// Whether the job was found.
    pub was_found: bool,
    /// Log chunks in order.
    pub chunks: Vec<CiLogChunkInfo>,
    /// Index of the last chunk returned.
    pub last_index: u32,
    /// Whether there are more chunks available.
    pub has_more: bool,
    /// Whether the log stream is complete (job finished).
    pub is_complete: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// CI subscribe logs response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiSubscribeLogsResponse {
    /// Whether the job was found.
    pub was_found: bool,
    /// KV prefix to watch via LOG_SUBSCRIBER_ALPN.
    pub watch_prefix: String,
    /// Current log index (for catch-up before subscribing).
    pub current_index: u64,
    /// Whether the job is still running (stream may have more data).
    pub is_running: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// CI get job output response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiGetJobOutputResponse {
    /// Whether the job was found.
    pub was_found: bool,
    /// Full stdout content (resolved from blob if needed).
    pub stdout: Option<String>,
    /// Full stderr content (resolved from blob if needed).
    pub stderr: Option<String>,
    /// Whether stdout was stored as a blob.
    pub stdout_was_blob: bool,
    /// Whether stderr was stored as a blob.
    pub stderr_was_blob: bool,
    /// Total stdout size in bytes.
    pub stdout_size: u64,
    /// Total stderr size in bytes.
    pub stderr_size: u64,
    /// Error message if retrieval failed.
    pub error: Option<String>,
}

// =============================================================================
// Nix Binary Cache Response Types
// =============================================================================

/// Cache entry returned by query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntryResponse {
    /// Full store path (e.g., /nix/store/abc...-name).
    pub store_path: String,
    /// Store path hash (the abc... part).
    pub store_hash: String,
    /// BLAKE3 hash of the NAR in blob store.
    pub blob_hash: String,
    /// Size of NAR archive in bytes.
    pub nar_size: u64,
    /// SHA256 hash of NAR (for Nix verification).
    pub nar_hash: String,
    /// Original file size in bytes.
    pub file_size: Option<u64>,
    /// Store path references (dependencies).
    pub references: Vec<String>,
    /// Deriver store path.
    pub deriver: Option<String>,
    /// Creation time (Unix timestamp in milliseconds).
    pub created_at_ms: u64,
    /// Node ID that built this.
    pub created_by_node: u64,
    /// CI job ID that created this.
    pub ci_job_id: Option<String>,
    /// CI run ID that created this.
    pub ci_run_id: Option<String>,
}

/// Cache query result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheQueryResultResponse {
    /// Whether the store path was found in cache.
    pub was_found: bool,
    /// Cache entry if found.
    pub entry: Option<CacheEntryResponse>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Cache statistics response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatsResultResponse {
    /// Total number of entries in cache.
    pub total_entries: u64,
    /// Total NAR bytes stored.
    pub total_nar_bytes: u64,
    /// Total query hits.
    pub query_hits: u64,
    /// Total query misses.
    pub query_misses: u64,
    /// Node ID.
    pub node_id: u64,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Cache download result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheDownloadResultResponse {
    /// Whether the store path was found.
    pub was_found: bool,
    /// Blob ticket for downloading the NAR (base64-encoded).
    pub blob_ticket: Option<String>,
    /// BLAKE3 hash of the NAR.
    pub blob_hash: Option<String>,
    /// Size of NAR archive in bytes.
    pub nar_size: Option<u64>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

// =============================================================================
// SNIX Response Types (for remote workers)
// =============================================================================

/// SNIX directory get result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnixDirectoryGetResultResponse {
    /// Whether the directory was found.
    pub was_found: bool,
    /// Protobuf-encoded directory (base64-encoded).
    pub directory_bytes: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// SNIX directory put result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnixDirectoryPutResultResponse {
    /// Whether the directory was stored successfully.
    pub is_success: bool,
    /// BLAKE3 digest of the stored directory (hex-encoded, 64 chars).
    pub digest: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// SNIX path info get result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnixPathInfoGetResultResponse {
    /// Whether the path info was found.
    pub was_found: bool,
    /// Protobuf-encoded PathInfo (base64-encoded).
    pub pathinfo_bytes: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// SNIX path info put result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnixPathInfoPutResultResponse {
    /// Whether the path info was stored successfully.
    pub is_success: bool,
    /// Store path that was registered (e.g., /nix/store/abc...-name).
    pub store_path: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

// =============================================================================
// Cache Migration Response Types
// =============================================================================

/// Cache migration start result response.
#[cfg(feature = "ci")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMigrationStartResultResponse {
    /// Whether migration was started successfully.
    pub started: bool,
    /// Migration status if available.
    pub status: Option<CacheMigrationProgressResponse>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Cache migration status result response.
#[cfg(feature = "ci")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMigrationStatusResultResponse {
    /// Whether migration is currently running.
    pub is_running: bool,
    /// Migration progress details.
    pub progress: Option<CacheMigrationProgressResponse>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Cache migration progress details.
#[cfg(feature = "ci")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMigrationProgressResponse {
    /// Total legacy entries discovered.
    pub total_entries: u64,
    /// Entries successfully migrated.
    pub migrated_count: u64,
    /// Entries that failed migration.
    pub failed_count: u64,
    /// Entries skipped (already migrated or invalid).
    pub skipped_count: u64,
    /// Unix timestamp when migration started.
    pub started_at: u64,
    /// Unix timestamp of last update.
    pub last_updated: u64,
    /// Last processed store hash (for resumption).
    pub last_processed_hash: Option<String>,
    /// Whether migration is complete.
    pub is_complete: bool,
    /// Progress percentage (0.0 - 100.0).
    pub progress_percent: f64,
    /// Error message if migration encountered an issue.
    pub error_message: Option<String>,
}

/// Cache migration cancel result response.
#[cfg(feature = "ci")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMigrationCancelResultResponse {
    /// Whether cancellation was successful.
    pub cancelled: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Cache migration validation result response.
#[cfg(feature = "ci")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMigrationValidateResultResponse {
    /// Whether all entries are migrated.
    #[serde(alias = "complete")]
    pub is_complete: bool,
    /// Number of entries validated.
    pub validated_count: u64,
    /// Number of entries missing from SNIX storage.
    pub missing_count: u64,
    /// Sample of missing entry hashes (limited by max_report).
    pub missing_hashes: Vec<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}
