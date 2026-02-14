//! Key-Value RPC handler for Aspen.
//!
//! This crate provides the KV handler extracted from aspen-rpc-handlers
//! for better modularity and faster incremental builds.
//!
//! Handles key-value operations:
//! - ReadKey: Read a single key
//! - WriteKey: Write a single key-value pair
//! - DeleteKey: Delete a key
//! - ScanKeys: Scan keys with a prefix
//! - BatchRead: Read multiple keys atomically
//! - BatchWrite: Write multiple key-value pairs atomically
//! - ConditionalBatchWrite: Conditional batch writes with preconditions
//! - CompareAndSwapKey: Atomic compare-and-swap operation
//! - CompareAndDeleteKey: Atomic compare-and-delete operation

mod error_sanitization;
mod handler;
mod verified;

use std::sync::Arc;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::HandlerFactory;
pub use aspen_rpc_core::RequestHandler;
pub use handler::KvHandler;

// =============================================================================
// Handler Factory (Plugin Registration)
// =============================================================================

/// Factory for creating `KvHandler` instances.
///
/// Priority 110 (core handler range: 100-199).
pub struct KvHandlerFactory;

impl KvHandlerFactory {
    /// Create a new factory instance.
    pub const fn new() -> Self {
        Self
    }
}

impl Default for KvHandlerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl HandlerFactory for KvHandlerFactory {
    fn create(&self, _ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
        Some(Arc::new(KvHandler))
    }

    fn name(&self) -> &'static str {
        "KvHandler"
    }

    fn priority(&self) -> u32 {
        110
    }
}

// Self-register via inventory
aspen_rpc_core::submit_handler_factory!(KvHandlerFactory);
