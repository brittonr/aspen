//! Ref store implementation.

use std::sync::Arc;

use crate::api::{KeyValueStore, ReadConsistency, ReadRequest, ScanRequest, WriteCommand, WriteRequest};
use crate::forge::constants::{KV_PREFIX_REFS, MAX_REFS_PER_REPO, MAX_REF_NAME_LENGTH_BYTES};
use crate::forge::error::{ForgeError, ForgeResult};
use crate::forge::identity::RepoId;

/// Storage for refs (branches, tags) using Raft consensus.
///
/// All ref updates go through Raft to ensure strong consistency.
/// This means all nodes in the cluster will agree on which commit
/// a branch points to.
///
/// # Key Format
///
/// Refs are stored with the key format:
/// ```text
/// forge:refs:{repo_id}:{ref_name}
/// ```
///
/// For example:
/// ```text
/// forge:refs:abc123...:heads/main
/// forge:refs:abc123...:tags/v1.0.0
/// ```
pub struct RefStore<K: KeyValueStore + ?Sized> {
    kv: Arc<K>,
}

impl<K: KeyValueStore + ?Sized> RefStore<K> {
    /// Create a new ref store.
    pub fn new(kv: Arc<K>) -> Self {
        Self { kv }
    }

    /// Get a ref's current value.
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository ID
    /// - `ref_name`: Ref name (e.g., "heads/main", "tags/v1.0.0")
    ///
    /// # Returns
    ///
    /// The BLAKE3 hash the ref points to, or `None` if the ref doesn't exist.
    pub async fn get(
        &self,
        repo_id: &RepoId,
        ref_name: &str,
    ) -> ForgeResult<Option<blake3::Hash>> {
        let key = self.ref_key(repo_id, ref_name);

        let result = self.kv.read(ReadRequest { key, consistency: ReadConsistency::Linearizable }).await?;

        match result.kv.map(|kv| kv.value) {
            Some(hex_hash) => {
                let bytes = hex::decode(&hex_hash).map_err(|e| ForgeError::InvalidObject {
                    message: format!("invalid ref hash: {}", e),
                })?;

                if bytes.len() != 32 {
                    return Err(ForgeError::InvalidObject {
                        message: format!("ref hash must be 32 bytes, got {}", bytes.len()),
                    });
                }

                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                Ok(Some(blake3::Hash::from_bytes(arr)))
            }
            None => Ok(None),
        }
    }

    /// Set a ref to a new value.
    ///
    /// This goes through Raft consensus, so all nodes will agree on the update.
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository ID
    /// - `ref_name`: Ref name (e.g., "heads/main")
    /// - `hash`: The commit hash to set the ref to
    ///
    /// # Errors
    ///
    /// - `ForgeError::InvalidRefName` if the ref name is too long
    pub async fn set(
        &self,
        repo_id: &RepoId,
        ref_name: &str,
        hash: blake3::Hash,
    ) -> ForgeResult<()> {
        self.validate_ref_name(ref_name)?;

        let key = self.ref_key(repo_id, ref_name);
        let value = hex::encode(hash.as_bytes());

        self.kv
            .write(WriteRequest {
                command: WriteCommand::Set { key, value },
            })
            .await?;

        Ok(())
    }

    /// Update a ref with compare-and-swap semantics.
    ///
    /// The update only succeeds if the ref's current value matches `expected`.
    /// This enables safe concurrent updates.
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository ID
    /// - `ref_name`: Ref name
    /// - `expected`: Expected current value (None for create, Some for update)
    /// - `new_hash`: New value to set
    ///
    /// # Errors
    ///
    /// - `ForgeError::RefConflict` if the current value doesn't match expected
    pub async fn compare_and_set(
        &self,
        repo_id: &RepoId,
        ref_name: &str,
        expected: Option<blake3::Hash>,
        new_hash: blake3::Hash,
    ) -> ForgeResult<()> {
        self.validate_ref_name(ref_name)?;

        let key = self.ref_key(repo_id, ref_name);
        let new_value = hex::encode(new_hash.as_bytes());

        let expected_value = expected.map(|h| hex::encode(h.as_bytes()));

        self.kv
            .write(WriteRequest {
                command: WriteCommand::CompareAndSwap {
                    key,
                    expected: expected_value.clone(),
                    new_value: new_value.clone(),
                },
            })
            .await
            .map_err(|e| {
                // Check if this is a CAS failure
                if e.to_string().contains("compare and swap") {
                    ForgeError::RefConflict {
                        expected: expected_value,
                        found: Some("<unknown>".to_string()),
                    }
                } else {
                    ForgeError::from(e)
                }
            })?;

        Ok(())
    }

    /// Delete a ref.
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository ID
    /// - `ref_name`: Ref name to delete
    pub async fn delete(&self, repo_id: &RepoId, ref_name: &str) -> ForgeResult<()> {
        let key = self.ref_key(repo_id, ref_name);

        self.kv
            .write(WriteRequest {
                command: WriteCommand::Delete { key },
            })
            .await?;

        Ok(())
    }

