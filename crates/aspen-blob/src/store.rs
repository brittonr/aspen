//! Blob storage trait and implementation.
//!
//! Provides the `BlobStore` trait for content-addressed blob storage
//! and `IrohBlobStore` implementation using iroh-blobs.

use std::io::Cursor;
use std::path::Path;
use std::pin::Pin;
use std::time::Duration;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use futures::StreamExt;
use iroh::Endpoint;
use iroh::PublicKey;
use iroh_blobs::BlobFormat;
use iroh_blobs::Hash;
use iroh_blobs::HashAndFormat;
use iroh_blobs::store::fs::FsStore;
use iroh_blobs::ticket::BlobTicket;
use snafu::Snafu;
use tokio::io::{AsyncRead, AsyncSeek};
use tracing::debug;
use tracing::info;
use tracing::instrument;
use tracing::warn;

/// Trait combining AsyncRead + AsyncSeek for streaming blob readers.
///
/// This helper trait is needed because Rust doesn't allow multiple non-auto
/// traits in a dyn trait object. By creating a supertrait that requires both,
/// we can use `dyn AsyncReadSeek` as a valid trait object.
pub trait AsyncReadSeek: AsyncRead + AsyncSeek + Send + Unpin {}

/// Blanket implementation for any type implementing both traits.
impl<T: AsyncRead + AsyncSeek + Send + Unpin> AsyncReadSeek for T {}

use super::constants::BLOB_WAIT_POLL_INTERVAL;
use super::constants::KV_TAG_PREFIX;
use super::constants::MAX_BLOB_SIZE;
use super::constants::USER_TAG_PREFIX;
use super::events::BlobEventBroadcaster;
use super::events::BlobSource;
use super::events::INLINE_BLOB_THRESHOLD;
use super::events::UnprotectReason;
use super::types::AddBlobResult;
use super::types::BlobListEntry;
use super::types::BlobListResult;
use super::types::BlobRef;
use super::types::BlobStatus;

/// Errors from blob store operations.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum BlobStoreError {
    /// Blob not found.
    #[snafu(display("blob not found: {hash}"))]
    NotFound {
        /// The hash of the blob that was not found.
        hash: String,
    },

    /// Blob exceeds maximum size.
    #[snafu(display("blob size {size} exceeds maximum {max}"))]
    TooLarge {
        /// Actual size of the blob in bytes.
        size: u64,
        /// Maximum allowed size in bytes.
        max: u64,
    },

    /// Storage error.
    #[snafu(display("storage error: {message}"))]
    Storage {
        /// Human-readable description of the storage error.
        message: String,
    },

    /// Network error during download.
    #[snafu(display("download error: {message}"))]
    Download {
        /// Human-readable description of the download error.
        message: String,
    },

    /// Invalid blob ticket.
    #[snafu(display("invalid ticket: {message}"))]
    InvalidTicket {
        /// Human-readable description of why the ticket is invalid.
        message: String,
    },
}

impl From<anyhow::Error> for BlobStoreError {
    fn from(e: anyhow::Error) -> Self {
        BlobStoreError::Storage { message: e.to_string() }
    }
}

/// Trait for content-addressed blob storage.
///
/// Provides operations for storing, retrieving, and managing blobs
/// with support for garbage collection via tags.
#[async_trait]
pub trait BlobStore: Send + Sync {
    /// Add bytes to the store, returns a reference to the stored blob.
    async fn add_bytes(&self, data: &[u8]) -> Result<AddBlobResult, BlobStoreError>;

    /// Add from a file path.
    async fn add_path(&self, path: &Path) -> Result<AddBlobResult, BlobStoreError>;

    /// Get bytes by hash.
    async fn get_bytes(&self, hash: &Hash) -> Result<Option<Bytes>, BlobStoreError>;

    /// Check if blob exists and is complete.
    async fn has(&self, hash: &Hash) -> Result<bool, BlobStoreError>;

    /// Get blob status.
    async fn status(&self, hash: &Hash) -> Result<Option<BlobStatus>, BlobStoreError>;

    /// Create a persistent tag to prevent GC.
    async fn protect(&self, hash: &Hash, tag_name: &str) -> Result<(), BlobStoreError>;

