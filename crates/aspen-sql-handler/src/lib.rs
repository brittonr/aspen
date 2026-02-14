//! SQL RPC handler for Aspen.
//!
//! This crate provides the SQL handler extracted from
//! aspen-rpc-handlers for better modularity and faster incremental builds.
//!
//! Handles SQL query operations via ExecuteSql request.
//!
//! # Plugin Registration
//!
//! This handler supports self-registration via `HandlerFactory`:
//!
//! ```ignore
//! // In your crate, use the factory for plugin-style registration:
//! use aspen_sql_handler::SqlHandlerFactory;
//! ```

mod handler;

use std::sync::Arc;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::HandlerFactory;
pub use aspen_rpc_core::RequestHandler;
pub use handler::SqlHandler;

// =============================================================================
// Handler Factory (Plugin Registration)
// =============================================================================

/// Factory for creating `SqlHandler` instances.
///
/// This factory enables plugin-style registration via the `inventory` crate.
/// The SQL handler is always created when the sql feature is enabled.
///
/// # Priority
///
/// Priority 500 (feature handler range: 500-599).
pub struct SqlHandlerFactory;

impl SqlHandlerFactory {
    /// Create a new factory instance.
    pub const fn new() -> Self {
        Self
    }
}

impl Default for SqlHandlerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl HandlerFactory for SqlHandlerFactory {
    fn create(&self, _ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
        // SQL handler is always available when the feature is enabled
        Some(Arc::new(SqlHandler))
    }

    fn name(&self) -> &'static str {
        "SqlHandler"
    }

    fn priority(&self) -> u32 {
        500
    }
}

// Self-register via inventory
aspen_rpc_core::submit_handler_factory!(SqlHandlerFactory);
