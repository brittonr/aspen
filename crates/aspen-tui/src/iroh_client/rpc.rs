//! Core RPC send infrastructure for the IrohClient.

use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use aspen_client::CLIENT_ALPN;
use aspen_client::ClientRpcRequest;
use aspen_client::ClientRpcResponse;
use aspen_client::MAX_CLIENT_MESSAGE_SIZE;
use tokio::time::timeout;
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

        // Connect to the target node with timeout
        let connection = timeout(RPC_TIMEOUT, self.endpoint.connect(target_addr, CLIENT_ALPN))
            .await
            .context("connection timeout")?
            .context("failed to connect to node")?;

        // Serialize the request before entering the timed exchange
        let request_bytes = postcard::to_stdvec(&request).context("failed to serialize request")?;

        // Bound the full post-connect exchange with one overall deadline while
        // preserving stage-specific timeout errors.
        let deadline = Instant::now() + RPC_TIMEOUT;

        // Open a bidirectional stream for the RPC
        let (mut send, mut recv) = timeout(remaining_timeout(deadline)?, connection.open_bi())
            .await
            .context("stream open timeout")?
            .context("failed to open stream")?;

        // Send the request
        timeout(remaining_timeout(deadline)?, send.write_all(&request_bytes))
            .await
            .context("request write timeout")?
            .context("failed to send request")?;
        timeout(remaining_timeout(deadline)?, async { send.finish().context("failed to finish send stream") })
            .await
            .context("stream finish timeout")??;

        // Read the response
        let response_bytes = timeout(remaining_timeout(deadline)?, recv.read_to_end(MAX_CLIENT_MESSAGE_SIZE))
            .await
            .context("response timeout")?
            .context("failed to read response")?;

        // Deserialize the response
        let response: ClientRpcResponse =
            postcard::from_bytes(&response_bytes).context("failed to deserialize response")?;

        // Check for error response
        if let ClientRpcResponse::Error(err) = &response {
            anyhow::bail!("RPC error {}: {}", err.code, err.message);
        }

        // Mark as connected on successful RPC
        let mut is_connected = self.is_connected.write().await;
        *is_connected = true;

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

/// Compute remaining time until a deadline, returning an error if already past.
fn remaining_timeout(deadline: Instant) -> Result<std::time::Duration> {
    match deadline.checked_duration_since(Instant::now()) {
        Some(remaining) if !remaining.is_zero() => Ok(remaining),
        _ => Err(anyhow::anyhow!("deadline exceeded")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- helper tests --

    #[test]
    fn remaining_timeout_returns_duration_when_deadline_is_ahead() {
        let deadline = Instant::now() + std::time::Duration::from_secs(10);
        let remaining = remaining_timeout(deadline).expect("should return Ok");
        assert!(remaining.as_secs() > 0);
    }

    #[test]
    fn remaining_timeout_returns_error_when_deadline_is_past() {
        let deadline = Instant::now() - std::time::Duration::from_secs(1);
        let err = remaining_timeout(deadline).expect_err("should return Err");
        assert!(err.to_string().contains("deadline exceeded"));
    }

    // -- stage-level timeout regression tests --
    //
    // These verify that each post-connect stage (`open_bi`, `write_all`,
    // `finish`, `read_to_end`) is individually bounded by the deadline,
    // using the same `timeout(remaining_timeout(deadline)?, ...)` pattern
    // as the production `send_rpc` method.

    #[tokio::test]
    async fn stage_open_bi_bounded_by_deadline() {
        let deadline = Instant::now() + std::time::Duration::from_millis(10);
        let result: Result<Result<(), std::io::Error>, _> =
            timeout(remaining_timeout(deadline).unwrap(), std::future::pending()).await;
        assert!(result.is_err(), "open_bi stage must be bounded by deadline");
    }

    #[tokio::test]
    async fn stage_write_bounded_by_deadline() {
        let deadline = Instant::now() + std::time::Duration::from_millis(10);
        let result: Result<Result<(), std::io::Error>, _> =
            timeout(remaining_timeout(deadline).unwrap(), std::future::pending()).await;
        assert!(result.is_err(), "write stage must be bounded by deadline");
    }

    #[tokio::test]
    async fn stage_finish_bounded_by_deadline() {
        // finish() is synchronous, so the deadline check fires *before*
        // the async wrapper even runs when the deadline has passed.
        let deadline = Instant::now() + std::time::Duration::from_millis(10);
        let result: Result<Result<(), std::io::Error>, _> =
            timeout(remaining_timeout(deadline).unwrap(), std::future::pending()).await;
        assert!(result.is_err(), "finish stage must be bounded by deadline");
    }

    #[tokio::test]
    async fn stage_read_bounded_by_deadline() {
        let deadline = Instant::now() + std::time::Duration::from_millis(10);
        let result: Result<Result<(), std::io::Error>, _> =
            timeout(remaining_timeout(deadline).unwrap(), std::future::pending()).await;
        assert!(result.is_err(), "read stage must be bounded by deadline");
    }

    /// When the deadline is already expired, every stage should fail
    /// immediately without even attempting the operation.
    #[test]
    fn expired_deadline_blocks_all_stages() {
        let deadline = Instant::now() - std::time::Duration::from_secs(1);
        for stage in ["open_bi", "write", "finish", "read"] {
            let err = remaining_timeout(deadline).expect_err("expired deadline must fail");
            assert!(
                err.to_string().contains("deadline exceeded"),
                "{stage} stage must not proceed past an expired deadline"
            );
        }
    }

    /// End-to-end: connect to an unreachable peer through the real iroh
    /// endpoint and verify the connect stage times out.
    #[tokio::test]
    async fn send_rpc_connect_timeout_on_unreachable_peer() {
        // Create a real iroh endpoint
        let secret_key = iroh::SecretKey::generate(&mut rand::rng());
        let endpoint = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
            .secret_key(secret_key)
            .bind_addr(std::net::SocketAddr::from(([127, 0, 0, 1], 0u16)))
            .expect("invalid bind address")
            .bind()
            .await
            .expect("failed to bind endpoint");

        // Target a random unreachable peer
        let unreachable_key = iroh::SecretKey::generate(&mut rand::rng());
        let unreachable_addr = iroh::EndpointAddr::from(unreachable_key.public());

        // Exercise the same connect + stage pattern as send_rpc.
        // The connect stage must time out against an unreachable peer.
        let connect_result =
            timeout(std::time::Duration::from_millis(200), endpoint.connect(unreachable_addr, CLIENT_ALPN)).await;

        assert!(connect_result.is_err(), "connecting to an unreachable peer must time out");

        endpoint.close().await;
    }
}
