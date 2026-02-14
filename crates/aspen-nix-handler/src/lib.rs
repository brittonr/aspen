//! Nix binary cache and SNIX RPC handlers for Aspen.
//!
//! This crate consolidates two closely related Nix handler crates:
//! - **SNIX**: DirectoryService and PathInfoService get/put operations for ephemeral CI workers to
//!   upload build artifacts
//! - **Cache**: Cache queries, statistics, download ticket generation, and migration management for
//!   the distributed Nix binary cache

#[cfg(feature = "cache")]
mod cache;
#[cfg(feature = "cache")]
mod migration;
#[cfg(feature = "snix")]
mod snix;

use std::sync::Arc;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::HandlerFactory;
pub use aspen_rpc_core::RequestHandler;
#[cfg(feature = "cache")]
pub use cache::CacheHandler;
#[cfg(feature = "cache")]
pub use migration::CacheMigrationHandler;
#[cfg(feature = "snix")]
pub use snix::SnixHandler;

// =============================================================================
// Handler Factories (Plugin Registration)
// =============================================================================

#[cfg(feature = "snix")]
mod snix_factory {
    use super::*;

    /// Factory for creating `SnixHandler` instances.
    pub struct SnixHandlerFactory;

    impl SnixHandlerFactory {
        pub const fn new() -> Self {
            Self
        }
    }

    impl Default for SnixHandlerFactory {
        fn default() -> Self {
            Self::new()
        }
    }

    impl HandlerFactory for SnixHandlerFactory {
        fn create(&self, _ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
            Some(Arc::new(SnixHandler))
        }

        fn name(&self) -> &'static str {
            "SnixHandler"
        }

        fn priority(&self) -> u32 {
            800
        }
    }

    aspen_rpc_core::submit_handler_factory!(SnixHandlerFactory);
}

#[cfg(feature = "cache")]
mod cache_factory {
    use super::*;

    /// Factory for creating `CacheHandler` instances.
    pub struct CacheHandlerFactory;

    impl CacheHandlerFactory {
        pub const fn new() -> Self {
            Self
        }
    }

    impl Default for CacheHandlerFactory {
        fn default() -> Self {
            Self::new()
        }
    }

    impl HandlerFactory for CacheHandlerFactory {
        fn create(&self, _ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
            Some(Arc::new(CacheHandler))
        }

        fn name(&self) -> &'static str {
            "CacheHandler"
        }

        fn priority(&self) -> u32 {
            810
        }
    }

    /// Factory for creating `CacheMigrationHandler` instances.
    pub struct CacheMigrationHandlerFactory;

    impl CacheMigrationHandlerFactory {
        pub const fn new() -> Self {
            Self
        }
    }

    impl Default for CacheMigrationHandlerFactory {
        fn default() -> Self {
            Self::new()
        }
    }

    impl HandlerFactory for CacheMigrationHandlerFactory {
        fn create(&self, _ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
            Some(Arc::new(CacheMigrationHandler))
        }

        fn name(&self) -> &'static str {
            "CacheMigrationHandler"
        }

        fn priority(&self) -> u32 {
            811
        }
    }

    aspen_rpc_core::submit_handler_factory!(CacheHandlerFactory);
    aspen_rpc_core::submit_handler_factory!(CacheMigrationHandlerFactory);
}
