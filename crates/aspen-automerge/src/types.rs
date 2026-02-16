//! Automerge document types and data structures.
//!
//! This module defines the core types for Automerge document management:
//! - `DocumentId`: Unique identifier for Automerge documents
//! - `Document`: Complete document with metadata
//! - `DocumentMetadata`: Document information without content
//! - `DocumentEvent`: Events emitted when documents change
//! - `SyncState`: Sync status tracking

use std::fmt;
use std::str::FromStr;

use serde::Deserialize;
use serde::Serialize;

use super::constants::DOC_ID_BYTES;
use super::constants::DOC_KEY_PREFIX;
use super::constants::DOC_META_PREFIX;
use super::constants::MAX_CUSTOM_DOC_ID_LENGTH;
use super::error::AutomergeError;

// ============================================================================
// Document ID
// ============================================================================

/// Unique identifier for an Automerge document.
///
/// Document IDs can be:
/// - Auto-generated: 16 random bytes (32 hex chars)
/// - Custom: User-provided string (max 128 chars)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DocumentId(String);

impl DocumentId {
    /// Create a new random document ID.
    pub fn new() -> Self {
        use std::time::SystemTime;
        use std::time::UNIX_EPOCH;

        // Generate a pseudo-random ID using timestamp + random suffix
        // In production, use proper random generation
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();

        let mut bytes = [0u8; DOC_ID_BYTES];
        // Use timestamp for first 8 bytes
        bytes[..8].copy_from_slice(&now.to_le_bytes()[..8]);
        // Use more entropy for remaining bytes
        let extra = now.wrapping_mul(0x517cc1b727220a95);
        bytes[8..].copy_from_slice(&extra.to_le_bytes()[..8]);

        Self(hex::encode(bytes))
    }

    /// Create a document ID from a custom string.
    ///
    /// # Errors
    ///
    /// Returns an error if the string is empty or exceeds the maximum length.
    pub fn from_string(s: impl Into<String>) -> Result<Self, AutomergeError> {
        let s = s.into();
        if s.is_empty() {
            return Err(AutomergeError::InvalidDocumentId {
                reason: "document ID cannot be empty".into(),
            });
        }
        if s.len() > MAX_CUSTOM_DOC_ID_LENGTH {
            return Err(AutomergeError::InvalidDocumentId {
                reason: format!("document ID exceeds maximum length of {}", MAX_CUSTOM_DOC_ID_LENGTH),
            });
        }
        // Validate characters: alphanumeric, dash, underscore, dot
        if !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.') {
            return Err(AutomergeError::InvalidDocumentId {
                reason: "document ID contains invalid characters (only alphanumeric, -, _, . allowed)".into(),
            });
        }
        Ok(Self(s))
    }

    /// Get the document ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Generate the KV store key for this document's content.
    pub fn content_key(&self) -> String {
        format!("{}{}", DOC_KEY_PREFIX, self.0)
    }

    /// Generate the KV store key for this document's metadata.
    pub fn metadata_key(&self) -> String {
        format!("{}{}", DOC_META_PREFIX, self.0)
    }

    /// Parse a document ID from a KV content key.
    pub fn from_content_key(key: &str) -> Option<Self> {
        key.strip_prefix(DOC_KEY_PREFIX).filter(|s| !s.starts_with("_meta:")).map(|s| Self(s.to_string()))
    }

    /// Parse a document ID from a KV metadata key.
    pub fn from_metadata_key(key: &str) -> Option<Self> {
        key.strip_prefix(DOC_META_PREFIX).map(|s| Self(s.to_string()))
    }
}

impl Default for DocumentId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for DocumentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for DocumentId {
    type Err = AutomergeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_string(s)
    }
}

impl AsRef<str> for DocumentId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// ============================================================================
// Document Metadata
// ============================================================================

