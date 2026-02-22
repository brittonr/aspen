//! Forge (decentralized Git) RPC handler for Aspen distributed cluster.
//!
//! This crate implements the **native-only** portion of the Forge handler.
//! Repos, objects, refs, issues, and patches have been migrated to
//! `aspen-forge-plugin` (WASM). This handler retains only operations
//! that require `ForgeNode` context access:
//!
//! - Federation (cross-cluster sync, delegate keys) — 9 ops
//! - Git Bridge (git-remote-aspen interop) — 6 ops
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
//!   ForgeHandler (this crate, priority 540)
//!        │
//!        ├──► Federation Operations (9)
//!        └──► Git Bridge Operations (6)
//!
//!   ForgePlugin (WASM, priority 950)
//!        │
//!        ├──► Repository Operations (3)
//!        ├──► Git Object Operations (7)
//!        ├──► Ref Operations (7)
//!        ├──► Issue Operations (6)
//!        └──► Patch Operations (7)
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
