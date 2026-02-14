//! Blob operation response types.
//!
//! Response types for content-addressed blob storage operations including
//! add, get, list, protect, delete, download, and replication.

use serde::{Deserialize, Serialize};

/// Add blob result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddBlobResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// BLAKE3 hash of the stored blob (hex-encoded).
    pub hash: Option<String>,
    /// Size of the blob in bytes.
    pub size: Option<u64>,
    /// Whether the blob was new (not already in store).
    pub was_new: Option<bool>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Get blob result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlobResultResponse {
    /// Whether the blob was found.
    pub found: bool,
    /// Blob data if found.
    pub data: Option<Vec<u8>>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Has blob result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HasBlobResultResponse {
    /// Whether the blob exists in the store.
    pub exists: bool,
    /// Error message if check failed.
    pub error: Option<String>,
}

/// Get blob ticket result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlobTicketResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Serialized BlobTicket.
    pub ticket: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Blob list entry for listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobListEntry {
    /// BLAKE3 hash (hex-encoded).
    pub hash: String,
    /// Size in bytes.
    pub size: u64,
}

/// List blobs result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListBlobsResultResponse {
    /// List of blobs.
    pub blobs: Vec<BlobListEntry>,
    /// Total count returned.
    pub count: u32,
    /// Whether more blobs are available.
    pub has_more: bool,
    /// Continuation token for next page.
    pub continuation_token: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Protect blob result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectBlobResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Unprotect blob result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnprotectBlobResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Delete blob result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteBlobResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Download blob result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadBlobResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// BLAKE3 hash of the downloaded blob (hex-encoded).
    pub hash: Option<String>,
    /// Size of the downloaded blob in bytes.
    pub size: Option<u64>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Get blob status result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlobStatusResultResponse {
    /// Whether the blob exists.
    pub found: bool,
    /// BLAKE3 hash of the blob (hex-encoded).
    pub hash: Option<String>,
    /// Size of the blob in bytes.
    pub size: Option<u64>,
    /// Whether the blob is complete (all chunks present).
    pub complete: Option<bool>,
    /// List of protection tags.
    pub tags: Option<Vec<String>>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Blob replicate pull result response.
///
/// Returned when a target node attempts to download a blob from a provider
/// as part of the replication system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobReplicatePullResultResponse {
    /// Whether the replication succeeded.
    pub success: bool,
    /// BLAKE3 hash of the replicated blob (hex-encoded).
    pub hash: Option<String>,
    /// Size of the replicated blob in bytes.
    pub size: Option<u64>,
    /// Time taken to download in milliseconds.
    pub duration_ms: Option<u64>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Get blob replication status result response.
///
/// Returns the current replication state of a blob including which nodes
/// have replicas, the policy, and health status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlobReplicationStatusResultResponse {
    /// Whether the blob has replication metadata.
    pub found: bool,
    /// BLAKE3 hash of the blob (hex-encoded).
    pub hash: Option<String>,
    /// Size of the blob in bytes.
    pub size: Option<u64>,
    /// Node IDs that have confirmed replicas.
    pub replica_nodes: Option<Vec<u64>>,
    /// Target replication factor from policy.
    pub replication_factor: Option<u32>,
    /// Minimum replicas required for durability.
    pub min_replicas: Option<u32>,
    /// Current replication status: "critical", "under_replicated", "degraded", "healthy",
    /// "over_replicated".
    pub status: Option<String>,
    /// Number of additional replicas needed to reach target.
    pub replicas_needed: Option<u32>,
    /// Timestamp when replication metadata was last updated (ISO 8601).
    pub updated_at: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Trigger blob replication result response.
///
/// Returned when manually triggering replication of a blob.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerBlobReplicationResultResponse {
    /// Whether the replication was triggered successfully.
    pub success: bool,
    /// BLAKE3 hash of the blob (hex-encoded).
    pub hash: Option<String>,
    /// Node IDs that successfully received the blob.
    pub successful_nodes: Option<Vec<u64>>,
    /// Node IDs that failed to receive the blob with error messages.
    pub failed_nodes: Option<Vec<(u64, String)>>,
    /// Total replication time in milliseconds.
    pub duration_ms: Option<u64>,
    /// Error message if operation failed entirely.
    pub error: Option<String>,
}

/// Run blob repair cycle result response.
///
/// Returned when manually triggering a full blob repair cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunBlobRepairCycleResultResponse {
    /// Whether the repair cycle was initiated successfully.
    pub success: bool,
    /// Error message if initiation failed.
    pub error: Option<String>,
}
