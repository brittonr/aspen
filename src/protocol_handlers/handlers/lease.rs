//! Lease request handler.
//!
//! Handles: LeaseGrant, LeaseRevoke, LeaseKeepalive, LeaseTimeToLive,
//! LeaseList, WriteKeyWithLease.

use super::ClientProtocolContext;
use super::RequestHandler;
use crate::client_rpc::ClientRpcRequest;
use crate::client_rpc::ClientRpcResponse;

/// Handler for lease operations.
pub struct LeaseHandler;

#[async_trait::async_trait]
impl RequestHandler for LeaseHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::LeaseGrant { .. }
                | ClientRpcRequest::LeaseRevoke { .. }
                | ClientRpcRequest::LeaseKeepalive { .. }
                | ClientRpcRequest::LeaseTimeToLive { .. }
                | ClientRpcRequest::LeaseList
                | ClientRpcRequest::WriteKeyWithLease { .. }
        )
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        _ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        // The actual lease implementation requires complex type mapping between
        // the WriteResult API and client_rpc response types. For now, we
        // delegate to a placeholder that indicates the handler exists but the
        // implementation remains in client.rs until the full extraction is completed.
        //
        // TODO: Complete extraction of Lease handlers once the API type alignment
        // is completed.
        match request {
            ClientRpcRequest::LeaseGrant { .. }
            | ClientRpcRequest::LeaseRevoke { .. }
            | ClientRpcRequest::LeaseKeepalive { .. }
            | ClientRpcRequest::LeaseTimeToLive { .. }
            | ClientRpcRequest::LeaseList
            | ClientRpcRequest::WriteKeyWithLease { .. } => Ok(ClientRpcResponse::error(
                "NOT_IMPLEMENTED",
                "Lease handler extraction in progress - operation handled by legacy path",
            )),
            _ => Err(anyhow::anyhow!("request not handled by LeaseHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "LeaseHandler"
    }
}
