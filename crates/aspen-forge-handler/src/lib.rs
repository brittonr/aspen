//! Forge (decentralized Git) RPC handler for Aspen distributed cluster.
//!
//! This crate implements the RPC handler for Forge operations, providing:
//! - Repository management (create, get, list)
//! - Git object operations (blob, tree, commit)
//! - Ref management (branches, tags)
//! - Issue and patch tracking (CRDT-based)
//! - Federation (cross-cluster sync)
//! - Git Bridge (git-remote-aspen interop)
//!
//! # Architecture
//!
//! The handler implements the `RequestHandler` trait from `aspen-rpc-core`,
//! allowing it to be registered with the `HandlerRegistry` in `aspen-rpc-handlers`.
//!
//! ## Plugin Registration
//!
//! This handler supports self-registration via `HandlerFactory`:
//!
//! ```ignore
//! // In your crate, enable self-registration:
//! aspen_forge_handler::register_forge_handler_factory();
//! ```
//!
//! Or use the `submit_handler_factory!` macro for automatic registration at link time.
//!
//! ```text
//! aspen-rpc-handlers
//!        │
//!        ▼
//!   HandlerRegistry
//!        │
//!        ├──► collect_handler_factories() <── inventory
//!        │
//!        ▼
//!   ForgeHandler (this crate)
//!        │
//!        ├──► Repository Operations
//!        ├──► Git Object Operations
//!        ├──► Ref Operations
//!        ├──► Issue Operations
//!        ├──► Patch Operations
//!        ├──► Federation Operations
//!        └──► Git Bridge Operations
//! ```

mod handler;

use std::sync::Arc;

// Re-export types that handlers need from aspen-rpc-core
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::HandlerFactory;
pub use aspen_rpc_core::RequestHandler;
pub use handler::ForgeHandler;

// =============================================================================
// Handler Factory (Plugin Registration)
// =============================================================================

/// Factory for creating `ForgeHandler` instances.
///
/// This factory enables plugin-style registration via the `inventory` crate.
/// The handler is only created if the `forge_node` is available in the context.
///
/// # Priority
///
/// Priority 540 (feature handler range: 500-599).
pub struct ForgeHandlerFactory;

impl ForgeHandlerFactory {
    /// Create a new factory instance.
    pub const fn new() -> Self {
        Self
    }
}

impl Default for ForgeHandlerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl HandlerFactory for ForgeHandlerFactory {
    fn create(&self, ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
        // Only create handler if forge node is configured
        if ctx.forge_node.is_some() {
            Some(Arc::new(ForgeHandler))
        } else {
            None
        }
    }

    fn name(&self) -> &'static str {
        "ForgeHandler"
    }

    fn priority(&self) -> u32 {
        540
    }

    fn app_id(&self) -> Option<&'static str> {
        Some("forge")
    }
}

// Self-register via inventory
aspen_rpc_core::submit_handler_factory!(ForgeHandlerFactory);
