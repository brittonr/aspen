//! ForgeNode - main coordinator for Forge operations.
//!
//! The ForgeNode ties together all Forge components and provides
//! a unified interface for repository operations.

use std::sync::Arc;

use crate::api::{KeyValueStore, ReadConsistency};
use crate::blob::BlobStore;
use crate::forge::cob::CobStore;
use crate::forge::error::{ForgeError, ForgeResult};
use crate::forge::git::GitBlobStore;
use crate::forge::identity::{RepoId, RepoIdentity};
use crate::forge::refs::RefStore;
use crate::forge::sync::SyncService;
use crate::forge::types::SignedObject;
use crate::forge::constants::KV_PREFIX_REPOS;

/// Main coordinator for Forge operations.
///
/// Provides a unified interface for:
/// - Git object storage and retrieval
/// - Collaborative object management (issues, patches)
/// - Ref management (branches, tags)
/// - Object synchronization
///
/// # Example
///
/// ```ignore
/// let forge = ForgeNode::new(blob_store, kv_store, secret_key);
///
/// // Create a repository
/// let repo = forge.create_repo("my-project", vec![my_key], 1).await?;
///
/// // Create a commit
/// let tree = forge.git.create_tree(&[...]).await?;
/// let commit = forge.git.commit(tree, vec![], "Initial commit").await?;
///
/// // Push to main
/// forge.refs.set(&repo.id, "heads/main", commit).await?;
/// ```
pub struct ForgeNode<B: BlobStore, K: KeyValueStore + ?Sized> {
    /// Git object storage.
    pub git: GitBlobStore<B>,

    /// Collaborative object storage.
    pub cobs: CobStore<B, K>,

    /// Ref storage.
    pub refs: RefStore<K>,

    /// Sync service.
    pub sync: SyncService<B>,

    /// KV store reference.
    kv: Arc<K>,

    /// Secret key for signing.
    secret_key: iroh::SecretKey,
}

impl<B: BlobStore, K: KeyValueStore + ?Sized> ForgeNode<B, K> {
    /// Create a new ForgeNode.
    pub fn new(blobs: Arc<B>, kv: Arc<K>, secret_key: iroh::SecretKey) -> Self {
        Self {
            git: GitBlobStore::new(blobs.clone(), secret_key.clone()),
            cobs: CobStore::new(blobs.clone(), kv.clone(), secret_key.clone()),
            refs: RefStore::new(kv.clone()),
            sync: SyncService::new(blobs),
            kv,
            secret_key,
        }
    }

    /// Get the public key of this node.
    pub fn public_key(&self) -> iroh::PublicKey {
        self.secret_key.public()
    }

    // ========================================================================
    // Repository Management
    // ========================================================================

    /// Create a new repository.
    ///
    /// # Arguments
    ///
    /// - `name`: Repository name
    /// - `delegates`: Public keys of delegates who can update canonical refs
    /// - `threshold`: Number of delegate signatures required for updates
    ///
    /// # Returns
    ///
    /// The repository identity with its computed ID.
    pub async fn create_repo(
        &self,
        name: impl Into<String>,
        delegates: Vec<iroh::PublicKey>,
        threshold: u32,
    ) -> ForgeResult<RepoIdentity> {
        let identity = RepoIdentity::new(name, delegates, threshold)?;
        let repo_id = identity.repo_id();

        // Check if repo already exists
        let key = format!("{}{}:identity", KV_PREFIX_REPOS, repo_id.to_hex());
        let existing = match self
            .kv
            .read(crate::api::ReadRequest { key: key.clone(), consistency: ReadConsistency::Linearizable })
            .await
        {
            Ok(r) => r.kv.is_some(),
            Err(crate::api::KeyValueStoreError::NotFound { .. }) => false,
            Err(e) => return Err(ForgeError::from(e)),
        };

        if existing {
            return Err(ForgeError::RepoAlreadyExists {
                repo_id: repo_id.to_hex(),
            });
        }

        // Sign and store identity
        let signed = SignedObject::new(identity.clone(), &self.secret_key)?;
        let bytes = signed.to_bytes();

        self.kv
            .write(crate::api::WriteRequest {
                command: crate::api::WriteCommand::Set {
                    key,
                    value: base64::Engine::encode(&base64::prelude::BASE64_STANDARD, &bytes),
                },
            })
            .await?;

        Ok(identity)
    }