    /// Remove protection tag.
    async fn unprotect(&self, tag_name: &str) -> Result<(), BlobStoreError>;

    /// Generate a BlobTicket for sharing.
    async fn ticket(&self, hash: &Hash) -> Result<BlobTicket, BlobStoreError>;

    /// Download a blob from a remote peer using a ticket.
    async fn download(&self, ticket: &BlobTicket) -> Result<BlobRef, BlobStoreError>;

    /// Download a blob directly from a specific peer by hash.
    ///
    /// This is used for P2P sync when we know which peer has the blob.
    /// Default implementation returns an error for stores that don't support P2P.
    async fn download_from_peer(&self, hash: &Hash, _provider: iroh::PublicKey) -> Result<BlobRef, BlobStoreError> {
        Err(BlobStoreError::Download {
            message: format!("P2P download not supported for hash {}", hash.to_hex()),
        })
    }

    /// List blobs with pagination.
    async fn list(&self, limit: u32, continuation_token: Option<&str>) -> Result<BlobListResult, BlobStoreError>;

    /// Wait for a blob to be available locally with bounded timeout.
    ///
    /// Tiger Style: Explicit timeout prevents unbounded waiting.
    ///
    /// # Arguments
    /// * `hash` - The blob hash to wait for
    /// * `timeout` - Maximum time to wait
    ///
    /// # Returns
    /// * `Ok(true)` - Blob is available
    /// * `Ok(false)` - Timeout reached, blob not available
    /// * `Err(_)` - Storage error
    async fn wait_available(&self, hash: &Hash, timeout: Duration) -> Result<bool, BlobStoreError>;

    /// Wait for multiple blobs to be available locally.
    ///
    /// Tiger Style: Bounded batch operation with single timeout for all.
    ///
    /// # Arguments
    /// * `hashes` - The blob hashes to wait for
    /// * `timeout` - Maximum time to wait for ALL blobs
    ///
    /// # Returns
    /// * `Ok(missing)` - Vec of hashes that are still missing (empty = all available)
    /// * `Err(_)` - Storage error
    async fn wait_available_all(&self, hashes: &[Hash], timeout: Duration) -> Result<Vec<Hash>, BlobStoreError>;

    /// Get a streaming reader for a blob.
    ///
    /// Returns an async reader positioned at the start of the blob.
    /// This enables streaming large blobs without loading them entirely into memory,
    /// which is critical for serving large NAR files over HTTP/3.
    ///
    /// The returned reader implements `AsyncRead + AsyncSeek + Unpin + Send`,
    /// allowing efficient partial reads and range request support.
    ///
    /// # Arguments
    /// * `hash` - The BLAKE3 hash of the blob to read
    ///
    /// # Returns
    /// * `Ok(Some(reader))` - Blob exists, reader is ready
    /// * `Ok(None)` - Blob not found
    /// * `Err(_)` - Storage error
    ///
    /// # Tiger Style
    /// Streaming operation - does not buffer entire blob in memory.
    async fn reader(
        &self,
        hash: &Hash,
    ) -> Result<Option<Pin<Box<dyn AsyncReadSeek>>>, BlobStoreError>;
}

/// Iroh-based blob store implementation.
///
/// Uses iroh-blobs FsStore for persistent content-addressed storage
/// with BLAKE3 hashing and efficient deduplication.
pub struct IrohBlobStore {
    /// The underlying iroh-blobs store.
    store: FsStore,
    /// Iroh endpoint for networking.
    endpoint: Endpoint,
    /// Optional event broadcaster for hook integration.
    broadcaster: Option<BlobEventBroadcaster>,
}

impl IrohBlobStore {
    /// Create a new blob store at the given path.
    #[instrument(skip_all, fields(path = %data_dir.display()))]
    pub async fn new(data_dir: &Path, endpoint: Endpoint) -> Result<Self> {
        let store = FsStore::load(data_dir).await.context("failed to create blob store")?;

        info!("blob store initialized at {}", data_dir.display());

        Ok(Self {
            store,
            endpoint,
            broadcaster: None,
        })
    }

