//! High-level blob storage client for the Aspen distributed system.
//!
//! This module provides an ergonomic async API for interacting with Aspen's
//! content-addressed blob storage system, including upload, download, and
//! P2P distribution features.

use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use aspen_client_rpc::AddBlobResultResponse;
use aspen_client_rpc::BlobListEntry;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use aspen_client_rpc::DeleteBlobResultResponse;
use aspen_client_rpc::DownloadBlobResultResponse;
use aspen_client_rpc::GetBlobResultResponse;
use aspen_client_rpc::GetBlobStatusResultResponse;
use aspen_client_rpc::GetBlobTicketResultResponse;
use aspen_client_rpc::HasBlobResultResponse;
use aspen_client_rpc::ListBlobsResultResponse;
use aspen_client_rpc::ProtectBlobResultResponse;
use aspen_client_rpc::UnprotectBlobResultResponse;
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
                    let size = result.size.context("Blob downloaded but no size returned")?;

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