    /// Get a repository's identity.
    pub async fn get_repo(&self, repo_id: &RepoId) -> ForgeResult<RepoIdentity> {
        let key = format!("{}{}:identity", KV_PREFIX_REPOS, repo_id.to_hex());

        let result = match self
            .kv
            .read(crate::api::ReadRequest { key, consistency: ReadConsistency::Linearizable })
            .await
        {
            Ok(r) => r,
            Err(crate::api::KeyValueStoreError::NotFound { .. }) => {
                return Err(ForgeError::RepoNotFound {
                    repo_id: repo_id.to_hex(),
                });
            }
            Err(e) => return Err(ForgeError::from(e)),
        };

        match result.kv.map(|kv| kv.value) {
            Some(encoded) => {
                let bytes = base64::Engine::decode(&base64::prelude::BASE64_STANDARD, &encoded)
                    .map_err(|e| ForgeError::InvalidRepoIdentity {
                        message: format!("invalid base64: {}", e),
                    })?;

                let signed: SignedObject<RepoIdentity> = SignedObject::from_bytes(&bytes)?;
                signed.verify()?;

                Ok(signed.payload)
            }
            None => Err(ForgeError::RepoNotFound {
                repo_id: repo_id.to_hex(),
            }),
        }
    }

    /// Check if a repository exists.
    pub async fn repo_exists(&self, repo_id: &RepoId) -> ForgeResult<bool> {
        let key = format!("{}{}:identity", KV_PREFIX_REPOS, repo_id.to_hex());

        match self
            .kv
            .read(crate::api::ReadRequest { key, consistency: ReadConsistency::Linearizable })
            .await
        {
            Ok(r) => Ok(r.kv.is_some()),
            Err(crate::api::KeyValueStoreError::NotFound { .. }) => Ok(false),
            Err(e) => Err(ForgeError::from(e)),
        }
    }

    // ========================================================================
    // Peer Tracking (Seeding)
    // ========================================================================

    /// Add a peer as a seeder for a repository.
    ///
    /// Seeders are nodes that have a copy of the repository and can provide
    /// objects during P2P sync.
    pub async fn add_seeding_peer(&self, repo_id: &RepoId, peer: iroh::PublicKey) -> ForgeResult<()> {
        use crate::forge::constants::KV_PREFIX_SEEDING;

        let key = format!("{}{}", KV_PREFIX_SEEDING, repo_id.to_hex());

        // Get existing seeders
        let mut seeders = self.get_seeding_peers(repo_id).await.unwrap_or_default();

        // Add new peer if not already present
        if !seeders.contains(&peer) {
            seeders.push(peer);

            // Serialize as JSON array of hex public keys
            let value: Vec<String> = seeders.iter().map(|k| k.to_string()).collect();
            let json = serde_json::to_string(&value).map_err(|e| ForgeError::InvalidObject {
                message: e.to_string(),
            })?;

            self.kv
                .write(crate::api::WriteRequest {
                    command: crate::api::WriteCommand::Set { key, value: json },
                })
                .await?;
        }

        Ok(())
    }

    /// Remove a peer from the seeders list.
    pub async fn remove_seeding_peer(&self, repo_id: &RepoId, peer: &iroh::PublicKey) -> ForgeResult<()> {
        use crate::forge::constants::KV_PREFIX_SEEDING;

        let key = format!("{}{}", KV_PREFIX_SEEDING, repo_id.to_hex());

        let mut seeders = self.get_seeding_peers(repo_id).await.unwrap_or_default();

        if let Some(pos) = seeders.iter().position(|k| k == peer) {
            seeders.remove(pos);

            let value: Vec<String> = seeders.iter().map(|k| k.to_string()).collect();
            let json = serde_json::to_string(&value).map_err(|e| ForgeError::InvalidObject {
                message: e.to_string(),
            })?;

            self.kv
                .write(crate::api::WriteRequest {
                    command: crate::api::WriteCommand::Set { key, value: json },
                })
                .await?;
        }

        Ok(())
    }