    /// Set the event broadcaster for this store.
    ///
    /// Events will be emitted for blob lifecycle operations:
    /// - `Added`: when blobs are added via `add_bytes()` or `add_path()`
    /// - `Downloaded`: when blobs are downloaded from peers
    /// - `Protected`: when blobs are protected with a tag
    /// - `Unprotected`: when blob protection is removed
    #[must_use]
    pub fn with_broadcaster(mut self, broadcaster: BlobEventBroadcaster) -> Self {
        self.broadcaster = Some(broadcaster);
        self
    }

    /// Set the event broadcaster (for use after construction).
    pub fn set_broadcaster(&mut self, broadcaster: BlobEventBroadcaster) {
        self.broadcaster = Some(broadcaster);
    }

    /// Get reference to broadcaster if set.
    pub fn broadcaster(&self) -> Option<&BlobEventBroadcaster> {
        self.broadcaster.as_ref()
    }

    /// Get the underlying store for protocol handler integration.
    pub fn store(&self) -> &FsStore {
        &self.store
    }

    /// Get the endpoint.
    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    /// Create a tag name for a KV-referenced blob.
    pub fn kv_tag(key: &str) -> String {
        format!("{}{}", KV_TAG_PREFIX, key)
    }

    /// Create a tag name for a user-protected blob.
    pub fn user_tag(name: &str) -> String {
        format!("{}{}", USER_TAG_PREFIX, name)
    }

    /// Create a BlobsProtocol handler for the Iroh Router.
    ///
    /// Returns a protocol handler that can be registered with the Router
    /// using `iroh_blobs::ALPN`.
    pub fn protocol_handler(&self) -> iroh_blobs::BlobsProtocol {
        iroh_blobs::BlobsProtocol::new(&self.store, None)
    }

    /// Shutdown the blob store.
    pub async fn shutdown(&self) -> Result<()> {
        self.store.shutdown().await.context("failed to shutdown blob store")?;
        Ok(())
    }

    /// Remove protection tag with a specific reason.
    ///
    /// This variant allows callers to specify why the protection is being removed,
    /// which is useful for hook event consumers to understand the context
    /// (e.g., KV delete vs overwrite vs user action).
    ///
    /// # Arguments
    /// * `tag_name` - The tag name to remove
    /// * `reason` - Why the protection is being removed
    #[instrument(skip(self))]
    pub async fn unprotect_with_reason(&self, tag_name: &str, reason: UnprotectReason) -> Result<(), BlobStoreError> {
        // Look up the hash before deleting the tag (for event emission)
        let hash = if self.broadcaster.is_some() {
            match self.store.tags().get(tag_name).await {
                Ok(Some(tag_info)) => Some(tag_info.hash_and_format().hash),
                Ok(None) => None,
                Err(_) => None,
            }
        } else {
            None
        };

        self.store
            .tags()
            .delete(tag_name)
            .await
            .map_err(|e| BlobStoreError::Storage { message: e.to_string() })?;

        debug!(tag = tag_name, reason = %reason, "blob protection removed");

        // Emit event if broadcaster is configured
        if let Some(broadcaster) = &self.broadcaster {
            broadcaster.emit_unprotected(hash.as_ref(), tag_name, reason);
        }

        Ok(())
    }

    /// Delete all user tags pointing to a specific hash.
    ///
    /// This method iterates through all tags with the user prefix (`user:`)
    /// and removes any that point to the specified hash. This effectively
    /// unprotects the blob from GC, allowing it to be garbage collected
    /// when no other tags reference it.
    ///
    /// # Arguments
    /// * `hash` - The BLAKE3 hash of the blob to unprotect
    ///
    /// # Returns
    /// * `Ok(count)` - Number of tags deleted
    /// * `Err(BlobStoreError)` - Storage error
    ///
    /// # Tiger Style
    /// Bounded operation: iterates only user-prefixed tags, not all tags.
    #[instrument(skip(self))]
    pub async fn delete_user_tags_for_hash(&self, hash: &Hash) -> Result<u32, BlobStoreError> {
        use futures::StreamExt;

        // List all tags with user prefix
        let tags_stream = self
            .store
            .tags()
            .list_prefix(USER_TAG_PREFIX)
            .await
            .map_err(|e| BlobStoreError::Storage { message: e.to_string() })?;

        // Collect tags that match the target hash
        let tags_to_delete: Vec<String> = tags_stream
            .filter_map(|result| async {
                match result {
                    Ok(tag_info) if tag_info.hash_and_format().hash == *hash => {
                        // Convert tag name bytes to string (Tag implements AsRef<[u8]>)
                        String::from_utf8(tag_info.name.as_ref().to_vec()).ok()
                    }
                    _ => None,
                }
            })
            .collect()
            .await;

        let count = tags_to_delete.len() as u32;

        // Delete each matching tag
        for tag_name in tags_to_delete {
            if let Err(e) = self.unprotect_with_reason(&tag_name, UnprotectReason::UserAction).await {
                warn!(tag = %tag_name, error = %e, "failed to delete tag during hash deletion");
            }
        }

        debug!(hash = %hash, deleted_tags = count, "deleted user tags for blob");

        Ok(count)
    }

