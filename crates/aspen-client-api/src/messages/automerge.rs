//! Automerge operation types.
//!
//! Request/response types for CRDT document management operations.
//! These types are only available when the `automerge` feature is enabled.

use alloc::string::String;
#[cfg(feature = "auth")]
use alloc::string::ToString;
use alloc::vec::Vec;

use serde::Deserialize;
use serde::Serialize;

/// Automerge domain request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AutomergeRequest {
    /// Create a new Automerge document.
    Create {
        document_id: Option<String>,
        namespace: Option<String>,
        title: Option<String>,
        description: Option<String>,
        tags: Vec<String>,
    },
    /// Get an Automerge document.
    Get { document_id: String },
    /// Save/update an Automerge document.
    Save {
        document_id: String,
        document_bytes: String,
    },
    /// Delete an Automerge document.
    Delete { document_id: String },
    /// Apply incremental changes to an Automerge document.
    ApplyChanges { document_id: String, changes: Vec<String> },
    /// Merge two Automerge documents.
    Merge {
        target_document_id: String,
        source_document_id: String,
    },
    /// List Automerge documents.
    List {
        namespace: Option<String>,
        tag: Option<String>,
        limit: Option<u32>,
        continuation_token: Option<String>,
    },
    /// Get Automerge document metadata (without content).
    GetMetadata { document_id: String },
    /// Check if an Automerge document exists.
    Exists { document_id: String },
    /// Generate a sync message for peer synchronization.
    GenerateSyncMessage {
        document_id: String,
        peer_id: String,
        sync_state: Option<String>,
    },
    /// Receive a sync message from a peer.
    ReceiveSyncMessage {
        document_id: String,
        peer_id: String,
        message: String,
        sync_state: Option<String>,
    },
}

#[cfg(feature = "auth")]
fn document_resource(document_id: &str) -> String {
    alloc::format!("doc:{document_id}")
}

#[cfg(feature = "auth")]
fn create_resource(document_id: Option<&str>, namespace: Option<&str>) -> String {
    if let Some(document_id) = document_id {
        return document_resource(document_id);
    }
    if let Some(namespace) = namespace {
        return alloc::format!("namespace:{namespace}");
    }
    "document:".to_string()
}

#[cfg(feature = "auth")]
fn list_resource(namespace: Option<&str>, tag: Option<&str>) -> String {
    if let Some(namespace) = namespace {
        return alloc::format!("namespace:{namespace}");
    }
    if let Some(tag) = tag {
        return alloc::format!("tag:{tag}");
    }
    "document:".to_string()
}

#[cfg(feature = "auth")]
impl AutomergeRequest {
    /// Convert to an authorization operation.
    pub fn to_operation(&self) -> Option<aspen_auth_core::Operation> {
        use aspen_auth_core::Operation;
        match self {
            Self::Create {
                document_id, namespace, ..
            } => Some(Operation::AutomergeWrite {
                resource: create_resource(document_id.as_deref(), namespace.as_deref()),
            }),
            Self::Save { document_id, .. }
            | Self::Delete { document_id }
            | Self::ApplyChanges { document_id, .. }
            | Self::ReceiveSyncMessage { document_id, .. } => Some(Operation::AutomergeWrite {
                resource: document_resource(document_id),
            }),
            Self::Merge { target_document_id, .. } => Some(Operation::AutomergeWrite {
                resource: document_resource(target_document_id),
            }),
            Self::Get { document_id }
            | Self::GetMetadata { document_id }
            | Self::Exists { document_id }
            | Self::GenerateSyncMessage { document_id, .. } => Some(Operation::AutomergeRead {
                resource: document_resource(document_id),
            }),
            Self::List { namespace, tag, .. } => Some(Operation::AutomergeRead {
                resource: list_resource(namespace.as_deref(), tag.as_deref()),
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
