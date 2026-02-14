//! Cluster membership handlers.
//!
//! Handles: AddLearner, ChangeMembership, PromoteLearner.

use std::str::FromStr;

use aspen_client_api::AddLearnerResultResponse;
use aspen_client_api::ChangeMembershipResultResponse;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::PromoteLearnerResultResponse;
use aspen_core::AddLearnerRequest;
use aspen_core::ChangeMembershipRequest;
use aspen_core::ClusterNode;
use aspen_rpc_core::ClientProtocolContext;
use iroh::EndpointAddr;
use iroh::EndpointId;
#[cfg(feature = "jobs")]
use tracing::info;
#[cfg(feature = "jobs")]
use tracing::warn;

use super::sanitize_control_error;

pub(crate) async fn handle_add_learner(
    ctx: &ClientProtocolContext,
    node_id: u64,
    addr: String,
) -> anyhow::Result<ClientRpcResponse> {
    // Parse the address as either JSON EndpointAddr or bare EndpointId
    let iroh_addr = if addr.starts_with('{') {
        serde_json::from_str::<EndpointAddr>(&addr).map_err(|e| format!("invalid JSON EndpointAddr: {e}"))
    } else {
        EndpointId::from_str(&addr).map(EndpointAddr::new).map_err(|e| format!("invalid EndpointId: {e}"))
    };

    let result = match iroh_addr {
        Ok(iroh_addr) => {
            ctx.controller
                .add_learner(AddLearnerRequest {
                    learner: ClusterNode::with_iroh_addr(node_id, iroh_addr),
                })
                .await
        }
        Err(parse_err) => Err(aspen_core::ControlPlaneError::InvalidRequest { reason: parse_err }),
    };

    // If we successfully added a learner AND it's our own node, initialize job queues
    // This handles the case where a node joins an existing cluster
    #[cfg(feature = "jobs")]
    if result.is_ok() && node_id == ctx.node_id {
        if let Some(ref job_manager) = ctx.job_manager {
            info!("initializing job queues after joining cluster as learner");
            if let Err(e) = job_manager.initialize().await {
                warn!("failed to initialize job queues: {}. Jobs may not work properly.", e);
                // Continue - job system is optional and shouldn't block cluster operations
            } else {
                info!("job manager initialized with priority queues");
            }
        }
    }

    Ok(ClientRpcResponse::AddLearnerResult(AddLearnerResultResponse {
        success: result.is_ok(),
        error: result.err().map(|e| sanitize_control_error(&e)),
    }))
}

pub(crate) async fn handle_change_membership(
    ctx: &ClientProtocolContext,
    members: Vec<u64>,
) -> anyhow::Result<ClientRpcResponse> {
    let result = ctx.controller.change_membership(ChangeMembershipRequest { members }).await;

    Ok(ClientRpcResponse::ChangeMembershipResult(ChangeMembershipResultResponse {
        success: result.is_ok(),
        error: result.err().map(|e| sanitize_control_error(&e)),
    }))
}

pub(crate) async fn handle_promote_learner(
    ctx: &ClientProtocolContext,
    learner_id: u64,
    replace_node: Option<u64>,
    _force: bool,
) -> anyhow::Result<ClientRpcResponse> {
    // Get current membership to build new voter set
    let cluster_state = ctx
        .controller
        .current_state()
        .await
        .map_err(|e| anyhow::anyhow!("failed to get cluster state: {}", e))?;

    let previous_voters: Vec<u64> = cluster_state.members.clone();

    // Build new membership: add learner, optionally remove replaced node
    let mut new_members: Vec<u64> = previous_voters.clone();
    if !new_members.contains(&learner_id) {
        new_members.push(learner_id);
    }
    if let Some(replace_id) = replace_node {
        new_members.retain(|&id| id != replace_id);
    }

    let result = ctx
        .controller
        .change_membership(ChangeMembershipRequest {
            members: new_members.clone(),
        })
        .await;

    Ok(ClientRpcResponse::PromoteLearnerResult(PromoteLearnerResultResponse {
        success: result.is_ok(),
        learner_id,
        previous_voters,
        new_voters: new_members,
        message: if result.is_ok() {
            format!("promoted learner {} to voter", learner_id)
        } else {
            "promotion failed".to_string()
        },
        error: result.err().map(|e| sanitize_control_error(&e)),
    }))
}
