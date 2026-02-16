//! RPC-based SNIX DirectoryService for ephemeral workers.
//!
//! This module provides [`RpcDirectoryService`], which implements the SNIX
//! [`DirectoryService`] trait by making RPC calls to cluster nodes.
//!
//! # Use Case
//!
//! Ephemeral CI workers (VMs) use this service to store directory metadata
//! in the cluster's Raft-backed DirectoryService without needing direct
//! access to the Raft state machine.
//!
//! # Architecture
//!
//! ```text
//! VM Worker                    Cluster Node
//! ┌──────────────────┐        ┌──────────────────┐
//! │ RpcDirectoryService │ ───► │ SNIX RPC Handler │
//! │   (this module)    │      │                  │
//! └──────────────────┘        │  ▼               │
//!                              │ RaftDirectoryService │
//!                              └──────────────────┘
//! ```

use std::sync::Arc;
use std::time::Duration;

use aspen_client_api::CLIENT_ALPN;
use aspen_client_api::messages::ClientRpcRequest;
use aspen_client_api::messages::ClientRpcResponse;
use aspen_client_api::messages::SnixDirectoryGetResultResponse;
use aspen_client_api::messages::SnixDirectoryPutResultResponse;
use async_trait::async_trait;
use base64::Engine;
use futures::stream::BoxStream;
use iroh::Endpoint;
use iroh::PublicKey;
use prost::Message;
use snix_castore::B3Digest;
use snix_castore::Directory;
use snix_castore::Error;
use snix_castore::directoryservice::DirectoryPutter;
use snix_castore::directoryservice::DirectoryService;
use tokio::time::timeout;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::instrument;
use tracing::trace;

use crate::constants::MAX_DIRECTORY_DEPTH;
use crate::constants::MAX_RECURSIVE_BUFFER;

/// RPC timeout for SNIX operations.
const RPC_TIMEOUT: Duration = Duration::from_secs(30);

/// RPC-based SNIX DirectoryService for ephemeral workers.
///
/// Implements the DirectoryService trait by making RPC calls to a cluster node.
/// Used by CI VMs to upload directory metadata to the cluster's SNIX storage.
pub struct RpcDirectoryService {
    /// Iroh endpoint for making connections.
    endpoint: Arc<Endpoint>,
    /// Gateway node's public key (the cluster node to route requests to).
    gateway_node: PublicKey,
}

impl Clone for RpcDirectoryService {
    fn clone(&self) -> Self {
        Self {
            endpoint: Arc::clone(&self.endpoint),
            gateway_node: self.gateway_node,
        }
    }
}

