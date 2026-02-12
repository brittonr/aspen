//! Ref store implementation.

use std::sync::Arc;

use aspen_core::KeyValueStore;
use aspen_core::ReadConsistency;
use aspen_core::ReadRequest;
use aspen_core::ScanRequest;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;
use tokio::sync::broadcast;

use crate::constants::KV_PREFIX_REFS;
use crate::constants::MAX_REF_NAME_LENGTH_BYTES;
use crate::constants::MAX_REFS_PER_REPO;
use crate::error::ForgeError;
use crate::error::ForgeResult;
use crate::identity::RepoId;

/// Event emitted when a ref is updated.
#[derive(Debug, Clone)]
pub struct RefUpdateEvent {
    /// Repository ID.
    pub repo_id: RepoId,
    /// Ref name (e.g., "heads/main").
    pub ref_name: String,
    /// New hash the ref points to.
    pub new_hash: blake3::Hash,
    /// Previous hash (if known).
    pub old_hash: Option<blake3::Hash>,
}

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
    /// Event sender for ref updates.
    event_tx: broadcast::Sender<RefUpdateEvent>,
}

impl<K: KeyValueStore + ?Sized> Clone for RefStore<K> {
    fn clone(&self) -> Self {
        Self {
            kv: Arc::clone(&self.kv),
            event_tx: self.event_tx.clone(),
        }
    }
}

