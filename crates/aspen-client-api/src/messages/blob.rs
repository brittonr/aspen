//! Blob operation types.
//!
//! Request/response types for content-addressed blob storage operations including
//! add, get, list, protect, delete, download, and replication.

use serde::Deserialize;
use serde::Serialize;

/// Blob domain request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlobRequest {
    /// Add a blob to the store.
    AddBlob { data: Vec<u8>, tag: Option<String> },
    /// Get a blob by hash.
    GetBlob { hash: String },
    /// Check if a blob exists.
    HasBlob { hash: String },
    /// Get a ticket for sharing a blob.
    GetBlobTicket { hash: String },
    /// List blobs in the store.
    ListBlobs {
        limit: u32,
        continuation_token: Option<String>,
    },
    /// Protect a blob from garbage collection.
    ProtectBlob { hash: String, tag: String },
    /// Remove protection from a blob.
    UnprotectBlob { tag: String },
    /// Delete a blob from the store.
    DeleteBlob { hash: String, is_force: bool },
    /// Download a blob from a remote peer using a ticket.
    DownloadBlob { ticket: String, tag: Option<String> },
    /// Download a blob by hash using DHT discovery.
    DownloadBlobByHash { hash: String, tag: Option<String> },
    /// Download a blob from a specific provider using DHT mutable item lookup.
    DownloadBlobByProvider {
        hash: String,
        provider: String,
        tag: Option<String>,
    },
    /// Get detailed status information about a blob.
    GetBlobStatus { hash: String },
    /// Request this node to download a blob from a provider for replication.
    BlobReplicatePull {
        hash: String,
        size_bytes: u64,
        provider: String,
        tag: Option<String>,
    },
    /// Get replication status for a blob.
    GetBlobReplicationStatus { hash: String },
    /// Trigger manual replication of a blob to additional nodes.
    TriggerBlobReplication {
        hash: String,
        target_nodes: Vec<u64>,
        replication_factor: u32,
    },
    /// Run a full blob repair cycle.
    RunBlobRepairCycle,
}

#[cfg(feature = "auth")]
impl BlobRequest {
    /// Convert to an authorization operation.
    pub fn to_operation(&self) -> Option<aspen_auth::Operation> {
        use aspen_auth::Operation;
        match self {
            Self::AddBlob { .. }
            | Self::ProtectBlob { .. }
            | Self::UnprotectBlob { .. }
            | Self::DeleteBlob { .. }
            | Self::DownloadBlob { .. }
            | Self::DownloadBlobByHash { .. }
            | Self::DownloadBlobByProvider { .. } => Some(Operation::Write {
                key: "_blob:".to_string(),
                value: vec![],
            }),
            Self::GetBlob { hash }
            | Self::HasBlob { hash }
            | Self::GetBlobTicket { hash }
            | Self::GetBlobStatus { hash }
            | Self::GetBlobReplicationStatus { hash } => Some(Operation::Read {
                key: format!("_blob:{hash}"),
            }),
            Self::ListBlobs { .. } => Some(Operation::Read {
                key: "_blob:".to_string(),
            }),
            Self::BlobReplicatePull { hash, .. } => Some(Operation::Write {
                key: format!("_blob:replica:{hash}"),
                value: vec![],
            }),
            Self::TriggerBlobReplication { hash, .. } => Some(Operation::Write {
                key: format!("_blob:replica:{hash}"),
                value: vec![],
            }),
            Self::RunBlobRepairCycle => Some(Operation::ClusterAdmin {
                action: "blob_repair_cycle".to_string(),
            }),
        }
    }
}

/// Add blob result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddBlobResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// BLAKE3 hash of the stored blob (hex-encoded).
    pub hash: Option<String>,
    /// Size of the blob in bytes.
    #[serde(rename = "size")]
    pub size_bytes: Option<u64>,
    /// Whether the blob was new (not already in store).
    pub was_new: Option<bool>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Get blob result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlobResultResponse {
    /// Whether the blob was found.
    pub was_found: bool,
    /// Blob data if found.
    pub data: Option<Vec<u8>>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Has blob result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HasBlobResultResponse {
    /// Whether the blob exists in the store.
    pub does_exist: bool,
    /// Error message if check failed.
    pub error: Option<String>,
}

/// Get blob ticket result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlobTicketResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
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
    /// Size in bytes (Tiger Style: units in name).
    #[serde(alias = "size")]
    pub size_bytes: u64,
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
    pub is_success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Unprotect blob result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnprotectBlobResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Delete blob result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteBlobResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Download blob result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadBlobResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// BLAKE3 hash of the downloaded blob (hex-encoded).
    pub hash: Option<String>,
    /// Size of the downloaded blob in bytes.
    #[serde(rename = "size")]
    pub size_bytes: Option<u64>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Get blob status result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlobStatusResultResponse {
    /// Whether the blob exists.
    pub was_found: bool,
    /// BLAKE3 hash of the blob (hex-encoded).
    pub hash: Option<String>,
    /// Size of the blob in bytes.
    #[serde(rename = "size")]
    pub size_bytes: Option<u64>,
    /// Whether the blob is complete (all chunks present).
    pub complete: Option<bool>,
    /// List of protection tags.
    pub tags: Option<Vec<String>>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Blob replicate pull result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobReplicatePullResultResponse {
    /// Whether the replication succeeded.
    pub is_success: bool,
    /// BLAKE3 hash of the replicated blob (hex-encoded).
    pub hash: Option<String>,
    /// Size of the replicated blob in bytes.
    #[serde(rename = "size")]
    pub size_bytes: Option<u64>,
    /// Time taken to download in milliseconds.
    pub duration_ms: Option<u64>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Get blob replication status result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlobReplicationStatusResultResponse {
    /// Whether the blob has replication metadata.
    pub was_found: bool,
    /// BLAKE3 hash of the blob (hex-encoded).
    pub hash: Option<String>,
    /// Size of the blob in bytes.
    #[serde(rename = "size")]
    pub size_bytes: Option<u64>,
    /// Node IDs that have confirmed replicas.
    pub replica_nodes: Option<Vec<u64>>,
    /// Target replication factor from policy.
    pub replication_factor: Option<u32>,
    /// Minimum replicas required for durability.
    pub min_replicas: Option<u32>,
    /// Current replication status.
    pub status: Option<String>,
    /// Number of additional replicas needed to reach target.
    pub replicas_needed: Option<u32>,
    /// Timestamp when replication metadata was last updated (ISO 8601).
    pub updated_at: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Trigger blob replication result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerBlobReplicationResultResponse {
    /// Whether the replication was triggered successfully.
    pub is_success: bool,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunBlobRepairCycleResultResponse {
    /// Whether the repair cycle was initiated successfully.
    pub is_success: bool,
    /// Error message if initiation failed.
    pub error: Option<String>,
}
