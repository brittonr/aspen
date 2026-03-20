//! Commit status store implementation.

use std::sync::Arc;

use aspen_core::KeyValueStore;
use aspen_core::ReadRequest;
use aspen_core::ScanRequest;
use aspen_core::WriteRequest;

use super::CommitCheckState;
use super::CommitStatus;
use crate::constants::KV_PREFIX_COMMIT_STATUS;
use crate::error::ForgeError;
use crate::error::ForgeResult;
use crate::identity::RepoId;

/// Maximum number of statuses to return for a single commit.
///
/// Tiger Style: Prevents unbounded scan results.
const MAX_STATUSES_PER_COMMIT: u32 = 100;

/// Storage for commit statuses using Raft consensus.
///
/// Commit statuses are plain KV entries — each `(repo, commit, context)` tuple
/// has exactly one current status. Only CI writes statuses; last-write-wins
/// via Raft is sufficient.
///
/// # Key Format
///
/// ```text
/// forge:status:{repo_hex}:{commit_hex}:{context}
/// ```
pub struct StatusStore<K: KeyValueStore + ?Sized> {
    kv: Arc<K>,
}

impl<K: KeyValueStore + ?Sized> Clone for StatusStore<K> {
    fn clone(&self) -> Self {
        Self {
            kv: Arc::clone(&self.kv),
        }
    }
}

impl<K: KeyValueStore + ?Sized> StatusStore<K> {
    /// Create a new status store.
    pub fn new(kv: Arc<K>) -> Self {
        Self { kv }
    }

    /// Set or update a commit status.
    ///
    /// Overwrites any existing status for the same `(repo, commit, context)` tuple.
    pub async fn set_status(&self, status: &CommitStatus) -> ForgeResult<()> {
        let key = status_key(&status.repo_id, &status.commit, &status.context);
        let value = serde_json::to_string(status).map_err(|e| ForgeError::Serialization { message: e.to_string() })?;
        self.kv
            .write(WriteRequest::set(key, value))
            .await
            .map_err(|e| ForgeError::KvStorage { message: e.to_string() })?;
        Ok(())
    }

    /// Get a specific commit status by repo, commit, and context.
    ///
    /// Returns `None` if no status exists for the given tuple.
    pub async fn get_status(
        &self,
        repo_id: &RepoId,
        commit: &[u8; 32],
        context: &str,
    ) -> ForgeResult<Option<CommitStatus>> {
        let key = status_key(repo_id, commit, context);
        let result = self.kv.read(ReadRequest::new(key)).await;
        match result {
            Ok(r) => match r.kv {
                Some(entry) => {
                    let status: CommitStatus = serde_json::from_str(&entry.value)
                        .map_err(|e| ForgeError::Serialization { message: e.to_string() })?;
                    Ok(Some(status))
                }
                None => Ok(None),
            },
            Err(e) => {
                // DeterministicKeyValueStore returns Err(NotFound) for missing keys
                let msg = e.to_string();
                if msg.contains("not found") || msg.contains("NotFound") {
                    Ok(None)
                } else {
                    Err(ForgeError::KvStorage { message: msg })
                }
            }
        }
    }

    /// List all statuses for a given commit across all contexts.
    ///
    /// Returns statuses sorted by context string.
    pub async fn list_statuses_for_commit(
        &self,
        repo_id: &RepoId,
        commit: &[u8; 32],
    ) -> ForgeResult<Vec<CommitStatus>> {
        let prefix = commit_prefix(repo_id, commit);
        let result = self
            .kv
            .scan(ScanRequest {
                prefix,
                limit_results: Some(MAX_STATUSES_PER_COMMIT),
                continuation_token: None,
            })
            .await
            .map_err(|e| ForgeError::KvStorage { message: e.to_string() })?;

        let mut statuses = Vec::with_capacity(result.entries.len());
        for entry in &result.entries {
            let status: CommitStatus =
                serde_json::from_str(&entry.value).map_err(|e| ForgeError::Serialization { message: e.to_string() })?;
            statuses.push(status);
        }
        Ok(statuses)
    }

