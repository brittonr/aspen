//! Iroh+H3 P2P listener for distributed communication
//!
//! This listener serves HTTP/3 over QUIC (iroh transport), enabling:
//! - Remote mvm-ci instances connecting to this node
//! - Blob storage synchronization
//! - Gossip protocol coordination
//! - Cryptographically authenticated P2P networking

use anyhow::Result;
use axum::Router;
use iroh::Endpoint;
use iroh_tickets::endpoint::EndpointTicket;

/// P2P listener configuration and state
///
/// Wraps an iroh endpoint and axum router for serving HTTP/3 over P2P transport.
pub struct Listener {
    endpoint: Endpoint,
    router: Router,
}

impl Listener {
    /// Create a new P2P listener (doesn't start yet)
    ///
    /// # Arguments
    /// * `endpoint` - Iroh endpoint (already online)
    /// * `router` - Axum router with application routes
    pub fn new(endpoint: Endpoint, router: Router) -> Self {
        Self { endpoint, router }
    }

    /// Get endpoint ticket for P2P connections
    ///
    /// This ticket can be shared with remote instances to establish connections.
    pub fn ticket(&self) -> EndpointTicket {
        EndpointTicket::new(self.endpoint.addr())
    }

    /// Get base URL for this endpoint
    ///
    /// Returns the iroh+h3:// URL that remote clients can use to connect.
    pub fn base_url(&self) -> String {
        format!("iroh+h3://{}/", self.ticket())
    }

    /// Get endpoint ID (node identity)
    pub fn endpoint_id(&self) -> iroh::EndpointId {
        self.endpoint.id()
    }

    /// Run the P2P listener (blocking until shutdown or error)
    ///
    /// This is the main server loop for P2P communication.
    /// It blocks the current task until:
    /// - A fatal error occurs (relay connection lost)
    /// - Graceful shutdown is triggered (future work - h3_iroh doesn't support yet)
    pub async fn run(self) -> Result<()> {
        tracing::info!(
            endpoint_id = %self.endpoint.id(),
            base_url = %self.base_url(),
            "Starting HTTP/3 server over iroh for P2P"
        );

        println!("Starting HTTP/3 server over iroh for P2P...");
        println!("  - Remote instances can connect via P2P");
        println!("  - Blob storage and gossip coordination");

        match h3_iroh::axum::serve(self.endpoint, self.router).await {
            Ok(_) => {
                tracing::info!("HTTP/3 over iroh server shut down gracefully");
                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "HTTP/3 over iroh server failed - relay connection may be lost"
                );
                Err(anyhow::anyhow!("P2P server error: {}", e))
            }
        }
    }
}

/// Wait for endpoint to be online (connected to relay)
///
/// Blocks until the iroh endpoint has established relay connections
/// and is ready to accept P2P connections.
///
/// Adds a timeout to prevent indefinite hanging if relay servers are unreachable.
pub async fn wait_for_online(endpoint: &Endpoint) -> Result<()> {
    println!("Waiting for endpoint to be online...");
    tracing::info!("Waiting for iroh endpoint to be online...");

    // Add a timeout to prevent hanging forever if relay is unreachable
    let online_future = endpoint.online();
    match tokio::time::timeout(std::time::Duration::from_secs(10), online_future).await {
        Ok(_) => {
            tracing::info!("Endpoint is online");
            Ok(())
        }
        Err(_) => {
            tracing::warn!("Timeout waiting for endpoint to come online - continuing anyway");
            println!("Warning: Could not connect to iroh relay servers (timeout)");
            println!("Continuing without relay connection - P2P features may be limited");
            // For tests, we can continue without relay connection
            // The endpoint will still work for local connections
            Ok(())
        }
    }
}

/// Print connection information for operators
///
/// Displays the endpoint ticket and connection URLs in a human-friendly format.
pub fn print_connection_info(endpoint: &Endpoint) {
    let ticket = EndpointTicket::new(endpoint.addr());
    let base_url = format!("iroh+h3://{}", ticket);

    println!("==================================================");
    println!("Iroh Endpoint Ticket:");
    println!("  {}/", base_url);
    println!("==================================================");
    println!("Remote instances and workers can connect using this P2P URL");
    println!();
}
