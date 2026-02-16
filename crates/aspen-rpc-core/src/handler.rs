//! Request handler trait and factory for domain-specific RPC handlers.
//!
//! This module defines the core `RequestHandler` trait that all domain-specific
//! handlers must implement, and the `HandlerFactory` trait for self-registration
//! using the inventory pattern.
//!
//! # Handler Plugin Architecture
//!
//! Handlers can self-register using the `inventory` crate, enabling a plugin-style
//! architecture where new handlers don't require modification of the central registry.
//!
//! ## Example: Self-Registering Handler
//!
//! ```ignore
//! use aspen_rpc_core::{HandlerFactory, RequestHandler, ClientProtocolContext};
//! use aspen_rpc_core::submit_handler_factory;
//! use std::sync::Arc;
//!
//! pub struct MyHandler;
//!
//! #[async_trait]
//! impl RequestHandler for MyHandler {
//!     fn can_handle(&self, request: &ClientRpcRequest) -> bool { /* ... */ }
//!     async fn handle(&self, request: ClientRpcRequest, ctx: &ClientProtocolContext) -> anyhow::Result<ClientRpcResponse> { /* ... */ }
//!     fn name(&self) -> &'static str { "MyHandler" }
//! }
//!
//! pub struct MyHandlerFactory;
//!
//! impl HandlerFactory for MyHandlerFactory {
//!     fn create(&self, ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
//!         // Return None if preconditions not met (e.g., optional service unavailable)
//!         Some(Arc::new(MyHandler))
//!     }
//!     fn name(&self) -> &'static str { "MyHandler" }
//!     fn priority(&self) -> u32 { 500 } // Default priority
//! }
//!
//! // Self-register at startup
//! submit_handler_factory!(MyHandlerFactory);
//! ```

use std::sync::Arc;

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use async_trait::async_trait;

use crate::ClientProtocolContext;

/// Trait for domain-specific request handlers.
///
/// Each handler is responsible for a subset of `ClientRpcRequest` variants.
/// The `can_handle` method determines if a handler can process a given request,
/// and `handle` processes the request and returns a response.
///
/// # Tiger Style
///
/// - Handlers must not hold state across requests
/// - All state comes from `ClientProtocolContext`
/// - Errors are sanitized before returning to client
///
/// # Implementing a Handler
///
/// ```ignore
/// use aspen_rpc_core::{RequestHandler, ClientProtocolContext};
/// use aspen_client_api::{ClientRpcRequest, ClientRpcResponse};
/// use async_trait::async_trait;
///
/// pub struct MyHandler;
///
/// #[async_trait]
/// impl RequestHandler for MyHandler {
///     fn can_handle(&self, request: &ClientRpcRequest) -> bool {
///         matches!(request, ClientRpcRequest::MyOperation { .. })
///     }
///
///     async fn handle(
///         &self,
///         request: ClientRpcRequest,
///         ctx: &ClientProtocolContext,
///     ) -> anyhow::Result<ClientRpcResponse> {
///         match request {
///             ClientRpcRequest::MyOperation { arg } => {
///                 // Process request using ctx
///                 Ok(ClientRpcResponse::Success)
///             }
///             _ => anyhow::bail!("unexpected request"),
///         }
///     }
///
///     fn name(&self) -> &'static str {
///         "MyHandler"
///     }
/// }
/// ```
#[async_trait]
pub trait RequestHandler: Send + Sync {
    /// Returns true if this handler can process the given request.
    ///
    /// This method should be fast and not perform any I/O. It's used
    /// by the handler registry to dispatch requests to the correct handler.
    fn can_handle(&self, request: &ClientRpcRequest) -> bool;

    /// Process the request and return a response.
    ///
    /// # Arguments
    ///
    /// * `request` - The request to process (ownership transferred)
    /// * `ctx` - The protocol context with all dependencies
    ///
    /// # Errors
    ///
    /// Returns an error if the request cannot be processed. The error will be
    /// sanitized before being sent to the client to avoid leaking internal
    /// implementation details.
    async fn handle(&self, request: ClientRpcRequest, ctx: &ClientProtocolContext) -> Result<ClientRpcResponse>;

    /// Returns the handler name for logging/debugging.
    ///
    /// This should be a short, descriptive name like "ForgeHandler" or "BlobHandler".
    fn name(&self) -> &'static str;
}

// =============================================================================
// Handler Factory (Plugin Registration)
// =============================================================================

