//! Domain store traits for commit and branch-tip persistence.
//!
//! These traits abstract over the storage backend so that:
//! - Commit DAG logic (walk, diff, GC traversal) is testable with in-memory stores
//! - The KV-backed adapter (`CommitStore`) remains the production implementation
//! - Future backends (SQL, embedded DB) can implement the same contract

use async_trait::async_trait;

use crate::error::CommitDagError;
use crate::types::Commit;
use crate::types::CommitId;

/// Read access to persisted commit objects.
#[async_trait]
pub trait CommitRead: Send + Sync {
    async fn load_commit(&self, id: &CommitId) -> Result<Commit, CommitDagError>;
}

/// Write access to persisted commit objects.
#[async_trait]
pub trait CommitWrite: Send + Sync {
    async fn store_commit(&self, commit: &Commit) -> Result<(), CommitDagError>;
}

/// Read access to branch tip pointers.
#[async_trait]
pub trait BranchTipRead: Send + Sync {
    async fn get_branch_tip(&self, branch_id: &str) -> Result<Option<CommitId>, CommitDagError>;
}

/// Write access to branch tip pointers.
#[async_trait]
pub trait BranchTipWrite: Send + Sync {
    async fn update_branch_tip(&self, branch_id: &str, commit_id: &CommitId) -> Result<(), CommitDagError>;
}

/// Walk the commit chain backwards from `start`, following parent links.
///
/// This is a pure function over `CommitRead` — no KV or Redb dependency.
#[allow(platform_dependent_cast)] // u32->usize safe on all supported platforms (>= 32-bit)
pub async fn walk_chain(
    start: CommitId,
    store: &dyn CommitRead,
    max_depth: u32,
) -> Result<Vec<Commit>, CommitDagError> {
    let mut chain = Vec::with_capacity(max_depth as usize);
    let mut current = Some(start);
    let mut depth = 0u32;

    while let Some(id) = current {
        if depth >= max_depth {
            break;
        }
        let commit = store.load_commit(&id).await?;
        current = commit.parent;
        chain.push(commit);
        depth = depth.saturating_add(1);
    }

    Ok(chain)
}

