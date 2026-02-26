//! Forge operation types.
//!
//! Request/response types for decentralized Git hosting operations including
//! repositories, blobs, trees, commits, refs, issues, patches, and git bridge.

pub use aspen_forge_protocol::*;
use serde::Deserialize;
use serde::Serialize;

/// Forge domain request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ForgeRequest {
    // Repository operations
    /// Create a new repository.
    ForgeCreateRepo {
        name: String,
        description: Option<String>,
        default_branch: Option<String>,
    },
    /// Get repository information by ID.
    ForgeGetRepo { repo_id: String },
    /// List repositories.
    ForgeListRepos { limit: Option<u32>, offset: Option<u32> },

    // Blob operations
    /// Store a blob (file content).
    ForgeStoreBlob { repo_id: String, content: Vec<u8> },
    /// Get a blob by hash.
    ForgeGetBlob { hash: String },

    // Tree operations
    /// Create a tree (directory).
    ForgeCreateTree { repo_id: String, entries_json: String },
    /// Get a tree by hash.
    ForgeGetTree { hash: String },

    // Commit operations
    /// Create a commit.
    ForgeCommit {
        repo_id: String,
        tree: String,
        parents: Vec<String>,
        message: String,
    },
    /// Get a commit by hash.
    ForgeGetCommit { hash: String },
    /// Get commit history from a ref.
    ForgeLog {
        repo_id: String,
        ref_name: Option<String>,
        limit: Option<u32>,
    },

    // Ref operations
    /// Get a ref value.
    ForgeGetRef { repo_id: String, ref_name: String },
    /// Set a ref value.
    ForgeSetRef {
        repo_id: String,
        ref_name: String,
        hash: String,
        signer: Option<String>,
        signature: Option<String>,
        timestamp_ms: Option<u64>,
    },
    /// Delete a ref.
    ForgeDeleteRef { repo_id: String, ref_name: String },
    /// Compare-and-set a ref.
    ForgeCasRef {
        repo_id: String,
        ref_name: String,
        expected: Option<String>,
        new_hash: String,
        signer: Option<String>,
        signature: Option<String>,
        timestamp_ms: Option<u64>,
    },
    /// List branches in a repository.
    ForgeListBranches { repo_id: String },
    /// List tags in a repository.
    ForgeListTags { repo_id: String },

    // Issue operations
    /// Create an issue.
    ForgeCreateIssue {
        repo_id: String,
        title: String,
        body: String,
        labels: Vec<String>,
    },
    /// List issues in a repository.
    ForgeListIssues {
        repo_id: String,
        state: Option<String>,
        limit: Option<u32>,
    },
    /// Get issue details.
    ForgeGetIssue { repo_id: String, issue_id: String },
    /// Add a comment to an issue.
    ForgeCommentIssue {
        repo_id: String,
        issue_id: String,
        body: String,
    },
    /// Close an issue.
    ForgeCloseIssue {
        repo_id: String,
        issue_id: String,
        reason: Option<String>,
    },
    /// Reopen an issue.
    ForgeReopenIssue { repo_id: String, issue_id: String },

    // Patch operations
    /// Create a patch (pull request equivalent).
    ForgeCreatePatch {
        repo_id: String,
        title: String,
        description: String,
        base: String,
        head: String,
    },
    /// List patches in a repository.
    ForgeListPatches {
        repo_id: String,
        state: Option<String>,
        limit: Option<u32>,
    },
    /// Get patch details.
    ForgeGetPatch { repo_id: String, patch_id: String },
    /// Update patch head (push new commits).
    ForgeUpdatePatch {
        repo_id: String,
        patch_id: String,
        head: String,
        message: Option<String>,
    },
    /// Approve a patch.
    ForgeApprovePatch {
        repo_id: String,
        patch_id: String,
        commit: String,
        message: Option<String>,
    },
    /// Merge a patch.
    ForgeMergePatch {
        repo_id: String,
        patch_id: String,
        merge_commit: String,
    },
    /// Close a patch without merging.
    ForgeClosePatch {
        repo_id: String,
        patch_id: String,
        reason: Option<String>,
    },

    // Delegate key
    /// Get the delegate key for a repository.
    ForgeGetDelegateKey { repo_id: String },

    // Git Bridge operations
    /// List refs with their SHA-1 hashes.
    GitBridgeListRefs { repo_id: String },
    /// Fetch objects for a ref.
    GitBridgeFetch {
        repo_id: String,
        want: Vec<String>,
        have: Vec<String>,
    },
    /// Push objects and update refs (deprecated, use chunked).
    GitBridgePush {
        repo_id: String,
        objects: Vec<GitBridgeObject>,
        refs: Vec<GitBridgeRefUpdate>,
    },
    /// Start a chunked git push operation.
    GitBridgePushStart {
        repo_id: String,
        total_objects: u64,
        total_size_bytes: u64,
        refs: Vec<GitBridgeRefUpdate>,
        metadata: Option<GitBridgePushMetadata>,
    },
    /// Send a chunk of objects for a push operation.
    GitBridgePushChunk {
        session_id: String,
        chunk_id: u64,
        total_chunks: u64,
        objects: Vec<GitBridgeObject>,
        chunk_hash: [u8; 32],
    },
    /// Complete a chunked git push operation.
    GitBridgePushComplete { session_id: String, content_hash: [u8; 32] },
    /// Probe which objects the server already has (for incremental push).
    ///
    /// The client sends SHA-1 hashes from `git rev-list --objects` and the
    /// server reports which ones it already has mappings for. The client then
    /// only reads and sends the missing objects — typically reducing push
    /// payload by 90-99% for repositories that have been pushed before.
    GitBridgeProbeObjects {
        repo_id: String,
        /// SHA-1 hashes to check (hex-encoded, 40 characters each).
        /// Bounded by MAX_GIT_OBJECTS_PER_PUSH.
        sha1s: Vec<String>,
    },
}

