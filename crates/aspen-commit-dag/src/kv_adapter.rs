//! KV-backed adapter implementing domain store traits for commit DAG persistence.
//!
//! `KvCommitAdapter<S>` wraps any KV store and implements the persistence traits
//! from [`crate::persistence`], bridging the domain-level commit/branch-tip
//! abstraction to the concrete KV key layout used by [`crate::store::CommitStore`].

use async_trait::async_trait;

use aspen_kv_types::ReadRequest;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KvRead;
use aspen_traits::KvWrite;

use crate::constants::COMMIT_KV_PREFIX;
use crate::constants::COMMIT_TIP_PREFIX;
use crate::error::CommitDagError;
use crate::persistence::{BranchTipRead, BranchTipWrite, CommitRead, CommitWrite};
use crate::types::Commit;
use crate::types::CommitId;
use crate::verified::hash::{hash_from_hex, hash_to_hex};

/// Adapter that implements domain store traits over a generic KV store.
///
/// Each trait impl requires only the minimum KV capability:
/// - [`CommitRead`] and [`BranchTipRead`] require `S: KvRead`
/// - [`CommitWrite`] and [`BranchTipWrite`] require `S: KvWrite`
pub struct KvCommitAdapter<S> {
    kv: S,
}

impl<S> KvCommitAdapter<S> {
    pub fn new(kv: S) -> Self {
        Self { kv }
    }
}

#[async_trait]
impl<S: KvRead> CommitRead for KvCommitAdapter<S> {
    async fn load_commit(&self, id: &CommitId) -> Result<Commit, CommitDagError> {
        let hex_id = hash_to_hex(id);
        let key = format!("{COMMIT_KV_PREFIX}{hex_id}");

        let result = self
            .kv
            .read(ReadRequest::new(key))
            .await
            .map_err(|_| CommitDagError::CommitNotFound {
                commit_id_hex: hex_id.clone(),
            })?;

        let kv_entry =
            result
                .kv
                .ok_or_else(|| CommitDagError::CommitNotFound { commit_id_hex: hex_id.clone() })?;

        let bytes = hex::decode(&kv_entry.value).map_err(|e| CommitDagError::CommitSerializationError {
            reason: format!("hex decode: {e}"),
        })?;

        postcard::from_bytes(&bytes)
            .map_err(|e| CommitDagError::CommitSerializationError { reason: e.to_string() })
    }
}

#[async_trait]
impl<S: KvWrite> CommitWrite for KvCommitAdapter<S> {
    async fn store_commit(&self, commit: &Commit) -> Result<(), CommitDagError> {
        let hex_id = hash_to_hex(&commit.id);
        let key = format!("{COMMIT_KV_PREFIX}{hex_id}");
        let value = postcard::to_allocvec(commit)
            .map_err(|e| CommitDagError::CommitSerializationError { reason: e.to_string() })?;

        let request = WriteRequest {
            command: WriteCommand::Set { key, value: hex::encode(&value) },
        };

        self.kv
            .write(request)
            .await
            .map_err(|e| CommitDagError::CommitStorageError { reason: e.to_string() })?;

        Ok(())
    }
}

#[async_trait]
impl<S: KvRead> BranchTipRead for KvCommitAdapter<S> {
    async fn get_branch_tip(&self, branch_id: &str) -> Result<Option<CommitId>, CommitDagError> {
        let key = format!("{COMMIT_TIP_PREFIX}{branch_id}");
        let result = self.kv.read(ReadRequest::new(key)).await;

        match result {
            Ok(r) => match r.kv {
                Some(entry) => {
                    let commit_id = hash_from_hex(&entry.value).ok_or_else(|| {
                        CommitDagError::CommitSerializationError {
                            reason: format!("invalid commit tip hex: {}", entry.value),
                        }
                    })?;
                    Ok(Some(commit_id))
                }
                None => Ok(None),
            },
            Err(_) => Ok(None),
        }
    }
}

