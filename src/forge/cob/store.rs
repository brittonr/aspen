//! COB storage and state resolution.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use super::change::{CobChange, CobOperation, CobType};
use super::issue::Issue;
use crate::api::{KeyValueStore, ReadConsistency};
use crate::blob::BlobStore;
use crate::forge::constants::{
    KV_PREFIX_COB_HEADS, MAX_COB_CHANGE_SIZE_BYTES, MAX_COB_CHANGES_TO_RESOLVE, MAX_COB_PARENTS,
    MAX_LABELS, MAX_LABEL_LENGTH_BYTES, MAX_TITLE_LENGTH_BYTES,
};
use crate::forge::error::{ForgeError, ForgeResult};
use crate::forge::identity::RepoId;
use crate::forge::types::SignedObject;

/// Storage and resolution for collaborative objects.
pub struct CobStore<B: BlobStore, K: KeyValueStore + ?Sized> {
    blobs: Arc<B>,
    kv: Arc<K>,
    secret_key: iroh::SecretKey,
}

impl<B: BlobStore, K: KeyValueStore + ?Sized> CobStore<B, K> {
    /// Create a new COB store.
    pub fn new(blobs: Arc<B>, kv: Arc<K>, secret_key: iroh::SecretKey) -> Self {
        Self {
            blobs,
            kv,
            secret_key,
        }
    }

    // ========================================================================
    // Issue Operations
    // ========================================================================

    /// Create a new issue.
    ///
    /// # Errors
    ///
    /// - `ForgeError::InvalidCobChange` if title or labels exceed limits
    pub async fn create_issue(
        &self,
        repo_id: &RepoId,
        title: impl Into<String>,
        body: impl Into<String>,
        labels: Vec<String>,
    ) -> ForgeResult<blake3::Hash> {
        let title = title.into();
        let body = body.into();

        // Validate
        if title.len() as u32 > MAX_TITLE_LENGTH_BYTES {
            return Err(ForgeError::InvalidCobChange {
                message: format!("title too long: {} > {}", title.len(), MAX_TITLE_LENGTH_BYTES),
            });
        }

        if labels.len() as u32 > MAX_LABELS {
            return Err(ForgeError::InvalidCobChange {
                message: format!("too many labels: {} > {}", labels.len(), MAX_LABELS),
            });
        }

        for label in &labels {
            if label.len() as u32 > MAX_LABEL_LENGTH_BYTES {
                return Err(ForgeError::InvalidCobChange {
                    message: format!(
                        "label too long: {} > {}",
                        label.len(),
                        MAX_LABEL_LENGTH_BYTES
                    ),
                });
            }
        }

        // Generate a unique COB ID from the content
        let cob_id = blake3::hash(
            &[
                repo_id.0.as_slice(),
                title.as_bytes(),
                &(std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64)
                    .to_le_bytes(),
            ]
            .concat(),
        );

        let op = CobOperation::CreateIssue {
            title,
            body,
            labels,
        };

        let change = CobChange::root(CobType::Issue, cob_id, op);
        self.store_change(repo_id, change).await
    }

    /// Add a comment to an issue.
    pub async fn add_comment(
        &self,
        repo_id: &RepoId,
        issue_id: &blake3::Hash,
        body: impl Into<String>,
    ) -> ForgeResult<blake3::Hash> {
        let heads = self.get_heads(repo_id, CobType::Issue, issue_id).await?;

        if heads.is_empty() {
            return Err(ForgeError::CobNotFound {
                cob_type: CobType::Issue.to_string(),
                cob_id: hex::encode(issue_id.as_bytes()),
            });
        }

        let change = CobChange::new(
            CobType::Issue,
            *issue_id,
            heads,
            CobOperation::Comment { body: body.into() },
        );

        self.store_change(repo_id, change).await
    }

    /// Close an issue.
    pub async fn close_issue(
        &self,
        repo_id: &RepoId,
        issue_id: &blake3::Hash,
        reason: Option<String>,
    ) -> ForgeResult<blake3::Hash> {
        let heads = self.get_heads(repo_id, CobType::Issue, issue_id).await?;

        if heads.is_empty() {
            return Err(ForgeError::CobNotFound {
                cob_type: CobType::Issue.to_string(),
                cob_id: hex::encode(issue_id.as_bytes()),
            });
        }

        let change = CobChange::new(
            CobType::Issue,
            *issue_id,
            heads,
            CobOperation::Close { reason },
        );

        self.store_change(repo_id, change).await
    }

