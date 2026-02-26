//! Forge response types.
//!
//! Response types for decentralized Git hosting operations including
//! repositories, blobs, trees, commits, refs, issues, and patches.

use serde::Deserialize;
use serde::Serialize;

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
    /// Signature threshold (number of delegates required).
    #[serde(rename = "threshold")]
    pub threshold_delegates: u32,
    /// Creation timestamp (ms since epoch).
    pub created_at_ms: u64,
}

/// Repository operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeRepoResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Repository info (if found/created).
    pub repo: Option<ForgeRepoInfo>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Repository list result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeRepoListResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
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
    pub is_success: bool,
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
    pub is_success: bool,
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
    pub is_success: bool,
    /// Commit info (if found/created).
    pub commit: Option<ForgeCommitInfo>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Commit log result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeLogResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
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
    pub is_success: bool,
    /// Whether the ref was found (for get/delete).
    pub was_found: bool,
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
    pub is_success: bool,
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
    pub is_success: bool,
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
    pub is_success: bool,
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
    pub is_success: bool,
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
    pub is_success: bool,
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
    pub is_success: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Delegate key result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeKeyResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
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
    pub is_force: bool,
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
    pub is_success: bool,
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
    pub is_success: bool,
    /// Objects in dependency order (dependencies before dependents).
    pub objects: Vec<GitBridgeObject>,
    /// Number of objects skipped (already in have list).
    pub skipped: u32,
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
    pub is_success: bool,
    /// Number of objects imported.
    pub objects_imported: u32,
    /// Number of objects skipped (already existed).
    pub objects_skipped: u32,
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
    pub is_success: bool,
    /// Error message if update failed.
    pub error: Option<String>,
}

/// Response to GitBridgePushStart - provides session ID for chunked transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgePushStartResponse {
    /// Unique session ID for this chunked push operation.
    pub session_id: String,
    /// Maximum chunk size in bytes that the server will accept.
    pub max_chunk_size_bytes: u64,
    /// Success indicator.
    pub is_success: bool,
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
    pub is_success: bool,
    /// Error message if chunk processing failed.
    pub error: Option<String>,
}

/// Response to GitBridgePushComplete - final push result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgePushCompleteResponse {
    /// Session ID that was completed.
    pub session_id: String,
    /// Whether the entire operation succeeded.
    pub is_success: bool,
    /// Number of objects imported.
    pub objects_imported: u32,
    /// Number of objects skipped (already existed).
    pub objects_skipped: u32,
    /// Results for each ref update.
    pub ref_results: Vec<GitBridgeRefResult>,
    /// Error message if operation failed.
    pub error: Option<String>,
}

