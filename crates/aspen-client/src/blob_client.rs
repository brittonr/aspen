//! High-level blob storage client for the Aspen distributed system.
//!
//! This module provides an ergonomic async API for interacting with Aspen's
//! content-addressed blob storage system, including upload, download, and
//! P2P distribution features.

#![allow(unused_imports)]

use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use aspen_client_api::AddBlobResultResponse;
use aspen_client_api::BlobListEntry;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::DeleteBlobResultResponse;
use aspen_client_api::DownloadBlobResultResponse;
use aspen_client_api::GetBlobResultResponse;
use aspen_client_api::GetBlobStatusResultResponse;
use aspen_client_api::GetBlobTicketResultResponse;
use aspen_client_api::HasBlobResultResponse;
use aspen_client_api::ListBlobsResultResponse;
use aspen_client_api::ProtectBlobResultResponse;
use aspen_client_api::UnprotectBlobResultResponse;
use serde::Deserialize;
use serde::Serialize;

use crate::AspenClient;

/// Result of a blob upload operation.
#[derive(Debug, Clone)]
pub struct BlobUploadResult {
    /// BLAKE3 hash of the blob.
    pub hash: String,
    /// Size of the blob in bytes.
    pub size: u64,
    /// Whether this was a new blob (false if already existed).
    pub was_new: bool,
}

/// Result of a blob download operation.
#[derive(Debug, Clone)]
pub struct BlobDownloadResult {
    /// BLAKE3 hash of the blob.
    pub hash: String,
    /// Size of the blob in bytes.
    pub size: u64,
    /// The blob data.
    pub data: Vec<u8>,
}

/// Status information for a blob.
#[derive(Debug, Clone)]
pub struct BlobStatus {
    /// BLAKE3 hash of the blob.
    pub hash: String,
    /// Size of the blob in bytes.
    pub size: Option<u64>,
    /// Whether the blob is complete.
    pub complete: bool,
    /// Protection tags applied to the blob.
    pub tags: Vec<String>,
}

/// Options for listing blobs.
#[derive(Debug, Clone, Default)]
pub struct BlobListOptions {
    /// Maximum number of results to return.
    pub limit: Option<u32>,
    /// Continuation token for pagination.
    pub continuation_token: Option<String>,
}

/// Result of a blob list operation.
#[derive(Debug, Clone)]
pub struct BlobListResult {
    /// List of blobs.
    pub blobs: Vec<BlobEntry>,
    /// Whether there are more results.
    pub has_more: bool,
    /// Continuation token for next page.
    pub continuation_token: Option<String>,
}

/// Entry in a blob list.
#[derive(Debug, Clone)]
pub struct BlobEntry {
    /// BLAKE3 hash of the blob.
    pub hash: String,
    /// Size of the blob in bytes.
    pub size: u64,
}

/// High-level blob storage client.
pub struct BlobClient<'a> {
    client: &'a AspenClient,
}

impl<'a> BlobClient<'a> {
    /// Create a new blob client wrapping an Aspen client.
    pub fn new(client: &'a AspenClient) -> Self {
        Self { client }
    }

    /// Upload a blob from bytes.
    ///
    /// # Example
    /// ```rust,ignore
    /// let result = client.blobs()
    ///     .upload(&data)
    ///     .await?;
    /// println!("Uploaded blob: {}", result.hash);
    /// ```
    pub async fn upload(&self, data: &[u8]) -> Result<BlobUploadResult> {
        self.upload_with_tag(data, None).await
    }

    /// Upload a blob with a protection tag.
    ///
    /// The tag prevents the blob from being garbage collected.
    pub async fn upload_with_tag(&self, data: &[u8], tag: Option<String>) -> Result<BlobUploadResult> {
        let request = ClientRpcRequest::AddBlob {
            data: data.to_vec(),
            tag,
        };

        let response = self.client.send(request).await?;

        match response {
            ClientRpcResponse::AddBlobResult(result) => {
                if result.success {
                    Ok(BlobUploadResult {
                        hash: result.hash.context("Blob uploaded but no hash returned")?,
                        size: result.size.context("Blob uploaded but no size returned")?,
                        was_new: result.was_new.unwrap_or(true),
                    })
                } else {
                    Err(anyhow::anyhow!(
                        "Failed to upload blob: {}",
                        result.error.unwrap_or_else(|| "Unknown error".to_string())
                    ))
                }
            }
            _ => Err(anyhow::anyhow!("Unexpected response type for blob upload")),
        }
    }

