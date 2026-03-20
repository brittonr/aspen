//! Branch protection rules for repositories.
//!
//! Stores protection rules that enforce policies on branches (e.g., required
//! approval counts, CI contexts) before allowing pushes. Rules are keyed by
//! repository and ref pattern, with last-write-wins semantics via Raft.
//!
//! Protection rules are plain KV entries (not COBs) — each `(repo, ref_pattern)`
//! tuple has exactly one current rule, written only by repository administrators.

pub mod merge_checker;
mod store;

pub use merge_checker::MergeChecker;
use serde::Deserialize;
use serde::Serialize;
pub use store::ProtectionStore;

/// Branch protection configuration for a repository.
///
/// Defines the requirements that must be met before allowing pushes
/// to matching branch patterns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchProtection {
    /// Ref pattern this protection applies to (e.g., "main", "release/*").
    pub ref_pattern: String,
    /// Number of required approvals before allowing a merge.
    pub required_approvals: u32,
    /// List of CI contexts that must have "Success" status.
    pub required_ci_contexts: Vec<String>,
    /// Whether to dismiss stale approvals when new commits are pushed.
    pub dismiss_stale_approvals: bool,
}
