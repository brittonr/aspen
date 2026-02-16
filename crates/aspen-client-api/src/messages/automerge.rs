//! Automerge operation types.
//!
//! Request/response types for CRDT document management operations.
//! These types are only available when the `automerge` feature is enabled.

use serde::Deserialize;
use serde::Serialize;

/// Automerge domain request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AutomergeRequest {
    /// Create a new Automerge document.
    AutomergeCreate {
        document_id: Option<String>,
        namespace: Option<String>,
        title: Option<String>,
        description: Option<String>,
        tags: Vec<String>,
    },
    /// Get an Automerge document.
    AutomergeGet { document_id: String },
    /// Save/update an Automerge document.
    AutomergeSave {
        document_id: String,
        document_bytes: String,
    },
    /// Delete an Automerge document.
    AutomergeDelete { document_id: String },
    /// Apply incremental changes to an Automerge document.
    AutomergeApplyChanges { document_id: String, changes: Vec<String> },
    /// Merge two Automerge documents.
    AutomergeMerge {
        target_document_id: String,
        source_document_id: String,
    },
    /// List Automerge documents.
    AutomergeList {
        namespace: Option<String>,
        tag: Option<String>,
        limit: Option<u32>,
        continuation_token: Option<String>,
    },
    /// Get Automerge document metadata (without content).
    AutomergeGetMetadata { document_id: String },
    /// Check if an Automerge document exists.
    AutomergeExists { document_id: String },
    /// Generate a sync message for peer synchronization.
    AutomergeGenerateSyncMessage {
        document_id: String,
        peer_id: String,
        sync_state: Option<String>,
    },
    /// Receive a sync message from a peer.
    AutomergeReceiveSyncMessage {
        document_id: String,
        peer_id: String,
        message: String,
        sync_state: Option<String>,
    },
}

impl AutomergeRequest {
    /// Convert to an authorization operation.
    pub fn to_operation(&self) -> Option<aspen_auth::Operation> {
        use aspen_auth::Operation;
        match self {
            Self::AutomergeCreate { .. }
            | Self::AutomergeSave { .. }
            | Self::AutomergeDelete { .. }
            | Self::AutomergeApplyChanges { .. }
            | Self::AutomergeMerge { .. }
            | Self::AutomergeReceiveSyncMessage { .. } => Some(Operation::Write {
                key: "_automerge:".to_string(),
                value: vec![],
            }),
            Self::AutomergeGet { .. }
            | Self::AutomergeList { .. }
            | Self::AutomergeGetMetadata { .. }
            | Self::AutomergeExists { .. }
            | Self::AutomergeGenerateSyncMessage { .. } => Some(Operation::Read {
                key: "_automerge:".to_string(),
            }),
        }
    }
}

/// Automerge create document result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomergeCreateResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Created document ID.
    pub document_id: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Automerge get document result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomergeGetResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Whether the document was found.
    pub was_found: bool,
    /// Document ID.
    pub document_id: Option<String>,
    /// Serialized Automerge document bytes (base64-encoded).
    pub document_bytes: Option<String>,
    /// Document metadata.
    pub metadata: Option<AutomergeDocumentMetadata>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Automerge save document result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomergeSaveResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Document size in bytes.
    pub size_bytes: Option<u64>,
    /// Number of changes in document.
    pub change_count: Option<u64>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Automerge delete document result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomergeDeleteResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Whether the document existed before deletion.
    pub existed: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Automerge apply changes result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomergeApplyChangesResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Whether any changes were applied.
    pub changes_applied: bool,
    /// Number of changes applied.
    pub change_count: Option<u64>,
    /// New document heads (hex-encoded change hashes).
    pub new_heads: Vec<String>,
    /// New document size in bytes.
    pub new_size: Option<u64>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Automerge merge documents result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomergeMergeResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Whether any changes were applied from merge.
    pub changes_applied: bool,
    /// Number of changes applied.
    pub change_count: Option<u64>,
    /// New document heads (hex-encoded change hashes).
    pub new_heads: Vec<String>,
    /// New document size in bytes.
    pub new_size: Option<u64>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Automerge list documents result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomergeListResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// List of document metadata.
    pub documents: Vec<AutomergeDocumentMetadata>,
    /// Whether there are more results.
    pub has_more: bool,
    /// Continuation token for fetching next page.
    pub continuation_token: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Automerge get metadata result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomergeGetMetadataResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Whether the document was found.
    pub was_found: bool,
    /// Document metadata.
    pub metadata: Option<AutomergeDocumentMetadata>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Automerge exists check result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomergeExistsResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Whether the document exists.
    pub does_exist: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Automerge generate sync message result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomergeGenerateSyncMessageResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Whether a sync message was generated (None means peer is up-to-date).
    pub has_message: bool,
    /// Sync message bytes (base64-encoded), if generated.
    pub message: Option<String>,
    /// Updated sync state (base64-encoded) for persistence.
    pub sync_state: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Automerge receive sync message result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomergeReceiveSyncMessageResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Whether any changes were applied from the sync message.
    pub changes_applied: bool,
    /// Updated sync state (base64-encoded) for persistence.
    pub sync_state: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
}

/// Automerge document metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomergeDocumentMetadata {
    /// Document ID.
    pub document_id: String,
    /// Optional namespace.
    pub namespace: Option<String>,
    /// Optional title.
    pub title: Option<String>,
    /// Optional description.
    pub description: Option<String>,
    /// Creation timestamp (milliseconds since epoch).
    pub created_at_ms: u64,
    /// Last update timestamp (milliseconds since epoch).
    pub updated_at_ms: u64,
    /// Document size in bytes.
    pub size_bytes: u64,
    /// Number of changes in document history.
    pub change_count: u64,
    /// Current document heads (hex-encoded change hashes).
    pub heads: Vec<String>,
    /// Creator actor ID (hex-encoded).
    pub creator_actor_id: Option<String>,
    /// Tags for categorization.
    pub tags: Vec<String>,
}
