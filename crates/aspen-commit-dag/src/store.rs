//! Commit DAG storage: read/write commits and branch tips in KV.

use aspen_kv_types::ReadRequest;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
use tracing::debug;

use crate::constants::COMMIT_KV_PREFIX;
use crate::constants::COMMIT_TIP_PREFIX;
use crate::error::CommitDagError;
use crate::types::Commit;
use crate::types::CommitId;
use crate::verified::hash::hash_from_hex;
use crate::verified::hash::hash_to_hex;

/// Provides read/write access to commit objects and branch tips in a `KeyValueStore`.
pub struct CommitStore;

impl CommitStore {
    /// Serialize and store a commit at `_sys:commit:{hex}`.
    pub async fn store_commit(commit: &Commit, kv: &dyn KeyValueStore) -> Result<(), CommitDagError> {
        let hex_id = hash_to_hex(&commit.id);
        let key = format!("{COMMIT_KV_PREFIX}{hex_id}");
        let value = postcard::to_allocvec(commit)
            .map_err(|e| CommitDagError::CommitSerializationError { reason: e.to_string() })?;

        let value_str = hex::encode(&value);
        let request = WriteRequest {
            command: WriteCommand::Set {
                key: key.clone(),
                value: value_str,
            },
        };

        kv.write(request).await.map_err(|e| CommitDagError::CommitStorageError { reason: e.to_string() })?;

        debug!(commit_id = %hex_id, "stored commit");
        Ok(())
    }

    /// Load a commit by its CommitId from `_sys:commit:{hex}`.
    pub async fn load_commit(id: &CommitId, kv: &dyn KeyValueStore) -> Result<Commit, CommitDagError> {
        let hex_id = hash_to_hex(id);
        let key = format!("{COMMIT_KV_PREFIX}{hex_id}");

        let result = kv.read(ReadRequest::new(key.clone())).await.map_err(|_| CommitDagError::CommitNotFound {
            commit_id_hex: hex_id.clone(),
        })?;

        let kv_entry = result.kv.ok_or_else(|| CommitDagError::CommitNotFound {
            commit_id_hex: hex_id.clone(),
        })?;

        let bytes = hex::decode(&kv_entry.value).map_err(|e| CommitDagError::CommitSerializationError {
            reason: format!("hex decode: {e}"),
        })?;

        let commit: Commit = postcard::from_bytes(&bytes)
            .map_err(|e| CommitDagError::CommitSerializationError { reason: e.to_string() })?;

        Ok(commit)
    }

    /// Update the branch tip pointer to point to the given commit.
    pub async fn update_branch_tip(
        branch_id: &str,
        commit_id: &CommitId,
        kv: &dyn KeyValueStore,
    ) -> Result<(), CommitDagError> {
        let hex_id = hash_to_hex(commit_id);
        let key = format!("{COMMIT_TIP_PREFIX}{branch_id}");

        let request = WriteRequest {
            command: WriteCommand::Set {
                key,
                value: hex_id.clone(),
            },
        };

        kv.write(request).await.map_err(|e| CommitDagError::CommitStorageError { reason: e.to_string() })?;

        debug!(branch_id, commit_id = %hex_id, "updated branch tip");
        Ok(())
    }

    /// Get the CommitId of the most recent commit on a branch.
    pub async fn get_branch_tip(branch_id: &str, kv: &dyn KeyValueStore) -> Result<Option<CommitId>, CommitDagError> {
        let key = format!("{COMMIT_TIP_PREFIX}{branch_id}");

        let result = kv.read(ReadRequest::new(key)).await;

        match result {
            Ok(r) => match r.kv {
                Some(entry) => {
                    let commit_id =
                        hash_from_hex(&entry.value).ok_or_else(|| CommitDagError::CommitSerializationError {
                            reason: format!("invalid commit tip hex: {}", entry.value),
                        })?;
                    Ok(Some(commit_id))
                }
                None => Ok(None),
            },
            Err(_) => Ok(None),
        }
    }

    /// Walk the commit chain from `start` backwards, following parent links.
    ///
    /// Returns commits in reverse chronological order (newest first).
    /// Stops after `max_depth` commits or when a commit has no parent.
    pub async fn walk_chain(
        start: CommitId,
        kv: &dyn KeyValueStore,
        max_depth: u32,
    ) -> Result<Vec<Commit>, CommitDagError> {
        let mut chain = Vec::new();
        let mut current = Some(start);
        let mut depth = 0u32;

        while let Some(id) = current {
            if depth >= max_depth {
                break;
            }

            let commit = Self::load_commit(&id, kv).await?;
            current = commit.parent;
            chain.push(commit);
            depth = depth.saturating_add(1);
        }

        Ok(chain)
    }