    /// Check if all required contexts have `Success` status for a commit.
    ///
    /// Returns the list of contexts that are missing or not successful.
    pub async fn check_required_contexts(
        &self,
        repo_id: &RepoId,
        commit: &[u8; 32],
        required_contexts: &[String],
    ) -> ForgeResult<Vec<String>> {
        if required_contexts.is_empty() {
            return Ok(Vec::new());
        }

        let statuses = self.list_statuses_for_commit(repo_id, commit).await?;

        let mut failing = Vec::new();
        for ctx in required_contexts {
            let found = statuses.iter().find(|s| s.context == *ctx);
            match found {
                Some(s) if s.state == CommitCheckState::Success => {}
                _ => failing.push(ctx.clone()),
            }
        }
        Ok(failing)
    }
}

/// Build the full KV key for a commit status.
fn status_key(repo_id: &RepoId, commit: &[u8; 32], context: &str) -> String {
    format!("{}{}:{}:{}", KV_PREFIX_COMMIT_STATUS, repo_id.to_hex(), hex::encode(commit), context)
}

/// Build the KV prefix for scanning all statuses of a commit.
fn commit_prefix(repo_id: &RepoId, commit: &[u8; 32]) -> String {
    format!("{}{}:{}:", KV_PREFIX_COMMIT_STATUS, repo_id.to_hex(), hex::encode(commit))
}

#[cfg(test)]
mod tests {
    use aspen_testing::DeterministicKeyValueStore;

    use super::*;

    fn test_repo_id() -> RepoId {
        RepoId::from_hash(blake3::hash(b"test-repo"))
    }

