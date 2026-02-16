//! Coordination primitives RPC handler for Aspen.
//!
//! This crate provides the coordination handler extracted from
//! aspen-rpc-handlers for better modularity and faster incremental builds.
//!
//! Handles distributed coordination primitives:
//! - Locks: Distributed mutual exclusion
//! - Counters: Atomic counters (unsigned and signed)
//! - Sequences: Monotonic ID generation
//! - Rate limiters: Token bucket rate limiting
//! - Barriers: Multi-party synchronization
//! - Semaphores: Counting semaphores
//! - RWLocks: Reader-writer locks
//! - Queues: Distributed message queues

mod handler;

use std::sync::Arc;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::HandlerFactory;
pub use aspen_rpc_core::RequestHandler;
pub use handler::CoordinationHandler;

// =============================================================================
// Handler Factory (Plugin Registration)
// =============================================================================

/// Factory for creating `CoordinationHandler` instances.
///
/// Priority 220 (essential handler range: 200-299).
pub struct CoordinationHandlerFactory;

impl CoordinationHandlerFactory {
    /// Create a new factory instance.
    pub const fn new() -> Self {
        Self
    }
}

impl Default for CoordinationHandlerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl HandlerFactory for CoordinationHandlerFactory {
    fn create(&self, _ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
        Some(Arc::new(CoordinationHandler))
    }

    fn name(&self) -> &'static str {
        "CoordinationHandler"
    }

    fn priority(&self) -> u32 {
        220
    }
}

// Self-register via inventory
aspen_rpc_core::submit_handler_factory!(CoordinationHandlerFactory);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factory_name() {
        let factory = CoordinationHandlerFactory;
        assert_eq!(factory.name(), "CoordinationHandler");
    }

    #[test]
    fn test_factory_priority() {
        let factory = CoordinationHandlerFactory;
        assert_eq!(factory.priority(), 220);
    }
}
