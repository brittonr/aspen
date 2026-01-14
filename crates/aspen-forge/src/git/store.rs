//! Git object storage using iroh-blobs.

use std::sync::Arc;
use std::time::Duration;

use aspen_blob::BlobStore;
use aspen_blob::DEFAULT_BLOB_WAIT_TIMEOUT;
use aspen_core::hlc::HLC;
use aspen_core::hlc::create_hlc;
use bytes::Bytes;

use super::object::BlobObject;
use super::object::CommitObject;
use super::object::GitObject;
use super::object::TreeEntry;
use super::object::TreeObject;
use crate::constants::MAX_BLOB_SIZE_BYTES;
use crate::constants::MAX_COMMIT_MESSAGE_BYTES;
use crate::constants::MAX_COMMIT_PARENTS;
use crate::constants::MAX_TREE_ENTRIES;
use crate::constants::MAX_TREE_SIZE_BYTES;
use crate::error::ForgeError;
use crate::error::ForgeResult;
use crate::identity::Author;
use crate::types::SignedObject;

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
    /// Hybrid Logical Clock for deterministic timestamp ordering.
    hlc: HLC,
}

impl<B: BlobStore> GitBlobStore<B> {
    /// Create a new Git blob store.
    ///
    /// # Arguments
    ///
    /// * `blobs` - Blob storage backend
    /// * `secret_key` - Ed25519 secret key for signing
    /// * `node_id` - Node identifier for HLC (e.g., public key hex)
    pub fn new(blobs: Arc<B>, secret_key: iroh::SecretKey, node_id: &str) -> Self {
        let hlc = create_hlc(node_id);
        Self { blobs, secret_key, hlc }
    }

    /// Get the public key of this store's signing identity.
    pub fn public_key(&self) -> iroh::PublicKey {
        self.secret_key.public()
    }

