//! Lease request handler.
//!
//! Handles: LeaseGrant, LeaseRevoke, LeaseKeepalive, LeaseTimeToLive,
//! LeaseList, WriteKeyWithLease.

use crate::context::ClientProtocolContext;
use crate::registry::RequestHandler;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;
use aspen_client_rpc::ClientRpcRequest;
use aspen_client_rpc::ClientRpcResponse;
use aspen_client_rpc::LeaseGrantResultResponse;
use aspen_client_rpc::LeaseInfo;
use aspen_client_rpc::LeaseKeepaliveResultResponse;
use aspen_client_rpc::LeaseListResultResponse;
use aspen_client_rpc::LeaseRevokeResultResponse;
use aspen_client_rpc::LeaseTimeToLiveResultResponse;
use aspen_client_rpc::WriteResultResponse;

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
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::LeaseGrant { ttl_seconds, lease_id } => {
                handle_lease_grant(ctx, ttl_seconds, lease_id).await
            }

            ClientRpcRequest::LeaseRevoke { lease_id } => handle_lease_revoke(ctx, lease_id).await,

            ClientRpcRequest::LeaseKeepalive { lease_id } => handle_lease_keepalive(ctx, lease_id).await,

            ClientRpcRequest::LeaseTimeToLive { lease_id, include_keys } => {
                handle_lease_time_to_live(ctx, lease_id, include_keys).await
            }

            ClientRpcRequest::LeaseList => handle_lease_list(ctx).await,

            ClientRpcRequest::WriteKeyWithLease { key, value, lease_id } => {
                handle_write_key_with_lease(ctx, key, value, lease_id).await
            }

            _ => Err(anyhow::anyhow!("request not handled by LeaseHandler")),
        }
    }

    fn name(&self) -> &'static str {
        "LeaseHandler"
    }
}

// ============================================================================
// Lease Operation Handlers
// ============================================================================

async fn handle_lease_grant(
    ctx: &ClientProtocolContext,
    ttl_seconds: u32,
    lease_id: Option<u64>,
) -> anyhow::Result<ClientRpcResponse> {
    let actual_lease_id = lease_id.unwrap_or(0);

    let result = ctx
        .kv_store
        .write(WriteRequest {
            command: WriteCommand::LeaseGrant {
                lease_id: actual_lease_id,
                ttl_seconds,
            },
        })
        .await;

    match result {
        Ok(response) => Ok(ClientRpcResponse::LeaseGrantResult(LeaseGrantResultResponse {
            success: true,
            lease_id: response.lease_id,
            ttl_seconds: response.ttl_seconds,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::LeaseGrantResult(LeaseGrantResultResponse {
            success: false,
            lease_id: None,
            ttl_seconds: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_lease_revoke(ctx: &ClientProtocolContext, lease_id: u64) -> anyhow::Result<ClientRpcResponse> {
    let result = ctx
        .kv_store
        .write(WriteRequest {
            command: WriteCommand::LeaseRevoke { lease_id },
        })
        .await;

    match result {
        Ok(response) => Ok(ClientRpcResponse::LeaseRevokeResult(LeaseRevokeResultResponse {
            success: true,
            keys_deleted: response.keys_deleted,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::LeaseRevokeResult(LeaseRevokeResultResponse {
            success: false,
            keys_deleted: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_lease_keepalive(ctx: &ClientProtocolContext, lease_id: u64) -> anyhow::Result<ClientRpcResponse> {
    let result = ctx
        .kv_store
        .write(WriteRequest {
            command: WriteCommand::LeaseKeepalive { lease_id },
        })
        .await;

    match result {
        Ok(response) => Ok(ClientRpcResponse::LeaseKeepaliveResult(LeaseKeepaliveResultResponse {
            success: true,
            lease_id: response.lease_id,
            ttl_seconds: response.ttl_seconds,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::LeaseKeepaliveResult(LeaseKeepaliveResultResponse {
            success: false,
            lease_id: None,
            ttl_seconds: None,
            error: Some(e.to_string()),
        })),
    }
}

async fn handle_lease_time_to_live(
    ctx: &ClientProtocolContext,
    lease_id: u64,
    include_keys: bool,
) -> anyhow::Result<ClientRpcResponse> {
    // Query state machine for lease info
    match &ctx.state_machine {
        Some(sm) => match sm.get_lease(lease_id) {
            Some((granted_ttl, remaining_ttl)) => {
                // Optionally include attached keys
                let keys = if include_keys {
                    Some(sm.get_lease_keys(lease_id))
                } else {
                    None
                };

                Ok(ClientRpcResponse::LeaseTimeToLiveResult(
                    LeaseTimeToLiveResultResponse {
                        success: true,
                        lease_id: Some(lease_id),
                        granted_ttl_seconds: Some(granted_ttl),
                        remaining_ttl_seconds: Some(remaining_ttl),
                        keys,
                        error: None,
                    },
                ))
            }
            None => Ok(ClientRpcResponse::LeaseTimeToLiveResult(
                LeaseTimeToLiveResultResponse {
                    success: false,
                    lease_id: Some(lease_id),
                    granted_ttl_seconds: None,
                    remaining_ttl_seconds: None,
                    keys: None,
                    error: Some("Lease not found or expired".to_string()),
                },
            )),
        },
        None => Ok(ClientRpcResponse::LeaseTimeToLiveResult(
            LeaseTimeToLiveResultResponse {
                success: false,
                lease_id: Some(lease_id),
                granted_ttl_seconds: None,
                remaining_ttl_seconds: None,
                keys: None,
                error: Some("State machine not available".to_string()),
            },
        )),
    }
}

async fn handle_lease_list(ctx: &ClientProtocolContext) -> anyhow::Result<ClientRpcResponse> {
    // Query state machine for all active leases
    match &ctx.state_machine {
        Some(sm) => {
            let leases_data = sm.list_leases();
            let leases: Vec<LeaseInfo> = leases_data
                .into_iter()
                .map(|(lease_id, granted_ttl, remaining_ttl)| LeaseInfo {
                    lease_id,
                    granted_ttl_seconds: granted_ttl,
                    remaining_ttl_seconds: remaining_ttl,
                    // Use LeaseTimeToLive with include_keys=true for key counts
                    attached_keys: 0,
                })
                .collect();

            Ok(ClientRpcResponse::LeaseListResult(LeaseListResultResponse {
                success: true,
                leases: Some(leases),
                error: None,
            }))
        }
        None => Ok(ClientRpcResponse::LeaseListResult(LeaseListResultResponse {
            success: false,
            leases: None,
            error: Some("State machine not available".to_string()),
        })),
    }
}

async fn handle_write_key_with_lease(
    ctx: &ClientProtocolContext,
    key: String,
    value: Vec<u8>,
    lease_id: u64,
) -> anyhow::Result<ClientRpcResponse> {
    // Convert Vec<u8> to String
    let value_str = String::from_utf8_lossy(&value).to_string();

    let result = ctx
        .kv_store
        .write(WriteRequest {
            command: WriteCommand::SetWithLease {
                key,
                value: value_str,
                lease_id,
            },
        })
        .await;

    match result {
        Ok(_) => Ok(ClientRpcResponse::WriteResult(WriteResultResponse {
            success: true,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::WriteResult(WriteResultResponse {
            success: false,
            error: Some(e.to_string()),
        })),
    }
}
