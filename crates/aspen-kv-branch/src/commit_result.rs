//! Result type for branch commit operations.

use aspen_kv_types::WriteResult;

/// Result of a branch commit operation.
///
/// Always contains the `WriteResult` from the Raft batch.
/// When the `commit-dag` feature is enabled, also contains
/// the `CommitId` of the newly created commit.
#[derive(Debug, Clone)]
pub struct CommitResult {
    /// The Raft write result.
    pub write_result: WriteResult,
    /// The CommitId of the stored commit (None when `commit-dag` feature is disabled).
    #[cfg(feature = "commit-dag")]
    pub commit_id: Option<aspen_commit_dag::CommitId>,
    #[cfg(not(feature = "commit-dag"))]
    pub commit_id: Option<()>,
}

impl CommitResult {
    /// Create a CommitResult with no commit DAG metadata.
    pub fn without_commit_dag(write_result: WriteResult) -> Self {
        Self {
            write_result,
            commit_id: None,
        }
    }
}