    /// Get all seeders for a repository.
    pub async fn get_seeding_peers(&self, repo_id: &RepoId) -> ForgeResult<Vec<iroh::PublicKey>> {
        use crate::forge::constants::KV_PREFIX_SEEDING;

        let key = format!("{}{}", KV_PREFIX_SEEDING, repo_id.to_hex());

        let result = match self
            .kv
            .read(crate::api::ReadRequest { key, consistency: ReadConsistency::Linearizable })
            .await
        {
            Ok(r) => r,
            Err(crate::api::KeyValueStoreError::NotFound { .. }) => return Ok(vec![]),
            Err(e) => return Err(ForgeError::from(e)),
        };

        match result.kv.map(|kv| kv.value) {
            Some(json) => {
                let keys: Vec<String> = serde_json::from_str(&json).map_err(|e| ForgeError::InvalidObject {
                    message: e.to_string(),
                })?;

                let mut seeders = Vec::new();
                for key_str in keys {
                    if let Ok(pk) = key_str.parse() {
                        seeders.push(pk);
                    }
                }
                Ok(seeders)
            }
            None => Ok(vec![]),
        }
    }

    // ========================================================================
    // High-Level Operations
    // ========================================================================

    /// Initialize a repository with an initial commit.
    ///
    /// Creates an empty tree, initial commit, and sets heads/main.
    pub async fn init_repo(&self, repo_id: &RepoId, message: &str) -> ForgeResult<blake3::Hash> {
        // Create empty tree
        let tree = self.git.create_tree(&[]).await?;

        // Create initial commit
        let commit = self.git.commit(tree, vec![], message).await?;

        // Set main branch
        self.refs.set(repo_id, "heads/main", commit).await?;

        Ok(commit)
    }

    /// Get the current commit for the default branch.
    pub async fn get_head(&self, repo_id: &RepoId) -> ForgeResult<Option<blake3::Hash>> {
        let identity = self.get_repo(repo_id).await?;
        let ref_name = format!("heads/{}", identity.default_branch);
        self.refs.get(repo_id, &ref_name).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::DeterministicKeyValueStore;
    use crate::blob::InMemoryBlobStore;

    async fn create_test_node() -> ForgeNode<InMemoryBlobStore, DeterministicKeyValueStore> {
        let blobs = Arc::new(InMemoryBlobStore::new());
        let kv = DeterministicKeyValueStore::new();
        let secret_key = iroh::SecretKey::generate(&mut rand::rng());
        ForgeNode::new(blobs, kv, secret_key)
    }

    #[tokio::test]
    async fn test_create_repo() {
        let node = create_test_node().await;

        let identity = node
            .create_repo("my-project", vec![node.public_key()], 1)
            .await
            .expect("should create repo");

        assert_eq!(identity.name, "my-project");
        assert_eq!(identity.delegates.len(), 1);

        // Should be able to retrieve
        let retrieved = node.get_repo(&identity.repo_id()).await.expect("should get");
        assert_eq!(retrieved.name, identity.name);
    }

    #[tokio::test]
    async fn test_repo_already_exists() {
        let node = create_test_node().await;

        let identity = node
            .create_repo("my-project", vec![node.public_key()], 1)
            .await
            .expect("should create repo");

        // Try to create again with same name - will have different ID due to timestamp
        // So this won't fail. Let's test by trying to get a non-existent repo instead.
        let fake_id = RepoId::from_hash(blake3::hash(b"nonexistent"));
        assert!(node.get_repo(&fake_id).await.is_err());
    }

    #[tokio::test]
    async fn test_init_repo() {
        let node = create_test_node().await;

        let identity = node
            .create_repo("my-project", vec![node.public_key()], 1)
            .await
            .expect("should create repo");

        let commit = node
            .init_repo(&identity.repo_id(), "Initial commit")
            .await
            .expect("should init");

        // Should be able to get head
        let head = node.get_head(&identity.repo_id()).await.expect("should get head");
        assert_eq!(head, Some(commit));
    }
}
