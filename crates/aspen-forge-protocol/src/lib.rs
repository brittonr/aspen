//! Forge response types.
//!
//! Response types for decentralized Git hosting operations including
//! repositories, blobs, trees, commits, refs, issues, and patches.

use serde::{Deserialize, Serialize};

/// Repository information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeRepoInfo {
    /// Repository ID (hex-encoded BLAKE3 hash).
    pub id: String,
    /// Repository name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// Default branch name.
    pub default_branch: String,
    /// Delegate public keys (hex-encoded).
    pub delegates: Vec<String>,
    /// Signature threshold.
    pub threshold: u32,
    /// Creation timestamp (ms since epoch).
    pub created_at_ms: u64,
}

/// Repository operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeRepoResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Repository info (if found/created).
    pub repo: Option<ForgeRepoInfo>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Repository list result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeRepoListResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// List of repositories.
    pub repos: Vec<ForgeRepoInfo>,
    /// Total count.
    pub count: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Blob operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeBlobResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Blob hash (hex-encoded BLAKE3).
    pub hash: Option<String>,
    /// Blob content (for get operations).
    pub content: Option<Vec<u8>>,
    /// Blob size in bytes.
    pub size: Option<u64>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Tree entry information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeTreeEntry {
    /// File mode (e.g., 0o100644 for regular file).
    pub mode: u32,
    /// Entry name.
    pub name: String,
    /// Entry hash (hex-encoded BLAKE3).
    pub hash: String,
}

/// Tree operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeTreeResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Tree hash (hex-encoded BLAKE3).
    pub hash: Option<String>,
    /// Tree entries (for get operations).
    pub entries: Option<Vec<ForgeTreeEntry>>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Commit information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeCommitInfo {
    /// Commit hash (hex-encoded BLAKE3).
    pub hash: String,
    /// Tree hash.
    pub tree: String,
    /// Parent commit hashes.
    pub parents: Vec<String>,
    /// Author name.
    pub author_name: String,
    /// Author email.
    pub author_email: Option<String>,
    /// Author public key (hex-encoded).
    pub author_key: Option<String>,
    /// Commit message.
    pub message: String,
    /// Timestamp (ms since epoch).
    pub timestamp_ms: u64,
}

/// Commit operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeCommitResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Commit info (if found/created).
    pub commit: Option<ForgeCommitInfo>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Commit log result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeLogResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// List of commits.
    pub commits: Vec<ForgeCommitInfo>,
    /// Total commits returned.
    pub count: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Ref information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeRefInfo {
    /// Ref name (e.g., "heads/main", "tags/v1.0").
    pub name: String,
    /// Target hash (hex-encoded BLAKE3).
    pub hash: String,
}

/// Ref operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeRefResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Whether the ref was found (for get/delete).
    pub found: bool,
    /// Ref info (if found).
    pub ref_info: Option<ForgeRefInfo>,
    /// Previous hash (for CAS operations).
    pub previous_hash: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Ref list result (branches or tags).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeRefListResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// List of refs.
    pub refs: Vec<ForgeRefInfo>,
    /// Total count.
    pub count: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Comment information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeCommentInfo {
    /// Comment hash (change ID).
    pub hash: String,
    /// Author public key (hex-encoded).
    pub author: String,
    /// Comment body.
    pub body: String,
    /// Timestamp (ms since epoch).
    pub timestamp_ms: u64,
}

/// Issue information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeIssueInfo {
    /// Issue ID (hex-encoded).
    pub id: String,
    /// Issue title.
    pub title: String,
    /// Issue body.
    pub body: String,
    /// State: "open" or "closed".
    pub state: String,
    /// Labels.
    pub labels: Vec<String>,
    /// Number of comments.
    pub comment_count: u32,
    /// Assignee public keys (hex-encoded).
    pub assignees: Vec<String>,
    /// Creation timestamp (ms since epoch).
    pub created_at_ms: u64,
    /// Last update timestamp (ms since epoch).
    pub updated_at_ms: u64,
}

/// Issue operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeIssueResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Issue info (if found/created).
    pub issue: Option<ForgeIssueInfo>,
    /// Comments (for detailed get).
    pub comments: Option<Vec<ForgeCommentInfo>>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Issue list result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeIssueListResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// List of issues.
    pub issues: Vec<ForgeIssueInfo>,
    /// Total count.
    pub count: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Patch revision information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgePatchRevision {
    /// Revision hash.
    pub hash: String,
    /// Head commit hash.
    pub head: String,
    /// Optional revision message.
    pub message: Option<String>,
    /// Author public key (hex-encoded).
    pub author: String,
    /// Timestamp (ms since epoch).
    pub timestamp_ms: u64,
}

/// Patch approval information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgePatchApproval {
    /// Approver public key (hex-encoded).
    pub author: String,
    /// Approved commit hash.
    pub commit: String,
    /// Optional approval message.
    pub message: Option<String>,
    /// Timestamp (ms since epoch).
    pub timestamp_ms: u64,
}