impl RpcDirectoryService {
    /// Create a new RpcDirectoryService.
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
    async fn send_rpc(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse, Error> {
        let request_type = format!("{:?}", std::mem::discriminant(&request));
        trace!(request_type = %request_type, "initiating RPC directory request");

        // Connect to gateway
        debug!(gateway = %self.gateway_node.fmt_short(), "connecting to gateway for directory RPC");
        let connection = match timeout(RPC_TIMEOUT, self.endpoint.connect(self.gateway_node, CLIENT_ALPN)).await {
            Ok(Ok(conn)) => {
                debug!(gateway = %self.gateway_node.fmt_short(), "connected to gateway");
                conn
            }
            Ok(Err(e)) => {
                error!(gateway = %self.gateway_node.fmt_short(), error = %e, "RPC connection failed");
                return Err(Error::StorageError(format!("RPC connection failed: {}", e)));
            }
            Err(_) => {
                error!(gateway = %self.gateway_node.fmt_short(), timeout_secs = RPC_TIMEOUT.as_secs(), "RPC connection timeout");
                return Err(Error::StorageError("RPC connection timeout".to_string()));
            }
        };

        // Open bidirectional stream
        trace!("opening bidirectional stream");
        let (mut send, mut recv) = connection.open_bi().await.map_err(|e| {
            error!(error = %e, "failed to open RPC stream");
            Error::StorageError(format!("failed to open RPC stream: {}", e))
        })?;

        // Serialize request
        let request_bytes = postcard::to_allocvec(&request).map_err(|e| {
            error!(error = %e, "failed to serialize request");
            Error::StorageError(format!("failed to serialize request: {}", e))
        })?;

        trace!(request_size = request_bytes.len(), "sending RPC request");

        // Send request data (no length prefix - gateway uses read_to_end)
        send.write_all(&request_bytes).await.map_err(|e| {
            error!(error = %e, "failed to send request body");
            Error::StorageError(format!("failed to send request: {}", e))
        })?;
        send.finish().map_err(|e| {
            error!(error = %e, "failed to finish send stream");
            Error::StorageError(format!("failed to finish send: {}", e))
        })?;

        trace!("request sent, waiting for response");

        // Read response (no length prefix - gateway sends raw postcard bytes)
        const MAX_RESPONSE_SIZE: usize = 256 * 1024 * 1024; // Match MAX_CLIENT_MESSAGE_SIZE
        let response_bytes = recv.read_to_end(MAX_RESPONSE_SIZE).await.map_err(|e| {
            error!(error = %e, "failed to read response");
            Error::StorageError(format!("failed to read response: {}", e))
        })?;

        trace!(response_size = response_bytes.len(), "received response");

        // Deserialize response
        let response: ClientRpcResponse = postcard::from_bytes(&response_bytes).map_err(|e| {
            error!(error = %e, response_size = response_bytes.len(), "failed to deserialize response");
            Error::StorageError(format!("failed to deserialize response: {}", e))
        })?;

        let response_type = format!("{:?}", std::mem::discriminant(&response));
        debug!(request_type = %request_type, response_type = %response_type, "RPC directory request completed");

        Ok(response)
    }
}

#[async_trait]
impl DirectoryService for RpcDirectoryService {
    #[instrument(skip(self), fields(digest = %digest))]
    async fn get(&self, digest: &B3Digest) -> Result<Option<Directory>, Error> {
        let digest_hex = hex::encode(digest.as_ref());
        debug!(digest = %digest_hex, "RPC directory get");

        let request = ClientRpcRequest::SnixDirectoryGet {
            digest: digest_hex.clone(),
        };

        let response = self.send_rpc(request).await?;

        match response {
            ClientRpcResponse::SnixDirectoryGetResult(SnixDirectoryGetResultResponse {
                was_found,
                directory_bytes,
                error,
            }) => {
                if let Some(err) = error {
                    return Err(Error::StorageError(format!("RPC error: {}", err)));
                }

                if !was_found {
                    debug!(digest = %digest_hex, "directory not found");
                    return Ok(None);
                }

                // Decode base64
                let bytes = directory_bytes
                    .ok_or_else(|| Error::StorageError("missing directory_bytes in response".to_string()))?;
                let decoded = base64::engine::general_purpose::STANDARD
                    .decode(&bytes)
                    .map_err(|e| Error::StorageError(format!("base64 decode error: {}", e)))?;

                // Decode protobuf
                let proto_dir = snix_castore::proto::Directory::decode(decoded.as_slice())
                    .map_err(|e| Error::StorageError(format!("protobuf decode error: {}", e)))?;

                // Convert to Directory
                let dir = Directory::try_from(proto_dir)
                    .map_err(|e| Error::StorageError(format!("directory conversion error: {}", e)))?;

                debug!(digest = %digest_hex, size = dir.size(), "directory retrieved via RPC");
                Ok(Some(dir))
            }
            ClientRpcResponse::Error(err) => Err(Error::StorageError(format!("RPC error: {}", err.message))),
            other => Err(Error::StorageError(format!("unexpected RPC response: {:?}", other))),
        }
    }

