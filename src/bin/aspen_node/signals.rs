//! Shutdown signal handling for aspen-node.
//!
//! Handles SIGINT (Ctrl-C) and SIGTERM signals for graceful shutdown
//! in production (systemd sends SIGTERM) and development (Ctrl-C sends SIGINT).

use tokio::signal;
use tracing::error;
use tracing::info;

/// Wait for shutdown signal (SIGINT or SIGTERM).
///
/// Tiger Style: Handles both signals for graceful shutdown in production
/// (systemd sends SIGTERM) and development (Ctrl-C sends SIGINT).
pub async fn shutdown_signal() {
    let ctrl_c = async {
        match signal::ctrl_c().await {
            Ok(()) => {}
            Err(err) => error!("failed to install Ctrl+C handler: {}", err),
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(err) => error!("failed to install SIGTERM handler: {}", err),
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("received SIGINT, initiating graceful shutdown");
        }
        _ = terminate => {
            info!("received SIGTERM, initiating graceful shutdown");
        }
    }
}