    /// Get a reference to the underlying blob store.
    ///
    /// Used for creating git bridge components.
    pub fn blobs(&self) -> &Arc<B> {
        &self.blobs
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

    /// Ensure all blob references in tree entries are available locally.
    ///
    /// Tiger Style: Bounded wait with explicit timeout prevents unbounded blocking.
    ///
    /// # Arguments
    /// * `entries` - Tree entries to check
    /// * `timeout` - Maximum time to wait for all blobs
    ///
    /// # Returns
    /// * `Ok(())` - All blobs available
    /// * `Err(ForgeError::BlobsNotAvailable)` - Some blobs missing after timeout
    async fn ensure_blobs_available(&self, entries: &[TreeEntry], timeout: Duration) -> ForgeResult<()> {
        // Collect blob hashes from file entries (not directories)
        let blob_hashes: Vec<iroh_blobs::Hash> =
            entries.iter().filter(|e| e.is_file()).map(|e| iroh_blobs::Hash::from_bytes(e.hash)).collect();

        if blob_hashes.is_empty() {
            return Ok(());
        }

        let missing = self
            .blobs
            .wait_available_all(&blob_hashes, timeout)
            .await
            .map_err(|e| ForgeError::BlobStorage { message: e.to_string() })?;

        if !missing.is_empty() {
            return Err(ForgeError::BlobsNotAvailable {
                count: missing.len() as u32,
                timeout_ms: timeout.as_millis() as u64,
            });
        }

        Ok(())
    }

    /// Create a tree from entries and return its hash.
    ///
    /// Entries will be sorted by name. This method waits for all referenced
    /// blobs to be available before creating the tree, preventing race
    /// conditions during replication.
    ///
    /// # Errors
    ///
    /// - `ForgeError::TooManyTreeEntries` if entries exceed `MAX_TREE_ENTRIES`
    /// - `ForgeError::ObjectTooLarge` if the tree exceeds `MAX_TREE_SIZE_BYTES`
    /// - `ForgeError::BlobsNotAvailable` if blobs aren't available within timeout
    pub async fn create_tree(&self, entries: &[TreeEntry]) -> ForgeResult<blake3::Hash> {
        self.create_tree_with_timeout(entries, DEFAULT_BLOB_WAIT_TIMEOUT).await
    }

    /// Create a tree with a custom timeout for blob availability.
    ///
    /// Use this variant when you need control over the timeout, such as
    /// for large trees or slow network conditions.
    ///
    /// # Errors
    ///
    /// - `ForgeError::TooManyTreeEntries` if entries exceed `MAX_TREE_ENTRIES`
    /// - `ForgeError::ObjectTooLarge` if the tree exceeds `MAX_TREE_SIZE_BYTES`
    /// - `ForgeError::BlobsNotAvailable` if blobs aren't available within timeout
    pub async fn create_tree_with_timeout(
        &self,
        entries: &[TreeEntry],
        blob_timeout: Duration,
    ) -> ForgeResult<blake3::Hash> {
        if entries.len() as u32 > MAX_TREE_ENTRIES {
            return Err(ForgeError::TooManyTreeEntries {
                count: entries.len() as u32,
                max: MAX_TREE_ENTRIES,
            });
        }

        // Wait for all referenced blobs to be available
        self.ensure_blobs_available(entries, blob_timeout).await?;

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
        let signed = SignedObject::new(object, &self.secret_key, &self.hlc)?;
        let hash = signed.hash();
        let bytes = signed.to_bytes();

        self.blobs.add_bytes(&bytes).await.map_err(|e| ForgeError::BlobStorage { message: e.to_string() })?;

        Ok(hash)
    }

    /// Get a Git object by hash.
    ///
    /// Tiger Style: Wait for blob availability before reading to handle
    /// eventual consistency in multi-node clusters.
    async fn get_object(&self, hash: &blake3::Hash) -> ForgeResult<SignedObject<GitObject>> {
        self.get_object_with_timeout(hash, DEFAULT_BLOB_WAIT_TIMEOUT).await
    }

    /// Get a Git object by hash with a custom timeout.
    ///
    /// Tiger Style: Bounded wait with explicit timeout prevents unbounded blocking.
    /// Optimization: Try reading locally first before waiting for distributed availability.
    async fn get_object_with_timeout(
        &self,
        hash: &blake3::Hash,
        timeout: Duration,
    ) -> ForgeResult<SignedObject<GitObject>> {
        let iroh_hash = iroh_blobs::Hash::from_bytes(*hash.as_bytes());

        // Optimization: Try reading locally first (fast path for locally available blobs)
        // This avoids the 30-second wait when the blob is already present
        if let Some(bytes) = self
            .blobs
            .get_bytes(&iroh_hash)
            .await
            .map_err(|e| ForgeError::BlobStorage { message: e.to_string() })?
        {
            return self.parse_and_verify_object(hash, bytes);
        }

        // Blob not available locally - wait for distributed availability
        let missing = self
            .blobs
            .wait_available_all(&[iroh_hash], timeout)
            .await
            .map_err(|e| ForgeError::BlobStorage { message: e.to_string() })?;

        if !missing.is_empty() {
            return Err(ForgeError::ObjectNotFound {
                hash: hex::encode(hash.as_bytes()),
            });
        }

        // Now read the blob (should be available after wait)
        let bytes = self
            .blobs
            .get_bytes(&iroh_hash)
            .await
            .map_err(|e| ForgeError::BlobStorage { message: e.to_string() })?
            .ok_or_else(|| ForgeError::ObjectNotFound {
                hash: hex::encode(hash.as_bytes()),
            })?;

        self.parse_and_verify_object(hash, bytes)
    }

    /// Parse and verify a signed Git object from bytes.
    fn parse_and_verify_object(&self, hash: &blake3::Hash, bytes: Bytes) -> ForgeResult<SignedObject<GitObject>> {
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
        self.blobs.has(&iroh_hash).await.map_err(|e| ForgeError::BlobStorage { message: e.to_string() })
    }
}

#[cfg(test)]
mod tests {
    use aspen_blob::InMemoryBlobStore;

    use super::*;

    async fn create_test_store() -> GitBlobStore<InMemoryBlobStore> {
        let blobs = Arc::new(InMemoryBlobStore::new());
        let secret_key = iroh::SecretKey::generate(&mut rand::rng());
        let node_id = hex::encode(secret_key.public().as_bytes());
        GitBlobStore::new(blobs, secret_key, &node_id)
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
        let tree_hash =
            store.create_tree(&[TreeEntry::file("README.md", readme_hash)]).await.expect("should create tree");

        // Create initial commit
        let commit_hash = store.commit(tree_hash, vec![], "Initial commit").await.expect("should create commit");

        // Retrieve and verify
        let commit = store.get_commit(&commit_hash).await.expect("should get commit");
        assert_eq!(commit.message, "Initial commit");
        assert!(commit.parents.is_empty());
        assert_eq!(commit.tree(), tree_hash);

        // Create second commit with parent
        let readme2_hash = store.store_blob(b"# Project v2".to_vec()).await.expect("should store");
        let tree2_hash =
            store.create_tree(&[TreeEntry::file("README.md", readme2_hash)]).await.expect("should create tree");

        let commit2_hash =
            store.commit(tree2_hash, vec![commit_hash], "Update readme").await.expect("should create commit");

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
        let entries: Vec<TreeEntry> =
            (0..MAX_TREE_ENTRIES + 1).map(|i| TreeEntry::file(format!("file{}.txt", i), hash)).collect();

        let result = store.create_tree(&entries).await;

        assert!(matches!(result, Err(ForgeError::TooManyTreeEntries { .. })));
    }
}
