//! RPC-based SNIX PathInfoService for ephemeral workers.
//!
//! This module provides [`RpcPathInfoService`], which implements the SNIX
//! [`PathInfoService`] trait by making RPC calls to cluster nodes.
//!
//! # Use Case
//!
//! Ephemeral CI workers (VMs) use this service to register Nix store paths
//! in the cluster's SNIX binary cache without needing direct access to the
//! Raft state machine.
//!
//! # Architecture
//!
//! ```text
//! VM Worker                    Cluster Node
//! ┌──────────────────┐        ┌──────────────────┐
//! │ RpcPathInfoService │ ───► │ SNIX RPC Handler │
//! │   (this module)    │      │                  │
//! └──────────────────┘        │  ▼               │
//!                              │ RaftPathInfoService │
//!                              └──────────────────┘
//! ```

use std::sync::Arc;
use std::time::Duration;

use aspen_client_api::CLIENT_ALPN;
use aspen_client_api::messages::ClientRpcRequest;
use aspen_client_api::messages::ClientRpcResponse;
use aspen_client_api::messages::SnixPathInfoGetResultResponse;
use aspen_client_api::messages::SnixPathInfoPutResultResponse;
use async_trait::async_trait;
use base64::Engine;
use futures::stream::BoxStream;
use iroh::Endpoint;
use iroh::PublicKey;
use prost::Message;
use snix_store::pathinfoservice::Error;
use snix_store::pathinfoservice::PathInfo;
use snix_store::pathinfoservice::PathInfoService;
use tokio::time::timeout;
use tracing::debug;
use tracing::instrument;
use tracing::warn;

use crate::constants::STORE_PATH_DIGEST_LENGTH;

/// RPC timeout for SNIX operations.
const RPC_TIMEOUT: Duration = Duration::from_secs(30);

/// RPC-based SNIX PathInfoService for ephemeral workers.
///
/// Implements the PathInfoService trait by making RPC calls to a cluster node.
/// Used by CI VMs to register store paths in the cluster's SNIX binary cache.
pub struct RpcPathInfoService {
    /// Iroh endpoint for making connections.
    endpoint: Arc<Endpoint>,
    /// Gateway node's public key (the cluster node to route requests to).
    gateway_node: PublicKey,
}

impl Clone for RpcPathInfoService {
    fn clone(&self) -> Self {
        Self {
            endpoint: Arc::clone(&self.endpoint),
            gateway_node: self.gateway_node,
        }
    }
}

impl RpcPathInfoService {
    /// Create a new RpcPathInfoService.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - Iroh endpoint for making connections
    /// * `gateway_node` - Public key of the cluster node to route requests to
    pub fn new(endpoint: Arc<Endpoint>, gateway_node: PublicKey) -> Self {
        Self { endpoint, gateway_node }
    }

