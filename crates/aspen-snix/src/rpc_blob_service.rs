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
use tracing::error;
use tracing::info;
use tracing::instrument;
use tracing::trace;
use tracing::warn;

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
    #[instrument(skip(self, request), fields(gateway = %self.gateway_node.fmt_short()))]
    async fn send_rpc(&self, request: ClientRpcRequest) -> io::Result<ClientRpcResponse> {
        let request_type = format!("{:?}", std::mem::discriminant(&request));
        trace!(request_type = %request_type, "initiating RPC blob request");

        // Connect to gateway
        debug!(gateway = %self.gateway_node.fmt_short(), "connecting to gateway for blob RPC");
        let connection = match timeout(RPC_TIMEOUT, self.endpoint.connect(self.gateway_node, CLIENT_ALPN)).await {
            Ok(Ok(conn)) => {
                debug!(gateway = %self.gateway_node.fmt_short(), "connected to gateway");
                conn
            }
            Ok(Err(e)) => {
                error!(gateway = %self.gateway_node.fmt_short(), error = %e, "RPC connection failed");
                return Err(io::Error::other(format!("RPC connection failed: {}", e)));
            }
            Err(_) => {
                error!(gateway = %self.gateway_node.fmt_short(), timeout_secs = RPC_TIMEOUT.as_secs(), "RPC connection timeout");
                return Err(io::Error::other("RPC connection timeout"));
            }
        };

        // Open bidirectional stream
        trace!("opening bidirectional stream");
        let (mut send, mut recv) = connection.open_bi().await.map_err(|e| {
            error!(error = %e, "failed to open RPC stream");
            io::Error::other(format!("failed to open RPC stream: {}", e))
        })?;

        // Serialize request
        let request_bytes = postcard::to_allocvec(&request).map_err(|e| {
            error!(error = %e, "failed to serialize request");
            io::Error::other(format!("failed to serialize request: {}", e))
        })?;

        trace!(request_size = request_bytes.len(), "sending RPC request");

        // Send request data (no length prefix - gateway uses read_to_end)
        send.write_all(&request_bytes).await.map_err(|e| {
            error!(error = %e, "failed to send request body");
            io::Error::other(format!("failed to send request: {}", e))
        })?;
        send.finish().map_err(|e| {
            error!(error = %e, "failed to finish send stream");
            io::Error::other(format!("failed to finish send: {}", e))
        })?;

        trace!("request sent, waiting for response");

        // Read response (no length prefix - gateway sends raw postcard bytes)
        // Use same max size as gateway for consistency
        const MAX_RESPONSE_SIZE: usize = 256 * 1024 * 1024; // Match MAX_CLIENT_MESSAGE_SIZE
        let response_bytes = recv.read_to_end(MAX_RESPONSE_SIZE).await.map_err(|e| {
            error!(error = %e, "failed to read response");
            io::Error::other(format!("failed to read response: {}", e))
        })?;

        trace!(response_size = response_bytes.len(), "received response");

        // Deserialize response
        let response: ClientRpcResponse = postcard::from_bytes(&response_bytes).map_err(|e| {
            error!(error = %e, response_size = response_bytes.len(), "failed to deserialize response");
            io::Error::other(format!("failed to deserialize response: {}", e))
        })?;

        let response_type = format!("{:?}", std::mem::discriminant(&response));
        debug!(request_type = %request_type, response_type = %response_type, "RPC blob request completed");

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
    is_closed: bool,
}

impl RpcBlobWriter {
    fn new(service: RpcBlobService) -> Self {
        Self {
            service,
            buffer: Vec::new(),
            digest: None,
            is_closed: false,
        }
    }
}

impl AsyncWrite for RpcBlobWriter {
    fn poll_write(mut self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        // Check size limit
        if self.buffer.len() + buf.len() > MAX_RPC_BLOB_SIZE {
            warn!(
                current_size = self.buffer.len(),
                incoming_size = buf.len(),
                max_size = MAX_RPC_BLOB_SIZE,
                "blob size exceeds RPC limit"
            );
            return Poll::Ready(Err(io::Error::other(format!(
                "blob size exceeds RPC limit of {} bytes",
                MAX_RPC_BLOB_SIZE
            ))));
        }
        trace!(chunk_size = buf.len(), total_buffered = self.buffer.len() + buf.len(), "buffering blob data");
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
        if self.is_closed {
            debug!("blob writer already closed, returning cached digest");
            return self.digest.ok_or_else(|| {
                error!("blob writer was closed but no digest available");
                io::Error::other("blob writer was closed but no digest available")
            });
        }

        if self.buffer.is_empty() {
            warn!("attempted to close empty blob writer");
            return Err(io::Error::other("cannot close empty blob writer"));
        }

        let data = std::mem::take(&mut self.buffer);
        let data_len = data.len();

        info!(size = data_len, "uploading blob via RPC AddBlob");

        let request = ClientRpcRequest::AddBlob {
            data,
            tag: Some("snix".to_string()), // Protect from GC
        };

        let response = self.service.send_rpc(request).await?;

        match response {
            ClientRpcResponse::AddBlobResult(AddBlobResultResponse { hash, error, .. }) => {
                if let Some(err) = error {
                    error!(error = %err, size = data_len, "RPC AddBlob returned error");
                    return Err(io::Error::other(format!("RPC error: {}", err)));
                }

                let hash_str = hash.ok_or_else(|| {
                    error!(size = data_len, "no hash in AddBlob response");
                    io::Error::other("no hash in add blob response")
                })?;
                let hash_bytes = hex::decode(&hash_str).map_err(|e| {
                    error!(hash = %hash_str, error = %e, "invalid hash hex in AddBlob response");
                    io::Error::other(format!("invalid hash hex: {}", e))
                })?;

                let digest: B3Digest = hash_bytes.as_slice().try_into().map_err(|e| {
                    error!(hash = %hash_str, error = %e, "invalid digest length in AddBlob response");
                    io::Error::other(format!("invalid digest length: {}", e))
                })?;

                info!(hash = %hash_str, size = data_len, "blob uploaded successfully via RPC");
                self.digest = Some(digest);
                self.is_closed = true;
                Ok(digest)
            }
            ClientRpcResponse::Error(err) => {
                error!(error = %err.message, size = data_len, "RPC AddBlob failed with error response");
                Err(io::Error::other(format!("RPC error: {}", err.message)))
            }
            other => {
                error!(response = ?other, size = data_len, "unexpected RPC response to AddBlob");
                Err(io::Error::other(format!("unexpected RPC response: {:?}", other)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // Integration tests require a running cluster node.
    // See tests/ directory for full integration tests.
}
