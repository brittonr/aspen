//! Watch RPC handler for Aspen.
//!
//! This crate provides the WatchHandler extracted from aspen-rpc-handlers
//! for better modularity and faster incremental builds.
//!
//! Handles watch operations:
//! - WatchCreate: Create a key prefix watch (requires streaming protocol)
//! - WatchCancel: Cancel an active watch (requires streaming protocol)
//! - WatchStatus: Query active watches (works via RPC)
//!
//! Note: Actual watch functionality requires LOG_SUBSCRIBER_ALPN streaming.
//! This handler provides informative errors for WatchCreate/WatchCancel
//! and implements WatchStatus when a WatchRegistry is configured.

mod handler;

use std::sync::Arc;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::HandlerFactory;
pub use aspen_rpc_core::RequestHandler;
pub use handler::WatchHandler;

// =============================================================================
// Handler Factory (Plugin Registration)
// =============================================================================

/// Factory for creating `WatchHandler` instances.
///
/// Priority 210 (essential handler range: 200-299).
pub struct WatchHandlerFactory;

impl WatchHandlerFactory {
    /// Create a new factory instance.
    pub const fn new() -> Self {
        Self
    }
}

impl Default for WatchHandlerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl HandlerFactory for WatchHandlerFactory {
    fn create(&self, _ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
        Some(Arc::new(WatchHandler))
    }

    fn name(&self) -> &'static str {
        "WatchHandler"
    }

    fn priority(&self) -> u32 {
        210
    }
}

// Self-register via inventory
aspen_rpc_core::submit_handler_factory!(WatchHandlerFactory);
