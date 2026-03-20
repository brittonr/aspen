//! Commit status tracking for CI pipeline results.
//!
//! Stores external check results (pending/success/failure/error) keyed by
//! repo + commit + context string. CI writes statuses back to Forge when
//! pipeline state changes, closing the feedback loop.
//!
//! Statuses are plain KV entries (not COBs) — each `(repo, commit, context)`
//! tuple has exactly one current status, written only by CI.
//! Last-write-wins via Raft is sufficient.

mod store;

use serde::Deserialize;
use serde::Serialize;
pub use store::StatusStore;

use crate::identity::RepoId;

/// State of an external check (CI pipeline, linter, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CommitCheckState {
    /// Check is queued or running.
    Pending,
    /// Check passed.
    Success,
    /// Check failed (test failures, build errors).
    Failure,
    /// Check errored (infrastructure issue, not a code problem).
    Error,
}

impl std::fmt::Display for CommitCheckState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommitCheckState::Pending => write!(f, "pending"),
            CommitCheckState::Success => write!(f, "success"),
            CommitCheckState::Failure => write!(f, "failure"),
            CommitCheckState::Error => write!(f, "error"),
        }
    }
}

/// A commit status entry recording the result of an external check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitStatus {
    /// Repository this status belongs to.
    pub repo_id: RepoId,
    /// Commit hash the status applies to.
    pub commit: [u8; 32],
    /// Context string identifying the check (e.g., "ci/pipeline").
    pub context: String,
    /// Current state of the check.
    pub state: CommitCheckState,
    /// Human-readable description of the status.
    pub description: String,
    /// Pipeline run ID that produced this status (if any).
    pub pipeline_run_id: Option<String>,
    /// Timestamp when this status was created/updated (milliseconds).
    pub created_at_ms: u64,
}
