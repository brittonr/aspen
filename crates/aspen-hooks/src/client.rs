//! Lightweight client library for triggering Aspen hooks from external programs.
//!
//! This module provides a minimal, easy-to-use client for triggering hooks on Aspen
//! clusters using hook trigger URLs. It requires no knowledge of the Aspen internals
//! or Iroh P2P networking.
//!
//! # Quick Start
//!
//! ```ignore
//! use aspen_hooks::client::HookClient;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Create client from a hook URL
//!     let url = "aspenhook7g2wc...";
//!     let client = HookClient::from_url(url)?;
//!
//!     // Trigger with default payload
//!     let result = client.trigger().await?;
//!     println!("Dispatched to {} handlers", result.dispatched_count);
//!
//!     // Or trigger with custom payload
//!     let result = client.trigger_with_payload(r#"{"key": "value"}"#).await?;
//!     Ok(())
//! }
//! ```
//!
//! # Features
//!
//! - **Zero-config**: Just provide a hook URL
//! - **Automatic peer rotation**: Tries multiple bootstrap peers for reliability
//! - **Connection pooling**: Reuses connections when possible
//! - **Async/await**: Built on Tokio for high-performance async I/O
//!
//! # Tiger Style
//!
//! - Bounded retries and timeouts
//! - Explicit error handling
//! - Fail-fast on invalid inputs

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use aspen_client_api::CLIENT_ALPN;
use aspen_cluster_types::NodeAddress;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::MAX_CLIENT_MESSAGE_SIZE;
use iroh::Endpoint;
use iroh::endpoint::VarInt;
use tokio::sync::OnceCell;
use tokio::time::timeout;

use crate::AspenHookTicket;

/// Default RPC timeout (5 seconds).
pub const DEFAULT_TIMEOUT_MS: u64 = 5000;

/// Maximum number of connection retries per peer.
const MAX_RETRIES_PER_PEER: u32 = 2;

/// Delay between retry attempts.
const RETRY_DELAY_MS: u64 = 500;

/// Result of a hook trigger operation.
#[derive(Debug, Clone)]
pub struct TriggerResult {
    /// Whether all handlers executed successfully.
    pub is_success: bool,
    /// Number of handlers that received the event.
    pub dispatched_count: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
    /// List of handler failures (handler_name, error_message).
    pub handler_failures: Vec<(String, String)>,
}

/// Error types for the hook client.
#[derive(Debug, thiserror::Error)]
pub enum HookClientError {
    /// Failed to parse the hook URL.
    #[error("invalid hook URL: {0}")]
    InvalidUrl(String),

    /// The hook URL has expired.
    #[error("hook URL has expired")]
    Expired,

    /// Failed to connect to any cluster node.
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    /// The trigger operation failed.
    #[error("trigger failed: {0}")]
    TriggerFailed(String),

    /// Invalid payload JSON.
    #[error("invalid payload JSON: {0}")]
    InvalidPayload(String),

    /// Network operation timed out.
    #[error("operation timed out")]
    Timeout,
}

/// Lightweight client for triggering Aspen hooks.
///
/// The client lazily initializes the Iroh endpoint on first use, minimizing
/// startup overhead for one-shot triggers.
///
/// # Example
///
/// ```ignore
/// let client = HookClient::from_url("aspenhook...")?;
///
/// // Trigger with default payload
/// client.trigger().await?;
///
/// // Trigger with custom payload
/// client.trigger_with_payload(r#"{"custom": "data"}"#).await?;
/// ```
pub struct HookClient {
    ticket: AspenHookTicket,
    endpoint: OnceCell<Arc<Endpoint>>,
    timeout: Duration,
}

