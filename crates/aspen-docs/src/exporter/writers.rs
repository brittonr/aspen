//! Concrete implementations of the `DocsWriter` trait.
//!
//! Provides multiple writer backends:
//! - `IrohDocsWriter`: Direct iroh-docs Store access
//! - `SyncHandleDocsWriter`: P2P sync via SyncHandle
//! - `BlobBackedDocsWriter`: Full P2P content transfer via iroh-blobs + iroh-docs
//! - `InMemoryDocsWriter`: In-memory storage for testing

use anyhow::Context;
use anyhow::Result;
use aspen_blob::prelude::*;
use async_trait::async_trait;
use iroh_docs::Author;
use iroh_docs::store::Store;
use tracing::debug;
use tracing::instrument;

use super::core::DocsWriter;

// ============================================================================
// Concrete iroh-docs Implementation
// ============================================================================

/// Concrete implementation of `DocsWriter` using iroh-docs.
///
/// Uses a `Store` and `Author` to write entries to an iroh-docs namespace.
/// The store must have a replica with write capability for the namespace.
///
/// # Example
///
/// ```ignore
/// use aspen::docs::IrohDocsWriter;
/// use iroh_docs::store::Store;
/// use iroh_docs::Author;
///
/// // Create store and get namespace
/// let store = Store::memory();
/// let namespace_id = /* ... */;
/// let author = Author::new(/* ... */);
///
/// let writer = IrohDocsWriter::new(store, namespace_id, author);
/// writer.set_entry(b"key".to_vec(), b"value".to_vec()).await?;
/// ```
pub struct IrohDocsWriter {
    /// The iroh-docs store (wrapped in Mutex for interior mutability).
    store: tokio::sync::Mutex<Store>,
    /// Namespace ID for the document.
    namespace_id: iroh_docs::NamespaceId,
    /// Author for signing entries.
    author: Author,
}

impl IrohDocsWriter {
    /// Create a new IrohDocsWriter.
    ///
    /// # Arguments
    /// * `store` - The iroh-docs store
    /// * `namespace_id` - The namespace ID for the document
    /// * `author` - The author for signing entries
    pub fn new(store: Store, namespace_id: iroh_docs::NamespaceId, author: Author) -> Self {
        Self {
            store: tokio::sync::Mutex::new(store),
            namespace_id,
            author,
        }
    }

    /// Get the namespace ID of this writer.
    pub fn namespace_id(&self) -> iroh_docs::NamespaceId {
        self.namespace_id
    }

    /// Get the author of this writer.
    pub fn author(&self) -> &Author {
        &self.author
    }
}

#[async_trait]
impl DocsWriter for IrohDocsWriter {
    #[instrument(skip(self, value), fields(key_len = key.len(), value_len = value.len()))]
    async fn set_entry(&self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        // Lock the store for writing
        let mut store = self.store.lock().await;

        // Open the replica for writing
        let mut replica = store.open_replica(&self.namespace_id).context("failed to open replica")?;

        // hash_and_insert computes the BLAKE3 hash and inserts the entry
        // Note: This stores the entry metadata (hash, size, timestamp) in the replica,
        // but the actual content data needs to be stored separately in a blob store.
        let _hash =
            replica.hash_and_insert(&key, &self.author, &value).context("failed to insert entry into replica")?;

        debug!(namespace = %self.namespace_id, "entry inserted into docs");
        Ok(())
    }

    #[instrument(skip(self), fields(key_len = key.len()))]
    async fn delete_entry(&self, key: Vec<u8>) -> Result<()> {
        // Lock the store for writing
        let mut store = self.store.lock().await;

        // Open the replica for writing
        let mut replica = store.open_replica(&self.namespace_id).context("failed to open replica")?;

        // In iroh-docs, deletion is done by inserting an empty entry as a tombstone.
        // The delete_prefix method does this by inserting an empty entry at the key.
        // Note: delete_prefix signature is (prefix, author)
        let _deleted_count =
            replica.delete_prefix(&key, &self.author).context("failed to delete entry from replica")?;

        debug!(namespace = %self.namespace_id, "entry deleted from docs");
        Ok(())
    }
}

