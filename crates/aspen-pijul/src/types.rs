//! Core types for the Pijul module.
//!
//! These types provide the foundation for Pijul integration, including
//! change hashes, channels, and repository identity.

use aspen_forge::identity::RepoId;
use serde::Deserialize;
use serde::Serialize;

use crate::error::PijulError;
use crate::error::PijulResult;

// ============================================================================
// Change Hash
// ============================================================================

/// A Pijul change hash (BLAKE3).
///
/// Both Pijul and Aspen use BLAKE3 for content addressing, enabling
/// zero-copy hash conversion between Pijul changes and iroh-blobs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChangeHash(pub [u8; 32]);

impl ChangeHash {
    /// Create a ChangeHash from raw bytes.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Create a ChangeHash from a BLAKE3 hash.
    pub fn from_blake3(hash: blake3::Hash) -> Self {
        Self(*hash.as_bytes())
    }

    /// Convert to a BLAKE3 hash.
    pub fn to_blake3(&self) -> blake3::Hash {
        blake3::Hash::from_bytes(self.0)
    }

    /// Convert to an iroh-blobs Hash (zero-copy, same underlying format).
    pub fn to_iroh_hash(&self) -> iroh_blobs::Hash {
        iroh_blobs::Hash::from_bytes(self.0)
    }

    /// Create from an iroh-blobs Hash.
    pub fn from_iroh_hash(hash: iroh_blobs::Hash) -> Self {
        Self(*hash.as_bytes())
    }

    /// Get the raw bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Convert to hex string.
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Parse from hex string.
    pub fn from_hex(s: &str) -> Result<Self, hex::FromHexError> {
        let bytes = hex::decode(s)?;
        if bytes.len() != 32 {
            return Err(hex::FromHexError::InvalidStringLength);
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

impl std::fmt::Display for ChangeHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl From<blake3::Hash> for ChangeHash {
    fn from(hash: blake3::Hash) -> Self {
        Self::from_blake3(hash)
    }
}

impl From<iroh_blobs::Hash> for ChangeHash {
    fn from(hash: iroh_blobs::Hash) -> Self {
        Self::from_iroh_hash(hash)
    }
}

// ============================================================================
// Channel
// ============================================================================

/// A Pijul channel (similar to a Git branch).
///
/// In Pijul, a channel is exactly a set of applied changes. Unlike Git branches
/// which point to a single commit, a channel represents the cumulative state
/// of all patches that have been applied to it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    /// Name of the channel.
    pub name: String,

    /// The current head of the channel (hash of the most recent change).
    /// None if the channel has no changes yet.
    pub head: Option<ChangeHash>,

    /// Timestamp of the last update (milliseconds since epoch).
    pub updated_at_ms: u64,
}

impl Channel {
    /// Create a new empty channel.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            head: None,
            updated_at_ms: chrono::Utc::now().timestamp_millis() as u64,
        }
    }

    /// Create a channel with an initial head.
    pub fn with_head(name: impl Into<String>, head: ChangeHash) -> Self {
        Self {
            name: name.into(),
            head: Some(head),
            updated_at_ms: chrono::Utc::now().timestamp_millis() as u64,
        }
    }
}

// ============================================================================
// Repository Identity
// ============================================================================

/// Pijul repository identity.
///
/// Analogous to `RepoIdentity` for Git repositories, this defines the
/// metadata and authorization for a Pijul repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulRepoIdentity {
    /// Human-readable name of the repository.
    pub name: String,

    /// Optional description.
    pub description: Option<String>,

    /// Default channel name (equivalent to default branch).
    pub default_channel: String,

    /// Delegates who can update canonical channels.
    pub delegates: Vec<iroh::PublicKey>,

    /// Number of delegate signatures required for canonical updates.
    pub threshold: u32,

    /// Creation timestamp (milliseconds since epoch).
    pub created_at_ms: u64,
}

