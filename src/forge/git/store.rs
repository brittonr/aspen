//! Git object storage using iroh-blobs.

use std::sync::Arc;

use super::object::{BlobObject, CommitObject, GitObject, TreeEntry, TreeObject};
use crate::blob::BlobStore;
use crate::forge::constants::{
    MAX_BLOB_SIZE_BYTES, MAX_COMMIT_MESSAGE_BYTES, MAX_COMMIT_PARENTS, MAX_TREE_ENTRIES,
    MAX_TREE_SIZE_BYTES,
};
use crate::forge::error::{ForgeError, ForgeResult};
use crate::forge::identity::Author;
use crate::forge::types::SignedObject;

/// Storage for Git objects using iroh-blobs.
///
/// Git objects are stored as `SignedObject<GitObject>`, providing:
/// - Content addressing via BLAKE3 hash
/// - Authentication via Ed25519 signature
/// - Efficient P2P distribution via iroh-blobs
///
/// # Example
///
/// ```ignore
/// let store = GitBlobStore::new(blob_store, secret_key);
///
/// // Store a blob
/// let content = b"Hello, world!";
/// let hash = store.store_blob(content).await?;
///
/// // Create a tree
/// let tree_hash = store.create_tree(&[
///     TreeEntry::file("hello.txt", hash),
/// ]).await?;
///
/// // Create a commit
/// let commit_hash = store.commit(tree_hash, vec![], "Initial commit").await?;
/// ```
pub struct GitBlobStore<B: BlobStore> {
    blobs: Arc<B>,
    secret_key: iroh::SecretKey,
}

impl<B: BlobStore> GitBlobStore<B> {
    /// Create a new Git blob store.
    pub fn new(blobs: Arc<B>, secret_key: iroh::SecretKey) -> Self {
        Self { blobs, secret_key }
    }

    /// Get the public key of this store's signing identity.
    pub fn public_key(&self) -> iroh::PublicKey {
        self.secret_key.public()
    }

    // ========================================================================
    // Blob Operations
    // ========================================================================

    /// Store a blob and return its hash.
    ///
    /// # Errors
    ///
    /// - `ForgeError::ObjectTooLarge` if the content exceeds `MAX_BLOB_SIZE_BYTES`
    pub async fn store_blob(&self, content: impl Into<Vec<u8>>) -> ForgeResult<blake3::Hash> {
        let content = content.into();

        if content.len() as u64 > MAX_BLOB_SIZE_BYTES {
            return Err(ForgeError::ObjectTooLarge {
                size: content.len() as u64,
                max: MAX_BLOB_SIZE_BYTES,
            });
        }

        let obj = GitObject::Blob(BlobObject::new(content));
        self.store_object(obj).await
    }

    /// Get a blob's content by hash.
    ///
    /// # Errors
    ///
    /// - `ForgeError::ObjectNotFound` if the hash doesn't exist
    /// - `ForgeError::InvalidObject` if the object is not a blob
    pub async fn get_blob(&self, hash: &blake3::Hash) -> ForgeResult<Vec<u8>> {
        let obj = self.get_object(hash).await?;

        match obj.payload {
            GitObject::Blob(blob) => Ok(blob.content),
            other => Err(ForgeError::InvalidObject {
                message: format!("expected blob, found {}", other.object_type()),
            }),
        }
    }

    // ========================================================================
    // Tree Operations
    // ========================================================================

    /// Create a tree from entries and return its hash.
    ///
    /// Entries will be sorted by name.
    ///
    /// # Errors
    ///
    /// - `ForgeError::TooManyTreeEntries` if entries exceed `MAX_TREE_ENTRIES`
    /// - `ForgeError::ObjectTooLarge` if the tree exceeds `MAX_TREE_SIZE_BYTES`
    pub async fn create_tree(&self, entries: &[TreeEntry]) -> ForgeResult<blake3::Hash> {
        if entries.len() as u32 > MAX_TREE_ENTRIES {
            return Err(ForgeError::TooManyTreeEntries {
                count: entries.len() as u32,
                max: MAX_TREE_ENTRIES,
            });
        }

        let tree = TreeObject::new(entries.to_vec());
        let obj = GitObject::Tree(tree);

        // Check serialized size
        let size = obj.size();
        if size as u64 > MAX_TREE_SIZE_BYTES {
            return Err(ForgeError::ObjectTooLarge {
                size: size as u64,
                max: MAX_TREE_SIZE_BYTES,
            });
        }

        self.store_object(obj).await
    }