impl HookClient {
    /// Create a new client from a hook trigger URL.
    ///
    /// The URL should be in the format: `aspenhook{base32-payload}`
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The URL is malformed or invalid
    /// - The URL has expired
    ///
    /// # Example
    ///
    /// ```ignore
    /// let client = HookClient::from_url("aspenhook7g2wc...")?;
    /// ```
    pub fn from_url(url: &str) -> Result<Self, HookClientError> {
        let ticket = match AspenHookTicket::deserialize(url) {
            Ok(ticket) => ticket,
            Err(crate::HookTicketError::ExpiredTicket { .. }) => return Err(HookClientError::Expired),
            Err(error) => return Err(HookClientError::InvalidUrl(error.to_string())),
        };

        Ok(Self {
            ticket,
            endpoint: OnceCell::new(),
            timeout: Duration::from_millis(DEFAULT_TIMEOUT_MS),
        })
    }

    /// Create a client with a custom timeout.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let client = HookClient::from_url("aspenhook...")?
    ///     .with_timeout(Duration::from_secs(10));
    /// ```
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Trigger the hook with the default payload.
    ///
    /// Uses the default payload embedded in the hook URL, or `{}` if none.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let result = client.trigger().await?;
    /// if result.is_success {
    ///     println!("Hook dispatched to {} handlers", result.dispatched_count);
    /// }
    /// ```
    pub async fn trigger(&self) -> Result<TriggerResult, HookClientError> {
        let payload = self.ticket.default_payload.clone().unwrap_or_else(|| "{}".to_string());

        self.trigger_with_payload(&payload).await
    }

    /// Trigger the hook with a custom payload.
    ///
    /// The payload must be valid JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The payload is not valid JSON
    /// - Connection to the cluster fails
    /// - The trigger operation fails
    ///
    /// # Example
    ///
    /// ```ignore
    /// let result = client.trigger_with_payload(r#"{"key": "value"}"#).await?;
    /// ```
    pub async fn trigger_with_payload(&self, payload: &str) -> Result<TriggerResult, HookClientError> {
        // Validate JSON
        let _: serde_json::Value =
            serde_json::from_str(payload).map_err(|e| HookClientError::InvalidPayload(e.to_string()))?;

        // Get or create endpoint
        let endpoint = self.get_or_create_endpoint().await?;

        // Try each bootstrap peer
        let mut last_error = None;
        for peer_addr in &self.ticket.bootstrap_peers {
            let runtime_peer_addr = match convert_bootstrap_peer(peer_addr) {
                Ok(peer_addr) => peer_addr,
                Err(error) => {
                    tracing::debug!(
                        endpoint_id = %peer_addr.endpoint_id(),
                        error = %error,
                        "invalid hook bootstrap peer"
                    );
                    last_error = Some(error);
                    continue;
                }
            };

            for attempt in 0..MAX_RETRIES_PER_PEER {
                if attempt > 0 {
                    tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                }

                match self.send_trigger(&endpoint, &runtime_peer_addr, payload).await {
                    Ok(result) => return Ok(result),
                    Err(e) => {
                        tracing::debug!(
                            peer = ?runtime_peer_addr,
                            attempt,
                            error = %e,
                            "trigger attempt failed"
                        );
                        last_error = Some(e);
                    }
                }
            }
        }

        Err(last_error.unwrap_or(HookClientError::ConnectionFailed("no bootstrap peers available".to_string())))
    }

    /// Get the cluster ID from the hook URL.
    pub fn cluster_id(&self) -> &str {
        &self.ticket.cluster_id
    }

    /// Get the event type that will be triggered.
    pub fn event_type(&self) -> &str {
        &self.ticket.event_type
    }

    /// Check if the hook URL requires authentication.
    pub fn requires_auth(&self) -> bool {
        self.ticket.requires_auth()
    }

    /// Get the expiration status.
    pub fn expiry_string(&self) -> String {
        self.ticket.expiry_string()
    }

    /// Lazily create the Iroh endpoint.
    async fn get_or_create_endpoint(&self) -> Result<Arc<Endpoint>, HookClientError> {
        self.endpoint
            .get_or_try_init(|| async {
                let secret_key = iroh::SecretKey::generate(&mut rand::rng());
                let endpoint = Endpoint::builder(iroh::endpoint::presets::N0)
                    .secret_key(secret_key)
                    .alpns(vec![CLIENT_ALPN.to_vec()])
                    .bind()
                    .await
                    .map_err(|e| HookClientError::ConnectionFailed(e.to_string()))?;

                Ok(Arc::new(endpoint))
            })
            .await
            .cloned()
    }