    /// Download a blob from a remote peer by hash and provider PublicKey.
    ///
    /// Unlike `download()` which requires a full `BlobTicket`, this method only
    /// needs the content hash and the PublicKey of the peer that has the content.
    /// This is useful for downloading content discovered through iroh-docs sync
    /// where we know the hash and the peer that sent the entry, but don't have
    /// a full ticket.
    ///
    /// # Arguments
    /// * `hash` - The BLAKE3 hash of the blob to download
    /// * `provider` - The PublicKey (NodeId) of the peer that has the content
    ///
    /// # Returns
    /// * `Ok(BlobRef)` - Successfully downloaded blob with hash and size
    /// * `Err(BlobStoreError)` - Download failed (network error, peer unavailable, etc.)
    #[instrument(skip(self))]
    pub async fn download_from_peer(&self, hash: &Hash, provider: PublicKey) -> Result<BlobRef, BlobStoreError> {
        let start = Instant::now();

        // Create downloader and download
        let downloader = self.store.downloader(&self.endpoint);

        let progress = downloader.download(HashAndFormat::raw(*hash), vec![provider]);

        // Wait for completion
        progress.await.map_err(|e| BlobStoreError::Download { message: e.to_string() })?;

        // Get size and verify blob was stored
        let bytes = self.get_bytes(hash).await?.ok_or_else(|| BlobStoreError::Download {
            message: "blob not found after download".to_string(),
        })?;

        let blob_ref = BlobRef::new(*hash, bytes.len() as u64, BlobFormat::Raw);
        let duration_ms = start.elapsed().as_millis() as u64;

        info!(hash = %hash, size = bytes.len(), provider = %provider.fmt_short(), duration_ms, "blob downloaded from peer");

        // Emit event if broadcaster is configured
        if let Some(broadcaster) = &self.broadcaster {
            let content = if bytes.len() <= INLINE_BLOB_THRESHOLD {
                Some(bytes.as_ref())
            } else {
                None
            };
            let provider_str = provider.fmt_short().to_string();
            broadcaster.emit_downloaded(&blob_ref, &provider_str, duration_ms, content);
        }

        Ok(blob_ref)
    }
}

#[async_trait]
impl BlobStore for IrohBlobStore {
    #[instrument(skip(self, data), fields(size = data.len()))]
    async fn add_bytes(&self, data: &[u8]) -> Result<AddBlobResult, BlobStoreError> {
        let size = data.len() as u64;
        if size > MAX_BLOB_SIZE {
            return Err(BlobStoreError::TooLarge {
                size,
                max: MAX_BLOB_SIZE,
            });
        }

        let tag_info = self
            .store
            .blobs()
            .add_bytes(Bytes::copy_from_slice(data))
            .with_tag()
            .await
            .map_err(|e| BlobStoreError::Storage { message: e.to_string() })?;

        let blob_ref = BlobRef::new(tag_info.hash, size, tag_info.format);

        debug!(hash = %tag_info.hash, size, "blob added");

        // Emit event if broadcaster is configured
        if let Some(broadcaster) = &self.broadcaster {
            broadcaster.emit_added(&blob_ref, BlobSource::AddBytes, data, true);
        }

        Ok(AddBlobResult {
            blob_ref,
            was_new: true, // FsStore doesn't tell us if it was new
        })
    }

