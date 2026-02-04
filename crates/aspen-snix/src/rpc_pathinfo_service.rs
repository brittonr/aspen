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
use tracing::error;
use tracing::info;
use tracing::instrument;
use tracing::trace;
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
    #[instrument(skip(self, request), fields(gateway = %self.gateway_node.fmt_short()))]
    async fn send_rpc(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse, Error> {
        let request_type = format!("{:?}", std::mem::discriminant(&request));
        trace!(request_type = %request_type, "initiating RPC pathinfo request");

        // Connect to gateway
        debug!(gateway = %self.gateway_node.fmt_short(), "connecting to gateway for pathinfo RPC");
        let connection = match timeout(RPC_TIMEOUT, self.endpoint.connect(self.gateway_node, CLIENT_ALPN)).await {
            Ok(Ok(conn)) => {
                debug!(gateway = %self.gateway_node.fmt_short(), "connected to gateway");
                conn
            }
            Ok(Err(e)) => {
                error!(gateway = %self.gateway_node.fmt_short(), error = %e, "RPC connection failed");
                return Err(Box::new(std::io::Error::other(format!("RPC connection failed: {}", e))));
            }
            Err(_) => {
                error!(gateway = %self.gateway_node.fmt_short(), timeout_secs = RPC_TIMEOUT.as_secs(), "RPC connection timeout");
                return Err(Box::new(std::io::Error::other("RPC connection timeout")));
            }
        };

        // Open bidirectional stream
        trace!("opening bidirectional stream");
        let (mut send, mut recv) = connection.open_bi().await.map_err(|e| {
            error!(error = %e, "failed to open RPC stream");
            Box::new(std::io::Error::other(format!("failed to open RPC stream: {}", e))) as Error
        })?;

        // Serialize request
        let request_bytes = postcard::to_allocvec(&request).map_err(|e| {
            error!(error = %e, "failed to serialize request");
            Box::new(std::io::Error::other(format!("failed to serialize request: {}", e))) as Error
        })?;

        trace!(request_size = request_bytes.len(), "sending RPC request");

        // Send request data (no length prefix - gateway uses read_to_end)
        send.write_all(&request_bytes).await.map_err(|e| {
            error!(error = %e, "failed to send request body");
            Box::new(std::io::Error::other(format!("failed to send request: {}", e))) as Error
        })?;
        send.finish().map_err(|e| {
            error!(error = %e, "failed to finish send stream");
            Box::new(std::io::Error::other(format!("failed to finish send: {}", e))) as Error
        })?;

        trace!("request sent, waiting for response");

        // Read response (no length prefix - gateway sends raw postcard bytes)
        const MAX_RESPONSE_SIZE: usize = 256 * 1024 * 1024; // Match MAX_CLIENT_MESSAGE_SIZE
        let response_bytes = recv.read_to_end(MAX_RESPONSE_SIZE).await.map_err(|e| {
            error!(error = %e, "failed to read response");
            Box::new(std::io::Error::other(format!("failed to read response: {}", e))) as Error
        })?;

        trace!(response_size = response_bytes.len(), "received response");

        // Deserialize response
        let response: ClientRpcResponse = postcard::from_bytes(&response_bytes).map_err(|e| {
            error!(error = %e, response_size = response_bytes.len(), "failed to deserialize response");
            Box::new(std::io::Error::other(format!("failed to deserialize response: {}", e))) as Error
        })?;

        let response_type = format!("{:?}", std::mem::discriminant(&response));
        debug!(request_type = %request_type, response_type = %response_type, "RPC pathinfo request completed");

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
        let store_path_str = path_info.store_path.to_string();
        let nar_size = path_info.nar_size;
        info!(store_path = %store_path_str, nar_size, "putting path info via RPC");

        // Convert to protobuf
        let proto_pathinfo = snix_store::proto::PathInfo::from(path_info.clone());

        // Encode to bytes
        let bytes = proto_pathinfo.encode_to_vec();

        // Encode to base64 for transport
        let pathinfo_bytes = base64::engine::general_purpose::STANDARD.encode(&bytes);

        debug!(store_path = %store_path_str, size_bytes = bytes.len(), base64_size = pathinfo_bytes.len(), "RPC path info put");

        let request = ClientRpcRequest::SnixPathInfoPut { pathinfo_bytes };

        let response = self.send_rpc(request).await?;

        match response {
            ClientRpcResponse::SnixPathInfoPutResult(SnixPathInfoPutResultResponse {
                success,
                store_path,
                error,
            }) => {
                if let Some(err) = error {
                    error!(error = %err, store_path = %store_path_str, "RPC path info put returned error");
                    return Err(Box::new(std::io::Error::other(format!("RPC error: {}", err))));
                }

                if !success {
                    error!(store_path = %store_path_str, "RPC path info put failed without error message");
                    return Err(Box::new(std::io::Error::other("path info put failed")));
                }

                info!(
                    store_path = store_path.as_deref().unwrap_or("unknown"),
                    nar_size, "path info stored successfully via RPC"
                );
                Ok(path_info)
            }
            ClientRpcResponse::Error(err) => {
                error!(error = %err.message, store_path = %store_path_str, "RPC path info put failed with error response");
                Err(Box::new(std::io::Error::other(format!("RPC error: {}", err.message))))
            }
            other => {
                error!(response = ?other, store_path = %store_path_str, "unexpected RPC response to path info put");
                Err(Box::new(std::io::Error::other(format!("unexpected RPC response: {:?}", other))))
            }
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
