//! Automerge plugin types and constants.
//!
//! Mirrors the types from `aspen-automerge` for WASM-side use without
//! pulling in the full `automerge` crate dependency.

use serde::Deserialize;
use serde::Serialize;

// =============================================================================
// Constants
// =============================================================================

/// Key prefix for document content in the KV store.
pub const DOC_KEY_PREFIX: &str = "automerge:";

/// Key prefix for document metadata in the KV store.
pub const DOC_META_PREFIX: &str = "automerge:_meta:";

/// Maximum document size (16 MB).
pub const MAX_DOCUMENT_SIZE: usize = 16 * 1024 * 1024;

/// Maximum single change size (1 MB).
pub const MAX_CHANGE_SIZE: usize = 1024 * 1024;

/// Maximum changes in a single batch.
pub const MAX_BATCH_CHANGES: usize = 1000;

/// Maximum documents returned in a scan.
pub const MAX_SCAN_RESULTS: u32 = 1000;

/// Default list limit.
pub const DEFAULT_LIST_LIMIT: u32 = 100;

/// Maximum custom document ID length.
pub const MAX_CUSTOM_DOC_ID_LENGTH: usize = 128;

/// Document ID byte size for auto-generated IDs.
pub const DOC_ID_BYTES: usize = 16;

// =============================================================================
// Document Metadata
// =============================================================================

/// Document metadata stored alongside document content.
///
/// Key format: `automerge:_meta:{document_id}`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// Document ID.
    pub document_id: String,

    /// Optional namespace for grouping.
    pub namespace: Option<String>,

    /// Human-readable title.
    pub title: Option<String>,

    /// Description.
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

impl DocumentMetadata {
    /// Create new metadata for a fresh document.
    pub fn new(document_id: String, now_ms: u64) -> Self {
        Self {
            document_id,
            namespace: None,
            title: None,
            description: None,
            created_at_ms: now_ms,
            updated_at_ms: now_ms,
            size_bytes: 0,
            change_count: 0,
            heads: Vec::new(),
            creator_actor_id: None,
            tags: Vec::new(),
        }
    }

    /// Serialize to JSON string for KV storage.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| format!("serialize metadata: {e}"))
    }

    /// Deserialize from JSON string.
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("corrupt metadata: {e}"))
    }
}

// =============================================================================
// Key Helpers
// =============================================================================

/// Build the KV key for document content.
pub fn content_key(doc_id: &str) -> String {
    format!("{DOC_KEY_PREFIX}{doc_id}")
}

/// Build the KV key for document metadata.
pub fn metadata_key(doc_id: &str) -> String {
    format!("{DOC_META_PREFIX}{doc_id}")
}

/// Parse a document ID from a metadata key.
pub fn doc_id_from_metadata_key(key: &str) -> Option<String> {
    key.strip_prefix(DOC_META_PREFIX).map(String::from)
}

/// Validate a document ID.
pub fn validate_document_id(id: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err("document ID cannot be empty".to_string());
    }
    if id.len() > MAX_CUSTOM_DOC_ID_LENGTH {
        return Err(format!("document ID exceeds maximum length of {}", MAX_CUSTOM_DOC_ID_LENGTH));
    }
    if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.') {
        return Err("document ID contains invalid characters (only alphanumeric, -, _, . allowed)".to_string());
    }
    Ok(())
}

/// Generate a random document ID (32 hex chars from 16 random bytes).
pub fn generate_doc_id() -> String {
    let bytes = aspen_wasm_guest_sdk::host::get_random_bytes(DOC_ID_BYTES as u32);
    hex::encode(bytes)
}

// =============================================================================
// Sync State (for sync protocol)
// =============================================================================

/// Simplified sync state stored per peer.
///
/// Since the WASM plugin can't use the `automerge` crate's sync protocol
/// directly, we use a hash-based sync approach:
/// - Track which document heads each peer has seen
/// - Generate sync messages as full document snapshots when heads differ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleSyncState {
    /// Last known heads for this peer.
    pub known_heads: Vec<String>,

    /// Last sync timestamp (milliseconds).
    pub last_sync_ms: u64,
}

impl SimpleSyncState {
    /// Create a new empty sync state.
    pub fn new() -> Self {
        Self {
            known_heads: Vec::new(),
            last_sync_ms: 0,
        }
    }
}
