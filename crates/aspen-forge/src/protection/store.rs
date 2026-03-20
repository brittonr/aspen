//! Branch protection store implementation.

use std::sync::Arc;

use aspen_core::KeyValueStore;
use aspen_core::ReadRequest;
use aspen_core::ScanRequest;
use aspen_core::WriteRequest;

use super::BranchProtection;
use crate::constants::KV_PREFIX_BRANCH_PROTECTION;
use crate::error::ForgeError;
use crate::error::ForgeResult;
use crate::identity::RepoId;

/// Maximum number of protection rules to return for a single repository.
///
/// Tiger Style: Prevents unbounded scan results.
const MAX_RULES_PER_REPO: u32 = 100;

/// Storage for branch protection rules using Raft consensus.
///
/// Protection rules are plain KV entries — each `(repo, ref_pattern)` tuple
/// has exactly one current rule. Only repository administrators write rules;
/// last-write-wins via Raft is sufficient.
///
/// # Key Format
///
/// ```text
/// forge:protection:{repo_hex}:{ref_pattern}
/// ```
pub struct ProtectionStore<K: KeyValueStore + ?Sized> {
    kv: Arc<K>,
}

impl<K: KeyValueStore + ?Sized> Clone for ProtectionStore<K> {
    fn clone(&self) -> Self {
        Self {
            kv: Arc::clone(&self.kv),
        }
    }
}

impl<K: KeyValueStore + ?Sized> ProtectionStore<K> {
    /// Create a new protection store.
    pub fn new(kv: Arc<K>) -> Self {
        Self { kv }
    }

    /// Set or update a branch protection rule.
    ///
    /// Overwrites any existing rule for the same `(repo, ref_pattern)` tuple.
    pub async fn set_rule(&self, repo_id: &RepoId, rule: &BranchProtection) -> ForgeResult<()> {
        let key = protection_key(repo_id, &rule.ref_pattern);
        let value = serde_json::to_string(rule).map_err(|e| ForgeError::Serialization { message: e.to_string() })?;
        self.kv
            .write(WriteRequest::set(key, value))
            .await
            .map_err(|e| ForgeError::KvStorage { message: e.to_string() })?;
        Ok(())
    }

