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
//!        ├──► explicit factory list
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

mod executor;
pub(crate) mod handler;

use std::sync::Arc;

// Re-export types that handlers need from aspen-rpc-core
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::HandlerFactory;
pub use aspen_rpc_core::RequestHandler;
pub use aspen_rpc_core::ServiceHandler;
pub use executor::ForgeServiceExecutor;
use executor::ForgeServiceExecutorDeps;

// =============================================================================
// Handler Factory (Plugin Registration)
// =============================================================================

/// Factory for creating `ForgeHandler` instances.
///
/// The handler is only created if the `forge_node` capability is available.
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
    fn create(&self, ctx: &ClientProtocolContext) -> anyhow::Result<Arc<dyn RequestHandler>> {
        let caps = ctx.forge_handler_context()?;
        let executor = Arc::new(ForgeServiceExecutor::new(caps.forge_node, ForgeServiceExecutorDeps {
            #[cfg(feature = "global-discovery")]
            content_discovery: caps.content_discovery,
            #[cfg(feature = "global-discovery")]
            federation_discovery: caps.federation_discovery,
            federation_identity: caps.federation_identity,
            federation_trust_manager: caps.federation_trust_manager,
            federation_cluster_identity: caps.federation_cluster_identity,
            iroh_endpoint: caps.iroh_endpoint,
            #[cfg(all(feature = "hooks", feature = "git-bridge"))]
            hook_service: caps.hook_service,
            node_id: caps.node_id,
        }));
        Ok(Arc::new(ServiceHandler::new(executor)))
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
