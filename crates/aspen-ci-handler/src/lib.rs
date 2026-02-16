//! CI/CD pipeline RPC handler for Aspen distributed cluster.
//!
//! This crate implements the RPC handler for CI/CD operations, providing:
//! - Pipeline triggering and execution
//! - Pipeline status queries
//! - Pipeline run listing and filtering
//! - Repository watching for automatic CI triggers
//! - Artifact listing and retrieval
//! - Job log fetching and subscription
//! - Job output retrieval
//!
//! # Architecture
//!
//! The handler implements the `RequestHandler` trait from `aspen-rpc-core`,
//! allowing it to be registered with the `HandlerRegistry` in `aspen-rpc-handlers`.
//!
//! ```text
//! aspen-rpc-handlers
//!        |
//!        v
//!   HandlerRegistry
//!        |
//!        v
//!   CiHandler (this crate)
//!        |
//!        +---> Pipeline Operations (trigger, status, list, cancel)
//!        +---> Watch Operations (watch/unwatch repo)
//!        +---> Artifact Operations (list, get)
//!        +---> Log Operations (get logs, subscribe, get output)
//! ```

mod handler;

use std::sync::Arc;

// Re-export types that handlers need from aspen-rpc-core
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::HandlerFactory;
pub use aspen_rpc_core::RequestHandler;
pub use handler::CiHandler;

// =============================================================================
// Handler Factory (Plugin Registration)
// =============================================================================

/// Factory for creating `CiHandler` instances.
///
/// This factory enables plugin-style registration via the `inventory` crate.
/// The handler is only created if CI services (orchestrator or trigger) are
/// available in the context.
///
/// # Priority
///
/// Priority 600 (feature handler range: 500-699).
pub struct CiHandlerFactory;

impl CiHandlerFactory {
    /// Create a new factory instance.
    pub const fn new() -> Self {
        Self
    }
}

impl Default for CiHandlerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl HandlerFactory for CiHandlerFactory {
    fn create(&self, ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
        // Only create handler if CI orchestrator or trigger service is configured
        let has_orchestrator = ctx.ci_orchestrator.is_some();
        let has_trigger = ctx.ci_trigger_service.is_some();
        if has_orchestrator || has_trigger {
            Some(Arc::new(CiHandler))
        } else {
            None
        }
    }

    fn name(&self) -> &'static str {
        "CiHandler"
    }

    fn priority(&self) -> u32 {
        600
    }
}

// Self-register via inventory
aspen_rpc_core::submit_handler_factory!(CiHandlerFactory);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factory_name() {
        let factory = CiHandlerFactory;
        assert_eq!(factory.name(), "CiHandler");
    }

    #[test]
    fn test_factory_priority() {
        let factory = CiHandlerFactory;
        assert_eq!(factory.priority(), 600);
    }
}