/// Metadata about an Automerge document.
///
/// Stored separately from the document content for efficient listing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// Unique document identifier.
    pub id: DocumentId,

    /// Optional namespace for grouping documents.
    pub namespace: Option<String>,

    /// Human-readable title/name.
    pub title: Option<String>,

    /// Document description.
    pub description: Option<String>,

    /// Unix timestamp when document was created (milliseconds).
    pub created_at_ms: u64,

    /// Unix timestamp when document was last modified (milliseconds).
    pub updated_at_ms: u64,

    /// Size of the document in bytes (approximate, for display).
    pub size_bytes: u64,

    /// Number of changes/commits in the document history.
    pub change_count: u64,

    /// Current heads of the document (hex-encoded change hashes).
    /// Multiple heads indicate concurrent changes that haven't been merged.
    pub heads: Vec<String>,

    /// Actor ID of the document creator (hex-encoded).
    pub creator_actor_id: Option<String>,

    /// Tags for categorization.
    pub tags: Vec<String>,
}

impl DocumentMetadata {
    /// Create new metadata for a fresh document.
    pub fn new(id: DocumentId) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Self {
            id,
            namespace: None,
            title: None,
            description: None,
            created_at_ms: now,
            updated_at_ms: now,
            size_bytes: 0,
            change_count: 0,
            heads: Vec::new(),
            creator_actor_id: None,
            tags: Vec::new(),
        }
    }

    /// Set the namespace.
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// Set the title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a tag.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Update the modification timestamp.
    pub fn touch(&mut self) {
        self.updated_at_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
    }

    /// Serialize to JSON bytes for storage.
    pub fn to_json_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    /// Deserialize from JSON bytes.
    pub fn from_json_bytes(bytes: &[u8]) -> Option<Self> {
        serde_json::from_slice(bytes).ok()
    }
}

// ============================================================================
// Change Types
// ============================================================================

/// A change to apply to an Automerge document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentChange {
    /// Raw Automerge change bytes (from AutoCommit::save_incremental).
    pub bytes: Vec<u8>,

    /// Optional description of the change.
    pub message: Option<String>,

    /// Unix timestamp when change was created (milliseconds).
    pub timestamp_ms: u64,
}

impl DocumentChange {
    /// Create a new change from raw bytes.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            message: None,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        }
    }

    /// Create a change with a message.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

/// Result of applying changes to a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyResult {
    /// Whether any changes were applied.
    pub changes_applied: bool,

    /// Number of changes applied.
    pub change_count: u32,

    /// New heads after applying changes.
    pub new_heads: Vec<String>,

    /// New document size in bytes.
    pub new_size: u64,
}

// ============================================================================
// Sync Types
// ============================================================================

/// Current sync status with a peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncStatus {
    /// Not connected to peer.
    Disconnected,
    /// Establishing connection.
    Connecting,
    /// Syncing document state.
    Syncing,
    /// Fully synchronized.
    Synced,
    /// Sync failed.
    Failed,
}

impl fmt::Display for SyncStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyncStatus::Disconnected => write!(f, "disconnected"),
            SyncStatus::Connecting => write!(f, "connecting"),
            SyncStatus::Syncing => write!(f, "syncing"),
            SyncStatus::Synced => write!(f, "synced"),
            SyncStatus::Failed => write!(f, "failed"),
        }
    }
}

/// State of document synchronization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    /// Current sync status.
    pub status: SyncStatus,

    /// Last successful sync timestamp (milliseconds).
    pub last_sync_ms: Option<u64>,

    /// Number of pending local changes to send.
    pub pending_local: u32,

    /// Number of pending remote changes to receive.
    pub pending_remote: u32,

    /// Error message if sync failed.
    pub error: Option<String>,
}

impl Default for SyncState {
    fn default() -> Self {
        Self {
            status: SyncStatus::Disconnected,
            last_sync_ms: None,
            pending_local: 0,
            pending_remote: 0,
            error: None,
        }
    }
}

// ============================================================================
// Document Events
// ============================================================================

