//! Cluster state query handler.

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::ClusterStateResponse;
use aspen_client_api::NodeDescriptor;
use aspen_rpc_core::ClientProtocolContext;

use super::MAX_CLUSTER_NODES;

pub(crate) async fn handle_get_cluster_state(ctx: &ClientProtocolContext) -> anyhow::Result<ClientRpcResponse> {
    // Get current cluster state from the Raft controller
    let cluster_state = ctx
        .controller
        .current_state()
        .await
        .map_err(|e| anyhow::anyhow!("failed to get cluster state: {}", e))?;

    // Get current leader from metrics
    let leader_id = ctx.controller.get_leader().await.unwrap_or(None);

    // Convert ClusterNode to NodeDescriptor with membership info
    // Tiger Style: Bounded to MAX_CLUSTER_NODES
    let mut nodes: Vec<NodeDescriptor> =
        Vec::with_capacity((cluster_state.nodes.len() + cluster_state.learners.len()).min(MAX_CLUSTER_NODES));

    // Track which nodes are voters (members)
    let member_ids: std::collections::HashSet<u64> = cluster_state.members.iter().copied().collect();

    // Add all nodes from the cluster state
    for node in cluster_state.nodes.iter().take(MAX_CLUSTER_NODES) {
        let is_voter = member_ids.contains(&node.id);
        let is_leader = leader_id == Some(node.id);

        // For our own node, use our actual endpoint address; for others use Raft-stored address
        let endpoint_addr = if node.id == ctx.node_id {
            format!("{:?}", ctx.endpoint_manager.node_addr())
        } else {
            node.iroh_addr().map(|a| format!("{:?}", a)).unwrap_or_default()
        };

        nodes.push(NodeDescriptor {
            node_id: node.id,
            endpoint_addr,
            is_voter,
            is_learner: !is_voter,
            is_leader,
        });
    }

    // Add learners that weren't in the nodes list
    for learner in cluster_state.learners.iter().take(MAX_CLUSTER_NODES - nodes.len()) {
        // Skip if already added
        if nodes.iter().any(|n| n.node_id == learner.id) {
            continue;
        }

        let endpoint_addr = learner.iroh_addr().map(|a| format!("{:?}", a)).unwrap_or_default();

        nodes.push(NodeDescriptor {
            node_id: learner.id,
            endpoint_addr,
            is_voter: false,
            is_learner: true,
            is_leader: false,
        });
    }

    Ok(ClientRpcResponse::ClusterState(ClusterStateResponse {
        nodes,
        leader_id,
        this_node_id: ctx.node_id,
    }))
}