/// Response to GitBridgeProbeObjects - reports which SHA-1 hashes the server already has.
///
/// Used for incremental push: the client sends all SHA-1s from `git rev-list`,
/// the server reports which ones already have hash mappings, and the client
/// only reads/sends the missing objects. Expected reduction: ~90-99% for
/// typical pushes after initial import.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBridgeProbeObjectsResponse {
    /// Whether the probe operation succeeded.
    pub is_success: bool,
    /// SHA-1 hashes that the server already has (hex-encoded, 40 characters each).
    ///
    /// The client should exclude these from the push — only send objects
    /// whose SHA-1 is NOT in this set.
    pub known_sha1s: Vec<String>,
    /// Error message if operation failed.
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forge_repo_info_roundtrip() {
        let repo = ForgeRepoInfo {
            id: "abc123".into(),
            name: "test-repo".into(),
            description: Some("A test repo".into()),
            default_branch: "main".into(),
            delegates: vec!["key1".into()],
            threshold_delegates: 1,
            created_at_ms: 1000,
        };
        let json = serde_json::to_string(&repo).unwrap();
        let decoded: ForgeRepoInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id, "abc123");
        assert_eq!(decoded.name, "test-repo");
        assert_eq!(decoded.default_branch, "main");
        assert_eq!(decoded.threshold_delegates, 1);
    }

    #[test]
    fn test_forge_repo_result_success() {
        let result = ForgeRepoResultResponse {
            is_success: true,
            repo: Some(ForgeRepoInfo {
                id: "abc".into(),
                name: "r".into(),
                description: None,
                default_branch: "main".into(),
                delegates: vec![],
                threshold_delegates: 0,
                created_at_ms: 0,
            }),
            error: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        let decoded: ForgeRepoResultResponse = serde_json::from_str(&json).unwrap();
        assert!(decoded.is_success);
        assert!(decoded.repo.is_some());
        assert!(decoded.error.is_none());
    }

    #[test]
    fn test_forge_repo_result_error() {
        let result = ForgeRepoResultResponse {
            is_success: false,
            repo: None,
            error: Some("not found".into()),
        };
        let json = serde_json::to_string(&result).unwrap();
        let decoded: ForgeRepoResultResponse = serde_json::from_str(&json).unwrap();
        assert!(!decoded.is_success);
        assert!(decoded.repo.is_none());
        assert_eq!(decoded.error.as_deref(), Some("not found"));
    }

    #[test]
    fn test_git_bridge_object_roundtrip() {
        let obj = GitBridgeObject {
            sha1: "a".repeat(40),
            object_type: "commit".into(),
            data: vec![1, 2, 3],
        };
        let json = serde_json::to_string(&obj).unwrap();
        let decoded: GitBridgeObject = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.sha1.len(), 40);
        assert_eq!(decoded.object_type, "commit");
        assert_eq!(decoded.data, vec![1, 2, 3]);
    }

    #[test]
    fn test_git_bridge_ref_update_roundtrip() {
        let update = GitBridgeRefUpdate {
            ref_name: "refs/heads/main".into(),
            old_sha1: "0".repeat(40),
            new_sha1: "a".repeat(40),
            is_force: false,
        };
        let json = serde_json::to_string(&update).unwrap();
        let decoded: GitBridgeRefUpdate = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.ref_name, "refs/heads/main");
        assert!(!decoded.is_force);
    }

    #[test]
    fn test_git_bridge_probe_objects_response_roundtrip() {
        let resp = GitBridgeProbeObjectsResponse {
            is_success: true,
            known_sha1s: vec!["a".repeat(40), "b".repeat(40)],
            error: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let decoded: GitBridgeProbeObjectsResponse = serde_json::from_str(&json).unwrap();
        assert!(decoded.is_success);
        assert_eq!(decoded.known_sha1s.len(), 2);
        assert!(decoded.error.is_none());
    }

    #[test]
    fn test_git_bridge_probe_objects_response_empty() {
        let resp = GitBridgeProbeObjectsResponse {
            is_success: true,
            known_sha1s: vec![],
            error: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let decoded: GitBridgeProbeObjectsResponse = serde_json::from_str(&json).unwrap();
        assert!(decoded.known_sha1s.is_empty());
    }

    #[test]
    fn test_git_bridge_probe_objects_response_error() {
        let resp = GitBridgeProbeObjectsResponse {
            is_success: false,
            known_sha1s: vec![],
            error: Some("repo not found".into()),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let decoded: GitBridgeProbeObjectsResponse = serde_json::from_str(&json).unwrap();
        assert!(!decoded.is_success);
        assert_eq!(decoded.error.as_deref(), Some("repo not found"));
    }

    #[test]
    fn test_git_bridge_push_response_roundtrip() {
        let resp = GitBridgePushResponse {
            is_success: true,
            objects_imported: 5,
            objects_skipped: 3,
            ref_results: vec![GitBridgeRefResult {
                ref_name: "refs/heads/main".into(),
                is_success: true,
                error: None,
            }],
            error: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let decoded: GitBridgePushResponse = serde_json::from_str(&json).unwrap();
        assert!(decoded.is_success);
        assert_eq!(decoded.objects_imported, 5);
        assert_eq!(decoded.objects_skipped, 3);
        assert_eq!(decoded.ref_results.len(), 1);
        assert!(decoded.ref_results[0].is_success);
    }

    #[test]
    fn test_git_bridge_list_refs_response_roundtrip() {
        let resp = GitBridgeListRefsResponse {
            is_success: true,
            refs: vec![GitBridgeRefInfo {
                ref_name: "refs/heads/main".into(),
                sha1: "a".repeat(40),
            }],
            head: Some("refs/heads/main".into()),
            error: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let decoded: GitBridgeListRefsResponse = serde_json::from_str(&json).unwrap();
        assert!(decoded.is_success);
        assert_eq!(decoded.refs.len(), 1);
        assert_eq!(decoded.head.as_deref(), Some("refs/heads/main"));
    }

    #[test]
    fn test_git_bridge_fetch_response_roundtrip() {
        let resp = GitBridgeFetchResponse {
            is_success: true,
            objects: vec![GitBridgeObject {
                sha1: "c".repeat(40),
                object_type: "blob".into(),
                data: vec![0xDE, 0xAD],
            }],
            skipped: 10,
            error: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let decoded: GitBridgeFetchResponse = serde_json::from_str(&json).unwrap();
        assert!(decoded.is_success);
        assert_eq!(decoded.objects.len(), 1);
        assert_eq!(decoded.skipped, 10);
    }

    #[test]
    fn test_forge_issue_info_roundtrip() {
        let issue = ForgeIssueInfo {
            id: "issue1".into(),
            title: "Bug report".into(),
            body: "Something broke".into(),
            state: "open".into(),
            labels: vec!["bug".into()],
            comment_count: 2,
            assignees: vec![],
            created_at_ms: 1000,
            updated_at_ms: 2000,
        };
        let json = serde_json::to_string(&issue).unwrap();
        let decoded: ForgeIssueInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id, "issue1");
        assert_eq!(decoded.state, "open");
        assert_eq!(decoded.labels, vec!["bug"]);
    }

    #[test]
    fn test_forge_patch_info_roundtrip() {
        let patch = ForgePatchInfo {
            id: "patch1".into(),
            title: "Fix bug".into(),
            description: "Fixes the thing".into(),
            state: "open".into(),
            base: "b".repeat(40),
            head: "h".repeat(40),
            labels: vec![],
            revision_count: 1,
            approval_count: 0,
            assignees: vec![],
            created_at_ms: 1000,
            updated_at_ms: 2000,
        };
        let json = serde_json::to_string(&patch).unwrap();
        let decoded: ForgePatchInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id, "patch1");
        assert_eq!(decoded.state, "open");
        assert_eq!(decoded.revision_count, 1);
    }

    #[test]
    fn test_chunked_push_start_response_roundtrip() {
        let resp = GitBridgePushStartResponse {
            session_id: "sess-123".into(),
            max_chunk_size_bytes: 4_194_304,
            is_success: true,
            error: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let decoded: GitBridgePushStartResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.session_id, "sess-123");
        assert_eq!(decoded.max_chunk_size_bytes, 4_194_304);
    }

    #[test]
    fn test_chunked_push_chunk_response_roundtrip() {
        let resp = GitBridgePushChunkResponse {
            session_id: "sess-123".into(),
            chunk_id: 7,
            is_success: true,
            error: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let decoded: GitBridgePushChunkResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.chunk_id, 7);
    }

    #[test]
    fn test_chunked_push_complete_response_roundtrip() {
        let resp = GitBridgePushCompleteResponse {
            session_id: "sess-123".into(),
            is_success: true,
            objects_imported: 42,
            objects_skipped: 8,
            ref_results: vec![],
            error: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let decoded: GitBridgePushCompleteResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.objects_imported, 42);
        assert_eq!(decoded.objects_skipped, 8);
        assert!(decoded.ref_results.is_empty());
    }
}
