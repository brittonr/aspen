//! Server lifecycle coordination
//!
//! Manages graceful shutdown of multiple server listeners (localhost + P2P).

use anyhow::Result;

use crate::server::iroh::Listener;
use crate::server::localhost::LocalhostHandle;

/// Handle to running server infrastructure
///
/// Coordinates both localhost HTTP and P2P iroh+h3 listeners,
/// ensuring proper shutdown sequencing and error propagation.
pub struct ServerHandle {
    localhost: LocalhostHandle,
    p2p: Listener,
}

impl ServerHandle {
    /// Create a new server handle
    pub(crate) fn new(localhost: LocalhostHandle, p2p: Listener) -> Self {
        Self { localhost, p2p }
    }

    /// Run the P2P listener (blocking)
    ///
    /// This runs the main P2P server loop. The localhost listener
    /// is already running in the background.
    ///
    /// When this returns (either success or error), the localhost listener
    /// is automatically shut down to ensure clean exit.
    pub async fn run(self) -> Result<()> {
        // Run P2P listener (blocks until done)
        let p2p_result = self.p2p.run().await;

        // When P2P exits (error or shutdown), stop localhost listener
        tracing::info!("P2P listener exited, shutting down localhost listener");
        if let Err(e) = self.localhost.shutdown().await {
            tracing::error!(error = %e, "Failed to shut down localhost listener");
        }

        // Return P2P result
        p2p_result
    }

    /// Graceful shutdown (future work)
    ///
    /// Currently h3_iroh doesn't support graceful shutdown,
    /// so this is a placeholder for future implementation.
    ///
    /// When h3_iroh adds shutdown support, this method will:
    /// 1. Signal P2P listener to stop accepting new connections
    /// 2. Wait for in-flight requests to complete
    /// 3. Shut down localhost listener
    /// 4. Return once both are stopped
    #[allow(dead_code)]
    pub async fn shutdown(self) -> Result<()> {
        tracing::info!("Initiating graceful server shutdown");

        // Shut down localhost
        self.localhost.shutdown().await?;

        // TODO: Shut down P2P when h3_iroh supports it
        // For now, this is incomplete
        tracing::warn!("P2P listener shutdown not yet implemented (h3_iroh limitation)");

        Ok(())
    }

    /// Check if localhost listener is running
    pub fn localhost_running(&self) -> bool {
        self.localhost.is_running()
    }

    /// Get the P2P base URL
    pub fn p2p_base_url(&self) -> String {
        self.p2p.base_url()
    }
}
