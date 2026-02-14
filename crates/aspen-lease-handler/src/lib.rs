//! Lease RPC handler for Aspen.
//!
//! This crate provides the LeaseHandler extracted from aspen-rpc-handlers
//! for better modularity and faster incremental builds.
//!
//! Handles lease operations:
//! - LeaseGrant: Create a new lease with TTL
//! - LeaseRevoke: Revoke an existing lease
//! - LeaseKeepalive: Refresh a lease's TTL
//! - LeaseTimeToLive: Query remaining TTL
//! - LeaseList: List all active leases
//! - WriteKeyWithLease: Write a key attached to a lease

mod handler;

use std::sync::Arc;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::HandlerFactory;
pub use aspen_rpc_core::RequestHandler;
pub use handler::LeaseHandler;

// =============================================================================
// Handler Factory (Plugin Registration)
// =============================================================================

/// Factory for creating `LeaseHandler` instances.
///
/// Priority 200 (essential handler range: 200-299).
pub struct LeaseHandlerFactory;

impl LeaseHandlerFactory {
    /// Create a new factory instance.
    pub const fn new() -> Self {
        Self
    }
}

impl Default for LeaseHandlerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl HandlerFactory for LeaseHandlerFactory {
    fn create(&self, _ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
        Some(Arc::new(LeaseHandler))
    }

    fn name(&self) -> &'static str {
        "LeaseHandler"
    }

    fn priority(&self) -> u32 {
        200
    }
}

// Self-register via inventory
aspen_rpc_core::submit_handler_factory!(LeaseHandlerFactory);
