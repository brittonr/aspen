//! Blob storage trait and implementation.
//!
//! Provides the `BlobStore` trait for content-addressed blob storage
//! and `IrohBlobStore` implementation using iroh-blobs.

use std::path::Path;

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
use tracing::debug;
use tracing::info;
use tracing::instrument;
use tracing::warn;

use super::constants::KV_TAG_PREFIX;
use super::constants::MAX_BLOB_SIZE;
use super::constants::USER_TAG_PREFIX;
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
        let store = FsStore::load(data_dir).await.context("failed to create blob store")?;

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
        self.store.shutdown().await.context("failed to shutdown blob store")?;
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
    pub async fn download_from_peer(&self, hash: &Hash, provider: PublicKey) -> Result<BlobRef, BlobStoreError> {
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
            .map_err(|e| BlobStoreError::Storage { message: e.to_string() })?;

        let blob_ref = BlobRef::new(tag_info.hash, size, tag_info.format);

        debug!(hash = %tag_info.hash, size, "blob added");

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
        Ok(())
    }

    #[instrument(skip(self))]
    async fn unprotect(&self, tag_name: &str) -> Result<(), BlobStoreError> {
        self.store
            .tags()
            .delete(tag_name)
            .await
            .map_err(|e| BlobStoreError::Storage { message: e.to_string() })?;

        debug!(tag = tag_name, "blob protection removed");
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

        info!(hash = %hash, size = bytes.len(), "blob downloaded");
        Ok(blob_ref)
    }

    #[instrument(skip(self))]
    async fn download_from_peer(&self, hash: &Hash, provider: iroh::PublicKey) -> Result<BlobRef, BlobStoreError> {
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

        info!(hash = %hash, size = bytes.len(), provider = %provider.fmt_short(), "blob downloaded from peer");
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
}

#[cfg(test)]
mod tests {
    #![allow(unused_imports)]
    use super::*;

    // Integration tests require an actual iroh endpoint, so they're in tests/
}