/// Patch information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgePatchInfo {
    /// Patch ID (hex-encoded).
    pub id: String,
    /// Patch title.
    pub title: String,
    /// Patch description.
    pub description: String,
    /// State: "open", "merged", or "closed".
    pub state: String,
    /// Base commit hash.
    pub base: String,
    /// Current head commit hash.
    pub head: String,
    /// Labels.
    pub labels: Vec<String>,
    /// Number of revisions.
    pub revision_count: u32,
    /// Number of approvals.
    pub approval_count: u32,
    /// Assignee/reviewer public keys (hex-encoded).
    pub assignees: Vec<String>,
    /// Creation timestamp (ms since epoch).
    pub created_at_ms: u64,
    /// Last update timestamp (ms since epoch).
    pub updated_at_ms: u64,
}

/// Patch operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgePatchResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Patch info (if found/created).
    pub patch: Option<ForgePatchInfo>,
    /// Comments (for detailed get).
    pub comments: Option<Vec<ForgeCommentInfo>>,
    /// Revisions (for detailed get).
    pub revisions: Option<Vec<ForgePatchRevision>>,
    /// Approvals (for detailed get).
    pub approvals: Option<Vec<ForgePatchApproval>>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Patch list result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgePatchListResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// List of patches.
    pub patches: Vec<ForgePatchInfo>,
    /// Total count.
    pub count: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Generic forge operation result (for simple success/error responses).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeOperationResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Delegate key result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeKeyResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Public key (hex-encoded).
    pub public_key: Option<String>,
    /// Secret key (hex-encoded). Only returned for authorized local requests.
    pub secret_key: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

// =============================================================================
// Git Bridge types (for git-remote-aspen)
// =============================================================================

/// Git object for import/export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgeObject {
    /// SHA-1 hash (hex-encoded, 40 characters).
    pub sha1: String,
    /// Object type: "blob", "tree", "commit", or "tag".
    pub object_type: String,
    /// Raw git object content (without header).
    pub data: Vec<u8>,
}

/// Ref update for git push.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgeRefUpdate {
    /// Ref name (e.g., "refs/heads/main").
    pub ref_name: String,
    /// Old SHA-1 hash (for CAS), empty string if creating.
    pub old_sha1: String,
    /// New SHA-1 hash.
    pub new_sha1: String,
    /// Force update (bypass fast-forward check).
    pub force: bool,
}

/// Ref info for git list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgeRefInfo {
    /// Ref name (e.g., "refs/heads/main").
    pub ref_name: String,
    /// SHA-1 hash (hex-encoded, 40 characters).
    pub sha1: String,
}

/// Git bridge list refs response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgeListRefsResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// List of refs with their SHA-1 hashes.
    pub refs: Vec<GitBridgeRefInfo>,
    /// HEAD symref target (e.g., "refs/heads/main"), if any.
    pub head: Option<String>,
    /// Error message if operation failed.
    pub error: Option<String>,
}

/// Git bridge fetch response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgeFetchResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Objects in dependency order (dependencies before dependents).
    pub objects: Vec<GitBridgeObject>,
    /// Number of objects skipped (already in have list).
    pub skipped: usize,
    /// Error message if operation failed.
    pub error: Option<String>,
}

/// Additional metadata for chunked git push operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgePushMetadata {
    /// Optional commit message for the push.
    pub commit_message: Option<String>,
    /// Optional author information.
    pub author: Option<String>,
    /// Optional committer information.
    pub committer: Option<String>,
    /// Optional timestamp.
    pub timestamp: Option<u64>,
    /// Optional additional metadata as key-value pairs.
    pub additional: Option<std::collections::HashMap<String, String>>,
}

/// Git bridge push response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgePushResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Number of objects imported.
    pub objects_imported: usize,
    /// Number of objects skipped (already existed).
    pub objects_skipped: usize,
    /// Results for each ref update.
    pub ref_results: Vec<GitBridgeRefResult>,
    /// Error message if operation failed.
    pub error: Option<String>,
}

/// Result of a single ref update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgeRefResult {
    /// Ref name.
    pub ref_name: String,
    /// Whether the update succeeded.
    pub success: bool,
    /// Error message if update failed.
    pub error: Option<String>,
}

/// Response to GitBridgePushStart - provides session ID for chunked transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgePushStartResponse {
    /// Unique session ID for this chunked push operation.
    pub session_id: String,
    /// Maximum chunk size in bytes that the server will accept.
    pub max_chunk_size: usize,
    /// Success indicator.
    pub success: bool,
    /// Error message if operation failed.
    pub error: Option<String>,
}

/// Response to GitBridgePushChunk - confirms chunk receipt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgePushChunkResponse {
    /// Session ID being processed.
    pub session_id: String,
    /// Chunk ID that was processed.
    pub chunk_id: u64,
    /// Whether this chunk was received successfully.
    pub success: bool,
    /// Error message if chunk processing failed.
    pub error: Option<String>,
}

/// Response to GitBridgePushComplete - final push result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgePushCompleteResponse {
    /// Session ID that was completed.
    pub session_id: String,
    /// Whether the entire operation succeeded.
    pub success: bool,
    /// Number of objects imported.
    pub objects_imported: usize,
    /// Number of objects skipped (already existed).
    pub objects_skipped: usize,
    /// Results for each ref update.
    pub ref_results: Vec<GitBridgeRefResult>,
    /// Error message if operation failed.
    pub error: Option<String>,
}