    #[instrument(skip(self, directory), fields(size = directory.size()))]
    async fn put(&self, directory: Directory) -> Result<B3Digest, Error> {
        let dir_size = directory.size();
        info!(entries = dir_size, "putting directory via RPC");

        // Convert to protobuf
        let proto_dir = snix_castore::proto::Directory::from(directory);

        // Encode to bytes
        let bytes = proto_dir.encode_to_vec();

        // Encode to base64 for transport
        let directory_bytes = base64::engine::general_purpose::STANDARD.encode(&bytes);

        debug!(size_bytes = bytes.len(), base64_size = directory_bytes.len(), "RPC directory put");

        let request = ClientRpcRequest::SnixDirectoryPut { directory_bytes };

        let response = self.send_rpc(request).await?;

        match response {
            ClientRpcResponse::SnixDirectoryPutResult(SnixDirectoryPutResultResponse {
                is_success,
                digest,
                error,
            }) => {
                if let Some(err) = error {
                    error!(error = %err, entries = dir_size, "RPC directory put returned error");
                    return Err(Error::StorageError(format!("RPC error: {}", err)));
                }

                if !is_success {
                    error!(entries = dir_size, "RPC directory put failed without error message");
                    return Err(Error::StorageError("directory put failed".to_string()));
                }

                // Decode the digest from hex
                let digest_hex = digest.ok_or_else(|| {
                    error!(entries = dir_size, "missing digest in RPC directory put response");
                    Error::StorageError("missing digest in response".to_string())
                })?;
                let digest_bytes = hex::decode(&digest_hex).map_err(|e| {
                    error!(digest = %digest_hex, error = %e, "hex decode error in directory put response");
                    Error::StorageError(format!("hex decode error: {}", e))
                })?;

                let b3_digest: B3Digest = digest_bytes.as_slice().try_into().map_err(|e| {
                    error!(digest = %digest_hex, error = %e, "invalid digest length in directory put response");
                    Error::StorageError(format!("invalid digest length: {}", e))
                })?;

                info!(digest = %digest_hex, entries = dir_size, "directory stored successfully via RPC");
                Ok(b3_digest)
            }
            ClientRpcResponse::Error(err) => {
                error!(error = %err.message, entries = dir_size, "RPC directory put failed with error response");
                Err(Error::StorageError(format!("RPC error: {}", err.message)))
            }
            other => {
                error!(response = ?other, entries = dir_size, "unexpected RPC response to directory put");
                Err(Error::StorageError(format!("unexpected RPC response: {:?}", other)))
            }
        }
    }

    /// Returns directories in root-to-leaves order (BFS traversal).
    fn get_recursive(&self, root_directory_digest: &B3Digest) -> BoxStream<'_, Result<Directory, Error>> {
        let root_digest = *root_directory_digest;

        Box::pin(async_stream::try_stream! {
            let mut queue: std::collections::VecDeque<B3Digest> = std::collections::VecDeque::new();
            let mut depth: u32 = 0;
            let mut visited: std::collections::HashSet<B3Digest> = std::collections::HashSet::new();

            queue.push_back(root_digest);

            while let Some(digest) = queue.pop_front() {
                // Check for cycles
                if visited.contains(&digest) {
                    continue;
                }
                visited.insert(digest);

                // Check depth limit
                if depth > MAX_DIRECTORY_DEPTH {
                    Err(Error::StorageError(format!(
                        "maximum directory depth {} exceeded",
                        MAX_DIRECTORY_DEPTH
                    )))?;
                }

                // Check buffer limit
                if queue.len() as u32 > MAX_RECURSIVE_BUFFER {
                    Err(Error::StorageError(format!(
                        "maximum recursive buffer {} exceeded",
                        MAX_RECURSIVE_BUFFER
                    )))?;
                }

                if let Some(dir) = self.get(&digest).await? {
                    // Queue child directories for BFS traversal
                    for (_name, node) in dir.nodes() {
                        if let snix_castore::Node::Directory { digest: child_digest, .. } = node
                            && !visited.contains(child_digest)
                        {
                            queue.push_back(*child_digest);
                        }
                    }

                    yield dir;
                    depth += 1;
                }
            }
        })
    }

    fn put_multiple_start(&self) -> Box<dyn DirectoryPutter + '_> {
        Box::new(RpcDirectoryPutter::new(self.clone()))
    }
}

/// DirectoryPutter implementation that batches directory writes via RPC.
pub struct RpcDirectoryPutter {
    service: RpcDirectoryService,
    last_digest: Option<B3Digest>,
}

impl RpcDirectoryPutter {
    fn new(service: RpcDirectoryService) -> Self {
        Self {
            service,
            last_digest: None,
        }
    }
}

#[async_trait]
impl DirectoryPutter for RpcDirectoryPutter {
    async fn put(&mut self, directory: Directory) -> Result<(), Error> {
        let digest = self.service.put(directory).await?;
        self.last_digest = Some(digest);
        Ok(())
    }

    async fn close(&mut self) -> Result<B3Digest, Error> {
        self.last_digest.take().ok_or_else(|| Error::StorageError("no directories were put".to_string()))
    }
}

#[cfg(test)]
mod tests {
    // Integration tests require a running cluster node.
    // See tests/ directory for full integration tests.
}
