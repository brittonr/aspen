//! Pijul operation types.
//!
//! Request/response types for patch-based version control operations.
//! These types are only available when the `pijul` feature is enabled.

use serde::Deserialize;
use serde::Serialize;

/// Pijul domain request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PijulRequest {
    /// Initialize a new Pijul repository.
    PijulRepoInit {
        name: String,
        description: Option<String>,
        default_channel: String,
    },
    /// List Pijul repositories.
    PijulRepoList { limit: u32 },
    /// Get Pijul repository info.
    PijulRepoInfo { repo_id: String },
    /// List channels in a Pijul repository.
    PijulChannelList { repo_id: String },
    /// Create a new channel.
    PijulChannelCreate { repo_id: String, name: String },
    /// Delete a channel.
    PijulChannelDelete { repo_id: String, name: String },
    /// Fork a channel.
    PijulChannelFork {
        repo_id: String,
        source: String,
        target: String,
    },
    /// Get channel info.
    PijulChannelInfo { repo_id: String, name: String },
    /// Record changes from working directory.
    PijulRecord {
        repo_id: String,
        channel: String,
        working_dir: String,
        message: String,
        author_name: Option<String>,
        author_email: Option<String>,
    },
    /// Apply a change to a channel.
    PijulApply {
        repo_id: String,
        channel: String,
        change_hash: String,
    },
    /// Unrecord (remove) a change from a channel.
    PijulUnrecord {
        repo_id: String,
        channel: String,
        change_hash: String,
    },
    /// Get change log for a channel.
    PijulLog {
        repo_id: String,
        channel: String,
        limit: u32,
    },
    /// Checkout pristine state to working directory.
    PijulCheckout {
        repo_id: String,
        channel: String,
        output_dir: String,
    },
    /// Show details of a specific change.
    PijulShow { repo_id: String, change_hash: String },
    /// Get blame/attribution for a file.
    PijulBlame {
        repo_id: String,
        channel: String,
        path: String,
    },
}

impl PijulRequest {
    /// Convert to an authorization operation.
    pub fn to_operation(&self) -> Option<aspen_auth::Operation> {
        use aspen_auth::Operation;
        match self {
            Self::PijulRepoInit { .. }
            | Self::PijulChannelCreate { .. }
            | Self::PijulChannelDelete { .. }
            | Self::PijulChannelFork { .. }
            | Self::PijulRecord { .. }
            | Self::PijulApply { .. }
            | Self::PijulUnrecord { .. } => Some(Operation::Write {
                key: "_pijul:".to_string(),
                value: vec![],
            }),
            Self::PijulRepoList { .. }
            | Self::PijulRepoInfo { .. }
            | Self::PijulChannelList { .. }
            | Self::PijulChannelInfo { .. }
            | Self::PijulLog { .. }
            | Self::PijulCheckout { .. }
            | Self::PijulShow { .. }
            | Self::PijulBlame { .. } => Some(Operation::Read {
                key: "_pijul:".to_string(),
            }),
        }
    }
}

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
    pub does_file_exist: bool,
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
