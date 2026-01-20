//! Constants for Automerge document management.
//!
//! Tiger Style: All operations are bounded to prevent resource exhaustion.

use std::time::Duration;

// ============================================================================
// Key Convention Constants
// ============================================================================

/// Key prefix for all Automerge documents in the KV store.
/// Format: `{DOC_KEY_PREFIX}{document_id}`
/// Example: `automerge:abc123def456`
pub const DOC_KEY_PREFIX: &str = "automerge:";

/// Key prefix for document metadata.
/// Format: `{DOC_META_PREFIX}{document_id}`
pub const DOC_META_PREFIX: &str = "automerge:_meta:";

/// Prefix for Automerge tickets (for P2P sync).
pub const AUTOMERGE_TICKET_PREFIX: &str = "aspenautomerge";

// ============================================================================
// Document ID Constants
// ============================================================================

/// Length of document IDs in bytes (before hex encoding).
/// Uses 16 bytes = 128 bits for collision resistance.
pub const DOC_ID_BYTES: usize = 16;

/// Length of document IDs as hex strings.
/// 16 bytes * 2 hex chars per byte = 32 chars.
pub const DOC_ID_HEX_LENGTH: usize = DOC_ID_BYTES * 2;

/// Maximum length of a custom document ID.
/// Tiger Style: Bounded to fit within MAX_KEY_SIZE.
pub const MAX_CUSTOM_DOC_ID_LENGTH: usize = 128;

// ============================================================================
// Document Size Limits - Tiger Style
// ============================================================================

/// Maximum size of a single Automerge document in bytes.
/// Tiger Style: Bounded to prevent memory exhaustion.
/// 16 MB should be sufficient for most collaborative documents.
pub const MAX_DOCUMENT_SIZE: usize = 16 * 1024 * 1024;

/// Maximum size of a single change/patch in bytes.
/// Tiger Style: Bounded to prevent denial of service via large changes.
pub const MAX_CHANGE_SIZE: usize = 1024 * 1024;

/// Maximum number of changes to apply in a single batch.
/// Tiger Style: Bounded to prevent unbounded CPU usage.
pub const MAX_BATCH_CHANGES: usize = 1000;

/// Maximum number of documents per namespace.
/// Tiger Style: Bounded to prevent unbounded growth.
pub const MAX_DOCUMENTS_PER_NAMESPACE: u32 = 100_000;

/// Maximum number of namespaces.
/// Tiger Style: Bounded.
pub const MAX_NAMESPACES: u32 = 1000;

// ============================================================================
// Sync Constants
// ============================================================================

/// Timeout for sync operations.
pub const SYNC_TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum number of sync messages to buffer.
pub const MAX_SYNC_BUFFER: usize = 100;

/// Interval for background sync checks.
pub const BACKGROUND_SYNC_INTERVAL: Duration = Duration::from_secs(60);

/// Maximum size of a single sync protocol message.
/// Tiger Style: Bounded to prevent memory exhaustion.
/// Set to document size + overhead for sync metadata.
pub const MAX_SYNC_MESSAGE_SIZE: usize = MAX_DOCUMENT_SIZE + 64 * 1024;

// ============================================================================
// Query Limits
// ============================================================================

/// Maximum number of documents returned in a scan.
/// Tiger Style: Bounded for pagination.
pub const MAX_SCAN_RESULTS: u32 = 1000;

/// Default number of documents in a list operation.
pub const DEFAULT_LIST_LIMIT: u32 = 100;

// ============================================================================
// History Limits
// ============================================================================

/// Maximum number of heads to track per document.
/// Tiger Style: Bounded to prevent unbounded metadata.
pub const MAX_HEADS: usize = 100;

/// Maximum depth of version history to return.
/// Tiger Style: Bounded to prevent expensive traversals.
pub const MAX_HISTORY_DEPTH: usize = 1000;
