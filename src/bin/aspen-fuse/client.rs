//! Synchronous client wrapper for FUSE filesystem operations.
//!
//! This module bridges the async Iroh client with synchronous FUSE operations
//! using a multi-threaded tokio runtime.
//!
//! # Design
//!
//! FUSE operations are synchronous (they block the calling thread), but the
//! Aspen client uses async I/O via tokio. We bridge this by:
//!
//! 1. Creating a multi-threaded tokio runtime at startup
//! 2. Using `runtime.block_on()` to execute async operations from sync contexts
//! 3. Sharing the client across FUSE handler threads via Arc
//!
//! # Tiger Style
//!
//! - Explicit timeouts on all operations
//! - Bounded retries with backoff
//! - Resource limits from constants.rs

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use aspen::CLIENT_ALPN;
use aspen::client_rpc::ClientRpcRequest;
use aspen::client_rpc::ClientRpcResponse;
use aspen::client_rpc::DeleteResultResponse;
use aspen::client_rpc::MAX_CLIENT_MESSAGE_SIZE;
use aspen::client_rpc::ReadResultResponse;
use aspen::client_rpc::ScanResultResponse;
use aspen::client_rpc::WriteResultResponse;
use iroh::Endpoint;
use iroh::EndpointAddr;
use iroh::SecretKey;
use tokio::runtime::Runtime;
use tracing::debug;
use tracing::warn;

use crate::constants::CONNECTION_TIMEOUT;
use crate::constants::READ_TIMEOUT;
use crate::constants::WRITE_TIMEOUT;

/// Maximum RPC retries before failing.
const MAX_RETRIES: u32 = 3;

/// Delay between retries.
const RETRY_DELAY: Duration = Duration::from_millis(500);

/// Synchronous wrapper around the async Aspen client.
///
/// Designed for use in FUSE filesystem operations which are synchronous.
/// Uses a multi-threaded tokio runtime internally.
pub struct FuseSyncClient {
    /// Multi-threaded tokio runtime for async operations.
    runtime: Runtime,
    /// Iroh endpoint for making connections.
    endpoint: Endpoint,
    /// Target node address.
    target_addr: EndpointAddr,
}

impl FuseSyncClient {
    /// Create a new synchronous FUSE client.
    ///
    /// Connects to the Aspen cluster using the provided endpoint address.
    pub fn new(target_addr: EndpointAddr) -> Result<Self> {
        // Create a multi-threaded runtime for async operations
        // IMPORTANT: Must be multi-threaded to avoid deadlocks when blocking
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .context("failed to create tokio runtime")?;

        // Create the Iroh endpoint inside the runtime
        let endpoint = runtime.block_on(async {
            // Generate random secret key for this client
            use rand::RngCore;
            let mut key_bytes = [0u8; 32];
            rand::rng().fill_bytes(&mut key_bytes);
            let secret_key = SecretKey::from(key_bytes);

            Endpoint::builder()
                .secret_key(secret_key)
                .alpns(vec![CLIENT_ALPN.to_vec()])
                .bind()
                .await
                .context("failed to bind Iroh endpoint")
        })?;

        Ok(Self {
            runtime,
            endpoint,
            target_addr,
        })
    }

    /// Create a client from a cluster ticket string.
    pub fn from_ticket(ticket: &str) -> Result<Self> {
        use aspen::cluster::ticket::AspenClusterTicket;

        let parsed = AspenClusterTicket::deserialize(ticket).context("failed to parse cluster ticket")?;

        if parsed.bootstrap.is_empty() {
            anyhow::bail!("no bootstrap peers in ticket");
        }

        // Use the first bootstrap peer
        let first_peer = parsed.bootstrap.iter().next().context("no bootstrap peers in ticket")?;
        let target_addr = EndpointAddr::new(*first_peer);

        Self::new(target_addr)
    }

    /// Send an RPC request with automatic retry.
    fn send_rpc(&self, request: ClientRpcRequest, timeout: Duration) -> Result<ClientRpcResponse> {
        self.runtime.block_on(async {
            let mut retries = 0u32;

            loop {
                match tokio::time::timeout(timeout, self.send_rpc_inner(request.clone())).await {
                    Ok(Ok(response)) => return Ok(response),
                    Ok(Err(err)) => {
                        warn!(
                            error = %err,
                            retries,
                            max_retries = MAX_RETRIES,
                            "FUSE RPC request failed"
                        );

                        if retries >= MAX_RETRIES {
                            return Err(err).context("max retries exceeded");
                        }

                        retries += 1;
                        tokio::time::sleep(RETRY_DELAY).await;
                    }
                    Err(_) => {
                        warn!(retries, max_retries = MAX_RETRIES, "FUSE RPC request timed out");

                        if retries >= MAX_RETRIES {
                            anyhow::bail!("RPC request timed out after {} retries", MAX_RETRIES);
                        }

                        retries += 1;
                        tokio::time::sleep(RETRY_DELAY).await;
                    }
                }
            }
        })
    }

