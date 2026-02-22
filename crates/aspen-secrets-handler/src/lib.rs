//! Secrets engine RPC handler for Aspen (native subset).
//!
//! Handles only operations that require native crypto libraries:
//! - PKI: Certificate authority with role-based issuance (rcgen/X.509)
//! - Nix Cache: Signing key management for Nix binary caches
//!
//! KV and Transit secrets operations have been migrated to the
//! `aspen-secrets-plugin` WASM plugin.

mod handler;

use std::sync::Arc;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::HandlerFactory;
pub use aspen_rpc_core::RequestHandler;
pub use handler::SecretsHandler;
pub use handler::SecretsService;

// =============================================================================
// Handler Factory (Plugin Registration)
// =============================================================================

/// Factory for creating `SecretsHandler` instances.
///
/// This factory enables plugin-style registration via the `inventory` crate.
/// The handler is only created if the `secrets_service` is available in the context.
///
/// # Priority
///
/// Priority 580 (feature handler range: 500-599).
pub struct SecretsHandlerFactory;

impl SecretsHandlerFactory {
    /// Create a new factory instance.
    pub const fn new() -> Self {
        Self
    }
}

impl Default for SecretsHandlerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl HandlerFactory for SecretsHandlerFactory {
    fn create(&self, ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
        // Only create handler if secrets service is configured
        if ctx.secrets_service.is_some() {
            Some(Arc::new(SecretsHandler))
        } else {
            None
        }
    }

    fn name(&self) -> &'static str {
        "SecretsHandler"
    }

    fn priority(&self) -> u32 {
        580
    }

    fn app_id(&self) -> Option<&'static str> {
        Some("secrets")
    }
}

// Self-register via inventory
aspen_rpc_core::submit_handler_factory!(SecretsHandlerFactory);
