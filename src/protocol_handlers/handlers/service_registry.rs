//! Service registry request handler.
//!
//! Handles: ServiceRegister, ServiceDeregister, ServiceDiscover, ServiceList,
//! ServiceGetInstance, ServiceHeartbeat, ServiceUpdateHealth, ServiceUpdateMetadata.

use super::ClientProtocolContext;
use super::RequestHandler;
use crate::client_rpc::ClientRpcRequest;
use crate::client_rpc::ClientRpcResponse;

/// Handler for service registry operations.
pub struct ServiceRegistryHandler;

#[async_trait::async_trait]
impl RequestHandler for ServiceRegistryHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::ServiceRegister { .. }
                | ClientRpcRequest::ServiceDeregister { .. }
                | ClientRpcRequest::ServiceDiscover { .. }
                | ClientRpcRequest::ServiceList { .. }
                | ClientRpcRequest::ServiceGetInstance { .. }
                | ClientRpcRequest::ServiceHeartbeat { .. }
                | ClientRpcRequest::ServiceUpdateHealth { .. }
                | ClientRpcRequest::ServiceUpdateMetadata { .. }
        )
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        _ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        // The actual service registry implementation requires complex type mapping
        // between the internal ServiceRegistry types and client_rpc response types.
        // For now, we delegate to a placeholder that indicates the handler exists but
        // the implementation remains in client.rs until the full extraction is completed.
        //
        // TODO: Complete extraction of ServiceRegistry handlers once the API type
        // alignment is completed.
        match request {
            ClientRpcRequest::ServiceRegister { .. }
            | ClientRpcRequest::ServiceDeregister { .. }
            | ClientRpcRequest::ServiceDiscover { .. }
            | ClientRpcRequest::ServiceList { .. }
            | ClientRpcRequest::ServiceGetInstance { .. }
            | ClientRpcRequest::ServiceHeartbeat { .. }
            | ClientRpcRequest::ServiceUpdateHealth { .. }
            | ClientRpcRequest::ServiceUpdateMetadata { .. } => Ok(ClientRpcResponse::error(
                "NOT_IMPLEMENTED",
                "ServiceRegistry handler extraction in progress - operation handled by legacy path",
            )),
            _ => Err(anyhow::anyhow!("request not handled by ServiceRegistryHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "ServiceRegistryHandler"
    }
}