    /// Upload a blob from a file.
    pub async fn upload_file(&self, path: impl AsRef<Path>) -> Result<BlobUploadResult> {
        let data = std::fs::read(path)?;
        self.upload(&data).await
    }

    /// Upload a file with a protection tag.
    pub async fn upload_file_with_tag(&self, path: impl AsRef<Path>, tag: String) -> Result<BlobUploadResult> {
        let data = std::fs::read(path)?;
        self.upload_with_tag(&data, Some(tag)).await
    }

    /// Download a blob by hash.
    ///
    /// Returns None if the blob doesn't exist locally.
    /// Use `download_from_network` to fetch from remote peers.
    pub async fn download(&self, hash: impl Into<String>) -> Result<Option<BlobDownloadResult>> {
        let hash_str = hash.into();
        let request = ClientRpcRequest::GetBlob { hash: hash_str.clone() };

        let response = self.client.send(request).await?;

        match response {
            ClientRpcResponse::GetBlobResult(result) => {
                if result.found {
                    let data = result.data.context("Blob found but no data returned")?;
                    Ok(Some(BlobDownloadResult {
                        hash: hash_str,
                        size: data.len() as u64,
                        data,
                    }))
                } else if let Some(error) = result.error {
                    Err(anyhow::anyhow!("Failed to get blob: {}", error))
                } else {
                    Ok(None)
                }
            }
            _ => Err(anyhow::anyhow!("Unexpected response type for blob get")),
        }
    }

    /// Download a blob from the network using a ticket.
    ///
    /// Fetches the blob from a remote peer and stores it locally.
    pub async fn download_from_ticket(
        &self,
        ticket: impl Into<String>,
        tag: Option<String>,
    ) -> Result<BlobDownloadResult> {
        let ticket_str = ticket.into();
        let request = ClientRpcRequest::DownloadBlob {
            ticket: ticket_str.clone(),
            tag,
        };

        let response = self.client.send(request).await?;

        match response {
            ClientRpcResponse::DownloadBlobResult(result) => {
                if result.success {
                    let hash = result.hash.context("Blob downloaded but no hash returned")?;
                    let _size = result.size.context("Blob downloaded but no size returned")?;

                    // Now fetch the actual data
                    if let Some(download) = self.download(&hash).await? {
                        Ok(download)
                    } else {
                        Err(anyhow::anyhow!("Blob downloaded but not found locally"))
                    }
                } else {
                    Err(anyhow::anyhow!(
                        "Failed to download blob: {}",
                        result.error.unwrap_or_else(|| "Unknown error".to_string())
                    ))
                }
            }
            _ => Err(anyhow::anyhow!("Unexpected response type for blob download")),
        }
    }

    /// Check if a blob exists locally.
    pub async fn exists(&self, hash: impl Into<String>) -> Result<bool> {
        let request = ClientRpcRequest::HasBlob { hash: hash.into() };

        let response = self.client.send(request).await?;

        match response {
            ClientRpcResponse::HasBlobResult(result) => {
                if let Some(error) = result.error {
                    Err(anyhow::anyhow!("Failed to check blob: {}", error))
                } else {
                    Ok(result.exists)
                }
            }
            _ => Err(anyhow::anyhow!("Unexpected response type for blob check")),
        }
    }

    /// Get a ticket for sharing a blob.
    ///
    /// The ticket can be used by other nodes to download the blob.
    pub async fn get_ticket(&self, hash: impl Into<String>) -> Result<String> {
        let request = ClientRpcRequest::GetBlobTicket { hash: hash.into() };

        let response = self.client.send(request).await?;

        match response {
            ClientRpcResponse::GetBlobTicketResult(result) => {
                if result.success {
                    result.ticket.context("Ticket generated but not returned")
                } else {
                    Err(anyhow::anyhow!(
                        "Failed to get blob ticket: {}",
                        result.error.unwrap_or_else(|| "Unknown error".to_string())
                    ))
                }
            }
            _ => Err(anyhow::anyhow!("Unexpected response type for blob ticket")),
        }
    }

