use std::sync::Arc;

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::DispatchRegistry;
use aspen_rpc_core::RequestHandler;
use async_trait::async_trait;

#[derive(Debug)]
pub struct PingHandler;

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

#[must_use]
pub fn ping_registry() -> DispatchRegistry {
    let mut registry = DispatchRegistry::new();
    registry.push(Arc::new(PingHandler));
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn dispatches_ping_handler() {
        let registry = ping_registry();
        let ctx = ClientProtocolContext::empty();

        let response = registry.dispatch(ClientRpcRequest::Ping, &ctx).await.expect("ping dispatch should pass");

        assert!(matches!(response, ClientRpcResponse::Pong));
    }

    #[tokio::test]
    async fn rejects_unregistered_request() {
        let registry = DispatchRegistry::new();
        let ctx = ClientProtocolContext::empty();

        let error = registry.dispatch(ClientRpcRequest::Ping, &ctx).await.expect_err("empty registry should reject");

        assert!(error.to_string().contains("no handler registered"));
    }
}
