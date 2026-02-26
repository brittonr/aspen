//! Service executor trait for domain-specific request handling.
//!
//! Provides a typed interface for domain services (docs, jobs, KV, SQL, etc.)
//! to handle requests without implementing the full RequestHandler trait.
//! The `ServiceHandler` wrapper auto-generates the RequestHandler implementation.
//!
//! # Architecture
//!
//! ```text
//! ServiceExecutor (typed request → typed response)
//!     ↕ wrapped by
//! ServiceHandler<E> (implements RequestHandler)
//!     ↕ registered via
//! submit_handler_factory! (inventory collection)
//! ```
//!
//! # Difference from RequestHandler
//!
//! Unlike `RequestHandler`, executors don't receive `ClientProtocolContext` as
//! a parameter to their execute method. Instead, they capture their dependencies
//! at construction time (in the factory's `create()` method), making them
//! self-contained units of business logic.
//!
//! This pattern:
//! - Makes executors easier to test (no need to mock the entire context)
//! - Clarifies dependencies (explicit in constructor vs implicit in context)
//! - Enables better composition (executors can be nested/wrapped)
//!
//! # Example
//!
//! ```ignore
//! use aspen_rpc_core::{ServiceExecutor, ServiceHandler, HandlerFactory};
//! use std::sync::Arc;
//!
//! pub struct DocsServiceExecutor {
//!     docs_sync: Arc<dyn DocsSyncProvider>,
//! }
//!
//! impl DocsServiceExecutor {
//!     pub fn new(docs_sync: Arc<dyn DocsSyncProvider>) -> Self {
//!         Self { docs_sync }
//!     }
//! }
//!
//! #[async_trait]
//! impl ServiceExecutor for DocsServiceExecutor {
//!     fn service_name(&self) -> &'static str { "docs" }
//!     fn handles(&self) -> &'static [&'static str] { &["DocsSet", "DocsGet", "DocsDelete", "DocsList", "DocsStatus"] }
//!     fn priority(&self) -> u32 { 530 }
//!     fn app_id(&self) -> Option<&'static str> { Some("docs") }
//!
//!     async fn execute(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
//!         match request {
//!             ClientRpcRequest::DocsSet { key, value } => {
//!                 self.docs_sync.set(&key, &value).await?;
//!                 Ok(ClientRpcResponse::DocsSetResult(DocsSetResultResponse::default()))
//!             }
//!             ClientRpcRequest::DocsGet { key } => {
//!                 let value = self.docs_sync.get(&key).await?;
//!                 Ok(ClientRpcResponse::DocsGetResult(DocsGetResultResponse { value }))
//!             }
//!             _ => anyhow::bail!("unhandled request variant"),
//!         }
//!     }
//! }
//!
//! // Factory for creating the wrapped handler
//! pub struct DocsHandlerFactory;
//!
//! impl DocsHandlerFactory {
//!     pub const fn new() -> Self {
//!         Self
//!     }
//! }
//!
//! impl HandlerFactory for DocsHandlerFactory {
//!     fn create(&self, ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
//!         let docs_sync = ctx.docs_sync.as_ref()?.clone();
//!         let executor = Arc::new(DocsServiceExecutor::new(docs_sync));
//!         Some(Arc::new(ServiceHandler::new(executor)))
//!     }
//!
//!     fn name(&self) -> &'static str { "DocsHandler" }
//!     fn priority(&self) -> u32 { 530 }
//!     fn app_id(&self) -> Option<&'static str> { Some("docs") }
//! }
//!
//! submit_handler_factory!(DocsHandlerFactory);
//! ```

use std::sync::Arc;

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use async_trait::async_trait;

use crate::ClientProtocolContext;
use crate::handler::RequestHandler;