    /// List blobs in the store.
    pub async fn list(&self, options: BlobListOptions) -> Result<BlobListResult> {
        let request = ClientRpcRequest::ListBlobs {
            limit: options.limit.unwrap_or(100),
            continuation_token: options.continuation_token,
        };

        let response = self.client.send(request).await?;

        match response {
            ClientRpcResponse::ListBlobsResult(result) => {
                if let Some(error) = result.error {
                    Err(anyhow::anyhow!("Failed to list blobs: {}", error))
                } else {
                    Ok(BlobListResult {
                        blobs: result
                            .blobs
                            .into_iter()
                            .map(|e| BlobEntry {
                                hash: e.hash,
                                size: e.size,
                            })
                            .collect(),
                        has_more: result.has_more,
                        continuation_token: result.continuation_token,
                    })
                }
            }
            _ => Err(anyhow::anyhow!("Unexpected response type for blob list")),
        }
    }

    /// Get detailed status of a blob.
    pub async fn status(&self, hash: impl Into<String>) -> Result<Option<BlobStatus>> {
        let request = ClientRpcRequest::GetBlobStatus { hash: hash.into() };

        let response = self.client.send(request).await?;

        match response {
            ClientRpcResponse::GetBlobStatusResult(result) => {
                if result.found {
                    Ok(Some(BlobStatus {
                        hash: result.hash.context("Status found but no hash returned")?,
                        size: result.size,
                        complete: result.complete.unwrap_or(false),
                        tags: result.tags.unwrap_or_default(),
                    }))
                } else if let Some(error) = result.error {
                    Err(anyhow::anyhow!("Failed to get blob status: {}", error))
                } else {
                    Ok(None)
                }
            }
            _ => Err(anyhow::anyhow!("Unexpected response type for blob status")),
        }
    }

    /// Protect a blob from garbage collection.
    pub async fn protect(&self, hash: impl Into<String>, tag: impl Into<String>) -> Result<()> {
        let request = ClientRpcRequest::ProtectBlob {
            hash: hash.into(),
            tag: tag.into(),
        };

        let response = self.client.send(request).await?;

        match response {
            ClientRpcResponse::ProtectBlobResult(result) => {
                if result.success {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Failed to protect blob: {}",
                        result.error.unwrap_or_else(|| "Unknown error".to_string())
                    ))
                }
            }
            _ => Err(anyhow::anyhow!("Unexpected response type for blob protect")),
        }
    }

    /// Remove protection from a blob.
    pub async fn unprotect(&self, tag: impl Into<String>) -> Result<()> {
        let request = ClientRpcRequest::UnprotectBlob { tag: tag.into() };

        let response = self.client.send(request).await?;

        match response {
            ClientRpcResponse::UnprotectBlobResult(result) => {
                if result.success {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Failed to unprotect blob: {}",
                        result.error.unwrap_or_else(|| "Unknown error".to_string())
                    ))
                }
            }
            _ => Err(anyhow::anyhow!("Unexpected response type for blob unprotect")),
        }
    }

    /// Delete a blob from the store.
    pub async fn delete(&self, hash: impl Into<String>, force: bool) -> Result<()> {
        let request = ClientRpcRequest::DeleteBlob {
            hash: hash.into(),
            force,
        };

        let response = self.client.send(request).await?;

        match response {
            ClientRpcResponse::DeleteBlobResult(result) => {
                if result.success {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Failed to delete blob: {}",
                        result.error.unwrap_or_else(|| "Unknown error".to_string())
                    ))
                }
            }
            _ => Err(anyhow::anyhow!("Unexpected response type for blob delete")),
        }
    }
}

/// Extension trait to add blob management to AspenClient.
pub trait AspenClientBlobExt {
    /// Get a blob management client.
    fn blobs(&self) -> BlobClient<'_>;
}

impl AspenClientBlobExt for AspenClient {
    fn blobs(&self) -> BlobClient<'_> {
        BlobClient::new(self)
    }
}

// ============================================================================
// RpcBlobStore - implements aspen_blob::BlobStore via RPC
// ============================================================================

#[cfg(feature = "blob-store")]
mod rpc_blob_store {
    use std::io::Cursor;
    use std::path::Path;
    use std::pin::Pin;
    use std::time::Duration;

    use aspen_blob::AddBlobResult;
    use aspen_blob::AsyncReadSeek;
    use aspen_blob::BlobRef;
    use aspen_blob::BlobStatus;
    use aspen_blob::BlobStore;
    use aspen_blob::BlobStoreError;
    use async_trait::async_trait;
    use bytes::Bytes;
    use iroh_blobs::BlobFormat;
    use iroh_blobs::Hash;
    use iroh_blobs::ticket::BlobTicket;
    use tracing::debug;
    use tracing::warn;

