//! Cluster topology query handler.

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::TopologyResultResponse;
use aspen_rpc_core::ClientProtocolContext;

pub(crate) async fn handle_get_topology(
    ctx: &ClientProtocolContext,
    client_version: Option<u64>,
) -> anyhow::Result<ClientRpcResponse> {
    match &ctx.topology {
        Some(topology) => {
            let topo = topology.read().await;

            // Check if client already has current version
            if let Some(cv) = client_version
                && cv == topo.version
            {
                return Ok(ClientRpcResponse::TopologyResult(TopologyResultResponse {
                    is_success: true,
                    version: topo.version,
                    was_updated: false,
                    topology_data: None,
                    shard_count: topo.shard_count() as u32,
                    error: None,
                }));
            }

            // Serialize topology for client
            let topology_data =
                serde_json::to_string(&*topo).map_err(|e| anyhow::anyhow!("failed to serialize topology: {}", e))?;

            Ok(ClientRpcResponse::TopologyResult(TopologyResultResponse {
                is_success: true,
                version: topo.version,
                was_updated: true,
                topology_data: Some(topology_data),
                shard_count: topo.shard_count() as u32,
                error: None,
            }))
        }
        None => Ok(ClientRpcResponse::TopologyResult(TopologyResultResponse {
            is_success: false,
            version: 0,
            was_updated: false,
            topology_data: None,
            shard_count: 0,
            error: Some("topology not available".to_string()),
        })),
    }
}