    /// Send a trigger request to a specific peer.
    async fn send_trigger(
        &self,
        endpoint: &Endpoint,
        peer_addr: &iroh::EndpointAddr,
        payload: &str,
    ) -> Result<TriggerResult, HookClientError> {
        // Connect to the peer
        let connection = timeout(self.timeout, async {
            endpoint.connect(peer_addr.clone(), CLIENT_ALPN).await.context("failed to connect")
        })
        .await
        .map_err(|_| HookClientError::Timeout)?
        .map_err(|e| HookClientError::ConnectionFailed(e.to_string()))?;

        // Build and serialize the request before entering the timed exchange
        let request = ClientRpcRequest::HookTrigger {
            event_type: self.ticket.event_type.clone(),
            payload_json: payload.to_string(),
        };
        let request_bytes = postcard::to_stdvec(&request).map_err(|e| HookClientError::TriggerFailed(e.to_string()))?;

        // Bound the full post-connect exchange with one overall deadline while
        // preserving stage-specific timeout errors.
        let deadline = std::time::Instant::now() + self.timeout;

        // Open bidirectional stream
        let (mut send, mut recv) = timeout(remaining_timeout_hook(deadline)?, connection.open_bi())
            .await
            .map_err(|_| HookClientError::Timeout)?
            .map_err(|e| HookClientError::ConnectionFailed(e.to_string()))?;

        // Send request
        timeout(remaining_timeout_hook(deadline)?, send.write_all(&request_bytes))
            .await
            .map_err(|_| HookClientError::Timeout)?
            .map_err(|e| HookClientError::ConnectionFailed(e.to_string()))?;
        timeout(remaining_timeout_hook(deadline)?, async {
            send.finish().map_err(|e| HookClientError::ConnectionFailed(e.to_string()))
        })
        .await
        .map_err(|_| HookClientError::Timeout)??;

        // Read response with deadline
        let response_bytes = timeout(remaining_timeout_hook(deadline)?, recv.read_to_end(MAX_CLIENT_MESSAGE_SIZE))
            .await
            .map_err(|_| HookClientError::Timeout)?
            .map_err(|e| HookClientError::ConnectionFailed(e.to_string()))?;

        // Deserialize response
        let response: ClientRpcResponse =
            postcard::from_bytes(&response_bytes).map_err(|e| HookClientError::TriggerFailed(e.to_string()))?;

        // Close connection gracefully
        connection.close(VarInt::from_u32(0), b"done");

        // Handle response
        match response {
            ClientRpcResponse::HookTriggerResult(result) => Ok(TriggerResult {
                is_success: result.is_success,
                dispatched_count: result.dispatched_count,
                error: result.error,
                handler_failures: result.handler_failures,
            }),
            ClientRpcResponse::Error(e) => Err(HookClientError::TriggerFailed(format!("{}: {}", e.code, e.message))),
            _ => Err(HookClientError::TriggerFailed("unexpected response type".to_string())),
        }
    }
}

/// Convenience function to trigger a hook in a single call.
///
/// This is useful for one-shot triggers where you don't need to reuse the client.
///
/// # Example
///
/// ```ignore
/// let result = aspen_hooks::client::trigger("aspenhook...", None).await?;
/// ```
pub async fn trigger(url: &str, payload: Option<&str>) -> Result<TriggerResult, HookClientError> {
    let client = HookClient::from_url(url)?;
    match payload {
        Some(p) => client.trigger_with_payload(p).await,
        None => client.trigger().await,
    }
}

/// Compute remaining time until a deadline, returning a `HookClientError` if already past.
fn remaining_timeout_hook(deadline: std::time::Instant) -> Result<std::time::Duration, HookClientError> {
    match deadline.checked_duration_since(std::time::Instant::now()) {
        Some(remaining) if !remaining.is_zero() => Ok(remaining),
        _ => Err(HookClientError::Timeout),
    }
}