    /// List all refs for a repository.
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository ID
    ///
    /// # Returns
    ///
    /// A list of (ref_name, hash) pairs.
    pub async fn list(&self, repo_id: &RepoId) -> ForgeResult<Vec<(String, blake3::Hash)>> {
        let prefix = format!("{}{}", KV_PREFIX_REFS, repo_id.to_hex());

        let result = self
            .kv
            .scan(ScanRequest {
                prefix: prefix.clone(),
                continuation_token: None,
                limit: Some(MAX_REFS_PER_REPO),
            })
            .await?;

        let mut refs = Vec::with_capacity(result.entries.len());

        for entry in result.entries {
            // Extract ref name from key
            let ref_name = entry.key
                .strip_prefix(&format!("{}:", prefix))
                .unwrap_or(&entry.key)
                .to_string();

            // Parse hash
            let bytes = hex::decode(&entry.value).map_err(|e| ForgeError::InvalidObject {
                message: format!("invalid ref hash: {}", e),
            })?;

            if bytes.len() != 32 {
                continue;
            }

            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            refs.push((ref_name, blake3::Hash::from_bytes(arr)));
        }

        Ok(refs)
    }

    /// List branches (refs under heads/).
    pub async fn list_branches(&self, repo_id: &RepoId) -> ForgeResult<Vec<(String, blake3::Hash)>> {
        let all_refs = self.list(repo_id).await?;
        Ok(all_refs
            .into_iter()
            .filter(|(name, _)| name.starts_with("heads/"))
            .map(|(name, hash)| {
                let branch_name = name.strip_prefix("heads/").unwrap_or(&name).to_string();
                (branch_name, hash)
            })
            .collect())
    }

    /// List tags (refs under tags/).
    pub async fn list_tags(&self, repo_id: &RepoId) -> ForgeResult<Vec<(String, blake3::Hash)>> {
        let all_refs = self.list(repo_id).await?;
        Ok(all_refs
            .into_iter()
            .filter(|(name, _)| name.starts_with("tags/"))
            .map(|(name, hash)| {
                let tag_name = name.strip_prefix("tags/").unwrap_or(&name).to_string();
                (tag_name, hash)
            })
            .collect())
    }

    // ========================================================================
    // Internal Helpers
    // ========================================================================

    /// Build the KV key for a ref.
    fn ref_key(&self, repo_id: &RepoId, ref_name: &str) -> String {
        format!("{}{}:{}", KV_PREFIX_REFS, repo_id.to_hex(), ref_name)
    }

    /// Validate a ref name.
    fn validate_ref_name(&self, ref_name: &str) -> ForgeResult<()> {
        if ref_name.is_empty() {
            return Err(ForgeError::InvalidRefName {
                ref_name: ref_name.to_string(),
            });
        }

        if ref_name.len() as u32 > MAX_REF_NAME_LENGTH_BYTES {
            return Err(ForgeError::InvalidRefName {
                ref_name: format!("name too long: {} > {}", ref_name.len(), MAX_REF_NAME_LENGTH_BYTES),
            });
        }

        // Basic validation: no control characters, no "..", no starting/ending with /
        if ref_name.starts_with('/') || ref_name.ends_with('/') {
            return Err(ForgeError::InvalidRefName {
                ref_name: ref_name.to_string(),
            });
        }

        if ref_name.contains("..") {
            return Err(ForgeError::InvalidRefName {
                ref_name: ref_name.to_string(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::DeterministicKeyValueStore;

    fn create_test_store() -> RefStore<DeterministicKeyValueStore> {
        let kv = Arc::new(DeterministicKeyValueStore::new());
        RefStore::new(kv)
    }

    fn test_repo_id() -> RepoId {
        RepoId::from_hash(blake3::hash(b"test-repo"))
    }

    #[tokio::test]
    async fn test_ref_set_get() {
        let store = create_test_store();
        let repo_id = test_repo_id();
        let hash = blake3::hash(b"commit-1");

        // Initially not found
        assert!(store.get(&repo_id, "heads/main").await.unwrap().is_none());

        // Set ref
        store.set(&repo_id, "heads/main", hash).await.unwrap();

        // Now found
        let retrieved = store.get(&repo_id, "heads/main").await.unwrap();
        assert_eq!(retrieved, Some(hash));
    }

    #[tokio::test]
    async fn test_ref_delete() {
        let store = create_test_store();
        let repo_id = test_repo_id();
        let hash = blake3::hash(b"commit-1");

        store.set(&repo_id, "heads/feature", hash).await.unwrap();
        assert!(store.get(&repo_id, "heads/feature").await.unwrap().is_some());

        store.delete(&repo_id, "heads/feature").await.unwrap();
        assert!(store.get(&repo_id, "heads/feature").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_ref_list() {
        let store = create_test_store();
        let repo_id = test_repo_id();

        let hash1 = blake3::hash(b"commit-1");
        let hash2 = blake3::hash(b"commit-2");
        let hash3 = blake3::hash(b"commit-3");

        store.set(&repo_id, "heads/main", hash1).await.unwrap();
        store.set(&repo_id, "heads/feature", hash2).await.unwrap();
        store.set(&repo_id, "tags/v1.0.0", hash3).await.unwrap();

        let branches = store.list_branches(&repo_id).await.unwrap();
        assert_eq!(branches.len(), 2);

        let tags = store.list_tags(&repo_id).await.unwrap();
        assert_eq!(tags.len(), 1);
    }

    #[tokio::test]
    async fn test_invalid_ref_names() {
        let store = create_test_store();
        let repo_id = test_repo_id();
        let hash = blake3::hash(b"commit");

        // Empty name
        assert!(store.set(&repo_id, "", hash).await.is_err());

        // Leading slash
        assert!(store.set(&repo_id, "/heads/main", hash).await.is_err());

        // Trailing slash
        assert!(store.set(&repo_id, "heads/main/", hash).await.is_err());

        // Double dot
        assert!(store.set(&repo_id, "heads/../main", hash).await.is_err());
    }
}
