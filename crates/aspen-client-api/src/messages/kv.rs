//! KV operation response types.
//!
//! Response types for key-value store operations including delete, scan,
//! and vault operations.

use serde::Deserialize;
use serde::Serialize;

/// Delete key result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteResultResponse {
    /// The key that was targeted.
    pub key: String,
    /// True if key existed and was deleted, false if not found.
    pub deleted: bool,
    /// Error message if operation failed.
    pub error: Option<String>,
}

/// Scan keys result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResultResponse {
    /// Matching key-value pairs.
    pub entries: Vec<ScanEntry>,
    /// Number of entries returned.
    pub count: u32,
    /// True if more results available.
    pub is_truncated: bool,
    /// Token for next page (if truncated).
    pub continuation_token: Option<String>,
    /// Error message if operation failed.
    pub error: Option<String>,
}

/// Single entry from scan operation with revision metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanEntry {
    /// Key name.
    pub key: String,
    /// Value (as UTF-8 string).
    pub value: String,
    /// Per-key version counter (1, 2, 3...). Reset to 1 on delete+recreate.
    #[serde(default)]
    pub version: u64,
    /// Raft log index when key was first created.
    #[serde(default)]
    pub create_revision: u64,
    /// Raft log index of last modification.
    #[serde(default)]
    pub mod_revision: u64,
}