impl PijulRepoIdentity {
    /// Create a new repository identity.
    pub fn new(name: impl Into<String>, delegates: Vec<iroh::PublicKey>) -> Self {
        let delegates_count = delegates.len() as u32;
        Self {
            name: name.into(),
            description: None,
            default_channel: "main".to_string(),
            delegates,
            threshold: delegates_count.max(1),
            created_at_ms: chrono::Utc::now().timestamp_millis() as u64,
        }
    }

    /// Create with a custom default channel.
    pub fn with_default_channel(mut self, channel: impl Into<String>) -> Self {
        self.default_channel = channel.into();
        self
    }

    /// Add a description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set a custom threshold.
    pub fn with_threshold(mut self, threshold: u32) -> Self {
        self.threshold = threshold;
        self
    }

    /// Compute the repository ID from this identity.
    ///
    /// The RepoId is the BLAKE3 hash of the serialized identity.
    pub fn repo_id(&self) -> PijulResult<RepoId> {
        let bytes = postcard::to_allocvec(self).map_err(|e| PijulError::Serialization {
            message: format!("PijulRepoIdentity: {e}"),
        })?;
        let hash = blake3::hash(&bytes);
        Ok(RepoId(*hash.as_bytes()))
    }
}

// ============================================================================
// Author
// ============================================================================

/// A Pijul change author.
///
/// Maps between Pijul's author format and Aspen's Ed25519 identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PijulAuthor {
    /// Author name.
    pub name: String,

    /// Author email (optional, for compatibility with Pijul format).
    pub email: Option<String>,

    /// Aspen public key (if the author is an Aspen identity).
    pub public_key: Option<iroh::PublicKey>,

    /// Timestamp of the authorship.
    pub timestamp_ms: u64,
}

impl PijulAuthor {
    /// Create an author from an Aspen public key.
    pub fn from_public_key(key: iroh::PublicKey) -> Self {
        Self {
            name: format!("aspen:{}", key.fmt_short()),
            email: None,
            public_key: Some(key),
            timestamp_ms: chrono::Utc::now().timestamp_millis() as u64,
        }
    }

    /// Create an author from name and email (Pijul-compatible format).
    pub fn from_name_email(name: impl Into<String>, email: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            email: Some(email.into()),
            public_key: None,
            timestamp_ms: chrono::Utc::now().timestamp_millis() as u64,
        }
    }
}

// ============================================================================
// Change Metadata
// ============================================================================

/// Metadata about a Pijul change (stored separately from the change itself).
///
/// This lightweight metadata is stored in Raft KV for quick lookups,
/// while the full change content is stored in iroh-blobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeMetadata {
    /// Hash of the change.
    pub hash: ChangeHash,

    /// Repository this change belongs to.
    pub repo_id: RepoId,

    /// Channel the change was recorded to.
    pub channel: String,

    /// Change message/description.
    pub message: String,

    /// Authors of the change.
    pub authors: Vec<PijulAuthor>,

    /// Hashes of changes this change depends on.
    pub dependencies: Vec<ChangeHash>,

    /// Size of the compressed change in bytes.
    pub size_bytes: u64,

    /// Timestamp when the change was recorded.
    pub recorded_at_ms: u64,
}

// ============================================================================
// Sync Types
// ============================================================================

/// Statistics from a sync operation.
#[derive(Debug, Clone, Default)]
pub struct SyncStats {
    /// Number of changes fetched.
    pub changes_fetched: u32,

    /// Number of changes applied.
    pub changes_applied: u32,

    /// Number of changes that were already present.
    pub changes_skipped: u32,

    /// Total bytes transferred.
    pub bytes_transferred: u64,

    /// Duration of the sync operation.
    pub duration_ms: u64,
}

/// Result of checking for available updates on a channel.
#[derive(Debug, Clone)]
pub struct ChannelStatus {
    /// Channel name.
    pub channel: String,

    /// Local head (if any).
    pub local_head: Option<ChangeHash>,

