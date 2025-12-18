//! Blob storage trait and implementation.
//!
//! Provides the `BlobStore` trait for content-addressed blob storage
//! and `IrohBlobStore` implementation using iroh-blobs.

use std::path::Path;

use anyhow::{Context, Result};
use async_trait::async_trait;
use bytes::Bytes;
use futures::StreamExt;
use iroh::Endpoint;
use iroh_blobs::store::fs::FsStore;
use iroh_blobs::ticket::BlobTicket;
use iroh_blobs::{BlobFormat, Hash, HashAndFormat};
use snafu::Snafu;
use tracing::{debug, info, instrument, warn};

use iroh::PublicKey;

use super::constants::{KV_TAG_PREFIX, MAX_BLOB_SIZE, USER_TAG_PREFIX};
use super::types::{AddBlobResult, BlobListEntry, BlobListResult, BlobRef, BlobStatus};

/// Errors from blob store operations.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum BlobStoreError {
    /// Blob not found.
    #[snafu(display("blob not found: {hash}"))]
    NotFound { hash: String },

    /// Blob exceeds maximum size.
    #[snafu(display("blob size {size} exceeds maximum {max}"))]
    TooLarge { size: u64, max: u64 },

    /// Storage error.
    #[snafu(display("storage error: {message}"))]
    Storage { message: String },

    /// Network error during download.
    #[snafu(display("download error: {message}"))]
    Download { message: String },

    /// Invalid blob ticket.
    #[snafu(display("invalid ticket: {message}"))]
    InvalidTicket { message: String },
}

impl From<anyhow::Error> for BlobStoreError {
    fn from(e: anyhow::Error) -> Self {
        BlobStoreError::Storage {
            message: e.to_string(),
        }
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

    /// List blobs with pagination.
    async fn list(
        &self,
        limit: u32,
        continuation_token: Option<&str>,
    ) -> Result<BlobListResult, BlobStoreError>;
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
}

impl IrohBlobStore {
    /// Create a new blob store at the given path.
    #[instrument(skip_all, fields(path = %data_dir.display()))]
    pub async fn new(data_dir: &Path, endpoint: Endpoint) -> Result<Self> {
        let store = FsStore::load(data_dir)
            .await
            .context("failed to create blob store")?;

        info!("blob store initialized at {}", data_dir.display());

        Ok(Self { store, endpoint })
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
        self.store
            .shutdown()
            .await
            .context("failed to shutdown blob store")?;
        Ok(())
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
    pub async fn download_from_peer(
        &self,
        hash: &Hash,
        provider: PublicKey,
    ) -> Result<BlobRef, BlobStoreError> {
        // Create downloader and download
        let downloader = self.store.downloader(&self.endpoint);

        let progress = downloader.download(HashAndFormat::raw(*hash), vec![provider]);

        // Wait for completion
        progress.await.map_err(|e| BlobStoreError::Download {
            message: e.to_string(),
        })?;

        // Get size and verify blob was stored
        let bytes = self
            .get_bytes(hash)
            .await?
            .ok_or_else(|| BlobStoreError::Download {
                message: "blob not found after download".to_string(),
            })?;

        let blob_ref = BlobRef::new(*hash, bytes.len() as u64, BlobFormat::Raw);

        info!(hash = %hash, size = bytes.len(), provider = %provider.fmt_short(), "blob downloaded from peer");
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
            .map_err(|e| BlobStoreError::Storage {
                message: e.to_string(),
            })?;

        let blob_ref = BlobRef::new(tag_info.hash, size, tag_info.format);

        debug!(hash = %tag_info.hash, size, "blob added");

        Ok(AddBlobResult {
            blob_ref,
            was_new: true, // FsStore doesn't tell us if it was new
        })
    }

    #[instrument(skip(self), fields(path = %path.display()))]
    async fn add_path(&self, path: &Path) -> Result<AddBlobResult, BlobStoreError> {
        let metadata = tokio::fs::metadata(path)
            .await
            .map_err(|e| BlobStoreError::Storage {
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
            .map_err(|e| BlobStoreError::Storage {
                message: e.to_string(),
            })?;

        let blob_ref = BlobRef::new(tag_info.hash, size, tag_info.format);

        debug!(hash = %tag_info.hash, size, "blob added from path");

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
        self.store
            .blobs()
            .has(*hash)
            .await
            .map_err(|e| BlobStoreError::Storage {
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
        let size = self
            .store
            .get_bytes(*hash)
            .await
            .ok()
            .map(|b| b.len() as u64);

        // TODO: Get tags from store when API is available
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
            .map_err(|e| BlobStoreError::Storage {
                message: e.to_string(),
            })?;

        debug!(hash = %hash, tag = tag_name, "blob protected");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn unprotect(&self, tag_name: &str) -> Result<(), BlobStoreError> {
        self.store
            .tags()
            .delete(tag_name)
            .await
            .map_err(|e| BlobStoreError::Storage {
                message: e.to_string(),
            })?;

        debug!(tag = tag_name, "blob protection removed");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn ticket(&self, hash: &Hash) -> Result<BlobTicket, BlobStoreError> {
        // Check blob exists
        if !self.has(hash).await? {
            return Err(BlobStoreError::NotFound {
                hash: hash.to_string(),
            });
        }

        // Get endpoint address
        let addr = self.endpoint.addr();

        let ticket = BlobTicket::new(addr, *hash, BlobFormat::Raw);

        debug!(hash = %hash, "ticket created");
        Ok(ticket)
    }

    #[instrument(skip(self))]
    async fn download(&self, ticket: &BlobTicket) -> Result<BlobRef, BlobStoreError> {
        let hash = ticket.hash();
        let format = ticket.format();

        // Get provider endpoint ID from ticket
        let provider = ticket.addr().id;

        // Create downloader and download
        let downloader = self.store.downloader(&self.endpoint);

        let progress = downloader.download(HashAndFormat::new(hash, format), vec![provider]);

        // Wait for completion
        progress.await.map_err(|e| BlobStoreError::Download {
            message: e.to_string(),
        })?;

        // Get size
        let bytes = self
            .get_bytes(&hash)
            .await?
            .ok_or_else(|| BlobStoreError::Download {
                message: "blob not found after download".to_string(),
            })?;

        let blob_ref = BlobRef::new(hash, bytes.len() as u64, format);

        info!(hash = %hash, size = bytes.len(), "blob downloaded");
        Ok(blob_ref)
    }

    #[instrument(skip(self))]
    async fn list(
        &self,
        limit: u32,
        _continuation_token: Option<&str>,
    ) -> Result<BlobListResult, BlobStoreError> {
        let mut blobs = Vec::new();
        let mut stream =
            self.store
                .blobs()
                .list()
                .stream()
                .await
                .map_err(|e| BlobStoreError::Storage {
                    message: e.to_string(),
                })?;

        while let Some(result) = stream.next().await {
            match result {
                Ok(hash) => {
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

                    if blobs.len() >= limit as usize {
                        break;
                    }
                }
                Err(e) => {
                    warn!(error = %e, "error listing blob");
                }
            }
        }

        // TODO: Implement proper pagination with continuation token
        Ok(BlobListResult {
            blobs,
            continuation_token: None,
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(unused_imports)]
    use super::*;

    // Integration tests require an actual iroh endpoint, so they're in tests/
}