    /// Send an RPC request to the gateway node.
    async fn send_rpc(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse, Error> {
        // Connect to gateway
        let connection = timeout(RPC_TIMEOUT, self.endpoint.connect(self.gateway_node, CLIENT_ALPN))
            .await
            .map_err(|_| Box::new(std::io::Error::other("RPC connection timeout")) as Error)?
            .map_err(|e| Box::new(std::io::Error::other(format!("RPC connection failed: {}", e))) as Error)?;

        // Open bidirectional stream
        let (mut send, mut recv) = connection
            .open_bi()
            .await
            .map_err(|e| Box::new(std::io::Error::other(format!("failed to open RPC stream: {}", e))) as Error)?;

        // Serialize request
        let request_bytes = postcard::to_allocvec(&request)
            .map_err(|e| Box::new(std::io::Error::other(format!("failed to serialize request: {}", e))) as Error)?;

        // Send request length and data
        let len_bytes = (request_bytes.len() as u32).to_be_bytes();
        send.write_all(&len_bytes)
            .await
            .map_err(|e| Box::new(std::io::Error::other(format!("failed to send request length: {}", e))) as Error)?;
        send.write_all(&request_bytes)
            .await
            .map_err(|e| Box::new(std::io::Error::other(format!("failed to send request: {}", e))) as Error)?;
        send.finish()
            .map_err(|e| Box::new(std::io::Error::other(format!("failed to finish send: {}", e))) as Error)?;

        // Read response length
        let mut len_buf = [0u8; 4];
        recv.read_exact(&mut len_buf)
            .await
            .map_err(|e| Box::new(std::io::Error::other(format!("failed to read response length: {}", e))) as Error)?;
        let response_len = u32::from_be_bytes(len_buf) as usize;

        // Read response
        let mut response_bytes = vec![0u8; response_len];
        recv.read_exact(&mut response_bytes)
            .await
            .map_err(|e| Box::new(std::io::Error::other(format!("failed to read response: {}", e))) as Error)?;

        // Deserialize response
        let response: ClientRpcResponse = postcard::from_bytes(&response_bytes)
            .map_err(|e| Box::new(std::io::Error::other(format!("failed to deserialize response: {}", e))) as Error)?;

        Ok(response)
    }
}

#[async_trait]
impl PathInfoService for RpcPathInfoService {
    #[instrument(skip(self), fields(digest = hex::encode(digest)))]
    async fn get(&self, digest: [u8; STORE_PATH_DIGEST_LENGTH]) -> Result<Option<PathInfo>, Error> {
        let digest_hex = hex::encode(digest);
        debug!(digest = %digest_hex, "RPC path info get");

        let request = ClientRpcRequest::SnixPathInfoGet {
            digest: digest_hex.clone(),
        };

        let response = self.send_rpc(request).await?;

        match response {
            ClientRpcResponse::SnixPathInfoGetResult(SnixPathInfoGetResultResponse {
                found,
                pathinfo_bytes,
                error,
            }) => {
                if let Some(err) = error {
                    return Err(Box::new(std::io::Error::other(format!("RPC error: {}", err))));
                }

                if !found {
                    debug!(digest = %digest_hex, "path info not found");
                    return Ok(None);
                }

                // Decode base64
                let bytes = pathinfo_bytes
                    .ok_or_else(|| Box::new(std::io::Error::other("missing pathinfo_bytes in response")) as Error)?;
                let decoded = base64::engine::general_purpose::STANDARD
                    .decode(&bytes)
                    .map_err(|e| Box::new(std::io::Error::other(format!("base64 decode error: {}", e))) as Error)?;

                // Decode protobuf
                let proto_pathinfo = snix_store::proto::PathInfo::decode(decoded.as_slice())
                    .map_err(|e| Box::new(std::io::Error::other(format!("protobuf decode error: {}", e))) as Error)?;

                // Convert to PathInfo
                let path_info = PathInfo::try_from(proto_pathinfo).map_err(|e| {
                    Box::new(std::io::Error::other(format!("pathinfo conversion error: {}", e))) as Error
                })?;

                debug!(digest = %digest_hex, store_path = %path_info.store_path, "path info retrieved via RPC");
                Ok(Some(path_info))
            }
            ClientRpcResponse::Error(err) => {
                Err(Box::new(std::io::Error::other(format!("RPC error: {}", err.message))))
            }
            other => Err(Box::new(std::io::Error::other(format!("unexpected RPC response: {:?}", other)))),
        }
    }

    #[instrument(skip(self, path_info), fields(store_path = %path_info.store_path))]
    async fn put(&self, path_info: PathInfo) -> Result<PathInfo, Error> {
        // Convert to protobuf
        let proto_pathinfo = snix_store::proto::PathInfo::from(path_info.clone());

        // Encode to bytes
        let bytes = proto_pathinfo.encode_to_vec();

        // Encode to base64 for transport
        let pathinfo_bytes = base64::engine::general_purpose::STANDARD.encode(&bytes);

        debug!(store_path = %path_info.store_path, size_bytes = bytes.len(), "RPC path info put");

        let request = ClientRpcRequest::SnixPathInfoPut { pathinfo_bytes };

        let response = self.send_rpc(request).await?;

        match response {
            ClientRpcResponse::SnixPathInfoPutResult(SnixPathInfoPutResultResponse {
                success,
                store_path,
                error,
            }) => {
                if let Some(err) = error {
                    return Err(Box::new(std::io::Error::other(format!("RPC error: {}", err))));
                }

                if !success {
                    return Err(Box::new(std::io::Error::other("path info put failed")));
                }

                debug!(store_path = store_path.as_deref().unwrap_or("unknown"), "path info stored via RPC");
                Ok(path_info)
            }
            ClientRpcResponse::Error(err) => {
                Err(Box::new(std::io::Error::other(format!("RPC error: {}", err.message))))
            }
            other => Err(Box::new(std::io::Error::other(format!("unexpected RPC response: {:?}", other)))),
        }
    }

    /// List all PathInfo entries.
    ///
    /// Note: This is not supported via RPC for ephemeral workers.
    /// Workers typically only need get/put operations, not listing.
    fn list(&self) -> BoxStream<'static, Result<PathInfo, Error>> {
        warn!("RpcPathInfoService::list() called - not supported for ephemeral workers");
        Box::pin(futures::stream::empty())
    }
}

#[cfg(test)]
mod tests {
    // Integration tests require a running cluster node.
    // See tests/ directory for full integration tests.
}