    use super::*;

    /// A BlobStore implementation that forwards all operations to a remote
    /// Aspen cluster via CLIENT_ALPN RPC.
    ///
    /// This is used by ephemeral workers (e.g., VMs in worker-only mode)
    /// that need to access blob storage without having a local blob store.
    ///
    /// # Example
    /// ```rust,ignore
    /// // In worker-only mode
    /// let client = AspenClient::new(endpoint, ticket, timeout, None);
    /// let rpc_blob_store = Arc::new(RpcBlobStore::new(client));
    /// let worker = LocalExecutorWorker::with_blob_store(config, rpc_blob_store);
    /// ```
    pub struct RpcBlobStore {
        client: AspenClient,
    }

    impl RpcBlobStore {
        /// Create a new RPC blob store wrapping an Aspen client.
        pub fn new(client: AspenClient) -> Self {
            Self { client }
        }

        /// Parse a hex hash string into a Hash.
        fn parse_hash(hash_str: &str) -> Result<Hash, BlobStoreError> {
            let hash_bytes = hex::decode(hash_str).map_err(|e| BlobStoreError::Storage {
                message: format!("Invalid hash format: {}", e),
            })?;
            let hash: [u8; 32] = hash_bytes.try_into().map_err(|_| BlobStoreError::Storage {
                message: "Hash has wrong length".to_string(),
            })?;
            Ok(Hash::from(hash))
        }
    }

    #[async_trait]
    impl BlobStore for RpcBlobStore {
        async fn add_bytes(&self, data: &[u8]) -> Result<AddBlobResult, BlobStoreError> {
            let request = ClientRpcRequest::AddBlob {
                data: data.to_vec(),
                tag: None,
            };

            match self.client.send(request).await {
                Ok(ClientRpcResponse::AddBlobResult(result)) => {
                    if result.success {
                        let hash_str = result.hash.ok_or_else(|| BlobStoreError::Storage {
                            message: "Blob uploaded but no hash returned".to_string(),
                        })?;
                        let hash = Self::parse_hash(&hash_str)?;
                        let size = result.size.unwrap_or(data.len() as u64);
                        Ok(AddBlobResult {
                            blob_ref: BlobRef::new(hash, size, BlobFormat::Raw),
                            was_new: result.was_new.unwrap_or(true),
                        })
                    } else {
                        Err(BlobStoreError::Storage {
                            message: result.error.unwrap_or_else(|| "Unknown error".to_string()),
                        })
                    }
                }
                Ok(_) => Err(BlobStoreError::Storage {
                    message: "Unexpected response type for blob add".to_string(),
                }),
                Err(e) => Err(BlobStoreError::Storage {
                    message: format!("RPC error: {}", e),
                }),
            }
        }

        async fn add_path(&self, path: &Path) -> Result<AddBlobResult, BlobStoreError> {
            let data = std::fs::read(path).map_err(|e| BlobStoreError::Storage {
                message: format!("Failed to read file: {}", e),
            })?;
            self.add_bytes(&data).await
        }

        async fn get_bytes(&self, hash: &Hash) -> Result<Option<Bytes>, BlobStoreError> {
            let hash_str = hash.to_hex().to_string();
            debug!(hash = %hash_str, "RpcBlobStore: fetching blob via RPC");

            let request = ClientRpcRequest::GetBlob { hash: hash_str.clone() };

            match self.client.send(request).await {
                Ok(ClientRpcResponse::GetBlobResult(result)) => {
                    if result.found {
                        match result.data {
                            Some(data) => {
                                debug!(hash = %hash_str, size = data.len(), "RpcBlobStore: blob retrieved");
                                Ok(Some(Bytes::from(data)))
                            }
                            None => {
                                warn!(hash = %hash_str, "RpcBlobStore: blob found but no data returned");
                                Ok(None)
                            }
                        }
                    } else if let Some(error) = result.error {
                        Err(BlobStoreError::Storage { message: error })
                    } else {
                        Ok(None)
                    }
                }
                Ok(_) => Err(BlobStoreError::Storage {
                    message: "Unexpected response type for blob get".to_string(),
                }),
                Err(e) => Err(BlobStoreError::Storage {
                    message: format!("RPC error: {}", e),
                }),
            }
        }