/// Typed executor for domain-specific request handling.
///
/// Implement this trait for each domain service (docs, jobs, KV, SQL, etc.).
/// The `ServiceHandler` wrapper converts this into a `RequestHandler` for
/// dispatch by the `HandlerRegistry`.
///
/// Unlike `RequestHandler`, executors don't receive `ClientProtocolContext` —
/// they capture their dependencies at construction time, making them
/// self-contained units of business logic.
///
/// # Example
///
/// ```ignore
/// use aspen_rpc_core::ServiceExecutor;
///
/// pub struct DocsServiceExecutor {
///     docs_sync: Arc<dyn DocsSyncProvider>,
/// }
///
/// #[async_trait]
/// impl ServiceExecutor for DocsServiceExecutor {
///     fn service_name(&self) -> &'static str { "docs" }
///     fn handles(&self) -> &'static [&'static str] { &["DocsSet", "DocsGet", ...] }
///     fn priority(&self) -> u32 { 530 }
///     fn app_id(&self) -> Option<&'static str> { Some("docs") }
///
///     async fn execute(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
///         match request {
///             ClientRpcRequest::DocsSet { key, value } => { ... }
///             _ => anyhow::bail!("unhandled"),
///         }
///     }
/// }
/// ```
#[async_trait]
pub trait ServiceExecutor: Send + Sync {
    /// Service name for logging/debugging (e.g., "docs", "jobs", "kv").
    fn service_name(&self) -> &'static str;

    /// List of ClientRpcRequest variant names this executor handles.
    ///
    /// These strings must match the variant names returned by
    /// `ClientRpcRequest::variant_name()` exactly (e.g., "DocsSet", "ForgeCreateRepo").
    fn handles(&self) -> &'static [&'static str];

    /// Priority for handler dispatch order (lower = checked first).
    ///
    /// See `HandlerFactory::priority()` documentation for priority ranges.
    fn priority(&self) -> u32;

    /// Optional app ID for federation capability advertisement.
    ///
    /// Return `Some("app-id")` for optional app handlers (docs, forge, ci, etc.).
    /// Return `None` for core handlers (kv, cluster, coordination).
    fn app_id(&self) -> Option<&'static str> {
        None
    }

    /// Execute a typed request and return a typed response.
    ///
    /// The executor should match on the request variants it handles
    /// (as declared in `handles()`) and return an appropriate response.
    ///
    /// # Errors
    ///
    /// Return an error if the request cannot be processed. The error
    /// will be sanitized before being sent to the client.
    async fn execute(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse>;
}

/// Generic handler that wraps a `ServiceExecutor` as a `RequestHandler`.
///
/// Provides the bridge between the typed `ServiceExecutor` interface and
/// the `RequestHandler` dispatch system used by `HandlerRegistry`.
///
/// The handler delegates `can_handle()` to the executor's `handles()` list
/// and `handle()` to the executor's `execute()` method.
///
/// # Example
///
/// ```ignore
/// use aspen_rpc_core::{ServiceHandler, ServiceExecutor};
///
/// let executor = Arc::new(DocsServiceExecutor::new(docs_sync));
/// let handler = ServiceHandler::new(executor);
/// // handler now implements RequestHandler and can be registered
/// ```
pub struct ServiceHandler {
    executor: Arc<dyn ServiceExecutor>,
    name: &'static str,
    handles: &'static [&'static str],
}

impl ServiceHandler {
    /// Create a new service handler wrapping the given executor.
    pub fn new(executor: Arc<dyn ServiceExecutor>) -> Self {
        let name = executor.service_name();
        let handles = executor.handles();
        Self {
            executor,
            name,
            handles,
        }
    }
}

#[async_trait]
impl RequestHandler for ServiceHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        let variant = request.variant_name();
        self.handles.contains(&variant)
    }

    async fn handle(&self, request: ClientRpcRequest, _ctx: &ClientProtocolContext) -> Result<ClientRpcResponse> {
        self.executor.execute(request).await
    }

    fn name(&self) -> &'static str {
        self.name
    }
}
