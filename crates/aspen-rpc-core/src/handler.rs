//! Request handler trait for domain-specific RPC handlers.
//!
//! This module defines the core `RequestHandler` trait that all domain-specific
//! handlers must implement. The trait enables modular handler extraction while
//! maintaining a unified dispatch mechanism.

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