    /// Get a tree by hash.
    ///
    /// # Errors
    ///
    /// - `ForgeError::ObjectNotFound` if the hash doesn't exist
    /// - `ForgeError::InvalidObject` if the object is not a tree
    pub async fn get_tree(&self, hash: &blake3::Hash) -> ForgeResult<TreeObject> {
        let obj = self.get_object(hash).await?;

        match obj.payload {
            GitObject::Tree(tree) => Ok(tree),
            other => Err(ForgeError::InvalidObject {
                message: format!("expected tree, found {}", other.object_type()),
            }),
        }
    }

    // ========================================================================
    // Commit Operations
    // ========================================================================

    /// Create a commit and return its hash.
    ///
    /// # Arguments
    ///
    /// - `tree`: Hash of the tree this commit points to
    /// - `parents`: Hashes of parent commits (empty for root commit)
    /// - `message`: Commit message
    ///
    /// # Errors
    ///
    /// - `ForgeError::TooManyParents` if parents exceed `MAX_COMMIT_PARENTS`
    /// - `ForgeError::ObjectTooLarge` if message exceeds `MAX_COMMIT_MESSAGE_BYTES`
    pub async fn commit(
        &self,
        tree: blake3::Hash,
        parents: Vec<blake3::Hash>,
        message: impl Into<String>,
    ) -> ForgeResult<blake3::Hash> {
        let message = message.into();

        if parents.len() as u32 > MAX_COMMIT_PARENTS {
            return Err(ForgeError::TooManyParents {
                count: parents.len() as u32,
                max: MAX_COMMIT_PARENTS,
            });
        }

        if message.len() as u32 > MAX_COMMIT_MESSAGE_BYTES {
            return Err(ForgeError::ObjectTooLarge {
                size: message.len() as u64,
                max: MAX_COMMIT_MESSAGE_BYTES as u64,
            });
        }

        let author = Author::from_public_key(self.secret_key.public());
        let commit = CommitObject::new(tree, parents, author, message);
        let obj = GitObject::Commit(commit);

        self.store_object(obj).await
    }

    /// Get a commit by hash.
    ///
    /// # Errors
    ///
    /// - `ForgeError::ObjectNotFound` if the hash doesn't exist
    /// - `ForgeError::InvalidObject` if the object is not a commit
    pub async fn get_commit(&self, hash: &blake3::Hash) -> ForgeResult<CommitObject> {
        let obj = self.get_object(hash).await?;

        match obj.payload {
            GitObject::Commit(commit) => Ok(commit),
            other => Err(ForgeError::InvalidObject {
                message: format!("expected commit, found {}", other.object_type()),
            }),
        }
    }

    // ========================================================================
    // Generic Object Operations
    // ========================================================================

    /// Store a Git object and return its hash.
    async fn store_object(&self, object: GitObject) -> ForgeResult<blake3::Hash> {
        let signed = SignedObject::new(object, &self.secret_key)?;
        let hash = signed.hash();
        let bytes = signed.to_bytes();

        self.blobs
            .add_bytes(&bytes)
            .await
            .map_err(|e| ForgeError::BlobStorage {
                message: e.to_string(),
            })?;

        Ok(hash)
    }

    /// Get a Git object by hash.
    async fn get_object(&self, hash: &blake3::Hash) -> ForgeResult<SignedObject<GitObject>> {
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

        let signed: SignedObject<GitObject> = SignedObject::from_bytes(&bytes)?;

        // Verify signature
        signed.verify()?;

        // Verify hash matches
        if signed.hash() != *hash {
            return Err(ForgeError::InvalidObject {
                message: "hash mismatch".to_string(),
            });
        }

        Ok(signed)
    }

