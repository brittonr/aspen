//! SQL query RPC handler for Aspen.
//!
//! Provides the `ExecuteSql` handler (feature-gated behind `sql`).
//!
//! **Note:** DNS operations have been migrated to the `aspen-dns-plugin`
//! WASM plugin. The `dns` feature is retained for backward compatibility
//! but the handler factory is no longer registered.

#[cfg(feature = "sql")]
mod sql;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::HandlerFactory;
pub use aspen_rpc_core::RequestHandler;
#[cfg(feature = "sql")]
pub use sql::SqlHandler;

// =============================================================================
// Handler Factories (Plugin Registration)
// =============================================================================

#[cfg(feature = "sql")]
mod sql_factory {
    use std::sync::Arc;

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

        fn app_id(&self) -> Option<&'static str> {
            Some("sql")
        }
    }

    aspen_rpc_core::submit_handler_factory!(SqlHandlerFactory);
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "sql")]
    #[test]
    fn test_sql_factory_priority() {
        use super::HandlerFactory;
        let factory = super::sql_factory::SqlHandlerFactory;
        assert_eq!(factory.priority(), 500);
    }
}