fn convert_bootstrap_peer(peer_addr: &NodeAddress) -> Result<iroh::EndpointAddr, HookClientError> {
    peer_addr.try_into_iroh().map_err(|error| {
        HookClientError::ConnectionFailed(format!("invalid hook bootstrap peer {}: {error}", peer_addr.endpoint_id()))
    })
}

#[cfg(test)]
mod tests {
    use aspen_cluster_types::NodeTransportAddr;
    use core::net::SocketAddr;
    use iroh_tickets::Ticket;
    use serde::Deserialize;
    use serde::Serialize;

    use super::*;

    const HOOK_TICKET_PREFIX: &str = "aspenhook";
    const TICKET_VERSION: u8 = 1;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct LegacyAspenHookTicket {
        version: u8,
        cluster_id: String,
        bootstrap_peers: Vec<iroh::EndpointAddr>,
        event_type: String,
        default_payload: Option<String>,
        auth_token: Option<[u8; 32]>,
        expires_at_secs: u64,
        relay_url: Option<String>,
        priority: u8,
    }

    impl Ticket for LegacyAspenHookTicket {
        const KIND: &'static str = HOOK_TICKET_PREFIX;

        fn to_bytes(&self) -> Vec<u8> {
            postcard::to_allocvec(self).expect("legacy hook ticket serialization should succeed in tests")
        }

        fn from_bytes(bytes: &[u8]) -> core::result::Result<Self, iroh_tickets::ParseError> {
            postcard::from_bytes(bytes).map_err(Into::into)
        }
    }

    fn valid_test_node_address(seed: u8) -> NodeAddress {
        let secret_key = iroh::SecretKey::from([seed; 32]);
        let endpoint_id = secret_key.public();
        let endpoint_addr = iroh::EndpointAddr::from(endpoint_id);
        NodeAddress::new(endpoint_addr)
    }

    #[test]
    fn test_invalid_url() {
        let result = HookClient::from_url("invalid");
        assert!(matches!(result, Err(HookClientError::InvalidUrl(_))));
    }

    #[test]
    fn test_invalid_prefix() {
        let result = HookClient::from_url("aspen7g2wc...");
        assert!(matches!(result, Err(HookClientError::InvalidUrl(_))));
    }

    #[test]
    fn test_expired_url_maps_to_expired_error() {
        let ticket = AspenHookTicket::new("cluster", vec![valid_test_node_address(1)])
            .with_event_type("write_committed")
            .with_expiry(1);
        let serialized = ticket.serialize();
        let result = HookClient::from_url(&serialized);
        assert!(matches!(result, Err(HookClientError::Expired)));
    }

    #[test]
    fn test_convert_bootstrap_peer_rejects_invalid_node_address() {
        let invalid_addr = NodeAddress::from_parts(
            "not-an-endpoint-id",
            [NodeTransportAddr::Ip(SocketAddr::from(([127, 0, 0, 1], 7777)))],
        );
        let result = convert_bootstrap_peer(&invalid_addr);
        assert!(matches!(result, Err(HookClientError::ConnectionFailed(_))));
    }

    #[test]
    fn test_convert_bootstrap_peer_accepts_valid_node_address() {
        let valid_addr = valid_test_node_address(2);
        let result = convert_bootstrap_peer(&valid_addr).expect("valid node address should convert");
        assert_eq!(result.id.to_string(), valid_addr.endpoint_id());
    }

    #[test]
    fn test_legacy_url_surfaces_decode_failure() {
        let secret_key = iroh::SecretKey::from([9u8; 32]);
        let legacy_ticket = LegacyAspenHookTicket {
            version: TICKET_VERSION,
            cluster_id: "legacy-cluster".to_string(),
            bootstrap_peers: vec![iroh::EndpointAddr::from(secret_key.public())],
            event_type: "write_committed".to_string(),
            default_payload: None,
            auth_token: None,
            expires_at_secs: 0,
            relay_url: None,
            priority: 0,
        };

        let serialized = Ticket::serialize(&legacy_ticket);
        let result = HookClient::from_url(&serialized);
        assert!(matches!(result, Err(HookClientError::InvalidUrl(_))));
    }
}
