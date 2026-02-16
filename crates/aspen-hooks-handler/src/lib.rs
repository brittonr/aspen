//! Hooks RPC handler for Aspen.
//!
//! This crate provides the hooks handler extracted from
//! aspen-rpc-handlers for better modularity and faster incremental builds.
//!
//! Handles hook system operations:
//! - HookList: List configured handlers
//! - HookGetMetrics: Get execution metrics
//! - HookTrigger: Manually trigger events

mod handler;

use std::sync::Arc;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::HandlerFactory;
pub use aspen_rpc_core::RequestHandler;
pub use handler::HooksHandler;

// =============================================================================
// Handler Factory (Plugin Registration)
// =============================================================================

/// Factory for creating `HooksHandler` instances.
///
/// This factory enables plugin-style registration via the `inventory` crate.
/// The handler is always created since it handles unavailability gracefully internally.
///
/// # Priority
///
/// Priority 570 (feature handler range: 500-599).
pub struct HooksHandlerFactory;

impl HooksHandlerFactory {
    /// Create a new factory instance.
    pub const fn new() -> Self {
        Self
    }
}

impl Default for HooksHandlerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl HandlerFactory for HooksHandlerFactory {
    fn create(&self, _ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
        Some(Arc::new(HooksHandler))
    }

    fn name(&self) -> &'static str {
        "HooksHandler"
    }

    fn priority(&self) -> u32 {
        570
    }
}

// Self-register via inventory
aspen_rpc_core::submit_handler_factory!(HooksHandlerFactory);