#[async_trait]
impl<S: KvWrite> BranchTipWrite for KvCommitAdapter<S> {
    async fn update_branch_tip(
        &self,
        branch_id: &str,
        commit_id: &CommitId,
    ) -> Result<(), CommitDagError> {
        let hex_id = hash_to_hex(commit_id);
        let key = format!("{COMMIT_TIP_PREFIX}{branch_id}");

        let request = WriteRequest {
            command: WriteCommand::Set { key, value: hex_id },
        };

        self.kv
            .write(request)
            .await
            .map_err(|e| CommitDagError::CommitStorageError { reason: e.to_string() })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::walk_chain;
    use crate::types::MutationType;
    use crate::verified::commit_hash::{compute_commit_id, compute_mutations_hash};
    use aspen_testing_core::DeterministicKeyValueStore;

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

    #[tokio::test]
    async fn kv_adapter_store_and_load_roundtrip() {
        let kv = DeterministicKeyValueStore::new();
        let adapter = KvCommitAdapter::new(&kv);
        let commit =
            make_commit("br", None, vec![("k".into(), MutationType::Set("v".into()))], 1, 1000);

        adapter.store_commit(&commit).await.unwrap();
        let loaded = adapter.load_commit(&commit.id).await.unwrap();
        assert_eq!(loaded, commit);
    }

    #[tokio::test]
    async fn kv_adapter_load_missing_commit() {
        let kv = DeterministicKeyValueStore::new();
        let adapter = KvCommitAdapter::new(&kv);
        let result = adapter.load_commit(&[0xAA; 32]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn kv_adapter_branch_tip_crud() {
        let kv = DeterministicKeyValueStore::new();
        let adapter = KvCommitAdapter::new(&kv);

        assert!(adapter.get_branch_tip("br").await.unwrap().is_none());

        let commit = make_commit("br", None, vec![], 1, 1000);
        adapter.update_branch_tip("br", &commit.id).await.unwrap();

        let tip = adapter.get_branch_tip("br").await.unwrap();
        assert_eq!(tip, Some(commit.id));
    }

    #[tokio::test]
    async fn kv_adapter_walk_chain_via_trait() {
        let kv = DeterministicKeyValueStore::new();
        let adapter = KvCommitAdapter::new(&kv);

        let c1 = make_commit("br", None, vec![("a".into(), MutationType::Set("1".into()))], 1, 1000);
        let c2 =
            make_commit("br", Some(c1.id), vec![("b".into(), MutationType::Set("2".into()))], 2, 2000);
        let c3 =
            make_commit("br", Some(c2.id), vec![("c".into(), MutationType::Set("3".into()))], 3, 3000);

        adapter.store_commit(&c1).await.unwrap();
        adapter.store_commit(&c2).await.unwrap();
        adapter.store_commit(&c3).await.unwrap();

        let chain = walk_chain(c3.id, &adapter, 10).await.unwrap();
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[0].id, c3.id);
        assert_eq!(chain[2].id, c1.id);
    }

    #[tokio::test]
    async fn kv_adapter_compatible_with_commit_store_format() {
        let kv = DeterministicKeyValueStore::new();
        let adapter = KvCommitAdapter::new(&kv);
        let commit = make_commit("br", None, vec![("k".into(), MutationType::Set("v".into()))], 1, 1000);

        // Write via static CommitStore, read via adapter
        crate::store::CommitStore::store_commit(&commit, &kv).await.unwrap();
        let loaded = adapter.load_commit(&commit.id).await.unwrap();
        assert_eq!(loaded, commit);
    }

    #[tokio::test]
    async fn kv_adapter_branch_tip_compatible_with_commit_store() {
        let kv = DeterministicKeyValueStore::new();
        let adapter = KvCommitAdapter::new(&kv);
        let commit = make_commit("br", None, vec![], 1, 1000);

        // Write tip via static CommitStore, read via adapter
        crate::store::CommitStore::update_branch_tip("br", &commit.id, &kv).await.unwrap();
        let tip = adapter.get_branch_tip("br").await.unwrap();
        assert_eq!(tip, Some(commit.id));
    }

    fn _assert_commit_read<T: CommitRead>() {}
    fn _assert_commit_write<T: CommitWrite>() {}
    fn _assert_branch_tip_read<T: BranchTipRead>() {}
    fn _assert_branch_tip_write<T: BranchTipWrite>() {}

    #[test]
    fn kv_adapter_implements_all_domain_traits() {
        _assert_commit_read::<KvCommitAdapter<DeterministicKeyValueStore>>();
        _assert_commit_write::<KvCommitAdapter<DeterministicKeyValueStore>>();
        _assert_branch_tip_read::<KvCommitAdapter<DeterministicKeyValueStore>>();
        _assert_branch_tip_write::<KvCommitAdapter<DeterministicKeyValueStore>>();
    }

    #[test]
    fn kv_adapter_read_only_ref_satisfies_read_traits() {
        _assert_commit_read::<KvCommitAdapter<&DeterministicKeyValueStore>>();
        _assert_branch_tip_read::<KvCommitAdapter<&DeterministicKeyValueStore>>();
    }
}
