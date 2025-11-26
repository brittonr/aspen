//! Localhost HTTP listener for local communication
//!
//! This listener serves HTTP/1.1 over TCP on localhost, enabling:
//! - WASM workflows to connect (via flawless-http)
//! - Web UI access from browser
//! - Local debugging and development

use anyhow::Result;
use axum::Router;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

/// Handle to a running localhost listener
///
/// Provides control over the background server task, including graceful shutdown.
pub struct LocalhostHandle {
    /// Task handle for the server
    task: JoinHandle<Result<()>>,
    /// Shutdown signal sender
    shutdown_tx: Option<oneshot::Sender<()>>,
    /// Bound address (for tests and logging)
    pub addr: String,
}

impl LocalhostHandle {
    /// Request graceful shutdown
    ///
    /// Sends shutdown signal and waits for the server to exit cleanly.
    pub async fn shutdown(mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        self.task.await??;
        Ok(())
    }

    /// Check if server is still running
    pub fn is_running(&self) -> bool {
        !self.task.is_finished()
    }
}

/// Spawn localhost HTTP listener in background
///
/// Creates a background tokio task serving HTTP/1.1 on the specified address.
/// The server runs independently and can be gracefully shut down via the returned handle.
///
/// # Arguments
/// * `bind_addr` - IP address to bind (e.g., "0.0.0.0" or "127.0.0.1")
/// * `port` - TCP port number
/// * `router` - Axum router with application routes
///
/// # Returns
/// A handle to control the background server task
pub async fn spawn(bind_addr: String, port: u16, router: Router) -> Result<LocalhostHandle> {
    let addr = format!("{}:{}", bind_addr, port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to bind {}: {}", addr, e))?;

    let bound_addr = listener.local_addr()?.to_string();
    tracing::info!("Localhost HTTP listener bound to {}", bound_addr);
    println!("🚀 HTTP server listening on http://{}", bound_addr);

    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let task = tokio::spawn(async move {
        tracing::info!("Starting localhost HTTP server on {}", addr);

        let result = axum::serve(listener, router)
            .with_graceful_shutdown(async {
                shutdown_rx.await.ok();
            })
            .await;

        match result {
            Ok(_) => {
                tracing::info!("Localhost HTTP server shut down gracefully");
                Ok(())
            }
            Err(e) => {
                tracing::error!(error = %e, "Localhost HTTP server failed");
                Err(anyhow::anyhow!("Localhost server error: {}", e))
            }
        }
    });

    println!("Local HTTP server: http://{}:{}", bind_addr, port);
    println!("  - WASM workflows can connect here");
    println!("  - Web UI accessible in browser");

    Ok(LocalhostHandle {
        task,
        shutdown_tx: Some(shutdown_tx),
        addr: bound_addr,
    })
}
