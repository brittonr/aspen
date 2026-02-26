//! Iroh-based blob store implementation.
//!
//! Uses iroh-blobs FsStore for persistent content-addressed storage
//! with BLAKE3 hashing and efficient deduplication.

use std::path::Path;
use std::pin::Pin;
use std::time::Duration;
use std::time::Instant;

use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use iroh::Endpoint;
use iroh::PublicKey;
use iroh_blobs::BlobFormat;
use iroh_blobs::Hash;
use iroh_blobs::HashAndFormat;
use iroh_blobs::store::fs::FsStore;
use iroh_blobs::ticket::BlobTicket;
use n0_future::StreamExt;
use tracing::debug;
use tracing::info;
use tracing::instrument;
use tracing::warn;

use super::async_read_seek::AsyncReadSeek;
use super::errors::BlobStoreError;
use crate::constants::BLOB_WAIT_POLL_INTERVAL;
use crate::constants::KV_TAG_PREFIX;
use crate::constants::MAX_BLOB_SIZE;
use crate::constants::USER_TAG_PREFIX;
use crate::events::BlobEventBroadcaster;
use crate::events::BlobSource;
use crate::events::INLINE_BLOB_THRESHOLD;
use crate::events::UnprotectReason;
use crate::traits::BlobQuery;
use crate::traits::BlobRead;
use crate::traits::BlobTransfer;
use crate::traits::BlobWrite;
use crate::types::AddBlobResult;
use crate::types::BlobListEntry;
use crate::types::BlobListResult;
use crate::types::BlobRef;
use crate::types::BlobStatus;