// ============================================================================
// SyncHandle-based Implementation for P2P Sync
// ============================================================================

/// Docs writer implementation that uses `SyncHandle` for P2P sync compatibility.
///
/// Unlike `IrohDocsWriter` which owns the Store directly, this implementation
/// works with a `SyncHandle` that manages the Store. This allows both the
/// DocsExporter and the P2P sync protocol handler to share the same Store.
///
/// Use this when P2P sync is enabled, use `IrohDocsWriter` when only local
/// export is needed (no P2P sync).
pub struct SyncHandleDocsWriter {
    /// Sync handle for replica operations.
    sync_handle: iroh_docs::actor::SyncHandle,
    /// Namespace ID for the document.
    namespace_id: iroh_docs::NamespaceId,
    /// Author for signing entries.
    author: Author,
}

impl SyncHandleDocsWriter {
    /// Create a new SyncHandleDocsWriter.
    ///
    /// # Arguments
    /// * `sync_handle` - The sync handle from DocsSyncResources
    /// * `namespace_id` - The namespace ID for the document
    /// * `author` - The author for signing entries
    pub fn new(
        sync_handle: iroh_docs::actor::SyncHandle,
        namespace_id: iroh_docs::NamespaceId,
        author: Author,
    ) -> Self {
        Self {
            sync_handle,
            namespace_id,
            author,
        }
    }
}

#[async_trait]
impl DocsWriter for SyncHandleDocsWriter {
    #[instrument(skip(self, value), fields(key_len = key.len(), value_len = value.len()))]
    async fn set_entry(&self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        // Compute BLAKE3 hash of the value (same as iroh-blobs uses)
        let hash = iroh_blobs::Hash::new(&value);
        let len = value.len() as u64;
        let author_id = self.author.id();

        // Insert entry metadata via sync handle
        // Note: The actual content should be stored in iroh-blobs for P2P fetching.
        // For now, this stores the metadata (hash, len) which allows sync of entry info.
        self.sync_handle
            .insert_local(self.namespace_id, author_id, bytes::Bytes::from(key), hash, len)
            .await
            .context("failed to insert entry via sync handle")?;

        debug!(namespace = %self.namespace_id, hash = %hash.fmt_short(), "entry inserted via sync handle");
        Ok(())
    }

    #[instrument(skip(self), fields(key_len = key.len()))]
    async fn delete_entry(&self, key: Vec<u8>) -> Result<()> {
        // iroh-docs doesn't support empty entries, so we use a tombstone marker
        // This is a convention: a single null byte indicates deletion
        const TOMBSTONE: &[u8] = b"\x00";
        let hash = iroh_blobs::Hash::new(TOMBSTONE);
        let author_id = self.author.id();

        self.sync_handle
            .insert_local(self.namespace_id, author_id, bytes::Bytes::from(key), hash, TOMBSTONE.len() as u64)
            .await
            .context("failed to delete entry via sync handle")?;

        debug!(namespace = %self.namespace_id, "entry deleted via sync handle (tombstone)");
        Ok(())
    }
}

// ============================================================================
// SyncHandle + Blob Store Implementation for Full P2P Content Transfer
// ============================================================================

/// Docs writer that stores content in iroh-blobs for full P2P content transfer.
///
/// This writer combines iroh-docs (for entry metadata sync) with iroh-blobs
/// (for actual content storage). When entries sync to peers, they can fetch
/// the actual content via iroh-blobs using the hash in the entry metadata.
///
/// # Data Flow
///
/// 1. Content is stored in iroh-blobs → returns Hash
/// 2. Entry metadata (key, hash, len) is stored in iroh-docs
/// 3. iroh-docs syncs entry metadata to peers
/// 4. Peers can fetch content from iroh-blobs using the Hash
pub struct BlobBackedDocsWriter {
    /// Sync handle for replica operations.
    sync_handle: iroh_docs::actor::SyncHandle,
    /// Namespace ID for the document.
    namespace_id: iroh_docs::NamespaceId,
    /// Author for signing entries.
    author: Author,
    /// Blob store for content storage.
    blob_store: std::sync::Arc<aspen_blob::store::IrohBlobStore>,
}