/// Events emitted when Automerge documents change.
#[derive(Debug, Clone)]
pub enum DocumentEvent {
    /// A new document was created.
    Created {
        /// The document ID.
        id: DocumentId,
        /// Document metadata.
        metadata: DocumentMetadata,
    },

    /// A document was updated.
    Updated {
        /// The document ID.
        id: DocumentId,
        /// Number of changes applied.
        change_count: usize,
        /// New heads after update.
        new_heads: Vec<String>,
    },

    /// A document was deleted.
    Deleted {
        /// The document ID.
        id: DocumentId,
    },

    /// Documents were merged.
    Merged {
        /// The target document ID.
        target_id: DocumentId,
        /// The source document ID that was merged in.
        source_id: DocumentId,
    },

    /// Sync status changed for a document.
    SyncStatusChanged {
        /// The document ID.
        id: DocumentId,
        /// New sync status.
        status: SyncStatus,
    },
}

// ============================================================================
// Query Types
// ============================================================================

/// Options for listing documents.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListOptions {
    /// Filter by namespace prefix.
    pub namespace: Option<String>,

    /// Filter by tag.
    pub tag: Option<String>,

    /// Maximum number of results.
    pub limit: Option<u32>,

    /// Continuation token for pagination.
    pub continuation_token: Option<String>,
}

/// Result of listing documents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResult {
    /// Document metadata entries.
    pub documents: Vec<DocumentMetadata>,

    /// Whether there are more results.
    pub has_more: bool,

    /// Token for fetching next page.
    pub continuation_token: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::DOC_ID_HEX_LENGTH;

    #[test]
    fn test_document_id_new() {
        let id1 = DocumentId::new();
        let id2 = DocumentId::new();
        assert_ne!(id1, id2);
        assert_eq!(id1.as_str().len(), DOC_ID_HEX_LENGTH);
    }

    #[test]
    fn test_document_id_from_string() {
        let id = DocumentId::from_string("my-document-123").unwrap();
        assert_eq!(id.as_str(), "my-document-123");
    }

    #[test]
    fn test_document_id_validation() {
        assert!(DocumentId::from_string("").is_err());
        assert!(DocumentId::from_string("a".repeat(200)).is_err());
        assert!(DocumentId::from_string("doc with spaces").is_err());
        assert!(DocumentId::from_string("doc/with/slashes").is_err());

        assert!(DocumentId::from_string("valid-doc_123.v2").is_ok());
    }

    #[test]
    fn test_document_id_keys() {
        let id = DocumentId::from_string("test-doc").unwrap();
        assert_eq!(id.content_key(), "automerge:test-doc");
        assert_eq!(id.metadata_key(), "automerge:_meta:test-doc");
    }

    #[test]
    fn test_parse_content_key() {
        let id = DocumentId::from_content_key("automerge:my-doc");
        assert!(id.is_some());
        assert_eq!(id.unwrap().as_str(), "my-doc");

        // Should not parse metadata keys
        assert!(DocumentId::from_content_key("automerge:_meta:my-doc").is_none());
    }

    #[test]
    fn test_parse_metadata_key() {
        let id = DocumentId::from_metadata_key("automerge:_meta:my-doc");
        assert!(id.is_some());
        assert_eq!(id.unwrap().as_str(), "my-doc");
    }

    #[test]
    fn test_metadata_json_roundtrip() {
        let id = DocumentId::from_string("test").unwrap();
        let meta = DocumentMetadata::new(id)
            .with_namespace("project-a")
            .with_title("My Document")
            .with_tag("important");

        let bytes = meta.to_json_bytes();
        let parsed = DocumentMetadata::from_json_bytes(&bytes).unwrap();

        assert_eq!(meta.id, parsed.id);
        assert_eq!(meta.namespace, parsed.namespace);
        assert_eq!(meta.title, parsed.title);
        assert_eq!(meta.tags, parsed.tags);
    }
}
