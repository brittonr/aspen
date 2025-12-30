//! Pijul (patch-based VCS) request handler.
//!
//! Handles all Pijul* operations for patch-based version control.
//! This module is only available with the `pijul` feature.

use super::ClientProtocolContext;
use super::RequestHandler;
use crate::client_rpc::ClientRpcRequest;
use crate::client_rpc::ClientRpcResponse;

/// Handler for Pijul operations.
pub struct PijulHandler;

#[async_trait::async_trait]
impl RequestHandler for PijulHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::PijulRepoInit { .. }
                | ClientRpcRequest::PijulRepoList { .. }
                | ClientRpcRequest::PijulRepoInfo { .. }
                | ClientRpcRequest::PijulChannelList { .. }
                | ClientRpcRequest::PijulChannelCreate { .. }
                | ClientRpcRequest::PijulChannelDelete { .. }
                | ClientRpcRequest::PijulChannelFork { .. }
                | ClientRpcRequest::PijulChannelInfo { .. }
                | ClientRpcRequest::PijulRecord { .. }
                | ClientRpcRequest::PijulApply { .. }
                | ClientRpcRequest::PijulUnrecord { .. }
                | ClientRpcRequest::PijulLog { .. }
                | ClientRpcRequest::PijulCheckout { .. }
        )
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        // Check if Pijul feature is available
        if ctx.pijul_store.is_none() {
            return Ok(ClientRpcResponse::error(
                "PIJUL_UNAVAILABLE",
                "Pijul feature not configured on this node",
            ));
        }

        // The actual Pijul implementation requires complex type mapping between
        // the pijul_store types and the client_rpc response types. For now, we
        // delegate to a placeholder that indicates the handler exists but the
        // implementation remains in client.rs until the full extraction is completed.
        //
        // TODO: Complete extraction of Pijul handlers once the basic handler
        // infrastructure is validated with simpler handlers.
        match request {
            ClientRpcRequest::PijulRepoInit { .. }
            | ClientRpcRequest::PijulRepoList { .. }
            | ClientRpcRequest::PijulRepoInfo { .. }
            | ClientRpcRequest::PijulChannelList { .. }
            | ClientRpcRequest::PijulChannelCreate { .. }
            | ClientRpcRequest::PijulChannelDelete { .. }
            | ClientRpcRequest::PijulChannelFork { .. }
            | ClientRpcRequest::PijulChannelInfo { .. }
            | ClientRpcRequest::PijulRecord { .. }
            | ClientRpcRequest::PijulApply { .. }
            | ClientRpcRequest::PijulUnrecord { .. }
            | ClientRpcRequest::PijulLog { .. }
            | ClientRpcRequest::PijulCheckout { .. } => {
                // Placeholder: These operations remain in client.rs for now
                // The handler framework is ready but full extraction is pending
                Ok(ClientRpcResponse::error(
                    "NOT_IMPLEMENTED",
                    "Pijul handler extraction in progress - operation handled by legacy path",
                ))
            }
            _ => Err(anyhow::anyhow!("request not handled by PijulHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "PijulHandler"
    }
}