    #[instrument(skip(self), fields(path = %path.display()))]
    async fn add_path(&self, path: &Path) -> Result<AddBlobResult, BlobStoreError> {
        let metadata = tokio::fs::metadata(path).await.map_err(|e| BlobStoreError::Storage {
            message: format!("failed to read file metadata: {}", e),
        })?;

        let size = metadata.len();
        if size > MAX_BLOB_SIZE {
            return Err(BlobStoreError::TooLarge {
                size,
                max: MAX_BLOB_SIZE,
            });
        }

        let tag_info = self
            .store
            .blobs()
            .add_path(path)
            .with_tag()
            .await
            .map_err(|e| BlobStoreError::Storage { message: e.to_string() })?;

        let blob_ref = BlobRef::new(tag_info.hash, size, tag_info.format);

        debug!(hash = %tag_info.hash, size, "blob added from path");

        // Emit event if broadcaster is configured
        if let Some(broadcaster) = &self.broadcaster {
            // Read content for small files (under inline threshold) to include in event
            let content = if size <= INLINE_BLOB_THRESHOLD as u64 {
                tokio::fs::read(path).await.ok()
            } else {
                None
            };
            let content_slice = content.as_deref().unwrap_or(&[]);
            broadcaster.emit_added(&blob_ref, BlobSource::AddPath, content_slice, true);
        }

        Ok(AddBlobResult {
            blob_ref,
            was_new: true,
        })
    }

    #[instrument(skip(self))]
    async fn get_bytes(&self, hash: &Hash) -> Result<Option<Bytes>, BlobStoreError> {
        match self.store.blobs().get_bytes(*hash).await {
            Ok(bytes) => {
                debug!(hash = %hash, size = bytes.len(), "blob retrieved");
                Ok(Some(bytes))
            }
            Err(e) => {
                // Check if it's a not-found error
                let err_str = e.to_string();
                if err_str.contains("not found") || err_str.contains("NotFound") {
                    Ok(None)
                } else {
                    Err(BlobStoreError::Storage { message: err_str })
                }
            }
        }
    }

    #[instrument(skip(self))]
    async fn has(&self, hash: &Hash) -> Result<bool, BlobStoreError> {
        self.store.blobs().has(*hash).await.map_err(|e| BlobStoreError::Storage { message: e.to_string() })
    }

    #[instrument(skip(self))]
    async fn status(&self, hash: &Hash) -> Result<Option<BlobStatus>, BlobStoreError> {
        let has = self.has(hash).await?;
        if !has {
            return Ok(None);
        }

        // Get size if available
        let size = self.store.get_bytes(*hash).await.ok().map(|b| b.len() as u64);

        // Get tags protecting this blob
        // Note: iroh-blobs doesn't provide a direct API to list tags for a specific blob,
        // so we return an empty list for now. Tags can still be managed via protect/unprotect.
        let tags = Vec::new();

        Ok(Some(BlobStatus {
            hash: *hash,
            size,
            complete: true,
            tags,
        }))
    }

    #[instrument(skip(self))]
    async fn protect(&self, hash: &Hash, tag_name: &str) -> Result<(), BlobStoreError> {
        let haf = HashAndFormat::raw(*hash);
        self.store
            .tags()
            .set(tag_name, haf)
            .await
            .map_err(|e| BlobStoreError::Storage { message: e.to_string() })?;

        debug!(hash = %hash, tag = tag_name, "blob protected");

        // Emit event if broadcaster is configured
        if let Some(broadcaster) = &self.broadcaster {
            broadcaster.emit_protected(hash, tag_name);
        }

        Ok(())
    }

    #[instrument(skip(self))]
    async fn unprotect(&self, tag_name: &str) -> Result<(), BlobStoreError> {
        // Look up the hash before deleting the tag (for event emission)
        let hash = if self.broadcaster.is_some() {
            match self.store.tags().get(tag_name).await {
                Ok(Some(tag_info)) => Some(tag_info.hash_and_format().hash),
                Ok(None) => None,
                Err(_) => None,
            }
        } else {
            None
        };

        self.store
            .tags()
            .delete(tag_name)
            .await
            .map_err(|e| BlobStoreError::Storage { message: e.to_string() })?;

        debug!(tag = tag_name, "blob protection removed");

        // Emit event if broadcaster is configured
        if let Some(broadcaster) = &self.broadcaster {
            broadcaster.emit_unprotected(hash.as_ref(), tag_name, UnprotectReason::UserAction);
        }

        Ok(())
    }