    /// Resolve the current state of an issue by walking its change DAG.
    ///
    /// # Errors
    ///
    /// - `ForgeError::CobNotFound` if the issue doesn't exist
    /// - `ForgeError::TooManyChanges` if the DAG is too large
    pub async fn resolve_issue(
        &self,
        repo_id: &RepoId,
        issue_id: &blake3::Hash,
    ) -> ForgeResult<Issue> {
        let heads = self.get_heads(repo_id, CobType::Issue, issue_id).await?;

        if heads.is_empty() {
            return Err(ForgeError::CobNotFound {
                cob_type: CobType::Issue.to_string(),
                cob_id: hex::encode(issue_id.as_bytes()),
            });
        }

        // Collect all changes by walking the DAG
        let changes = self.collect_changes(heads).await?;

        // Topologically sort changes (parents before children)
        let sorted = self.topological_sort(&changes)?;

        // Apply changes in order
        let mut issue = Issue::default();
        for (hash, signed) in sorted {
            issue.apply_change(
                hash,
                &signed.author,
                signed.timestamp_ms,
                &signed.payload.op,
            );
        }

        Ok(issue)
    }

    // ========================================================================
    // Internal Operations
    // ========================================================================

    /// Store a change and update heads.
    async fn store_change(
        &self,
        repo_id: &RepoId,
        change: CobChange,
    ) -> ForgeResult<blake3::Hash> {
        // Validate parents count
        if change.parents.len() as u32 > MAX_COB_PARENTS {
            return Err(ForgeError::TooManyParents {
                count: change.parents.len() as u32,
                max: MAX_COB_PARENTS,
            });
        }

        // Sign and store
        let signed = SignedObject::new(change.clone(), &self.secret_key)?;
        let hash = signed.hash();
        let bytes = signed.to_bytes();

        // Check size limit
        if bytes.len() as u64 > MAX_COB_CHANGE_SIZE_BYTES {
            return Err(ForgeError::ObjectTooLarge {
                size: bytes.len() as u64,
                max: MAX_COB_CHANGE_SIZE_BYTES,
            });
        }

        // Store in blobs
        self.blobs
            .add_bytes(&bytes)
            .await
            .map_err(|e| ForgeError::BlobStorage {
                message: e.to_string(),
            })?;

        // Update heads: remove parents, add new hash
        self.update_heads(repo_id, &change.cob_type, &change.cob_id(), &change.parents(), hash)
            .await?;

        Ok(hash)
    }

    /// Get the current heads for a COB.
    async fn get_heads(
        &self,
        repo_id: &RepoId,
        cob_type: CobType,
        cob_id: &blake3::Hash,
    ) -> ForgeResult<Vec<blake3::Hash>> {
        let key = format!(
            "{}{}:{}:{}",
            KV_PREFIX_COB_HEADS,
            repo_id.to_hex(),
            cob_type.as_str(),
            hex::encode(cob_id.as_bytes())
        );

        let result = self
            .kv
            .read(crate::api::ReadRequest { key: key.clone(), consistency: ReadConsistency::Linearizable })
            .await?;

        match result.kv.map(|kv| kv.value) {
            Some(value) => {
                // Parse heads from stored format (newline-separated hex hashes)
                let heads: Vec<blake3::Hash> = value
                    .lines()
                    .filter(|s| !s.is_empty())
                    .filter_map(|s| {
                        let bytes = hex::decode(s).ok()?;
                        if bytes.len() == 32 {
                            let mut arr = [0u8; 32];
                            arr.copy_from_slice(&bytes);
                            Some(blake3::Hash::from_bytes(arr))
                        } else {
                            None
                        }
                    })
                    .collect();
                Ok(heads)
            }
            None => Ok(vec![]),
        }
    }

    /// Update heads after storing a change.
    async fn update_heads(
        &self,
        repo_id: &RepoId,
        cob_type: &CobType,
        cob_id: &blake3::Hash,
        parents: &[blake3::Hash],
        new_head: blake3::Hash,
    ) -> ForgeResult<()> {
        let key = format!(
            "{}{}:{}:{}",
            KV_PREFIX_COB_HEADS,
            repo_id.to_hex(),
            cob_type.as_str(),
            hex::encode(cob_id.as_bytes())
        );

        // Get current heads
        let mut heads: HashSet<blake3::Hash> = self
            .get_heads(repo_id, *cob_type, cob_id)
            .await?
            .into_iter()
            .collect();

        // Remove parents (they're no longer heads)
        for parent in parents {
            heads.remove(parent);
        }

        // Add new head
        heads.insert(new_head);

        // Serialize heads
        let value = heads
            .iter()
            .map(|h| hex::encode(h.as_bytes()))
            .collect::<Vec<_>>()
            .join("\n");

        // Store
        self.kv
            .write(crate::api::WriteRequest {
                command: crate::api::WriteCommand::Set { key, value },
            })
            .await?;

        Ok(())
    }

    /// Collect all changes reachable from the given heads.
    async fn collect_changes(
        &self,
        heads: Vec<blake3::Hash>,
    ) -> ForgeResult<HashMap<blake3::Hash, SignedObject<CobChange>>> {
        let mut changes = HashMap::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::from(heads);

        while let Some(hash) = queue.pop_front() {
            if changes.len() as u32 >= MAX_COB_CHANGES_TO_RESOLVE {
                return Err(ForgeError::TooManyChanges {
                    count: changes.len() as u32,
                    max: MAX_COB_CHANGES_TO_RESOLVE,
                });
            }

            if visited.insert(hash) {
                let signed = self.get_change(&hash).await?;

                // Queue parents
                for parent in signed.payload.parents() {
                    if !visited.contains(&parent) {
                        queue.push_back(parent);
                    }
                }

                changes.insert(hash, signed);
            }
        }

        Ok(changes)
    }

