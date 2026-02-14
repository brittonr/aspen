//! SQL and DNS query RPC handlers for Aspen.
//!
//! This crate consolidates two query/lookup handler crates:
//! - **SQL**: ExecuteSql query operations (feature-gated behind `sql`)
//! - **DNS**: DNS record and zone management (feature-gated behind `dns`)

#[cfg(feature = "dns")]
mod dns;
#[cfg(feature = "sql")]
mod sql;

use std::sync::Arc;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::HandlerFactory;
pub use aspen_rpc_core::RequestHandler;
#[cfg(feature = "dns")]
pub use dns::DnsHandler;
#[cfg(feature = "sql")]
pub use sql::SqlHandler;

// =============================================================================
// Handler Factories (Plugin Registration)
// =============================================================================

#[cfg(feature = "sql")]
mod sql_factory {
    use super::*;

    /// Factory for creating `SqlHandler` instances.
    ///
    /// Priority 500 (feature handler range: 500-599).
    pub struct SqlHandlerFactory;

    impl SqlHandlerFactory {
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
            Some(Arc::new(SqlHandler))
        }

        fn name(&self) -> &'static str {
            "SqlHandler"
        }

        fn priority(&self) -> u32 {
            500
        }
    }

    aspen_rpc_core::submit_handler_factory!(SqlHandlerFactory);
}

#[cfg(feature = "dns")]
mod dns_factory {
    use super::*;

    /// Factory for creating `DnsHandler` instances.
    ///
    /// Priority 510 (feature handler range: 500-599).
    pub struct DnsHandlerFactory;

    impl DnsHandlerFactory {
        pub const fn new() -> Self {
            Self
        }
    }

    impl Default for DnsHandlerFactory {
        fn default() -> Self {
            Self::new()
        }
    }

    impl HandlerFactory for DnsHandlerFactory {
        fn create(&self, _ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
            Some(Arc::new(DnsHandler))
        }

        fn name(&self) -> &'static str {
            "DnsHandler"
        }

        fn priority(&self) -> u32 {
            510
        }
    }

    aspen_rpc_core::submit_handler_factory!(DnsHandlerFactory);
}