// SECURITY NOTE: This blob store has no built-in authentication or authorization.
// All operations (add, get, protect, unprotect, download, replicate) are accessible
// to any caller. This design is appropriate for trusted cluster environments where
// all nodes are authenticated at the transport layer (Iroh QUIC).
//
// For multi-tenant or public deployments, an additional authorization layer must be
// implemented at the RPC handler level to enforce per-blob access control.

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
        let store = anyhow::Context::context(FsStore::load(data_dir).await, "failed to create blob store")?;

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
        anyhow::Context::context(self.store.shutdown().await, "failed to shutdown blob store")?;
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

        self.store.tags().delete(tag_name).await.map_err(|e| BlobStoreError::DeleteTag {
            tag: tag_name.to_string(),
            message: e.to_string(),
        })?;

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
        use n0_future::StreamExt;

        // List all tags with user prefix
        let tags_stream =
            self.store.tags().list_prefix(USER_TAG_PREFIX).await.map_err(|e| BlobStoreError::ListTags {
                prefix: USER_TAG_PREFIX.to_string(),
                message: e.to_string(),
            })?;

        // Collect tags that match the target hash
        let tags_to_delete: Vec<String> = tags_stream
            .filter_map(|result| match result {
                Ok(tag_info) if tag_info.hash_and_format().hash == *hash => {
                    // Convert tag name bytes to string (Tag implements AsRef<[u8]>)
                    String::from_utf8(tag_info.name.as_ref().to_vec())
                        .inspect_err(|e| tracing::debug!("tag name is not valid UTF-8: {e}"))
                        .ok()
                }
                _ => None,
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
        progress.await.map_err(|e| BlobStoreError::DownloadBlob {
            hash: hash.to_string(),
            message: e.to_string(),
        })?;

        // Get size and verify blob was stored
        let bytes = self.get_bytes(hash).await?.ok_or_else(|| BlobStoreError::Download {
            message: "blob not found after download".to_string(),
        })?;

        let blob_ref = BlobRef::new(*hash, bytes.len() as u64, BlobFormat::Raw);
        let duration_ms = start.elapsed().as_millis() as u64;

        info!(hash = %hash, size = bytes.len(), provider = %provider.fmt_short(), duration_ms, "blob downloaded from peer");

        // Emit event if broadcaster is configured
        if let Some(broadcaster) = &self.broadcaster {
            let content = if bytes.len() <= INLINE_BLOB_THRESHOLD as usize {
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

// =============================================================================
// IrohBlobStore trait implementations
// =============================================================================

#[async_trait]
impl BlobWrite for IrohBlobStore {
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
            .map_err(|e| BlobStoreError::AddBytes { message: e.to_string() })?;

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
        let metadata = tokio::fs::metadata(path).await.map_err(|e| BlobStoreError::ReadFileMetadata {
            path: path.to_path_buf(),
            source: e,
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
            .map_err(|e| BlobStoreError::AddPath { message: e.to_string() })?;

        let blob_ref = BlobRef::new(tag_info.hash, size, tag_info.format);

        debug!(hash = %tag_info.hash, size, "blob added from path");

        // Emit event if broadcaster is configured
        if let Some(broadcaster) = &self.broadcaster {
            // Read content for small files (under inline threshold) to include in event
            let content = if size <= INLINE_BLOB_THRESHOLD as u64 {
                tokio::fs::read(path)
                    .await
                    .inspect_err(|e| warn!("failed to read blob content for event: {e}"))
                    .ok()
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
    async fn protect(&self, hash: &Hash, tag_name: &str) -> Result<(), BlobStoreError> {
        let haf = HashAndFormat::raw(*hash);
        self.store.tags().set(tag_name, haf).await.map_err(|e| BlobStoreError::SetTag {
            tag: tag_name.to_string(),
            message: e.to_string(),
        })?;

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

        self.store.tags().delete(tag_name).await.map_err(|e| BlobStoreError::DeleteTag {
            tag: tag_name.to_string(),
            message: e.to_string(),
        })?;

        debug!(tag = tag_name, "blob protection removed");

        // Emit event if broadcaster is configured
        if let Some(broadcaster) = &self.broadcaster {
            broadcaster.emit_unprotected(hash.as_ref(), tag_name, UnprotectReason::UserAction);
        }

        Ok(())
    }
}

#[async_trait]
impl BlobRead for IrohBlobStore {
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
        self.store.blobs().has(*hash).await.map_err(|e| BlobStoreError::CheckExistence {
            hash: hash.to_string(),
            message: e.to_string(),
        })
    }

    #[instrument(skip(self))]
    async fn status(&self, hash: &Hash) -> Result<Option<BlobStatus>, BlobStoreError> {
        let has = self.has(hash).await?;
        if !has {
            return Ok(None);
        }

        // Get size if available
        let size_bytes = self.store.get_bytes(*hash).await.ok().map(|b| b.len() as u64);

        // Get tags protecting this blob
        // Note: iroh-blobs doesn't provide a direct API to list tags for a specific blob,
        // so we return an empty list for now. Tags can still be managed via protect/unprotect.
        let tags = Vec::new();

        Ok(Some(BlobStatus {
            hash: *hash,
            size_bytes,
            is_complete: true,
            tags,
        }))
    }

    #[instrument(skip(self))]
    async fn reader(&self, hash: &Hash) -> Result<Option<Pin<Box<dyn AsyncReadSeek>>>, BlobStoreError> {
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

#[async_trait]
impl BlobTransfer for IrohBlobStore {
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
        progress.await.map_err(|e| BlobStoreError::DownloadBlob {
            hash: hash.to_string(),
            message: e.to_string(),
        })?;

        // Get size
        let bytes = self.get_bytes(&hash).await?.ok_or_else(|| BlobStoreError::Download {
            message: "blob not found after download".to_string(),
        })?;

        let blob_ref = BlobRef::new(hash, bytes.len() as u64, format);
        let duration_ms = start.elapsed().as_millis() as u64;

        info!(hash = %hash, size = bytes.len(), duration_ms, "blob downloaded");

        // Emit event if broadcaster is configured
        if let Some(broadcaster) = &self.broadcaster {
            let content = if bytes.len() <= INLINE_BLOB_THRESHOLD as usize {
                Some(bytes.as_ref())
            } else {
                None
            };
            broadcaster.emit_downloaded(&blob_ref, &provider.to_string(), duration_ms, content);
        }

        Ok(blob_ref)
    }

    #[instrument(skip(self))]
    async fn download_from_peer(&self, hash: &Hash, provider: PublicKey) -> Result<BlobRef, BlobStoreError> {
        // Delegate to the inherent impl to avoid code duplication
        IrohBlobStore::download_from_peer(self, hash, provider).await
    }
}

#[async_trait]
impl BlobQuery for IrohBlobStore {
    #[instrument(skip(self))]
    async fn list(&self, limit: u32, continuation_token: Option<&str>) -> Result<BlobListResult, BlobStoreError> {
        let mut blobs = Vec::new();
        let mut stream = self
            .store
            .blobs()
            .list()
            .stream()
            .await
            .map_err(|e| BlobStoreError::ListBlobs { message: e.to_string() })?;

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
                        size_bytes: size,
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
}