/// Factory trait for creating handlers with runtime context.
///
/// This trait enables a plugin architecture where handlers can self-register
/// using the `inventory` crate. Each handler crate implements this trait and
/// calls `submit_handler_factory!` to register at link time.
///
/// # Priority System
///
/// Handlers are tried in priority order (lowest first). Default priority is 500.
/// Core handlers (KV, Cluster) use lower priorities (100-200) to ensure they
/// are checked first. Optional/extension handlers use higher priorities.
///
/// # Conditional Registration
///
/// The `create` method returns `Option<Arc<dyn RequestHandler>>`. Return `None`
/// when preconditions are not met (e.g., required service not configured).
/// This allows feature-gated handlers to skip registration cleanly.
///
/// # Tiger Style
///
/// - Factories are stateless and cheap to create
/// - `create()` performs all validation before returning a handler
/// - Priority is fixed at compile time for deterministic ordering
pub trait HandlerFactory: Send + Sync + 'static {
    /// Create a handler instance if preconditions are met.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The protocol context with all dependencies
    ///
    /// # Returns
    ///
    /// - `Some(handler)` if the handler should be registered
    /// - `None` if preconditions not met (e.g., required service unavailable)
    fn create(&self, ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>>;

    /// Returns the factory/handler name for logging.
    fn name(&self) -> &'static str;

    /// Priority for handler dispatch order (lower = checked first).
    ///
    /// Default: 500
    ///
    /// ## Priority Ranges
    ///
    /// - 100-199: Core handlers
    ///   - 100: CoreHandler (health, metrics, node info)
    ///   - 110: KvHandler (key-value operations)
    ///   - 120: ClusterHandler (cluster management)
    /// - 200-299: Essential handlers
    ///   - 200: LeaseHandler (TTL leases)
    ///   - 210: WatchHandler (key change notifications)
    ///   - 220: CoordinationHandler (locks, counters, queues, etc.)
    /// - 300-399: Infrastructure
    ///   - 300: ServiceRegistryHandler (service discovery)
    /// - 400-499: Reserved
    /// - 500-599: Feature handlers
    ///   - 500: SqlHandler (SQL queries)
    ///   - 510: DnsHandler (DNS records)
    ///   - 520: BlobHandler (blob storage)
    ///   - 530: DocsHandler (document sync)
    ///   - 540: ForgeHandler (Git hosting)
    ///   - 550: PijulHandler (Pijul VCS)
    ///   - 560: JobHandler (job queue)
    ///   - 570: HooksHandler (event hooks)
    ///   - 580: SecretsHandler (secrets engine)
    ///   - 590: AutomergeHandler (CRDT documents)
    /// - 600-699: Worker/CI handlers
    ///   - 600: CiHandler (CI/CD pipelines)
    ///   - 640: WorkerHandler (job worker coordination)
    /// - 800+: Extension handlers
    ///   - 800: SnixHandler (Nix store)
    ///   - 810: CacheHandler (binary cache)
    ///   - 811: CacheMigrationHandler (cache migration)
    fn priority(&self) -> u32 {
        500
    }
}

// Inventory collection for handler factories
inventory::collect!(&'static dyn HandlerFactory);

/// Collect all registered handler factories.
///
/// Returns a vector of references to all factories registered via
/// `submit_handler_factory!`. Factories are returned in registration order;
/// callers should sort by priority before use.
pub fn collect_handler_factories() -> Vec<&'static dyn HandlerFactory> {
    inventory::iter::<&'static dyn HandlerFactory>.into_iter().copied().collect()
}

/// Macro to submit a handler factory for inventory collection.
///
/// This macro creates a static instance of the factory and registers it
/// with the inventory system. The factory will be discovered at link time
/// and included in the handler registry.
///
/// # Example
///
/// ```ignore
/// use aspen_rpc_core::submit_handler_factory;
///
/// pub struct MyHandlerFactory;
/// impl HandlerFactory for MyHandlerFactory { /* ... */ }
///
/// submit_handler_factory!(MyHandlerFactory);
/// ```
#[macro_export]
macro_rules! submit_handler_factory {
    ($factory:ty) => {
        $crate::inventory::submit! {
            &<$factory>::new() as &'static dyn $crate::HandlerFactory
        }
    };
    // Variant for factories that don't have new()
    ($factory:ty, $instance:expr) => {
        $crate::inventory::submit! {
            &$instance as &'static dyn $crate::HandlerFactory
        }
    };
}
