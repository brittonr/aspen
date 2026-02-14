//! Core RPC handler for Aspen.
//!
//! This crate provides the CoreHandler extracted from aspen-rpc-handlers
//! for better modularity and faster incremental builds.
//!
//! Handles basic operations:
//! - Ping: Simple health check
//! - GetHealth: Detailed health status
//! - GetRaftMetrics: Raft consensus metrics
//! - GetNodeInfo: Node endpoint information
//! - GetLeader: Current cluster leader
//! - GetMetrics: Prometheus-format metrics
//! - CheckpointWal: WAL checkpoint request (deprecated)
//! - ListVaults: Vault listing (deprecated)
//! - GetVaultKeys: Vault keys (deprecated)

mod handler;

use std::sync::Arc;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::HandlerFactory;
pub use aspen_rpc_core::RequestHandler;
pub use handler::CoreHandler;

// =============================================================================
// Handler Factory (Plugin Registration)
// =============================================================================

/// Factory for creating `CoreHandler` instances.
///
/// Priority 100 (core handler range: 100-199).
pub struct CoreHandlerFactory;

impl CoreHandlerFactory {
    /// Create a new factory instance.
    pub const fn new() -> Self {
        Self
    }
}

impl Default for CoreHandlerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl HandlerFactory for CoreHandlerFactory {
    fn create(&self, _ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
        Some(Arc::new(CoreHandler))
    }

    fn name(&self) -> &'static str {
        "CoreHandler"
    }

    fn priority(&self) -> u32 {
        100
    }
}

// Self-register via inventory
aspen_rpc_core::submit_handler_factory!(CoreHandlerFactory);