        async fn has(&self, hash: &Hash) -> Result<bool, BlobStoreError> {
            let request = ClientRpcRequest::HasBlob {
                hash: hash.to_hex().to_string(),
            };

            match self.client.send(request).await {
                Ok(ClientRpcResponse::HasBlobResult(result)) => {
                    if let Some(error) = result.error {
                        Err(BlobStoreError::Storage { message: error })
                    } else {
                        Ok(result.exists)
                    }
                }
                Ok(_) => Err(BlobStoreError::Storage {
                    message: "Unexpected response type for blob has".to_string(),
                }),
                Err(e) => Err(BlobStoreError::Storage {
                    message: format!("RPC error: {}", e),
                }),
            }
        }

        async fn status(&self, hash: &Hash) -> Result<Option<BlobStatus>, BlobStoreError> {
            let request = ClientRpcRequest::GetBlobStatus {
                hash: hash.to_hex().to_string(),
            };

            match self.client.send(request).await {
                Ok(ClientRpcResponse::GetBlobStatusResult(result)) => {
                    if result.found {
                        Ok(Some(BlobStatus {
                            hash: *hash,
                            size: result.size,
                            complete: result.complete.unwrap_or(false),
                            tags: result.tags.unwrap_or_default(),
                        }))
                    } else if let Some(error) = result.error {
                        Err(BlobStoreError::Storage { message: error })
                    } else {
                        Ok(None)
                    }
                }
                Ok(_) => Err(BlobStoreError::Storage {
                    message: "Unexpected response type for blob status".to_string(),
                }),
                Err(e) => Err(BlobStoreError::Storage {
                    message: format!("RPC error: {}", e),
                }),
            }
        }

        async fn protect(&self, hash: &Hash, tag_name: &str) -> Result<(), BlobStoreError> {
            let request = ClientRpcRequest::ProtectBlob {
                hash: hash.to_hex().to_string(),
                tag: tag_name.to_string(),
            };

            match self.client.send(request).await {
                Ok(ClientRpcResponse::ProtectBlobResult(result)) => {
                    if result.success {
                        Ok(())
                    } else {
                        Err(BlobStoreError::Storage {
                            message: result.error.unwrap_or_else(|| "Unknown error".to_string()),
                        })
                    }
                }
                Ok(_) => Err(BlobStoreError::Storage {
                    message: "Unexpected response type for blob protect".to_string(),
                }),
                Err(e) => Err(BlobStoreError::Storage {
                    message: format!("RPC error: {}", e),
                }),
            }
        }

        async fn unprotect(&self, tag_name: &str) -> Result<(), BlobStoreError> {
            let request = ClientRpcRequest::UnprotectBlob {
                tag: tag_name.to_string(),
            };

            match self.client.send(request).await {
                Ok(ClientRpcResponse::UnprotectBlobResult(result)) => {
                    if result.success {
                        Ok(())
                    } else {
                        Err(BlobStoreError::Storage {
                            message: result.error.unwrap_or_else(|| "Unknown error".to_string()),
                        })
                    }
                }
                Ok(_) => Err(BlobStoreError::Storage {
                    message: "Unexpected response type for blob unprotect".to_string(),
                }),
                Err(e) => Err(BlobStoreError::Storage {
                    message: format!("RPC error: {}", e),
                }),
            }
        }

        async fn ticket(&self, hash: &Hash) -> Result<BlobTicket, BlobStoreError> {
            let request = ClientRpcRequest::GetBlobTicket {
                hash: hash.to_hex().to_string(),
            };

            match self.client.send(request).await {
                Ok(ClientRpcResponse::GetBlobTicketResult(result)) => {
                    if result.success {
                        let ticket_str = result.ticket.ok_or_else(|| BlobStoreError::Storage {
                            message: "Ticket generated but not returned".to_string(),
                        })?;
                        ticket_str.parse::<BlobTicket>().map_err(|e| BlobStoreError::Storage {
                            message: format!("Invalid ticket format: {}", e),
                        })
                    } else {
                        Err(BlobStoreError::Storage {
                            message: result.error.unwrap_or_else(|| "Unknown error".to_string()),
                        })
                    }
                }
                Ok(_) => Err(BlobStoreError::Storage {
                    message: "Unexpected response type for blob ticket".to_string(),
                }),
                Err(e) => Err(BlobStoreError::Storage {
                    message: format!("RPC error: {}", e),
                }),
            }
        }