    /// Generate the KV key for a commit.
    pub fn commit_key(commit_id: &CommitId) -> String {
        format!("{COMMIT_KV_PREFIX}{}", hash_to_hex(commit_id))
    }

    /// Generate the KV key for a branch tip.
    pub fn branch_tip_key(branch_id: &str) -> String {
        format!("{COMMIT_TIP_PREFIX}{branch_id}")
    }

    /// Serialize a commit to bytes (for inclusion in Raft batches).
    pub fn serialize_commit(commit: &Commit) -> Result<String, CommitDagError> {
        let bytes = postcard::to_allocvec(commit)
            .map_err(|e| CommitDagError::CommitSerializationError { reason: e.to_string() })?;
        Ok(hex::encode(&bytes))
    }
}

#[cfg(test)]
mod tests {
    use aspen_testing_core::DeterministicKeyValueStore;

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
        let id = compute_commit_id(&parent, branch_id, &mutations_hash, revision, ts);
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

    #[tokio::test]
    async fn store_load_roundtrip() {
        let kv = DeterministicKeyValueStore::new();
        let commit = make_commit("test-branch", None, vec![("a".into(), MutationType::Set("1".into()))], 1, 1000);

        CommitStore::store_commit(&commit, &kv).await.unwrap();
        let loaded = CommitStore::load_commit(&commit.id, &kv).await.unwrap();

        assert_eq!(commit, loaded);
    }

    #[tokio::test]
    async fn branch_tip_update_and_read() {
        let kv = DeterministicKeyValueStore::new();
        let commit = make_commit("br", None, vec![], 1, 1000);

        // No tip initially
        let tip = CommitStore::get_branch_tip("br", &kv).await.unwrap();
        assert!(tip.is_none());

        // Update tip
        CommitStore::update_branch_tip("br", &commit.id, &kv).await.unwrap();

        let tip = CommitStore::get_branch_tip("br", &kv).await.unwrap();
        assert_eq!(tip, Some(commit.id));
    }

    #[tokio::test]
    async fn walk_chain_follows_parents() {
        let kv = DeterministicKeyValueStore::new();

        let c1 = make_commit("br", None, vec![("a".into(), MutationType::Set("1".into()))], 1, 1000);
        let c2 = make_commit("br", Some(c1.id), vec![("b".into(), MutationType::Set("2".into()))], 2, 2000);
        let c3 = make_commit("br", Some(c2.id), vec![("c".into(), MutationType::Set("3".into()))], 3, 3000);

        CommitStore::store_commit(&c1, &kv).await.unwrap();
        CommitStore::store_commit(&c2, &kv).await.unwrap();
        CommitStore::store_commit(&c3, &kv).await.unwrap();

        let chain = CommitStore::walk_chain(c3.id, &kv, 10).await.unwrap();
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[0].id, c3.id);
        assert_eq!(chain[1].id, c2.id);
        assert_eq!(chain[2].id, c1.id);
    }

    #[tokio::test]
    async fn walk_chain_respects_max_depth() {
        let kv = DeterministicKeyValueStore::new();

        let c1 = make_commit("br", None, vec![], 1, 1000);
        let c2 = make_commit("br", Some(c1.id), vec![], 2, 2000);
        let c3 = make_commit("br", Some(c2.id), vec![], 3, 3000);

        CommitStore::store_commit(&c1, &kv).await.unwrap();
        CommitStore::store_commit(&c2, &kv).await.unwrap();
        CommitStore::store_commit(&c3, &kv).await.unwrap();

        let chain = CommitStore::walk_chain(c3.id, &kv, 2).await.unwrap();
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].id, c3.id);
        assert_eq!(chain[1].id, c2.id);
    }

    #[tokio::test]
    async fn load_nonexistent_commit_returns_not_found() {
        let kv = DeterministicKeyValueStore::new();
        let fake_id = [0xAA; 32];

        let result = CommitStore::load_commit(&fake_id, &kv).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            CommitDagError::CommitNotFound { .. } => {}
            other => panic!("expected CommitNotFound, got: {other}"),
        }
    }
}
