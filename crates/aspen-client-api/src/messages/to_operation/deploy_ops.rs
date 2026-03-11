use aspen_auth::Operation;

use super::super::ClientRpcRequest;

/// Deploy operations require cluster admin authorization.
pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        ClientRpcRequest::ClusterDeploy { .. }
        | ClientRpcRequest::ClusterRollback
        | ClientRpcRequest::NodeUpgrade { .. }
        | ClientRpcRequest::NodeRollback { .. } => Some(Some(Operation::ClusterAdmin {
            action: "cluster_operation".to_string(),
        })),

        // Status queries are read-only (no authorization required)
        ClientRpcRequest::ClusterDeployStatus => Some(None),

        _ => None,
    }
}