    /// Check if an object exists.
    pub async fn has_object(&self, hash: &blake3::Hash) -> ForgeResult<bool> {
        let iroh_hash = iroh_blobs::Hash::from_bytes(*hash.as_bytes());
        self.blobs
            .has(&iroh_hash)
            .await
            .map_err(|e| ForgeError::BlobStorage {
                message: e.to_string(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blob::InMemoryBlobStore;

    async fn create_test_store() -> GitBlobStore<InMemoryBlobStore> {
        let blobs = Arc::new(InMemoryBlobStore::new());
        let secret_key = iroh::SecretKey::generate(&mut rand::rng());
        GitBlobStore::new(blobs, secret_key)
    }

    #[tokio::test]
    async fn test_blob_roundtrip() {
        let store = create_test_store().await;

        let content = b"Hello, world!";
        let hash = store.store_blob(content.to_vec()).await.expect("should store");

        let retrieved = store.get_blob(&hash).await.expect("should get");
        assert_eq!(retrieved, content);
    }

    #[tokio::test]
    async fn test_tree_creation() {
        let store = create_test_store().await;

        // Create some blobs
        let readme_hash = store.store_blob(b"# README".to_vec()).await.expect("should store");
        let main_hash = store.store_blob(b"fn main() {}".to_vec()).await.expect("should store");

        // Create a tree
        let entries = vec![
            TreeEntry::file("README.md", readme_hash),
            TreeEntry::file("main.rs", main_hash),
        ];

        let tree_hash = store.create_tree(&entries).await.expect("should create tree");

        // Retrieve and verify
        let tree = store.get_tree(&tree_hash).await.expect("should get tree");
        assert_eq!(tree.entries.len(), 2);
        assert_eq!(tree.entries[0].name, "README.md");
        assert_eq!(tree.entries[1].name, "main.rs");
    }

    #[tokio::test]
    async fn test_commit_creation() {
        let store = create_test_store().await;

        // Create a tree
        let readme_hash = store.store_blob(b"# Project".to_vec()).await.expect("should store");
        let tree_hash = store
            .create_tree(&[TreeEntry::file("README.md", readme_hash)])
            .await
            .expect("should create tree");

        // Create initial commit
        let commit_hash = store
            .commit(tree_hash, vec![], "Initial commit")
            .await
            .expect("should create commit");

        // Retrieve and verify
        let commit = store.get_commit(&commit_hash).await.expect("should get commit");
        assert_eq!(commit.message, "Initial commit");
        assert!(commit.parents.is_empty());
        assert_eq!(commit.tree(), tree_hash);

        // Create second commit with parent
        let readme2_hash = store.store_blob(b"# Project v2".to_vec()).await.expect("should store");
        let tree2_hash = store
            .create_tree(&[TreeEntry::file("README.md", readme2_hash)])
            .await
            .expect("should create tree");

        let commit2_hash = store
            .commit(tree2_hash, vec![commit_hash], "Update readme")
            .await
            .expect("should create commit");

        let commit2 = store.get_commit(&commit2_hash).await.expect("should get commit");
        assert_eq!(commit2.parents().len(), 1);
        assert_eq!(commit2.parents()[0], commit_hash);
    }

    #[tokio::test]
    async fn test_blob_size_limit() {
        let store = create_test_store().await;

        // Create oversized blob
        let content = vec![0u8; (MAX_BLOB_SIZE_BYTES + 1) as usize];
        let result = store.store_blob(content).await;

        assert!(matches!(result, Err(ForgeError::ObjectTooLarge { .. })));
    }

    #[tokio::test]
    async fn test_tree_entry_limit() {
        let store = create_test_store().await;

        let hash = blake3::hash(b"test");
        let entries: Vec<TreeEntry> = (0..MAX_TREE_ENTRIES + 1)
            .map(|i| TreeEntry::file(format!("file{}.txt", i), hash))
            .collect();

        let result = store.create_tree(&entries).await;

        assert!(matches!(result, Err(ForgeError::TooManyTreeEntries { .. })));
    }
}
