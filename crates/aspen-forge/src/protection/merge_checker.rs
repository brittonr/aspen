//! Merge gating enforcement.
//!
//! Checks branch protection rules before allowing merges,
//! verifying required CI contexts and approval counts.

use std::sync::Arc;

use aspen_core::KeyValueStore;

use super::ProtectionStore;
use crate::cob::Approval;
use crate::error::ForgeError;
use crate::error::ForgeResult;
use crate::identity::RepoId;
use crate::status::CommitCheckState;
use crate::status::StatusStore;

/// Enforces branch protection rules at merge time.
pub struct MergeChecker<K: KeyValueStore + ?Sized> {
    status_store: StatusStore<K>,
    protection_store: ProtectionStore<K>,
}

impl<K: KeyValueStore + ?Sized> Clone for MergeChecker<K> {
    fn clone(&self) -> Self {
        Self {
            status_store: self.status_store.clone(),
            protection_store: self.protection_store.clone(),
        }
    }
}

impl<K: KeyValueStore + ?Sized> MergeChecker<K> {
    /// Create a new merge checker.
    pub fn new(kv: Arc<K>) -> Self {
        Self {
            status_store: StatusStore::new(Arc::clone(&kv)),
            protection_store: ProtectionStore::new(kv),
        }
    }

    /// Check if a merge is allowed for the given target ref.
    ///
    /// Returns `Ok(())` if the merge is allowed, or `Err(ForgeError::MergeBlocked)`
    /// with a description of what's blocking.
    pub async fn check_merge_allowed(
        &self,
        repo_id: &RepoId,
        target_ref: &str,
        head_commit: &[u8; 32],
        approvals: &[Approval],
    ) -> ForgeResult<()> {
        let rule = self.protection_store.get_rule(repo_id, target_ref).await?;
        let rule = match rule {
            Some(r) => r,
            None => return Ok(()), // No protection rule — merge freely
        };

        // Check required CI contexts
        if !rule.required_ci_contexts.is_empty() {
            let statuses = self.status_store.list_statuses_for_commit(repo_id, head_commit).await?;

            let mut missing_or_failing = Vec::new();
            for ctx in &rule.required_ci_contexts {
                let found = statuses.iter().find(|s| s.context == *ctx);
                match found {
                    Some(s) if s.state == CommitCheckState::Success => {}
                    Some(s) => {
                        missing_or_failing.push(format!("{} ({})", ctx, s.state));
                    }
                    None => {
                        missing_or_failing.push(format!("{} (missing)", ctx));
                    }
                }
            }

            if !missing_or_failing.is_empty() {
                return Err(ForgeError::MergeBlocked {
                    reason: format!("required CI checks not passing: {}", missing_or_failing.join(", ")),
                });
            }
        }

        // Check required approvals
        if rule.required_approvals > 0 {
            let valid_approvals: u32 = if rule.dismiss_stale_approvals {
                // Only count approvals for the current head commit
                approvals.iter().filter(|a| a.commit == *head_commit).count() as u32
            } else {
                approvals.len() as u32
            };

            if valid_approvals < rule.required_approvals {
                return Err(ForgeError::MergeBlocked {
                    reason: format!(
                        "insufficient approvals: {} of {} required (dismiss_stale={})",
                        valid_approvals, rule.required_approvals, rule.dismiss_stale_approvals
                    ),
                });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use aspen_testing::DeterministicKeyValueStore;

    use super::*;
    use crate::protection::BranchProtection;
    use crate::status::CommitStatus;

    fn test_repo_id() -> RepoId {
        RepoId::from_hash(blake3::hash(b"test-repo"))
    }

    fn test_commit() -> [u8; 32] {
        [0xab; 32]
    }

    fn make_approval(commit: [u8; 32]) -> Approval {
        Approval {
            author: [1u8; 32],
            commit,
            message: None,
            timestamp_ms: 1_000,
            change_hash: [2u8; 32],
        }
    }

    fn make_status(context: &str, state: CommitCheckState) -> CommitStatus {
        CommitStatus {
            repo_id: test_repo_id(),
            commit: test_commit(),
            context: context.to_string(),
            state,
            description: format!("{context} check"),
            pipeline_run_id: Some("run-1".to_string()),
            created_at_ms: 1_700_000_000_000,
        }
    }

    async fn setup() -> (MergeChecker<dyn KeyValueStore>, Arc<dyn KeyValueStore>) {
        let kv = DeterministicKeyValueStore::new();
        let checker = MergeChecker::new(kv.clone() as Arc<dyn KeyValueStore>);
        (checker, kv)
    }

    #[tokio::test]
    async fn test_unprotected_ref_allows_merge() {
        let (checker, _kv) = setup().await;
        let result = checker.check_merge_allowed(&test_repo_id(), "heads/main", &test_commit(), &[]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_merge_allowed_when_all_checks_pass() {
        let (checker, _kv) = setup().await;
        let repo = test_repo_id();
        let commit = test_commit();

        // Set protection rule
        let rule = BranchProtection {
            ref_pattern: "heads/main".to_string(),
            required_approvals: 1,
            required_ci_contexts: vec!["ci/pipeline".to_string()],
            dismiss_stale_approvals: false,
        };
        checker.protection_store.set_rule(&repo, &rule).await.unwrap();

        // Set passing CI status
        checker
            .status_store
            .set_status(&make_status("ci/pipeline", CommitCheckState::Success))
            .await
            .unwrap();

        // Provide approval
        let approvals = vec![make_approval(commit)];

        let result = checker.check_merge_allowed(&repo, "heads/main", &commit, &approvals).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_merge_blocked_by_failing_ci() {
        let (checker, _kv) = setup().await;
        let repo = test_repo_id();
        let commit = test_commit();

        let rule = BranchProtection {
            ref_pattern: "heads/main".to_string(),
            required_approvals: 0,
            required_ci_contexts: vec!["ci/pipeline".to_string()],
            dismiss_stale_approvals: false,
        };
        checker.protection_store.set_rule(&repo, &rule).await.unwrap();

        checker
            .status_store
            .set_status(&make_status("ci/pipeline", CommitCheckState::Failure))
            .await
            .unwrap();

        let result = checker.check_merge_allowed(&repo, "heads/main", &commit, &[]).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("ci/pipeline"), "error should mention context: {err}");
    }

    #[tokio::test]
    async fn test_merge_blocked_by_missing_status() {
        let (checker, _kv) = setup().await;
        let repo = test_repo_id();
        let commit = test_commit();

        let rule = BranchProtection {
            ref_pattern: "heads/main".to_string(),
            required_approvals: 0,
            required_ci_contexts: vec!["ci/pipeline".to_string()],
            dismiss_stale_approvals: false,
        };
        checker.protection_store.set_rule(&repo, &rule).await.unwrap();

        // No status set — should block
        let result = checker.check_merge_allowed(&repo, "heads/main", &commit, &[]).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("missing"), "error should mention missing: {err}");
    }

    #[tokio::test]
    async fn test_merge_blocked_by_insufficient_approvals() {
        let (checker, _kv) = setup().await;
        let repo = test_repo_id();
        let commit = test_commit();

        let rule = BranchProtection {
            ref_pattern: "heads/main".to_string(),
            required_approvals: 2,
            required_ci_contexts: vec![],
            dismiss_stale_approvals: false,
        };
        checker.protection_store.set_rule(&repo, &rule).await.unwrap();

        let approvals = vec![make_approval(commit)]; // Only 1

        let result = checker.check_merge_allowed(&repo, "heads/main", &commit, &approvals).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("insufficient approvals"), "error should mention approvals: {err}");
    }

    #[tokio::test]
    async fn test_stale_approvals_dismissed() {
        let (checker, _kv) = setup().await;
        let repo = test_repo_id();
        let commit = test_commit();
        let old_commit = [0xcd; 32];

        let rule = BranchProtection {
            ref_pattern: "heads/main".to_string(),
            required_approvals: 1,
            required_ci_contexts: vec![],
            dismiss_stale_approvals: true,
        };
        checker.protection_store.set_rule(&repo, &rule).await.unwrap();

        // Approval is for an old commit
        let approvals = vec![make_approval(old_commit)];

        let result = checker.check_merge_allowed(&repo, "heads/main", &commit, &approvals).await;
        assert!(result.is_err());

        // Now add approval for current commit
        let approvals = vec![make_approval(old_commit), make_approval(commit)];

        let result = checker.check_merge_allowed(&repo, "heads/main", &commit, &approvals).await;
        assert!(result.is_ok());
    }
}
