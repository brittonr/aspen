//! Core RPC send infrastructure for the IrohClient.

use anyhow::Context;
use anyhow::Result;
use aspen_client::CLIENT_ALPN;
use aspen_client::ClientRpcRequest;
use aspen_client::ClientRpcResponse;
use aspen_client::MAX_CLIENT_MESSAGE_SIZE;
use tracing::debug;
use tracing::warn;

use super::IrohClient;
use super::MAX_RETRIES;
use super::RETRY_DELAY;
use super::RPC_TIMEOUT;

impl IrohClient {
    /// Send an RPC request to the target node.
    pub(super) async fn send_rpc(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        let target_addr = self.target_addr.read().await.clone();

        debug!(
            target_node_id = %target_addr.id,
            request_type = ?request,
            "sending TUI RPC request"
        );

        // Connect to the target node
        let connection = self.endpoint.connect(target_addr, CLIENT_ALPN).await.context("failed to connect to node")?;

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

        // Mark as connected on successful RPC
        let mut connected = self.connected.write().await;
        *connected = true;

        Ok(response)
    }

    /// Send an RPC request with automatic retry on failure.
    pub(super) async fn send_rpc_with_retry(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        let mut retries = 0;

        loop {
            match tokio::time::timeout(RPC_TIMEOUT, self.send_rpc(request.clone())).await {
                Ok(Ok(response)) => return Ok(response),
                Ok(Err(err)) => {
                    warn!(
                        error = %err,
                        retries,
                        max_retries = MAX_RETRIES,
                        "RPC request failed"
                    );

                    if retries >= MAX_RETRIES {
                        return Err(err).context("max retries exceeded");
                    }

                    retries += 1;
                    tokio::time::sleep(RETRY_DELAY).await;
                }
                Err(_) => {
                    warn!(retries, max_retries = MAX_RETRIES, "RPC request timed out");

                    if retries >= MAX_RETRIES {
                        anyhow::bail!("RPC request timed out after {} retries", MAX_RETRIES);
                    }

                    retries += 1;
                    tokio::time::sleep(RETRY_DELAY).await;
                }
            }
        }
    }
}