#[cfg(feature = "auth")]
impl ForgeRequest {
    /// Convert to an authorization operation.
    pub fn to_operation(&self) -> Option<aspen_auth::Operation> {
        use aspen_auth::Operation;
        match self {
            // Write operations
            Self::ForgeCreateRepo { .. }
            | Self::ForgeStoreBlob { .. }
            | Self::ForgeCreateTree { .. }
            | Self::ForgeCommit { .. }
            | Self::ForgeSetRef { .. }
            | Self::ForgeDeleteRef { .. }
            | Self::ForgeCasRef { .. }
            | Self::ForgeCreateIssue { .. }
            | Self::ForgeCommentIssue { .. }
            | Self::ForgeCloseIssue { .. }
            | Self::ForgeReopenIssue { .. }
            | Self::ForgeCreatePatch { .. }
            | Self::ForgeUpdatePatch { .. }
            | Self::ForgeApprovePatch { .. }
            | Self::ForgeMergePatch { .. }
            | Self::ForgeClosePatch { .. }
            | Self::GitBridgePush { .. }
            | Self::GitBridgePushStart { .. }
            | Self::GitBridgePushChunk { .. }
            | Self::GitBridgePushComplete { .. } => Some(Operation::Write {
                key: "_forge:".to_string(),
                value: vec![],
            }),
            // Read operations
            Self::ForgeGetRepo { .. }
            | Self::ForgeListRepos { .. }
            | Self::ForgeGetBlob { .. }
            | Self::ForgeGetTree { .. }
            | Self::ForgeGetCommit { .. }
            | Self::ForgeLog { .. }
            | Self::ForgeGetRef { .. }
            | Self::ForgeListBranches { .. }
            | Self::ForgeListTags { .. }
            | Self::ForgeListIssues { .. }
            | Self::ForgeGetIssue { .. }
            | Self::ForgeListPatches { .. }
            | Self::ForgeGetPatch { .. }
            | Self::ForgeGetDelegateKey { .. }
            | Self::GitBridgeListRefs { .. }
            | Self::GitBridgeFetch { .. }
            | Self::GitBridgeProbeObjects { .. } => Some(Operation::Read {
                key: "_forge:".to_string(),
            }),
        }
    }
}
