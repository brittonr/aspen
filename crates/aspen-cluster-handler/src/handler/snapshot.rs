//! Snapshot trigger handler.

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::SnapshotResultResponse;
use aspen_rpc_core::ClientProtocolContext;

use super::sanitize_control_error;

pub(crate) async fn handle_trigger_snapshot(ctx: &ClientProtocolContext) -> anyhow::Result<ClientRpcResponse> {
    let result = ctx.controller.trigger_snapshot().await;

    match result {
        Ok(snapshot) => Ok(ClientRpcResponse::SnapshotResult(SnapshotResultResponse {
            is_success: true,
            snapshot_index: snapshot.as_ref().map(|log_id| log_id.index),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::SnapshotResult(SnapshotResultResponse {
            is_success: false,
            snapshot_index: None,
            error: Some(sanitize_control_error(&e)),
        })),
    }
}