        async fn download(&self, ticket: &BlobTicket) -> Result<BlobRef, BlobStoreError> {
            let request = ClientRpcRequest::DownloadBlob {
                ticket: ticket.to_string(),
                tag: None,
            };

            match self.client.send(request).await {
                Ok(ClientRpcResponse::DownloadBlobResult(result)) => {
                    if result.success {
                        let hash_str = result.hash.ok_or_else(|| BlobStoreError::Download {
                            message: "Blob downloaded but no hash returned".to_string(),
                        })?;
                        let hash = Self::parse_hash(&hash_str).map_err(|e| BlobStoreError::Download {
                            message: format!("{}", e),
                        })?;
                        let size = result.size.unwrap_or(0);
                        Ok(BlobRef::new(hash, size, BlobFormat::Raw))
                    } else {
                        Err(BlobStoreError::Download {
                            message: result.error.unwrap_or_else(|| "Unknown error".to_string()),
                        })
                    }
                }
                Ok(_) => Err(BlobStoreError::Download {
                    message: "Unexpected response type for blob download".to_string(),
                }),
                Err(e) => Err(BlobStoreError::Download {
                    message: format!("RPC error: {}", e),
                }),
            }
        }

        async fn list(
            &self,
            limit: u32,
            continuation_token: Option<&str>,
        ) -> Result<aspen_blob::BlobListResult, BlobStoreError> {
            let request = ClientRpcRequest::ListBlobs {
                limit,
                continuation_token: continuation_token.map(|s| s.to_string()),
            };

            match self.client.send(request).await {
                Ok(ClientRpcResponse::ListBlobsResult(result)) => {
                    if let Some(error) = result.error {
                        Err(BlobStoreError::Storage { message: error })
                    } else {
                        let blobs = result
                            .blobs
                            .into_iter()
                            .filter_map(|entry| {
                                let hash = Self::parse_hash(&entry.hash).ok()?;
                                Some(aspen_blob::BlobListEntry {
                                    hash,
                                    size: entry.size,
                                    format: BlobFormat::Raw,
                                })
                            })
                            .collect();
                        Ok(aspen_blob::BlobListResult {
                            blobs,
                            continuation_token: result.continuation_token,
                        })
                    }
                }
                Ok(_) => Err(BlobStoreError::Storage {
                    message: "Unexpected response type for blob list".to_string(),
                }),
                Err(e) => Err(BlobStoreError::Storage {
                    message: format!("RPC error: {}", e),
                }),
            }
        }

        async fn wait_available(&self, hash: &Hash, timeout: Duration) -> Result<bool, BlobStoreError> {
            // For RPC blob store, we poll until the blob is available or timeout
            let start = std::time::Instant::now();
            loop {
                if self.has(hash).await? {
                    return Ok(true);
                }
                if start.elapsed() >= timeout {
                    return Ok(false);
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        async fn wait_available_all(&self, hashes: &[Hash], timeout: Duration) -> Result<Vec<Hash>, BlobStoreError> {
            // Return hashes that are NOT available after waiting
            let start = std::time::Instant::now();
            let mut missing: std::collections::HashSet<Hash> = hashes.iter().copied().collect();

            loop {
                let mut still_missing = std::collections::HashSet::new();
                for hash in &missing {
                    if !self.has(hash).await? {
                        still_missing.insert(*hash);
                    }
                }
                missing = still_missing;

                if missing.is_empty() {
                    return Ok(Vec::new());
                }
                if start.elapsed() >= timeout {
                    return Ok(missing.into_iter().collect());
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        async fn reader(&self, hash: &Hash) -> Result<Option<Pin<Box<dyn AsyncReadSeek>>>, BlobStoreError> {
            // For RPC blob store, we fetch the entire blob and wrap it in a Cursor
            // This is not optimal for large blobs but works for reasonable sizes
            match self.get_bytes(hash).await? {
                Some(bytes) => {
                    let cursor = Cursor::new(bytes.to_vec());
                    Ok(Some(Box::pin(cursor)))
                }
                None => Ok(None),
            }
        }
    }
}

#[cfg(feature = "blob-store")]
pub use rpc_blob_store::RpcBlobStore;
