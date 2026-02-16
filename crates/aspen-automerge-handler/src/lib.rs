//! Automerge CRDT RPC handler for Aspen.
//!
//! This crate provides the Automerge handler extracted from
//! aspen-rpc-handlers for better modularity and faster incremental builds.
//!
//! Handles Automerge CRDT document operations:
//! - Create, Get, Save, Delete: Document lifecycle
//! - ApplyChanges, Merge: Change operations
//! - List, GetMetadata, Exists: Query operations
//! - GenerateSyncMessage, ReceiveSyncMessage: Sync operations

mod handler;

use std::sync::Arc;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::HandlerFactory;
pub use aspen_rpc_core::RequestHandler;
pub use handler::AutomergeHandler;

// =============================================================================
// Handler Factory (Plugin Registration)
// =============================================================================

/// Factory for creating `AutomergeHandler` instances.
///
/// This factory enables plugin-style registration via the `inventory` crate.
/// The handler is always created since it uses the KV store which is always available.
///
/// # Priority
///
/// Priority 590 (feature handler range: 500-599).
pub struct AutomergeHandlerFactory;

impl AutomergeHandlerFactory {
    /// Create a new factory instance.
    pub const fn new() -> Self {
        Self
    }
}

impl Default for AutomergeHandlerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl HandlerFactory for AutomergeHandlerFactory {
    fn create(&self, _ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
        Some(Arc::new(AutomergeHandler))
    }

    fn name(&self) -> &'static str {
        "AutomergeHandler"
    }

    fn priority(&self) -> u32 {
        590
    }
}

// Self-register via inventory
aspen_rpc_core::submit_handler_factory!(AutomergeHandlerFactory);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factory_name() {
        let factory = AutomergeHandlerFactory;
        assert_eq!(factory.name(), "AutomergeHandler");
    }

    #[test]
    fn test_factory_priority() {
        let factory = AutomergeHandlerFactory;
        assert_eq!(factory.priority(), 590);
    }
}
