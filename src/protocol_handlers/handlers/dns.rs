//! DNS request handler.
//!
//! Handles all DNS* operations for DNS record management.
//! This module is only available with the `dns` feature.

use super::ClientProtocolContext;
use super::RequestHandler;
use crate::client_rpc::ClientRpcRequest;
use crate::client_rpc::ClientRpcResponse;

/// Handler for DNS operations.
pub struct DnsHandler;

#[async_trait::async_trait]
impl RequestHandler for DnsHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::DnsSetRecord { .. }
                | ClientRpcRequest::DnsGetRecord { .. }
                | ClientRpcRequest::DnsGetRecords { .. }
                | ClientRpcRequest::DnsDeleteRecord { .. }
                | ClientRpcRequest::DnsResolve { .. }
                | ClientRpcRequest::DnsScanRecords { .. }
                | ClientRpcRequest::DnsSetZone { .. }
                | ClientRpcRequest::DnsGetZone { .. }
                | ClientRpcRequest::DnsListZones
                | ClientRpcRequest::DnsDeleteZone { .. }
        )
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        _ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        // Check if DNS feature is available
        #[cfg(not(feature = "dns"))]
        {
            let _ = request; // Suppress unused warning
            return Ok(ClientRpcResponse::error(
                "DNS_UNAVAILABLE",
                "DNS feature not enabled. Compile with --features dns",
            ));
        }

        // The actual DNS implementation is complex and tightly coupled to the
        // existing client.rs code. For now, we delegate to a placeholder that
        // indicates the handler exists but the implementation remains in client.rs
        // until the full extraction is completed.
        //
        // TODO: Complete extraction of DNS handlers once the basic handler
        // infrastructure is validated with simpler handlers.
        #[cfg(feature = "dns")]
        match request {
            ClientRpcRequest::DnsSetRecord { .. }
            | ClientRpcRequest::DnsGetRecord { .. }
            | ClientRpcRequest::DnsGetRecords { .. }
            | ClientRpcRequest::DnsDeleteRecord { .. }
            | ClientRpcRequest::DnsResolve { .. }
            | ClientRpcRequest::DnsScanRecords { .. }
            | ClientRpcRequest::DnsSetZone { .. }
            | ClientRpcRequest::DnsGetZone { .. }
            | ClientRpcRequest::DnsListZones
            | ClientRpcRequest::DnsDeleteZone { .. } => {
                // Placeholder: These operations remain in client.rs for now
                // The handler framework is ready but full extraction is pending
                Ok(ClientRpcResponse::error(
                    "NOT_IMPLEMENTED",
                    "DNS handler extraction in progress - operation handled by legacy path",
                ))
            }
            _ => Err(anyhow::anyhow!("request not handled by DnsHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "DnsHandler"
    }
}
