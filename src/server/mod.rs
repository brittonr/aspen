//! Server lifecycle management
//!
//! This module provides a clean API for starting and stopping the dual-listener
//! server architecture (localhost HTTP + iroh+h3 P2P).
//!
//! # Architecture
//!
//! The application runs **two listeners simultaneously**:
//! - **Localhost HTTP** (`0.0.0.0:3020`) - For WASM workflows and web UI
//! - **Iroh+H3 P2P** - For distributed inter-instance communication
//!
//! Both listeners serve the same Axum routes, enabling transparent
//! communication across different transports.
//!
//! # Example
//!
//! ```no_run
//! use mvm_ci::server::{ServerConfig, start};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let config = ServerConfig {
//!     app_config: config,
//!     endpoint,
//!     state,
//! };
//!
//! let handle = start(config).await?;
//! handle.run().await?;
//! # Ok(())
//! # }
//! ```

use anyhow::Result;

mod iroh;
mod lifecycle;
mod localhost;
mod router;

pub use lifecycle::ServerHandle;

use crate::{config::AppConfig, state::AppState};

/// Server configuration bundle
///
/// Contains all dependencies needed to start the dual-listener server.
pub struct ServerConfig {
    /// Application configuration (network settings, paths, etc.)
    pub app_config: AppConfig,
    /// Iroh endpoint (pre-initialized and online)
    pub endpoint: ::iroh::Endpoint,
    /// Application state (repositories and services)
    pub state: AppState,
}

/// Start both server listeners (localhost + P2P)
///
/// This is the main entry point for starting the server. It:
/// 1. Creates the Axum router with all routes
/// 2. Spawns localhost HTTP listener in background
/// 3. Creates P2P listener (ready to run)
/// 4. Returns a handle coordinating both
///
/// # Arguments
/// * `config` - Server configuration with all dependencies
///
/// # Returns
/// A [`ServerHandle`] that can be used to run the P2P listener
/// and coordinate graceful shutdown.
///
/// # Example
/// ```no_run
/// # async fn example() -> anyhow::Result<()> {
/// let config = ServerConfig { /* ... */ };
/// let handle = server::start(config).await?;
/// handle.run().await?; // Blocks until shutdown
/// # Ok(())
/// # }
/// ```
pub async fn start(config: ServerConfig) -> Result<ServerHandle> {
    println!("Starting server...");

    // Print connection information for operators
    self::iroh::print_connection_info(&config.endpoint);

    // Build router with all application routes
    println!("Building router...");
    let router = router::build_router(&config.state);

    // Spawn localhost listener in background
    println!("Starting localhost HTTP listener on port {}...", config.app_config.network.http_port);
    let localhost_handle = localhost::spawn(
        config.app_config.network.http_bind_addr.clone(),
        config.app_config.network.http_port,
        router.clone(),
    )
    .await?;

    // Create P2P listener (doesn't start yet)
    let p2p_listener = self::iroh::Listener::new(config.endpoint, router);

    // Return handle that coordinates both
    Ok(ServerHandle::new(localhost_handle, p2p_listener))
}

// Re-export iroh utilities for main.rs to use
pub use self::iroh::wait_for_online;