    /// Inner async RPC send implementation.
    async fn send_rpc_inner(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        debug!(
            target_node_id = %self.target_addr.id,
            request_type = ?std::mem::discriminant(&request),
            "sending FUSE RPC request"
        );

        // Connect to the target node
        let connection =
            tokio::time::timeout(CONNECTION_TIMEOUT, self.endpoint.connect(self.target_addr.clone(), CLIENT_ALPN))
                .await
                .context("connection timeout")?
                .context("failed to connect to node")?;

        // Open a bidirectional stream for the RPC
        let (mut send, mut recv) = connection.open_bi().await.context("failed to open stream")?;

        // Serialize and send the request
        let request_bytes = postcard::to_stdvec(&request).context("failed to serialize request")?;

        send.write_all(&request_bytes).await.context("failed to send request")?;
        send.finish().context("failed to finish send stream")?;

        // Read the response
        let response_bytes = recv.read_to_end(MAX_CLIENT_MESSAGE_SIZE).await.context("failed to read response")?;

        // Deserialize the response
        let response: ClientRpcResponse =
            postcard::from_bytes(&response_bytes).context("failed to deserialize response")?;

        // Check for error response
        if let ClientRpcResponse::Error(err) = &response {
            anyhow::bail!("RPC error {}: {}", err.code, err.message);
        }

        Ok(response)
    }

    /// Read a key from the KV store.
    ///
    /// Returns the value if found, None if not found.
    pub fn read_key(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let response = self.send_rpc(ClientRpcRequest::ReadKey { key: key.to_string() }, READ_TIMEOUT)?;

        match response {
            ClientRpcResponse::ReadResult(ReadResultResponse { value, found, error }) => {
                if let Some(err) = error {
                    anyhow::bail!("read error: {}", err);
                }
                if found { Ok(value) } else { Ok(None) }
            }
            _ => anyhow::bail!("unexpected response type for ReadKey"),
        }
    }

    /// Write a key-value pair to the KV store.
    ///
    /// Returns true on success, false on failure.
    pub fn write_key(&self, key: &str, value: &[u8]) -> Result<bool> {
        let response = self.send_rpc(
            ClientRpcRequest::WriteKey {
                key: key.to_string(),
                value: value.to_vec(),
            },
            WRITE_TIMEOUT,
        )?;

        match response {
            ClientRpcResponse::WriteResult(WriteResultResponse { success, error }) => {
                if let Some(err) = error
                    && !success
                {
                    anyhow::bail!("write error: {}", err);
                }
                Ok(success)
            }
            _ => anyhow::bail!("unexpected response type for WriteKey"),
        }
    }

    /// Delete a key from the KV store.
    ///
    /// Returns true if the key existed, false if not found.
    pub fn delete_key(&self, key: &str) -> Result<bool> {
        let response = self.send_rpc(ClientRpcRequest::DeleteKey { key: key.to_string() }, WRITE_TIMEOUT)?;

        match response {
            ClientRpcResponse::DeleteResult(DeleteResultResponse { deleted, error, .. }) => {
                if let Some(err) = error {
                    anyhow::bail!("delete error: {}", err);
                }
                Ok(deleted)
            }
            _ => anyhow::bail!("unexpected response type for DeleteKey"),
        }
    }

    /// Scan keys with a prefix.
    ///
    /// Returns a list of (key, value) pairs.
    pub fn scan_keys(&self, prefix: &str, limit: u32) -> Result<Vec<(String, Vec<u8>)>> {
        let response = self.send_rpc(
            ClientRpcRequest::ScanKeys {
                prefix: prefix.to_string(),
                limit: Some(limit),
                continuation_token: None,
            },
            READ_TIMEOUT,
        )?;

        match response {
            ClientRpcResponse::ScanResult(ScanResultResponse { entries, error, .. }) => {
                if let Some(err) = error {
                    anyhow::bail!("scan error: {}", err);
                }
                Ok(entries.into_iter().map(|e| (e.key, e.value.into_bytes())).collect())
            }
            _ => anyhow::bail!("unexpected response type for ScanKeys"),
        }
    }

    /// Shutdown the client.
    #[allow(dead_code)]
    pub fn shutdown(self) {
        self.runtime.block_on(async {
            self.endpoint.close().await;
        });
    }
}

/// Create a thread-safe client wrapper.
pub type SharedClient = Arc<FuseSyncClient>;
