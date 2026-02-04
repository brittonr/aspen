//! RPC-based SNIX BlobService for ephemeral workers.
//!
//! This module provides [`RpcBlobService`], which implements the SNIX
//! [`BlobService`] trait by making RPC calls to cluster nodes.
//!
//! # Use Case
//!
//! Ephemeral CI workers (VMs) use this service to store NAR content chunks
//! in the cluster's blob storage without needing direct access to iroh-blobs.
//!
//! # Architecture
//!
//! ```text
//! VM Worker                    Cluster Node
//! ┌──────────────────┐        ┌──────────────────┐
//! │ RpcBlobService   │ ─────► │ Blob RPC Handler │
//! │   (this module)  │        │                  │
//! └──────────────────┘        │  ▼               │
//!                              │ IrohBlobStore   │
//!                              └──────────────────┘
//! ```

use std::io::Cursor;
use std::io::{self};
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;
use std::time::Duration;

use aspen_client_api::CLIENT_ALPN;
use aspen_client_api::messages::AddBlobResultResponse;
use aspen_client_api::messages::ClientRpcRequest;
use aspen_client_api::messages::ClientRpcResponse;
use aspen_client_api::messages::GetBlobResultResponse;
use aspen_client_api::messages::HasBlobResultResponse;
use async_trait::async_trait;
use iroh::Endpoint;
use iroh::PublicKey;
use snix_castore::B3Digest;
use snix_castore::blobservice::BlobReader;
use snix_castore::blobservice::BlobService;
use snix_castore::blobservice::BlobWriter;
use snix_castore::proto::stat_blob_response::ChunkMeta;
use tokio::io::AsyncWrite;
use tokio::time::timeout;
use tracing::debug;
use tracing::instrument;

/// RPC timeout for blob operations.
const RPC_TIMEOUT: Duration = Duration::from_secs(60);

/// Maximum blob size for RPC transfer (50 MB to stay within reasonable RPC limits).
const MAX_RPC_BLOB_SIZE: usize = 50 * 1024 * 1024;

/// RPC-based SNIX BlobService for ephemeral workers.
///
/// Implements the BlobService trait by making RPC calls to a cluster node.
/// Used by CI VMs to store NAR content in the cluster's blob storage.
pub struct RpcBlobService {
    /// Iroh endpoint for making connections.
    endpoint: Arc<Endpoint>,
    /// Gateway node's public key (the cluster node to route requests to).
    gateway_node: PublicKey,
}

impl Clone for RpcBlobService {
    fn clone(&self) -> Self {
        Self {
            endpoint: Arc::clone(&self.endpoint),
            gateway_node: self.gateway_node,
        }
    }
}

impl RpcBlobService {
    /// Create a new RpcBlobService.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - Iroh endpoint for making connections
    /// * `gateway_node` - Public key of the cluster node to route requests to
    pub fn new(endpoint: Arc<Endpoint>, gateway_node: PublicKey) -> Self {
        Self { endpoint, gateway_node }
    }

    /// Send an RPC request to the gateway node.
    async fn send_rpc(&self, request: ClientRpcRequest) -> io::Result<ClientRpcResponse> {
        // Connect to gateway
        let connection = timeout(RPC_TIMEOUT, self.endpoint.connect(self.gateway_node, CLIENT_ALPN))
            .await
            .map_err(|_| io::Error::other("RPC connection timeout"))?
            .map_err(|e| io::Error::other(format!("RPC connection failed: {}", e)))?;

        // Open bidirectional stream
        let (mut send, mut recv) = connection
            .open_bi()
            .await
            .map_err(|e| io::Error::other(format!("failed to open RPC stream: {}", e)))?;

        // Serialize request
        let request_bytes = postcard::to_allocvec(&request)
            .map_err(|e| io::Error::other(format!("failed to serialize request: {}", e)))?;

        // Send request length and data
        let len_bytes = (request_bytes.len() as u32).to_be_bytes();
        send.write_all(&len_bytes)
            .await
            .map_err(|e| io::Error::other(format!("failed to send request length: {}", e)))?;
        send.write_all(&request_bytes)
            .await
            .map_err(|e| io::Error::other(format!("failed to send request: {}", e)))?;
        send.finish().map_err(|e| io::Error::other(format!("failed to finish send: {}", e)))?;

        // Read response length
        let mut len_buf = [0u8; 4];
        recv.read_exact(&mut len_buf)
            .await
            .map_err(|e| io::Error::other(format!("failed to read response length: {}", e)))?;
        let response_len = u32::from_be_bytes(len_buf) as usize;

        // Read response
        let mut response_bytes = vec![0u8; response_len];
        recv.read_exact(&mut response_bytes)
            .await
            .map_err(|e| io::Error::other(format!("failed to read response: {}", e)))?;

        // Deserialize response
        let response: ClientRpcResponse = postcard::from_bytes(&response_bytes)
            .map_err(|e| io::Error::other(format!("failed to deserialize response: {}", e)))?;

        Ok(response)
    }
}