    #[instrument(skip(self))]
    async fn ticket(&self, hash: &Hash) -> Result<BlobTicket, BlobStoreError> {
        // Check blob exists
        if !self.has(hash).await? {
            return Err(BlobStoreError::NotFound { hash: hash.to_string() });
        }

        // Get endpoint address
        let addr = self.endpoint.addr();

        let ticket = BlobTicket::new(addr, *hash, BlobFormat::Raw);

        debug!(hash = %hash, "ticket created");
        Ok(ticket)
    }

    #[instrument(skip(self))]
    async fn download(&self, ticket: &BlobTicket) -> Result<BlobRef, BlobStoreError> {
        let start = Instant::now();
        let hash = ticket.hash();
        let format = ticket.format();

        // Get provider endpoint ID from ticket
        let provider = ticket.addr().id;

        // Create downloader and download
        let downloader = self.store.downloader(&self.endpoint);

        let progress = downloader.download(HashAndFormat::new(hash, format), vec![provider]);

        // Wait for completion
        progress.await.map_err(|e| BlobStoreError::Download { message: e.to_string() })?;

        // Get size
        let bytes = self.get_bytes(&hash).await?.ok_or_else(|| BlobStoreError::Download {
            message: "blob not found after download".to_string(),
        })?;

        let blob_ref = BlobRef::new(hash, bytes.len() as u64, format);
        let duration_ms = start.elapsed().as_millis() as u64;

        info!(hash = %hash, size = bytes.len(), duration_ms, "blob downloaded");

        // Emit event if broadcaster is configured
        if let Some(broadcaster) = &self.broadcaster {
            let content = if bytes.len() <= INLINE_BLOB_THRESHOLD {
                Some(bytes.as_ref())
            } else {
                None
            };
            broadcaster.emit_downloaded(&blob_ref, &provider.to_string(), duration_ms, content);
        }

        Ok(blob_ref)
    }

    #[instrument(skip(self))]
    async fn download_from_peer(&self, hash: &Hash, provider: iroh::PublicKey) -> Result<BlobRef, BlobStoreError> {
        let start = Instant::now();

        // Create downloader and download
        let downloader = self.store.downloader(&self.endpoint);

        let progress = downloader.download(HashAndFormat::raw(*hash), vec![provider]);

        // Wait for completion
        progress.await.map_err(|e| BlobStoreError::Download { message: e.to_string() })?;

        // Get size and verify blob was stored
        let bytes = self.get_bytes(hash).await?.ok_or_else(|| BlobStoreError::Download {
            message: "blob not found after download".to_string(),
        })?;

        let blob_ref = BlobRef::new(*hash, bytes.len() as u64, BlobFormat::Raw);
        let duration_ms = start.elapsed().as_millis() as u64;

        info!(hash = %hash, size = bytes.len(), provider = %provider.fmt_short(), duration_ms, "blob downloaded from peer");

        // Emit event if broadcaster is configured
        if let Some(broadcaster) = &self.broadcaster {
            let content = if bytes.len() <= INLINE_BLOB_THRESHOLD {
                Some(bytes.as_ref())
            } else {
                None
            };
            let provider_str = provider.fmt_short().to_string();
            broadcaster.emit_downloaded(&blob_ref, &provider_str, duration_ms, content);
        }

        Ok(blob_ref)
    }