#[cfg(test)]
mod tests {
    #![allow(
        ambiguous_params,
        multi_lock_ordering,
        reason = "test-only in-memory store and fixture builder keep tests compact"
    )]

    use std::collections::HashMap;
    use std::sync::Mutex;

    use super::*;
    use crate::types::MutationType;
    use crate::verified::commit_hash::compute_commit_id;
    use crate::verified::commit_hash::compute_mutations_hash;

    fn make_commit(
        branch_id: &str,
        parent: Option<CommitId>,
        mutations: Vec<(String, MutationType)>,
        revision: u64,
        ts: u64,
    ) -> Commit {
        let mutations_hash = compute_mutations_hash(&mutations);
        let id = compute_commit_id(&parent, &mutations_hash, branch_id, revision, ts);
        Commit {
            id,
            parent,
            branch_id: branch_id.to_string(),
            mutations,
            mutations_hash,
            raft_revision: revision,
            chain_hash_at_commit: [0u8; 32],
            timestamp_ms: ts,
        }
    }

    struct InMemoryCommitStore {
        commits: Mutex<HashMap<CommitId, Commit>>,
        tips: Mutex<HashMap<String, CommitId>>,
    }

    impl InMemoryCommitStore {
        fn new() -> Self {
            Self {
                commits: Mutex::new(HashMap::new()),
                tips: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl CommitRead for InMemoryCommitStore {
        async fn load_commit(&self, id: &CommitId) -> Result<Commit, CommitDagError> {
            self.commits.lock().unwrap().get(id).cloned().ok_or_else(|| CommitDagError::CommitNotFound {
                commit_id_hex: hex::encode(id),
            })
        }
    }

    #[async_trait]
    impl CommitWrite for InMemoryCommitStore {
        async fn store_commit(&self, commit: &Commit) -> Result<(), CommitDagError> {
            self.commits.lock().unwrap().insert(commit.id, commit.clone());
            Ok(())
        }
    }

    #[async_trait]
    impl BranchTipRead for InMemoryCommitStore {
        async fn get_branch_tip(&self, branch_id: &str) -> Result<Option<CommitId>, CommitDagError> {
            Ok(self.tips.lock().unwrap().get(branch_id).copied())
        }
    }

    #[async_trait]
    impl BranchTipWrite for InMemoryCommitStore {
        async fn update_branch_tip(&self, branch_id: &str, commit_id: &CommitId) -> Result<(), CommitDagError> {
            self.tips.lock().unwrap().insert(branch_id.to_string(), *commit_id);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_store_and_load_commit() {
        let store = InMemoryCommitStore::new();
        let commit = make_commit("br", None, vec![("k".into(), MutationType::Set("v".into()))], 1, 1000);

        store.store_commit(&commit).await.unwrap();
        let loaded = store.load_commit(&commit.id).await.unwrap();
        assert_eq!(loaded, commit);
    }

    #[tokio::test]
    async fn test_load_missing_commit() {
        let store = InMemoryCommitStore::new();
        let result = store.load_commit(&[0xAA; 32]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_branch_tip_read_write() {
        let store = InMemoryCommitStore::new();
        assert!(store.get_branch_tip("br").await.unwrap().is_none());

        let commit = make_commit("br", None, vec![], 1, 1000);
        store.update_branch_tip("br", &commit.id).await.unwrap();

        let tip = store.get_branch_tip("br").await.unwrap();
        assert_eq!(tip, Some(commit.id));
    }

    #[tokio::test]
    async fn test_branch_tip_overwrite() {
        let store = InMemoryCommitStore::new();
        let c1 = make_commit("br", None, vec![], 1, 1000);
        let c2 = make_commit("br", Some(c1.id), vec![], 2, 2000);

        store.update_branch_tip("br", &c1.id).await.unwrap();
        store.update_branch_tip("br", &c2.id).await.unwrap();

        let tip = store.get_branch_tip("br").await.unwrap();
        assert_eq!(tip, Some(c2.id));
    }

    #[tokio::test]
    async fn test_walk_chain_via_trait() {
        let store = InMemoryCommitStore::new();
        let c1 = make_commit("br", None, vec![("a".into(), MutationType::Set("1".into()))], 1, 1000);
        let c2 = make_commit("br", Some(c1.id), vec![("b".into(), MutationType::Set("2".into()))], 2, 2000);
        let c3 = make_commit("br", Some(c2.id), vec![("c".into(), MutationType::Set("3".into()))], 3, 3000);

        store.store_commit(&c1).await.unwrap();
        store.store_commit(&c2).await.unwrap();
        store.store_commit(&c3).await.unwrap();

        let chain = walk_chain(c3.id, &store, 10).await.unwrap();
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[0].id, c3.id);
        assert_eq!(chain[2].id, c1.id);
    }

    #[tokio::test]
    async fn test_walk_chain_max_depth() {
        let store = InMemoryCommitStore::new();
        let c1 = make_commit("br", None, vec![], 1, 1000);
        let c2 = make_commit("br", Some(c1.id), vec![], 2, 2000);
        let c3 = make_commit("br", Some(c2.id), vec![], 3, 3000);

        store.store_commit(&c1).await.unwrap();
        store.store_commit(&c2).await.unwrap();
        store.store_commit(&c3).await.unwrap();

        let chain = walk_chain(c3.id, &store, 2).await.unwrap();
        assert_eq!(chain.len(), 2);
    }

    #[tokio::test]
    async fn test_walk_chain_missing_parent() {
        let store = InMemoryCommitStore::new();
        let c2 = make_commit("br", Some([0xFF; 32]), vec![], 2, 2000);
        store.store_commit(&c2).await.unwrap();

        let result = walk_chain(c2.id, &store, 10).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_multiple_branches_independent() {
        let store = InMemoryCommitStore::new();
        let c_a = make_commit("branch-a", None, vec![], 1, 1000);
        let c_b = make_commit("branch-b", None, vec![], 1, 1000);

        store.update_branch_tip("branch-a", &c_a.id).await.unwrap();
        store.update_branch_tip("branch-b", &c_b.id).await.unwrap();

        assert_eq!(store.get_branch_tip("branch-a").await.unwrap(), Some(c_a.id));
        assert_eq!(store.get_branch_tip("branch-b").await.unwrap(), Some(c_b.id));
        assert_ne!(c_a.id, c_b.id);
    }

    fn _assert_commit_read<T: CommitRead>() {}
    fn _assert_commit_write<T: CommitWrite>() {}
    fn _assert_branch_tip_read<T: BranchTipRead>() {}
    fn _assert_branch_tip_write<T: BranchTipWrite>() {}

    #[test]
    fn test_in_memory_store_implements_all_traits() {
        _assert_commit_read::<InMemoryCommitStore>();
        _assert_commit_write::<InMemoryCommitStore>();
        _assert_branch_tip_read::<InMemoryCommitStore>();
        _assert_branch_tip_write::<InMemoryCommitStore>();
    }
}
