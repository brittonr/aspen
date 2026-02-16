//! Pijul VCS RPC handler for Aspen.
//!
//! This crate provides the Pijul handler extracted from
//! aspen-rpc-handlers for better modularity and faster incremental builds.
//!
//! Handles Pijul operations:
//! - Repository: Init, List, Info
//! - Channel: List, Create, Delete, Fork, Info
//! - Change: Apply, Unrecord, Log, Show, Blame

mod handler;

use std::sync::Arc;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::HandlerFactory;
pub use aspen_rpc_core::RequestHandler;
pub use handler::PijulHandler;

// =============================================================================
// Handler Factory (Plugin Registration)
// =============================================================================

/// Factory for creating `PijulHandler` instances.
///
/// This factory enables plugin-style registration via the `inventory` crate.
/// The handler is only created if the `pijul_store` is available in the context.
///
/// # Priority
///
/// Priority 550 (feature handler range: 500-599).
pub struct PijulHandlerFactory;

impl PijulHandlerFactory {
    /// Create a new factory instance.
    pub const fn new() -> Self {
        Self
    }
}

impl Default for PijulHandlerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl HandlerFactory for PijulHandlerFactory {
    fn create(&self, ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
        // Only create handler if pijul store is configured
        if ctx.pijul_store.is_some() {
            Some(Arc::new(PijulHandler))
        } else {
            None
        }
    }

    fn name(&self) -> &'static str {
        "PijulHandler"
    }

    fn priority(&self) -> u32 {
        550
    }
}

// Self-register via inventory
aspen_rpc_core::submit_handler_factory!(PijulHandlerFactory);