    #[instrument(skip(self))]
    async fn list(&self, limit: u32, continuation_token: Option<&str>) -> Result<BlobListResult, BlobStoreError> {
        let mut blobs = Vec::new();
        let mut stream = self
            .store
            .blobs()
            .list()
            .stream()
            .await
            .map_err(|e| BlobStoreError::Storage { message: e.to_string() })?;

        // Parse continuation token to get last seen hash
        let skip_until = if let Some(token) = continuation_token {
            // Parse hex string to Hash
            if token.len() == 64 {
                let mut bytes = [0u8; 32];
                if hex::decode_to_slice(token, &mut bytes).is_ok() {
                    Some(Hash::from(bytes))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let mut found_continuation_point = skip_until.is_none();
        let mut last_hash = None;

        while let Some(result) = stream.next().await {
            match result {
                Ok(hash) => {
                    // Skip entries until we reach the continuation point
                    if !found_continuation_point {
                        if Some(hash) == skip_until {
                            found_continuation_point = true;
                        }
                        continue;
                    }

                    // Get size by reading the blob
                    let size = match self.store.blobs().get_bytes(hash).await {
                        Ok(bytes) => bytes.len() as u64,
                        Err(_) => continue,
                    };

                    blobs.push(BlobListEntry {
                        hash,
                        size,
                        format: BlobFormat::Raw,
                    });

                    last_hash = Some(hash);

                    if blobs.len() >= limit as usize {
                        break;
                    }
                }
                Err(e) => {
                    warn!(error = %e, "error listing blob");
                }
            }
        }

        // Generate continuation token if we hit the limit and there might be more results
        let continuation_token = if blobs.len() >= limit as usize {
            last_hash.map(|h| hex::encode(h.as_bytes()))
        } else {
            None
        };

        Ok(BlobListResult {
            blobs,
            continuation_token,
        })
    }

    #[instrument(skip(self))]
    async fn wait_available(&self, hash: &Hash, timeout: Duration) -> Result<bool, BlobStoreError> {
        let start = Instant::now();

        loop {
            // Check if blob exists and is complete
            if self.has(hash).await? {
                debug!(hash = %hash, elapsed_ms = start.elapsed().as_millis(), "blob available");
                return Ok(true);
            }

            // Check timeout
            if start.elapsed() >= timeout {
                debug!(hash = %hash, elapsed_ms = start.elapsed().as_millis(), "blob wait timed out");
                return Ok(false);
            }

            // Sleep before next poll
            tokio::time::sleep(BLOB_WAIT_POLL_INTERVAL).await;
        }
    }

    #[instrument(skip(self, hashes), fields(count = hashes.len()))]
    async fn wait_available_all(&self, hashes: &[Hash], timeout: Duration) -> Result<Vec<Hash>, BlobStoreError> {
        let start = Instant::now();
        let mut pending: std::collections::HashSet<Hash> = hashes.iter().copied().collect();

        while !pending.is_empty() {
            // Check all pending hashes
            let mut newly_available = Vec::new();
            for hash in &pending {
                if self.has(hash).await? {
                    newly_available.push(*hash);
                }
            }

            // Remove available hashes from pending set
            for hash in newly_available {
                pending.remove(&hash);
            }

            if pending.is_empty() {
                debug!(elapsed_ms = start.elapsed().as_millis(), "all blobs available");
                return Ok(Vec::new());
            }

            // Check timeout
            if start.elapsed() >= timeout {
                debug!(
                    remaining = pending.len(),
                    elapsed_ms = start.elapsed().as_millis(),
                    "blob batch wait timed out"
                );
                return Ok(pending.into_iter().collect());
            }

            tokio::time::sleep(BLOB_WAIT_POLL_INTERVAL).await;
        }

        Ok(Vec::new())
    }

    #[instrument(skip(self))]
    async fn reader(
        &self,
        hash: &Hash,
    ) -> Result<Option<Pin<Box<dyn AsyncReadSeek>>>, BlobStoreError> {
        // Check if blob exists first
        if !self.has(hash).await? {
            return Ok(None);
        }

        // Get a reader from the FsStore
        // iroh-blobs FsStore.blobs().reader() returns a BlobReader (not a Future)
        // which implements AsyncRead + AsyncSeek
        let reader = self.store.blobs().reader(*hash);

        debug!(hash = %hash, "blob reader created for streaming");

        Ok(Some(Box::pin(reader)))
    }
}

/// In-memory blob store for testing.
///
/// This is a simple hash-based store that keeps blobs in memory.
/// Useful for unit tests that don't need full iroh-blobs functionality.
///
/// This type is Clone-able - clones share the same underlying storage.
#[derive(Clone, Default)]
pub struct InMemoryBlobStore {
    blobs: std::sync::Arc<std::sync::RwLock<std::collections::HashMap<Hash, Bytes>>>,
}

impl InMemoryBlobStore {
    /// Create a new in-memory blob store.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl BlobStore for InMemoryBlobStore {
    async fn add_bytes(&self, data: &[u8]) -> Result<AddBlobResult, BlobStoreError> {
        let size = data.len() as u64;
        if size > MAX_BLOB_SIZE {
            return Err(BlobStoreError::TooLarge {
                size,
                max: MAX_BLOB_SIZE,
            });
        }

        let hash = Hash::new(data);
        let bytes = Bytes::copy_from_slice(data);

        let was_new = {
            let mut blobs = self.blobs.write().unwrap();
            blobs.insert(hash, bytes).is_none()
        };

        Ok(AddBlobResult {
            blob_ref: BlobRef::new(hash, size, BlobFormat::Raw),
            was_new,
        })
    }

    async fn add_path(&self, path: &Path) -> Result<AddBlobResult, BlobStoreError> {
        let data = std::fs::read(path).map_err(|e| BlobStoreError::Storage {
            message: format!("failed to read file: {}", e),
        })?;
        self.add_bytes(&data).await
    }

    async fn get_bytes(&self, hash: &Hash) -> Result<Option<Bytes>, BlobStoreError> {
        let blobs = self.blobs.read().unwrap();
        Ok(blobs.get(hash).cloned())
    }

    async fn has(&self, hash: &Hash) -> Result<bool, BlobStoreError> {
        let blobs = self.blobs.read().unwrap();
        Ok(blobs.contains_key(hash))
    }

    async fn status(&self, hash: &Hash) -> Result<Option<BlobStatus>, BlobStoreError> {
        let blobs = self.blobs.read().unwrap();
        Ok(blobs.get(hash).map(|b| BlobStatus {
            hash: *hash,
            size: Some(b.len() as u64),
            complete: true,
            tags: Vec::new(),
        }))
    }

    async fn protect(&self, _hash: &Hash, _tag_name: &str) -> Result<(), BlobStoreError> {
        // No-op for in-memory store
        Ok(())
    }

    async fn unprotect(&self, _tag_name: &str) -> Result<(), BlobStoreError> {
        // No-op for in-memory store
        Ok(())
    }

    async fn ticket(&self, hash: &Hash) -> Result<BlobTicket, BlobStoreError> {
        // In-memory store can't create tickets since there's no endpoint
        Err(BlobStoreError::Storage {
            message: format!("in-memory store cannot create tickets for {}", hash),
        })
    }

    async fn download(&self, _ticket: &BlobTicket) -> Result<BlobRef, BlobStoreError> {
        // In-memory store can't download from peers
        Err(BlobStoreError::Storage {
            message: "in-memory store cannot download from peers".to_string(),
        })
    }

    async fn list(&self, limit: u32, _continuation_token: Option<&str>) -> Result<BlobListResult, BlobStoreError> {
        let blobs = self.blobs.read().unwrap();
        let entries: Vec<_> = blobs
            .iter()
            .take(limit as usize)
            .map(|(hash, bytes)| BlobListEntry {
                hash: *hash,
                size: bytes.len() as u64,
                format: BlobFormat::Raw,
            })
            .collect();

        Ok(BlobListResult {
            blobs: entries,
            continuation_token: None,
        })
    }

    async fn wait_available(&self, hash: &Hash, _timeout: Duration) -> Result<bool, BlobStoreError> {
        // In-memory store: blobs are immediately available after add_bytes
        // No need to wait - just check if it exists
        self.has(hash).await
    }

    async fn wait_available_all(&self, hashes: &[Hash], _timeout: Duration) -> Result<Vec<Hash>, BlobStoreError> {
        // In-memory store: blobs are immediately available after add_bytes
        // Return any that don't exist
        let mut missing = Vec::new();
        for hash in hashes {
            if !self.has(hash).await? {
                missing.push(*hash);
            }
        }
        Ok(missing)
    }

    async fn reader(
        &self,
        hash: &Hash,
    ) -> Result<Option<Pin<Box<dyn AsyncReadSeek>>>, BlobStoreError> {
        // For in-memory store, we wrap the bytes in a Cursor
        let blobs = self.blobs.read().unwrap();
        match blobs.get(hash).cloned() {
            Some(bytes) => {
                let cursor = Cursor::new(bytes);
                Ok(Some(Box::pin(cursor)))
            }
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(unused_imports)]
    use super::*;

    // Integration tests require an actual iroh endpoint, so they're in tests/
}