    /// Remote head (if any).
    pub remote_head: Option<ChangeHash>,

    /// Number of changes behind the remote.
    pub changes_behind: u32,

    /// Number of changes ahead of the remote.
    pub changes_ahead: u32,
}

// ============================================================================
// Conflict Types
// ============================================================================

/// Kind of conflict in Pijul.
///
/// Pijul has different conflict types that require different resolution approaches:
/// - Name conflicts: Same file/dir has different names
/// - Order conflicts: Ambiguous line ordering
/// - Cyclic conflicts: Circular dependencies in text
/// - Zombie conflicts: Deleted then modified
/// - Multiple names: A vertex has multiple names
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictKind {
    /// A file or directory has multiple names.
    Name,
    /// Lines have ambiguous ordering.
    Order,
    /// Cyclic dependencies between lines.
    Cyclic,
    /// Content was deleted then modified.
    Zombie,
    /// A vertex has multiple names assigned.
    MultipleNames,
}

impl std::fmt::Display for ConflictKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConflictKind::Name => write!(f, "name"),
            ConflictKind::Order => write!(f, "order"),
            ConflictKind::Cyclic => write!(f, "cyclic"),
            ConflictKind::Zombie => write!(f, "zombie"),
            ConflictKind::MultipleNames => write!(f, "multiple_names"),
        }
    }
}

/// Status of conflict resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ResolutionStatus {
    /// Conflict has not been addressed.
    #[default]
    Unresolved,
    /// User is currently working on resolution.
    InProgress,
    /// Resolution is prepared but not committed.
    Pending,
    /// Conflict has been resolved.
    Resolved,
}

impl std::fmt::Display for ResolutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolutionStatus::Unresolved => write!(f, "unresolved"),
            ResolutionStatus::InProgress => write!(f, "in_progress"),
            ResolutionStatus::Pending => write!(f, "pending"),
            ResolutionStatus::Resolved => write!(f, "resolved"),
        }
    }
}

/// Information about a conflict in a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileConflict {
    /// Path to the conflicting file (relative to repo root).
    pub path: String,

    /// Changes involved in this conflict.
    pub involved_changes: Vec<ChangeHash>,

    /// When the conflict was detected.
    pub detected_at_ms: u64,
}

/// Detailed information about a conflict for resolution UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedFileConflict {
    /// Path to the conflicting file.
    pub path: String,

    /// Kind of conflict.
    pub kind: ConflictKind,

    /// Changes involved in this conflict.
    pub involved_changes: Vec<ChangeHash>,

    /// Current resolution status.
    pub resolution_status: ResolutionStatus,

    /// When the conflict was detected.
    pub detected_at_ms: u64,

    /// Conflict markers in the file (if applicable).
    pub markers: Option<ConflictMarkers>,
}

/// Conflict markers found in a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictMarkers {
    /// Start line of the conflict (1-indexed).
    pub start_line: u32,

    /// End line of the conflict (1-indexed).
    pub end_line: u32,

    /// The "ours" side of the conflict.
    pub ours: String,

    /// The "theirs" side of the conflict.
    pub theirs: String,
}

/// Conflict state for a channel.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChannelConflictState {
    /// List of files with conflicts.
    pub conflicts: Vec<FileConflict>,

    /// When the conflict state was last checked.
    pub checked_at_ms: u64,

    /// Channel head when conflicts were detected.
    pub head_at_check: Option<ChangeHash>,
}

impl ChannelConflictState {
    /// Check if there are any conflicts.
    pub fn has_conflicts(&self) -> bool {
        !self.conflicts.is_empty()
    }

    /// Get the number of conflicts.
    pub fn conflict_count(&self) -> usize {
        self.conflicts.len()
    }

    /// Get the list of conflicting file paths.
    pub fn conflicting_paths(&self) -> Vec<&str> {
        self.conflicts.iter().map(|c| c.path.as_str()).collect()
    }
}
