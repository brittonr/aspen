use aspen_auth::Operation;

use super::super::ClientRpcRequest;

/// Returns `Some(op)` if this domain handles the request, `None` if not handled.
/// The inner `Option<Operation>` follows the original semantics: `None` means
/// no authorization required, `Some(op)` means authorization is needed.
pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        // Cluster admin operations
        ClientRpcRequest::InitCluster
        | ClientRpcRequest::AddLearner { .. }
        | ClientRpcRequest::ChangeMembership { .. }
        | ClientRpcRequest::TriggerSnapshot
        | ClientRpcRequest::PromoteLearner { .. }
        | ClientRpcRequest::AddPeer { .. }
        | ClientRpcRequest::CheckpointWal => Some(Some(Operation::ClusterAdmin {
            action: "cluster_operation".to_string(),
        })),

        // Read-only operations (no authorization required)
        ClientRpcRequest::Ping
        | ClientRpcRequest::GetHealth
        | ClientRpcRequest::GetNodeInfo
        | ClientRpcRequest::GetRaftMetrics
        | ClientRpcRequest::GetLeader
        | ClientRpcRequest::GetClusterTicket
        | ClientRpcRequest::GetClusterState
        | ClientRpcRequest::GetClusterTicketCombined { .. }
        | ClientRpcRequest::GetMetrics
        | ClientRpcRequest::ListVaults
        | ClientRpcRequest::GetFederationStatus
        | ClientRpcRequest::ListDiscoveredClusters
        | ClientRpcRequest::GetDiscoveredCluster { .. }
        | ClientRpcRequest::ListFederatedRepositories => Some(None),

        // Sharding operations (no authorization required)
        ClientRpcRequest::GetTopology { .. } => Some(None),

        _ => None,
    }
}