    fn test_commit() -> [u8; 32] {
        [0xab; 32]
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

    #[tokio::test]
    async fn test_set_and_get_status() {
        let kv = DeterministicKeyValueStore::new();
        let store = StatusStore::new(kv);

        let status = make_status("ci/pipeline", CommitCheckState::Success);
        store.set_status(&status).await.unwrap();

        let got = store.get_status(&test_repo_id(), &test_commit(), "ci/pipeline").await.unwrap();
        assert_eq!(got, Some(status));
    }

    #[tokio::test]
    async fn test_get_missing_status_returns_none() {
        let kv = DeterministicKeyValueStore::new();
        let store = StatusStore::new(kv);

        let got = store.get_status(&test_repo_id(), &test_commit(), "ci/pipeline").await.unwrap();
        assert_eq!(got, None);
    }

    #[tokio::test]
    async fn test_overwrite_status() {
        let kv = DeterministicKeyValueStore::new();
        let store = StatusStore::new(kv);

        let pending = make_status("ci/pipeline", CommitCheckState::Pending);
        store.set_status(&pending).await.unwrap();

        let success = CommitStatus {
            state: CommitCheckState::Success,
            description: "all tests passed".to_string(),
            created_at_ms: 1_700_000_001_000,
            ..pending.clone()
        };
        store.set_status(&success).await.unwrap();

        let got = store.get_status(&test_repo_id(), &test_commit(), "ci/pipeline").await.unwrap().unwrap();
        assert_eq!(got.state, CommitCheckState::Success);
        assert_eq!(got.description, "all tests passed");
    }

    #[tokio::test]
    async fn test_list_statuses_for_commit() {
        let kv = DeterministicKeyValueStore::new();
        let store = StatusStore::new(kv);

        let s1 = make_status("ci/pipeline", CommitCheckState::Success);
        let s2 = make_status("ci/lint", CommitCheckState::Failure);
        let s3 = make_status("ci/security", CommitCheckState::Pending);
        store.set_status(&s1).await.unwrap();
        store.set_status(&s2).await.unwrap();
        store.set_status(&s3).await.unwrap();

        let statuses = store.list_statuses_for_commit(&test_repo_id(), &test_commit()).await.unwrap();
        assert_eq!(statuses.len(), 3);

        let contexts: Vec<&str> = statuses.iter().map(|s| s.context.as_str()).collect();
        assert!(contexts.contains(&"ci/pipeline"));
        assert!(contexts.contains(&"ci/lint"));
        assert!(contexts.contains(&"ci/security"));
    }

    #[tokio::test]
    async fn test_list_statuses_empty_commit() {
        let kv = DeterministicKeyValueStore::new();
        let store = StatusStore::new(kv);

        let statuses = store.list_statuses_for_commit(&test_repo_id(), &test_commit()).await.unwrap();
        assert!(statuses.is_empty());
    }

    #[tokio::test]
    async fn test_check_required_contexts_all_passing() {
        let kv = DeterministicKeyValueStore::new();
        let store = StatusStore::new(kv);

        store.set_status(&make_status("ci/pipeline", CommitCheckState::Success)).await.unwrap();
        store.set_status(&make_status("ci/lint", CommitCheckState::Success)).await.unwrap();

        let failing = store
            .check_required_contexts(&test_repo_id(), &test_commit(), &[
                "ci/pipeline".to_string(),
                "ci/lint".to_string(),
            ])
            .await
            .unwrap();
        assert!(failing.is_empty());
    }

    #[tokio::test]
    async fn test_check_required_contexts_some_failing() {
        let kv = DeterministicKeyValueStore::new();
        let store = StatusStore::new(kv);

        store.set_status(&make_status("ci/pipeline", CommitCheckState::Success)).await.unwrap();
        store.set_status(&make_status("ci/lint", CommitCheckState::Failure)).await.unwrap();

        let failing = store
            .check_required_contexts(&test_repo_id(), &test_commit(), &[
                "ci/pipeline".to_string(),
                "ci/lint".to_string(),
                "ci/security".to_string(),
            ])
            .await
            .unwrap();
        assert_eq!(failing.len(), 2);
        assert!(failing.contains(&"ci/lint".to_string()));
        assert!(failing.contains(&"ci/security".to_string()));
    }

    #[tokio::test]
    async fn test_check_required_contexts_empty_list() {
        let kv = DeterministicKeyValueStore::new();
        let store = StatusStore::new(kv);

        let failing = store.check_required_contexts(&test_repo_id(), &test_commit(), &[]).await.unwrap();
        assert!(failing.is_empty());
    }

    #[tokio::test]
    async fn test_statuses_isolated_between_repos() {
        let kv = DeterministicKeyValueStore::new();
        let store = StatusStore::new(kv);

        let repo1 = test_repo_id();
        let repo2 = RepoId::from_hash(blake3::hash(b"other-repo"));
        let commit = test_commit();

        let s1 = CommitStatus {
            repo_id: repo1,
            commit,
            context: "ci/pipeline".to_string(),
            state: CommitCheckState::Success,
            description: "repo1".to_string(),
            pipeline_run_id: None,
            created_at_ms: 1_000,
        };
        let s2 = CommitStatus {
            repo_id: repo2,
            commit,
            context: "ci/pipeline".to_string(),
            state: CommitCheckState::Failure,
            description: "repo2".to_string(),
            pipeline_run_id: None,
            created_at_ms: 2_000,
        };

        store.set_status(&s1).await.unwrap();
        store.set_status(&s2).await.unwrap();

        let r1 = store.list_statuses_for_commit(&repo1, &commit).await.unwrap();
        assert_eq!(r1.len(), 1);
        assert_eq!(r1[0].state, CommitCheckState::Success);

        let r2 = store.list_statuses_for_commit(&repo2, &commit).await.unwrap();
        assert_eq!(r2.len(), 1);
        assert_eq!(r2[0].state, CommitCheckState::Failure);
    }
}