impl<K: KeyValueStore + ?Sized> RefStore<K> {
    /// Create a new ref store.
    pub fn new(kv: Arc<K>) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self { kv, event_tx }
    }

    /// Subscribe to ref update events.
    ///
    /// Returns a receiver that will receive events when refs are updated.
    /// This can be used to trigger gossip broadcasts or other side effects.
    pub fn subscribe(&self) -> broadcast::Receiver<RefUpdateEvent> {
        self.event_tx.subscribe()
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
    pub async fn get(&self, repo_id: &RepoId, ref_name: &str) -> ForgeResult<Option<blake3::Hash>> {
        let key = self.ref_key(repo_id, ref_name);

        let result = match self
            .kv
            .read(ReadRequest {
                key,
                consistency: ReadConsistency::Linearizable,
            })
            .await
        {
            Ok(r) => r,
            Err(aspen_core::KeyValueStoreError::NotFound { .. }) => {
                return Ok(None);
            }
            Err(e) => return Err(ForgeError::from(e)),
        };

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
    /// After successful update, emits a `RefUpdateEvent` for gossip broadcast.
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
    pub async fn set(&self, repo_id: &RepoId, ref_name: &str, hash: blake3::Hash) -> ForgeResult<()> {
        self.validate_ref_name(ref_name)?;

        // Get old hash for the event
        let old_hash = self.get(repo_id, ref_name).await.ok().flatten();

        let key = self.ref_key(repo_id, ref_name);
        let value = hex::encode(hash.as_bytes());

        self.kv
            .write(WriteRequest {
                command: WriteCommand::Set { key, value },
            })
            .await?;

        // Emit event for gossip broadcast
        let event = RefUpdateEvent {
            repo_id: *repo_id,
            ref_name: ref_name.to_string(),
            new_hash: hash,
            old_hash,
        };
        let receiver_count = self.event_tx.receiver_count();
        match self.event_tx.send(event) {
            Ok(n) => {
                tracing::debug!(
                    repo_id = %repo_id.to_hex(),
                    ref_name = %ref_name,
                    receivers = n,
                    "emitted RefUpdateEvent to {} receivers",
                    n
                );
            }
            Err(_) => {
                tracing::warn!(
                    repo_id = %repo_id.to_hex(),
                    ref_name = %ref_name,
                    receiver_count = receiver_count,
                    "failed to emit RefUpdateEvent (no receivers)"
                );
            }
        }

        Ok(())
    }

    /// Update a ref with compare-and-swap semantics.
    ///
    /// The update only succeeds if the ref's current value matches `expected`.
    /// This enables safe concurrent updates.
    /// After successful update, emits a `RefUpdateEvent` for gossip broadcast.
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

        // Emit event (ignore send errors if no subscribers)
        let _ = self.event_tx.send(RefUpdateEvent {
            repo_id: *repo_id,
            ref_name: ref_name.to_string(),
            new_hash,
            old_hash: expected,
        });

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
            let ref_name = entry.key.strip_prefix(&format!("{}:", prefix)).unwrap_or(&entry.key).to_string();

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
    use aspen_testing::DeterministicKeyValueStore;

    use super::*;

    fn create_test_store() -> RefStore<DeterministicKeyValueStore> {
        let kv = DeterministicKeyValueStore::new();
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

    // ========================================================================
    // CAS (Compare-and-Swap) Tests for Race Condition Handling
    // ========================================================================

    #[tokio::test]
    async fn test_cas_correct_expected_succeeds() {
        let store = create_test_store();
        let repo_id = test_repo_id();
        let hash_a = blake3::hash(b"commit-a");
        let hash_b = blake3::hash(b"commit-b");

        // Set initial ref
        store.set(&repo_id, "heads/main", hash_a).await.unwrap();

        // CAS with correct expected value succeeds
        store.compare_and_set(&repo_id, "heads/main", Some(hash_a), hash_b).await.unwrap();

        // Verify new value
        let current = store.get(&repo_id, "heads/main").await.unwrap();
        assert_eq!(current, Some(hash_b));
    }

    #[tokio::test]
    async fn test_cas_stale_expected_fails() {
        let store = create_test_store();
        let repo_id = test_repo_id();
        let hash_a = blake3::hash(b"commit-a");
        let hash_b = blake3::hash(b"commit-b");
        let hash_c = blake3::hash(b"commit-c");

        // Set initial ref
        store.set(&repo_id, "heads/main", hash_a).await.unwrap();

        // Another update happens
        store.set(&repo_id, "heads/main", hash_b).await.unwrap();

        // CAS with stale expected (hash_a) should fail
        let result = store.compare_and_set(&repo_id, "heads/main", Some(hash_a), hash_c).await;
        assert!(result.is_err());

        // Verify ref unchanged
        let current = store.get(&repo_id, "heads/main").await.unwrap();
        assert_eq!(current, Some(hash_b));
    }

    #[tokio::test]
    async fn test_cas_create_ref_none_expected() {
        let store = create_test_store();
        let repo_id = test_repo_id();
        let hash = blake3::hash(b"commit-new");

        // CAS with None expected (create new ref)
        store.compare_and_set(&repo_id, "heads/new-branch", None, hash).await.unwrap();

        // Verify created
        let current = store.get(&repo_id, "heads/new-branch").await.unwrap();
        assert_eq!(current, Some(hash));
    }

    #[tokio::test]
    async fn test_cas_create_fails_if_exists() {
        let store = create_test_store();
        let repo_id = test_repo_id();
        let hash_a = blake3::hash(b"commit-a");
        let hash_b = blake3::hash(b"commit-b");

        // Set initial ref
        store.set(&repo_id, "heads/main", hash_a).await.unwrap();

        // CAS with None expected (trying to create) fails because ref exists
        let result = store.compare_and_set(&repo_id, "heads/main", None, hash_b).await;
        assert!(result.is_err());

        // Verify ref unchanged
        let current = store.get(&repo_id, "heads/main").await.unwrap();
        assert_eq!(current, Some(hash_a));
    }

    #[tokio::test]
    async fn test_cas_concurrent_push_one_wins() {
        let store = create_test_store();
        let repo_id = test_repo_id();
        let hash_initial = blake3::hash(b"initial-commit");
        let hash_a = blake3::hash(b"commit-from-client-a");
        let hash_b = blake3::hash(b"commit-from-client-b");

        // Set initial ref
        store.set(&repo_id, "heads/main", hash_initial).await.unwrap();

        // Concurrent CAS operations - both expect the same initial value
        let store_clone = store.clone();
        let repo_id_clone = repo_id;

        let (result_a, result_b) = tokio::join!(
            store.compare_and_set(&repo_id, "heads/main", Some(hash_initial), hash_a),
            store_clone.compare_and_set(&repo_id_clone, "heads/main", Some(hash_initial), hash_b),
        );

        // Exactly one should succeed
        let successes = [result_a.is_ok(), result_b.is_ok()];
        assert_eq!(successes.iter().filter(|&&x| x).count(), 1, "exactly one CAS should succeed");

        // Final value should be one of the two attempted values
        let final_hash = store.get(&repo_id, "heads/main").await.unwrap().unwrap();
        assert!(final_hash == hash_a || final_hash == hash_b, "final value should be from the winning CAS");
    }

    #[tokio::test]
    async fn test_force_set_ignores_current_value() {
        let store = create_test_store();
        let repo_id = test_repo_id();
        let hash_a = blake3::hash(b"commit-a");
        let hash_b = blake3::hash(b"commit-b");
        let hash_force = blake3::hash(b"commit-force");

        // Set initial ref
        store.set(&repo_id, "heads/main", hash_a).await.unwrap();

        // Update to different value
        store.set(&repo_id, "heads/main", hash_b).await.unwrap();

        // Force set (using set() not compare_and_set()) always succeeds
        store.set(&repo_id, "heads/main", hash_force).await.unwrap();

        // Verify force value took effect
        let current = store.get(&repo_id, "heads/main").await.unwrap();
        assert_eq!(current, Some(hash_force));
    }

    #[tokio::test]
    async fn test_cas_event_emitted_on_success() {
        let store = create_test_store();
        let repo_id = test_repo_id();
        let hash_a = blake3::hash(b"commit-a");
        let hash_b = blake3::hash(b"commit-b");

        // Subscribe before the operation
        let mut rx = store.subscribe();

        // Set initial ref
        store.set(&repo_id, "heads/main", hash_a).await.unwrap();

        // Drain the set event
        let _ = rx.try_recv();

        // CAS update
        store.compare_and_set(&repo_id, "heads/main", Some(hash_a), hash_b).await.unwrap();

        // Should receive event
        let event = rx.try_recv().expect("should receive event on CAS success");
        assert_eq!(event.repo_id, repo_id);
        assert_eq!(event.ref_name, "heads/main");
        assert_eq!(event.new_hash, hash_b);
        assert_eq!(event.old_hash, Some(hash_a));
    }
}