impl BlobBackedDocsWriter {
    /// Create a new BlobBackedDocsWriter.
    ///
    /// # Arguments
    /// * `sync_handle` - The sync handle from DocsSyncResources
    /// * `namespace_id` - The namespace ID for the document
    /// * `author` - The author for signing entries
    /// * `blob_store` - The blob store for content storage
    pub fn new(
        sync_handle: iroh_docs::actor::SyncHandle,
        namespace_id: iroh_docs::NamespaceId,
        author: Author,
        blob_store: std::sync::Arc<aspen_blob::store::IrohBlobStore>,
    ) -> Self {
        Self {
            sync_handle,
            namespace_id,
            author,
            blob_store,
        }
    }
}

#[async_trait]
impl DocsWriter for BlobBackedDocsWriter {
    #[instrument(skip(self, value), fields(key_len = key.len(), value_len = value.len()))]
    async fn set_entry(&self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        // Store content in iroh-blobs first
        let blob_result = self.blob_store.add_bytes(&value).await.context("failed to store content in blob store")?;

        let hash = blob_result.blob_ref.hash;
        let len = blob_result.blob_ref.size_bytes;
        let author_id = self.author.id();

        // Insert entry metadata via sync handle
        // Now the content is available for P2P fetching via iroh-blobs
        self.sync_handle
            .insert_local(self.namespace_id, author_id, bytes::Bytes::from(key), hash, len)
            .await
            .context("failed to insert entry via sync handle")?;

        debug!(
            namespace = %self.namespace_id,
            hash = %hash.fmt_short(),
            size = len,
            "entry inserted with blob content"
        );
        Ok(())
    }

    #[instrument(skip(self), fields(key_len = key.len()))]
    async fn delete_entry(&self, key: Vec<u8>) -> Result<()> {
        // iroh-docs doesn't support empty entries, so we use a tombstone marker
        const TOMBSTONE: &[u8] = b"\x00";

        // Store tombstone in blobs for consistency
        let blob_result =
            self.blob_store.add_bytes(TOMBSTONE).await.context("failed to store tombstone in blob store")?;

        let hash = blob_result.blob_ref.hash;
        let author_id = self.author.id();

        self.sync_handle
            .insert_local(self.namespace_id, author_id, bytes::Bytes::from(key), hash, TOMBSTONE.len() as u64)
            .await
            .context("failed to delete entry via sync handle")?;

        debug!(namespace = %self.namespace_id, "entry deleted via sync handle (tombstone in blob)");
        Ok(())
    }
}

// ============================================================================
// In-Memory Implementation for Testing
// ============================================================================

/// In-memory docs writer for development and testing.
///
/// Stores entries in a thread-safe HashMap. Useful when:
/// - No iroh-docs store is available yet
/// - Testing DocsExporter integration
/// - Development environments without persistent storage
///
/// In production, use `IrohDocsWriter` for actual iroh-docs CRDT sync.
pub struct InMemoryDocsWriter {
    entries: tokio::sync::RwLock<std::collections::HashMap<Vec<u8>, Vec<u8>>>,
}

impl Default for InMemoryDocsWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryDocsWriter {
    /// Create a new in-memory docs writer.
    pub fn new() -> Self {
        Self {
            entries: tokio::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// Get an entry by key.
    pub async fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.entries.read().await.get(key).cloned()
    }

    /// Get the number of entries.
    pub async fn len(&self) -> usize {
        self.entries.read().await.len()
    }

    /// Check if the writer is empty.
    pub async fn is_empty(&self) -> bool {
        self.entries.read().await.is_empty()
    }
}

#[async_trait]
impl DocsWriter for InMemoryDocsWriter {
    async fn set_entry(&self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        self.entries.write().await.insert(key, value);
        Ok(())
    }

    async fn delete_entry(&self, key: Vec<u8>) -> Result<()> {
        // Use empty value as tombstone (like iroh-docs)
        self.entries.write().await.insert(key, vec![]);
        Ok(())
    }
}
