//! Pijul response types.
//!
//! Response types for patch-based version control operations.
//! These types are only available when the `pijul` feature is enabled.

use serde::Deserialize;
use serde::Serialize;

/// Pijul repository response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulRepoResponse {
    /// Repository ID (hex-encoded).
    pub id: String,
    /// Repository name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// Default channel name.
    pub default_channel: String,
    /// Number of channels.
    pub channel_count: u32,
    /// Created timestamp (ms since epoch).
    pub created_at_ms: u64,
}

/// Pijul repository list response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulRepoListResponse {
    /// Repositories.
    pub repos: Vec<PijulRepoResponse>,
    /// Total count.
    pub count: u32,
}

/// Pijul channel response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulChannelResponse {
    /// Channel name.
    pub name: String,
    /// Head change hash (hex-encoded, None if empty).
    pub head: Option<String>,
    /// Last updated timestamp (ms since epoch).
    pub updated_at_ms: u64,
}

/// Pijul channel list response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulChannelListResponse {
    /// Channels.
    pub channels: Vec<PijulChannelResponse>,
    /// Total count.
    pub count: u32,
}

/// Pijul recorded change info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulRecordedChange {
    /// Change hash (hex-encoded BLAKE3).
    pub hash: String,
    /// Change message.
    pub message: String,
    /// Number of hunks.
    pub hunks: u32,
    /// Size in bytes.
    pub size_bytes: u64,
}

/// Pijul record response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulRecordResponse {
    /// Recorded change (None if no changes).
    pub change: Option<PijulRecordedChange>,
}

/// Pijul apply response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulApplyResponse {
    /// Number of operations applied.
    pub operations: u64,
}

/// Pijul unrecord response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulUnrecordResponse {
    /// Whether the change was in the channel and was unrecorded.
    pub unrecorded: bool,
}

/// Pijul log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulLogEntry {
    /// Change hash (hex-encoded).
    pub change_hash: String,
    /// Change message.
    pub message: String,
    /// Author (name or key).
    pub author: Option<String>,
    /// Timestamp (ms since epoch).
    pub timestamp_ms: u64,
}

/// Pijul log response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulLogResponse {
    /// Log entries.
    pub entries: Vec<PijulLogEntry>,
    /// Total count.
    pub count: u32,
}

/// Pijul checkout response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulCheckoutResponse {
    /// Number of files written.
    pub files_written: u32,
    /// Number of conflicts.
    pub conflicts: u32,
}

/// Pijul show response - details of a specific change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulShowResponse {
    /// Full change hash (hex-encoded BLAKE3).
    pub change_hash: String,
    /// Repository ID.
    pub repo_id: String,
    /// Channel the change was recorded to.
    pub channel: String,
    /// Change message/description.
    pub message: String,
    /// Authors of the change.
    pub authors: Vec<PijulAuthorInfo>,
    /// Hashes of changes this change depends on.
    pub dependencies: Vec<String>,
    /// Size of the change in bytes.
    pub size_bytes: u64,
    /// Timestamp when recorded (milliseconds since epoch).
    pub recorded_at_ms: u64,
}

/// Author information for a Pijul change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulAuthorInfo {
    /// Author name.
    pub name: String,
    /// Author email (optional).
    pub email: Option<String>,
}

/// Pijul blame response - attribution information for a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulBlameResponse {
    /// Path of the file being blamed.
    pub path: String,
    /// Channel the blame was performed on.
    pub channel: String,
    /// Repository ID.
    pub repo_id: String,
    /// List of changes that contributed to this file.
    pub attributions: Vec<PijulBlameEntry>,
    /// Whether the file currently exists in the channel.
    pub file_exists: bool,
}

/// A single attribution entry in blame output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulBlameEntry {
    /// Hash of the change (hex-encoded).
    pub change_hash: String,
    /// Author name.
    pub author: Option<String>,
    /// Author email.
    pub author_email: Option<String>,
    /// Change message (first line).
    pub message: String,
    /// Timestamp when recorded (milliseconds since epoch).
    pub recorded_at_ms: u64,
    /// Type of change: "add", "modify", "delete", "rename", "unknown".
    pub change_type: String,
}