    /// Get a change by hash.
    async fn get_change(&self, hash: &blake3::Hash) -> ForgeResult<SignedObject<CobChange>> {
        let iroh_hash = iroh_blobs::Hash::from_bytes(*hash.as_bytes());

        let bytes = self
            .blobs
            .get_bytes(&iroh_hash)
            .await
            .map_err(|e| ForgeError::BlobStorage {
                message: e.to_string(),
            })?
            .ok_or_else(|| ForgeError::ObjectNotFound {
                hash: hex::encode(hash.as_bytes()),
            })?;

        let signed: SignedObject<CobChange> = SignedObject::from_bytes(&bytes)?;
        signed.verify()?;

        Ok(signed)
    }

    /// Topologically sort changes (parents before children).
    fn topological_sort(
        &self,
        changes: &HashMap<blake3::Hash, SignedObject<CobChange>>,
    ) -> ForgeResult<Vec<(blake3::Hash, SignedObject<CobChange>)>> {
        let mut result = Vec::with_capacity(changes.len());
        let mut visited = HashSet::new();
        let mut temp_mark = HashSet::new();

        fn visit(
            hash: blake3::Hash,
            changes: &HashMap<blake3::Hash, SignedObject<CobChange>>,
            visited: &mut HashSet<blake3::Hash>,
            temp_mark: &mut HashSet<blake3::Hash>,
            result: &mut Vec<(blake3::Hash, SignedObject<CobChange>)>,
        ) -> ForgeResult<()> {
            if visited.contains(&hash) {
                return Ok(());
            }

            if temp_mark.contains(&hash) {
                return Err(ForgeError::CobResolutionFailed {
                    message: "cycle detected in change graph".to_string(),
                });
            }

            temp_mark.insert(hash);

            if let Some(signed) = changes.get(&hash) {
                for parent in signed.payload.parents() {
                    visit(parent, changes, visited, temp_mark, result)?;
                }

                visited.insert(hash);
                temp_mark.remove(&hash);
                result.push((hash, signed.clone()));
            }

            Ok(())
        }

        for hash in changes.keys() {
            visit(*hash, changes, &mut visited, &mut temp_mark, &mut result)?;
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::DeterministicKeyValueStore;
    use crate::blob::InMemoryBlobStore;

    async fn create_test_store(
    ) -> CobStore<InMemoryBlobStore, DeterministicKeyValueStore> {
        let blobs = Arc::new(InMemoryBlobStore::new());
        let kv = Arc::new(DeterministicKeyValueStore::new());
        let secret_key = iroh::SecretKey::generate(rand::rngs::OsRng);
        CobStore::new(blobs, kv, secret_key)
    }

    fn test_repo_id() -> RepoId {
        RepoId::from_hash(blake3::hash(b"test-repo"))
    }

    #[tokio::test]
    async fn test_issue_creation() {
        let store = create_test_store().await;
        let repo_id = test_repo_id();

        let issue_id = store
            .create_issue(&repo_id, "Bug report", "Something is broken", vec!["bug".to_string()])
            .await
            .expect("should create issue");

        // Resolve and verify
        let issue = store
            .resolve_issue(&repo_id, &issue_id)
            .await
            .expect("should resolve");

        assert_eq!(issue.title, "Bug report");
        assert_eq!(issue.body, "Something is broken");
        assert!(issue.labels.contains("bug"));
        assert!(issue.state.is_open());
    }

    #[tokio::test]
    async fn test_issue_comments() {
        let store = create_test_store().await;
        let repo_id = test_repo_id();

        let issue_id = store
            .create_issue(&repo_id, "Test issue", "", vec![])
            .await
            .expect("should create issue");

        // Add comments
        store
            .add_comment(&repo_id, &issue_id, "First comment")
            .await
            .expect("should add comment");

        store
            .add_comment(&repo_id, &issue_id, "Second comment")
            .await
            .expect("should add comment");

        // Resolve and verify
        let issue = store
            .resolve_issue(&repo_id, &issue_id)
            .await
            .expect("should resolve");

        assert_eq!(issue.comments.len(), 2);
    }

    #[tokio::test]
    async fn test_issue_close() {
        let store = create_test_store().await;
        let repo_id = test_repo_id();

        let issue_id = store
            .create_issue(&repo_id, "To be closed", "", vec![])
            .await
            .expect("should create issue");

        store
            .close_issue(&repo_id, &issue_id, Some("Fixed".to_string()))
            .await
            .expect("should close");

        let issue = store
            .resolve_issue(&repo_id, &issue_id)
            .await
            .expect("should resolve");

        assert!(issue.state.is_closed());
    }
}
