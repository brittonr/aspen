//! Iroh client for connecting to Aspen nodes over QUIC.
//!
//! This module provides the client-side implementation for connecting to
//! Aspen nodes using Iroh's peer-to-peer transport with the Client ALPN.

mod ci;
mod cluster;
mod jobs;
mod kv;
mod multi_node;
mod rpc;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use aspen_client::AspenClusterTicket;
use aspen_client::CLIENT_ALPN;
use iroh::Endpoint;
use iroh::EndpointAddr;
use iroh::SecretKey;
pub use multi_node::MultiNodeClient;
pub use multi_node::NodeConnection;
use tokio::sync::RwLock;
use tracing::info;

/// Timeout for individual RPC calls.
/// Reduced for TUI to keep UI responsive during network issues.
const RPC_TIMEOUT: Duration = Duration::from_secs(2); // Reduced from 10s

/// Connection retry delay.
/// Shorter delay for TUI to fail fast and keep UI responsive.
const RETRY_DELAY: Duration = Duration::from_millis(500); // Reduced from 5s

/// Maximum connection retries before giving up.
/// Balance between responsiveness and reliability on lossy networks.
const MAX_RETRIES: u32 = 3;

/// Iroh client for TUI connections to Aspen nodes.
pub struct IrohClient {
    /// Iroh endpoint for making connections.
    endpoint: Endpoint,
    /// Target node address.
    target_addr: Arc<RwLock<EndpointAddr>>,
    /// Connection status.
    connected: Arc<RwLock<bool>>,
}

impl IrohClient {
    /// Create a new Iroh client for TUI connections.
    ///
    /// # Arguments
    /// * `target_addr` - The endpoint address of the target node
    ///
    /// # Returns
    /// A new IrohClient instance.
    pub async fn new(target_addr: EndpointAddr) -> Result<Self> {
        // Generate a new secret key for the TUI client
        use rand::RngCore;
        let mut key_bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut key_bytes);
        let secret_key = SecretKey::from(key_bytes);

        // Build the Iroh endpoint with TUI ALPN
        let endpoint = Endpoint::builder()
            .secret_key(secret_key)
            .alpns(vec![CLIENT_ALPN.to_vec()])
            .bind()
            .await
            .context("failed to bind Iroh endpoint")?;

        info!(
            node_id = %endpoint.id(),
            target_node_id = %target_addr.id,
            "TUI client endpoint created"
        );

        Ok(Self {
            endpoint,
            target_addr: Arc::new(RwLock::new(target_addr)),
            connected: Arc::new(RwLock::new(false)),
        })
    }

    /// Update the target node address.
    pub async fn set_target(&self, addr: EndpointAddr) {
        let mut target = self.target_addr.write().await;
        *target = addr;

        // Mark as disconnected so next request will reconnect
        let mut connected = self.connected.write().await;
        *connected = false;
    }

    /// Check if the client is connected.
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    /// Shutdown the client and close all connections.
    pub async fn shutdown(self) -> Result<()> {
        self.endpoint.close().await;
        Ok(())
    }
}

/// Parse an Aspen cluster ticket into a list of EndpointAddrs.
///
/// Tickets have the format: "aspen{base32-encoded-data}"
/// Returns all bootstrap peers in the ticket with their direct socket addresses.
pub fn parse_cluster_ticket(ticket: &str) -> Result<Vec<EndpointAddr>> {
    let ticket = AspenClusterTicket::deserialize(ticket)?;

    // Extract all bootstrap peer endpoint addresses (includes direct socket addrs)
    let endpoints = ticket.endpoint_addrs();

    if endpoints.is_empty() {
        anyhow::bail!("no bootstrap peers in ticket");
    }

    Ok(endpoints)
}