#[async_trait]
impl BlobService for RpcBlobService {
    #[instrument(skip(self), fields(digest = %digest))]
    async fn has(&self, digest: &B3Digest) -> io::Result<bool> {
        let hash_hex = hex::encode(digest.as_ref());
        debug!(hash = %hash_hex, "RPC blob has");

        let request = ClientRpcRequest::HasBlob { hash: hash_hex.clone() };
        let response = self.send_rpc(request).await?;

        match response {
            ClientRpcResponse::HasBlobResult(HasBlobResultResponse { exists, error }) => {
                if let Some(err) = error {
                    return Err(io::Error::other(format!("RPC error: {}", err)));
                }
                debug!(hash = %hash_hex, exists, "blob has check completed");
                Ok(exists)
            }
            ClientRpcResponse::Error(err) => Err(io::Error::other(format!("RPC error: {}", err.message))),
            other => Err(io::Error::other(format!("unexpected RPC response: {:?}", other))),
        }
    }

    #[instrument(skip(self), fields(digest = %digest))]
    async fn open_read(&self, digest: &B3Digest) -> io::Result<Option<Box<dyn BlobReader>>> {
        let hash_hex = hex::encode(digest.as_ref());
        debug!(hash = %hash_hex, "RPC blob get");

        let request = ClientRpcRequest::GetBlob { hash: hash_hex.clone() };
        let response = self.send_rpc(request).await?;

        match response {
            ClientRpcResponse::GetBlobResult(GetBlobResultResponse { found, data, error, .. }) => {
                if let Some(err) = error {
                    return Err(io::Error::other(format!("RPC error: {}", err)));
                }

                if !found {
                    debug!(hash = %hash_hex, "blob not found");
                    return Ok(None);
                }

                let blob_data = data.ok_or_else(|| io::Error::other("blob exists but no data returned"))?;
                debug!(hash = %hash_hex, size = blob_data.len(), "blob retrieved via RPC");

                // Wrap bytes in a Cursor which implements BlobReader
                let cursor = Cursor::new(blob_data);
                Ok(Some(Box::new(cursor)))
            }
            ClientRpcResponse::Error(err) => Err(io::Error::other(format!("RPC error: {}", err.message))),
            other => Err(io::Error::other(format!("unexpected RPC response: {:?}", other))),
        }
    }

    async fn open_write(&self) -> Box<dyn BlobWriter> {
        Box::new(RpcBlobWriter::new(self.clone()))
    }

    #[instrument(skip(self), fields(digest = %digest))]
    async fn chunks(&self, digest: &B3Digest) -> io::Result<Option<Vec<ChunkMeta>>> {
        // For RPC-based service, we don't support granular chunking - return None
        // to indicate the blob should be fetched as a whole.
        // The caller can use open_read() to get the full blob.
        if self.has(digest).await? {
            Ok(Some(vec![]))
        } else {
            Ok(None)
        }
    }
}

/// BlobWriter implementation that buffers data and uploads via RPC on close.
pub struct RpcBlobWriter {
    service: RpcBlobService,
    buffer: Vec<u8>,
    digest: Option<B3Digest>,
    closed: bool,
}

impl RpcBlobWriter {
    fn new(service: RpcBlobService) -> Self {
        Self {
            service,
            buffer: Vec::new(),
            digest: None,
            closed: false,
        }
    }
}

impl AsyncWrite for RpcBlobWriter {
    fn poll_write(mut self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        // Check size limit
        if self.buffer.len() + buf.len() > MAX_RPC_BLOB_SIZE {
            return Poll::Ready(Err(io::Error::other(format!(
                "blob size exceeds RPC limit of {} bytes",
                MAX_RPC_BLOB_SIZE
            ))));
        }
        self.buffer.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // Actual upload happens in BlobWriter::close()
        Poll::Ready(Ok(()))
    }
}

impl Unpin for RpcBlobWriter {}

#[async_trait]
impl BlobWriter for RpcBlobWriter {
    async fn close(&mut self) -> io::Result<B3Digest> {
        // If already closed, return the cached digest
        if self.closed {
            return self.digest.ok_or_else(|| io::Error::other("blob writer was closed but no digest available"));
        }

        if self.buffer.is_empty() {
            return Err(io::Error::other("cannot close empty blob writer"));
        }

        let data = std::mem::take(&mut self.buffer);
        let data_len = data.len();

        debug!(size = data_len, "uploading blob via RPC");

        let request = ClientRpcRequest::AddBlob {
            data,
            tag: Some("snix".to_string()), // Protect from GC
        };

        let response = self.service.send_rpc(request).await?;

        match response {
            ClientRpcResponse::AddBlobResult(AddBlobResultResponse { hash, error, .. }) => {
                if let Some(err) = error {
                    return Err(io::Error::other(format!("RPC error: {}", err)));
                }

                let hash_str = hash.ok_or_else(|| io::Error::other("no hash in add blob response"))?;
                let hash_bytes =
                    hex::decode(&hash_str).map_err(|e| io::Error::other(format!("invalid hash hex: {}", e)))?;

                let digest: B3Digest = hash_bytes
                    .as_slice()
                    .try_into()
                    .map_err(|e| io::Error::other(format!("invalid digest length: {}", e)))?;

                debug!(hash = %hash_str, size = data_len, "blob uploaded via RPC");
                self.digest = Some(digest);
                self.closed = true;
                Ok(digest)
            }
            ClientRpcResponse::Error(err) => Err(io::Error::other(format!("RPC error: {}", err.message))),
            other => Err(io::Error::other(format!("unexpected RPC response: {:?}", other))),
        }
    }
}

#[cfg(test)]
mod tests {
    // Integration tests require a running cluster node.
    // See tests/ directory for full integration tests.
}