    /// Get a specific branch protection rule by repo and ref pattern.
    ///
    /// Returns `None` if no rule exists for the given tuple.
    pub async fn get_rule(&self, repo_id: &RepoId, ref_pattern: &str) -> ForgeResult<Option<BranchProtection>> {
        let key = protection_key(repo_id, ref_pattern);
        let result = self.kv.read(ReadRequest::new(key)).await;
        match result {
            Ok(r) => match r.kv {
                Some(entry) => {
                    let rule: BranchProtection = serde_json::from_str(&entry.value)
                        .map_err(|e| ForgeError::Serialization { message: e.to_string() })?;
                    Ok(Some(rule))
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

    /// Delete a branch protection rule.
    ///
    /// Returns `Ok(())` even if the rule doesn't exist.
    pub async fn delete_rule(&self, repo_id: &RepoId, ref_pattern: &str) -> ForgeResult<()> {
        let key = protection_key(repo_id, ref_pattern);
        self.kv
            .write(WriteRequest::delete(key))
            .await
            .map_err(|e| ForgeError::KvStorage { message: e.to_string() })?;
        Ok(())
    }

    /// List all protection rules for a given repository.
    ///
    /// Returns rules sorted by ref pattern.
    pub async fn list_rules(&self, repo_id: &RepoId) -> ForgeResult<Vec<BranchProtection>> {
        let prefix = repo_prefix(repo_id);
        let result = self
            .kv
            .scan(ScanRequest {
                prefix,
                limit_results: Some(MAX_RULES_PER_REPO),
                continuation_token: None,
            })
            .await
            .map_err(|e| ForgeError::KvStorage { message: e.to_string() })?;

        let mut rules = Vec::with_capacity(result.entries.len());
        for entry in &result.entries {
            let rule: BranchProtection =
                serde_json::from_str(&entry.value).map_err(|e| ForgeError::Serialization { message: e.to_string() })?;
            rules.push(rule);
        }
        Ok(rules)
    }
}

/// Build the full KV key for a branch protection rule.
fn protection_key(repo_id: &RepoId, ref_pattern: &str) -> String {
    format!("{}{}:{}", KV_PREFIX_BRANCH_PROTECTION, repo_id.to_hex(), ref_pattern)
}

/// Build the KV prefix for scanning all protection rules of a repository.
fn repo_prefix(repo_id: &RepoId) -> String {
    format!("{}{}:", KV_PREFIX_BRANCH_PROTECTION, repo_id.to_hex())
}

#[cfg(test)]
mod tests {
    use aspen_testing::DeterministicKeyValueStore;

    use super::*;

    fn test_repo_id() -> RepoId {
        RepoId::from_hash(blake3::hash(b"test-repo"))
    }

    fn make_rule(ref_pattern: &str, approvals: u32, contexts: &[&str]) -> BranchProtection {
        BranchProtection {
            ref_pattern: ref_pattern.to_string(),
            required_approvals: approvals,
            required_ci_contexts: contexts.iter().map(|s| s.to_string()).collect(),
            dismiss_stale_approvals: false,
        }
    }

    #[tokio::test]
    async fn test_set_and_get_rule() {
        let kv = DeterministicKeyValueStore::new();
        let store = ProtectionStore::new(kv);

        let rule = make_rule("main", 2, &["ci/pipeline", "ci/lint"]);
        store.set_rule(&test_repo_id(), &rule).await.unwrap();

        let got = store.get_rule(&test_repo_id(), "main").await.unwrap();
        assert_eq!(got, Some(rule));
    }

    #[tokio::test]
    async fn test_get_missing_rule_returns_none() {
        let kv = DeterministicKeyValueStore::new();
        let store = ProtectionStore::new(kv);

        let got = store.get_rule(&test_repo_id(), "main").await.unwrap();
        assert_eq!(got, None);
    }

    #[tokio::test]
    async fn test_overwrite_rule() {
        let kv = DeterministicKeyValueStore::new();
        let store = ProtectionStore::new(kv);

        let rule1 = make_rule("main", 1, &["ci/basic"]);
        store.set_rule(&test_repo_id(), &rule1).await.unwrap();

        let rule2 = BranchProtection {
            required_approvals: 3,
            required_ci_contexts: vec!["ci/pipeline".to_string(), "ci/security".to_string()],
            dismiss_stale_approvals: true,
            ..rule1.clone()
        };
        store.set_rule(&test_repo_id(), &rule2).await.unwrap();

        let got = store.get_rule(&test_repo_id(), "main").await.unwrap().unwrap();
        assert_eq!(got.required_approvals, 3);
        assert_eq!(got.required_ci_contexts.len(), 2);
        assert_eq!(got.dismiss_stale_approvals, true);
    }

    #[tokio::test]
    async fn test_delete_rule() {
        let kv = DeterministicKeyValueStore::new();
        let store = ProtectionStore::new(kv);

        let rule = make_rule("main", 2, &["ci/pipeline"]);
        store.set_rule(&test_repo_id(), &rule).await.unwrap();

        // Verify it exists
        let got = store.get_rule(&test_repo_id(), "main").await.unwrap();
        assert!(got.is_some());

        // Delete it
        store.delete_rule(&test_repo_id(), "main").await.unwrap();

        // Verify it's gone
        let got = store.get_rule(&test_repo_id(), "main").await.unwrap();
        assert!(got.is_none());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_rule() {
        let kv = DeterministicKeyValueStore::new();
        let store = ProtectionStore::new(kv);

        // Deleting a non-existent rule should succeed
        store.delete_rule(&test_repo_id(), "main").await.unwrap();
    }

    #[tokio::test]
    async fn test_list_rules() {
        let kv = DeterministicKeyValueStore::new();
        let store = ProtectionStore::new(kv);

        let rule1 = make_rule("main", 2, &["ci/pipeline"]);
        let rule2 = make_rule("release/*", 3, &["ci/pipeline", "ci/security"]);
        let rule3 = make_rule("develop", 1, &["ci/basic"]);

        store.set_rule(&test_repo_id(), &rule1).await.unwrap();
        store.set_rule(&test_repo_id(), &rule2).await.unwrap();
        store.set_rule(&test_repo_id(), &rule3).await.unwrap();

        let rules = store.list_rules(&test_repo_id()).await.unwrap();
        assert_eq!(rules.len(), 3);

        let patterns: Vec<&str> = rules.iter().map(|r| r.ref_pattern.as_str()).collect();
        assert!(patterns.contains(&"main"));
        assert!(patterns.contains(&"release/*"));
        assert!(patterns.contains(&"develop"));
    }

    #[tokio::test]
    async fn test_list_rules_empty() {
        let kv = DeterministicKeyValueStore::new();
        let store = ProtectionStore::new(kv);

        let rules = store.list_rules(&test_repo_id()).await.unwrap();
        assert!(rules.is_empty());
    }

    #[tokio::test]
    async fn test_rules_isolated_between_repos() {
        let kv = DeterministicKeyValueStore::new();
        let store = ProtectionStore::new(kv);

        let repo1 = test_repo_id();
        let repo2 = RepoId::from_hash(blake3::hash(b"other-repo"));

        let rule1 = make_rule("main", 1, &["ci/basic"]);
        let rule2 = make_rule("main", 3, &["ci/security"]);

        store.set_rule(&repo1, &rule1).await.unwrap();
        store.set_rule(&repo2, &rule2).await.unwrap();

        let r1 = store.list_rules(&repo1).await.unwrap();
        assert_eq!(r1.len(), 1);
        assert_eq!(r1[0].required_approvals, 1);

        let r2 = store.list_rules(&repo2).await.unwrap();
        assert_eq!(r2.len(), 1);
        assert_eq!(r2[0].required_approvals, 3);
    }
}
