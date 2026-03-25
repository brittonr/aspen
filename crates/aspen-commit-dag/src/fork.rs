//! Fork a branch from a historical commit.

use aspen_traits::KeyValueStore;

use crate::error::CommitDagError;
use crate::store::CommitStore;
use crate::types::CommitId;
use crate::verified::commit_hash::verify_commit_integrity;

/// Load a commit, verify its integrity, and return its mutations and ID
/// for use in populating a new branch overlay.
///
/// The caller (BranchOverlay) is responsible for constructing the overlay
/// and populating it with the returned mutations. This keeps the fork logic
/// in aspen-commit-dag without creating a circular dependency.
pub async fn load_fork_source(commit_id: CommitId, kv: &dyn KeyValueStore) -> Result<ForkSource, CommitDagError> {
    let commit = CommitStore::load_commit(&commit_id, kv).await?;

    if !verify_commit_integrity(&commit) {
        let hex_id = hex::encode(commit_id);
        return Err(CommitDagError::CommitCorrupted { commit_id_hex: hex_id });
    }

    Ok(ForkSource {
        commit_id: commit.id,
        mutations: commit.mutations,
    })
}

/// Data extracted from a commit for forking.
pub struct ForkSource {
    /// The CommitId to use as the new branch's parent.
    pub commit_id: CommitId,
    /// The mutations to pre-populate the new branch's dirty map.
    pub mutations: Vec<(String, crate::types::MutationType)>,
}

#[cfg(test)]
mod tests {
    use aspen_testing_core::DeterministicKeyValueStore;

    use super::*;
    use crate::store::CommitStore;
    use crate::types::Commit;
    use crate::types::MutationType;
    use crate::verified::commit_hash::compute_commit_id;
    use crate::verified::commit_hash::compute_mutations_hash;

    fn make_commit(mutations: Vec<(String, MutationType)>) -> Commit {
        let mutations_hash = compute_mutations_hash(&mutations);
        let id = compute_commit_id(&None, "source", &mutations_hash, 1, 1000);
        Commit {
            id,
            parent: None,
            branch_id: "source".to_string(),
            mutations,
            mutations_hash,
            raft_revision: 1,
            chain_hash_at_commit: [0u8; 32],
            timestamp_ms: 1000,
        }
    }

    #[tokio::test]
    async fn fork_from_valid_commit() {
        let kv = DeterministicKeyValueStore::new();
        let mutations = vec![
            ("a".into(), MutationType::Set("1".into())),
            ("b".into(), MutationType::Set("2".into())),
        ];
        let commit = make_commit(mutations.clone());
        CommitStore::store_commit(&commit, &kv).await.unwrap();

        let source = load_fork_source(commit.id, &kv).await.unwrap();
        assert_eq!(source.commit_id, commit.id);
        assert_eq!(source.mutations, mutations);
    }

    #[tokio::test]
    async fn fork_from_nonexistent_commit() {
        let kv = DeterministicKeyValueStore::new();
        let fake_id = [0xAA; 32];

        let result = load_fork_source(fake_id, &kv).await;
        assert!(matches!(result, Err(CommitDagError::CommitNotFound { .. })));
    }

    #[tokio::test]
    async fn fork_from_corrupted_commit() {
        let kv = DeterministicKeyValueStore::new();
        let mutations = vec![("a".into(), MutationType::Set("1".into()))];
        let mut commit = make_commit(mutations);

        // Tamper with mutations after hashing
        commit.mutations.push(("z".into(), MutationType::Set("injected".into())));
        CommitStore::store_commit(&commit, &kv).await.unwrap();

        let result = load_fork_source(commit.id, &kv).await;
        assert!(matches!(result, Err(CommitDagError::CommitCorrupted { .. })));
    }
}
