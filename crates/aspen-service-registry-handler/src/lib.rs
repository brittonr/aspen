//! Service registry RPC handler for Aspen.
//!
//! This crate provides the ServiceRegistryHandler extracted from aspen-rpc-handlers
//! for better modularity and faster incremental builds.
//!
//! Handles service registry operations:
//! - ServiceRegister: Register a service instance
//! - ServiceDeregister: Deregister a service instance
//! - ServiceDiscover: Discover service instances with filters
//! - ServiceList: List registered services
//! - ServiceGetInstance: Get details of a specific instance
//! - ServiceHeartbeat: Send heartbeat for an instance
//! - ServiceUpdateHealth: Update instance health status
//! - ServiceUpdateMetadata: Update instance metadata

mod handler;

use std::sync::Arc;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::HandlerFactory;
pub use aspen_rpc_core::RequestHandler;
pub use handler::ServiceRegistryHandler;

// =============================================================================
// Handler Factory (Plugin Registration)
// =============================================================================

/// Factory for creating `ServiceRegistryHandler` instances.
///
/// Priority 300 (infrastructure handler range: 300-399).
pub struct ServiceRegistryHandlerFactory;

impl ServiceRegistryHandlerFactory {
    /// Create a new factory instance.
    pub const fn new() -> Self {
        Self
    }
}

impl Default for ServiceRegistryHandlerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl HandlerFactory for ServiceRegistryHandlerFactory {
    fn create(&self, _ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
        Some(Arc::new(ServiceRegistryHandler))
    }

    fn name(&self) -> &'static str {
        "ServiceRegistryHandler"
    }

    fn priority(&self) -> u32 {
        300
    }
}

// Self-register via inventory
aspen_rpc_core::submit_handler_factory!(ServiceRegistryHandlerFactory);
