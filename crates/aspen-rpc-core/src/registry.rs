//! Minimal reusable dispatch registry.
//!
//! This registry is intentionally small: it owns handler ordering and request
//! dispatch only. Aspen runtime assembly, hot reload, proxying, and plugin
//! integration remain in `aspen-rpc-handlers`.

use std::sync::Arc;

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;

use crate::ClientProtocolContext;
use crate::RequestHandler;

/// Ordered handler registry for downstream-style dispatch tests.
#[derive(Clone, Default)]
pub struct DispatchRegistry {
    handlers: Vec<Arc<dyn RequestHandler>>,
}

impl DispatchRegistry {
    /// Create an empty registry.
    #[must_use]
    pub const fn new() -> Self {
        Self { handlers: Vec::new() }
    }

    /// Register one handler at the end of the dispatch order.
    pub fn push(&mut self, handler: Arc<dyn RequestHandler>) {
        self.handlers.push(handler);
    }

    /// Number of registered handlers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    /// Whether no handlers are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }

    /// Dispatch request to the first handler that accepts it.
    ///
    /// # Errors
    ///
    /// Returns an error when no handler accepts the request or the selected
    /// handler fails.
    pub async fn dispatch(&self, request: ClientRpcRequest, ctx: &ClientProtocolContext) -> Result<ClientRpcResponse> {
        for handler in &self.handlers {
            if handler.can_handle(&request) {
                return handler.handle(request, ctx).await;
            }
        }

        anyhow::bail!("no handler registered for {}", request.variant_name())
    }
}

#[cfg(all(test, not(feature = "runtime-context")))]
mod tests {
    use async_trait::async_trait;

    use super::*;

    #[derive(Debug)]
    struct PingHandler;

    #[async_trait]
    impl RequestHandler for PingHandler {
        fn can_handle(&self, request: &ClientRpcRequest) -> bool {
            matches!(request, ClientRpcRequest::Ping)
        }

        async fn handle(&self, request: ClientRpcRequest, _ctx: &ClientProtocolContext) -> Result<ClientRpcResponse> {
            assert!(matches!(request, ClientRpcRequest::Ping));
            Ok(ClientRpcResponse::Pong)
        }

        fn name(&self) -> &'static str {
            "ping"
        }
    }

    #[tokio::test]
    async fn dispatch_registered_handler() {
        let mut registry = DispatchRegistry::new();
        registry.push(Arc::new(PingHandler));

        let ctx = ClientProtocolContext::empty();
        let response = registry.dispatch(ClientRpcRequest::Ping, &ctx).await.expect("dispatch ping");

        assert!(matches!(response, ClientRpcResponse::Pong));
    }

    #[tokio::test]
    async fn dispatch_rejects_unhandled_request() {
        let registry = DispatchRegistry::new();
        let ctx = ClientProtocolContext::empty();

        let error = registry.dispatch(ClientRpcRequest::Ping, &ctx).await.expect_err("missing handler");

        assert!(error.to_string().contains("no handler registered"));
    }
}
