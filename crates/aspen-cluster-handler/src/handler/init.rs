//! Cluster initialization handler.

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::InitResultResponse;
use aspen_core::ClusterNode;
use aspen_core::InitRequest;
use aspen_core::TrustConfig;
use aspen_rpc_core::ClientProtocolContext;
#[cfg(feature = "jobs")]
use tracing::info;
#[cfg(feature = "jobs")]
use tracing::warn;

use super::sanitize_control_error;

pub(crate) async fn handle_init_cluster(
    ctx: &ClientProtocolContext,
    is_trust_enabled: bool,
    trust_threshold: Option<u8>,
) -> anyhow::Result<ClientRpcResponse> {
    // Build ClusterNode for the current node to initialize as single-node cluster
    let endpoint_addr = ctx.endpoint_manager.node_addr();
    let this_node = ClusterNode::with_iroh_addr(ctx.node_id, endpoint_addr.clone());

    let trust = if is_trust_enabled {
        match trust_threshold {
            Some(t) => TrustConfig::with_threshold(t),
            None => TrustConfig::enabled(),
        }
    } else {
        TrustConfig {
            enabled: false,
            threshold: None,
        }
    };

    let result = ctx
        .controller
        .init(InitRequest {
            initial_members: vec![this_node],
            trust,
        })
        .await;

    // If cluster initialization succeeded, initialize job queues
    #[cfg(feature = "jobs")]
    if result.is_ok() {
        if let Some(ref job_manager) = ctx.job_manager {
            info!("initializing job queues after cluster initialization");
            if let Err(e) = job_manager.initialize().await {
                warn!("failed to initialize job queues: {}. Jobs may not work properly.", e);
                // Continue - job system is optional and shouldn't block cluster operations
            } else {
                info!("job manager initialized with priority queues");
            }
        }
    }

    Ok(ClientRpcResponse::InitResult(InitResultResponse {
        is_success: result.is_ok(),
        error: result.err().map(|e| sanitize_control_error(&e)),
    }))
}
