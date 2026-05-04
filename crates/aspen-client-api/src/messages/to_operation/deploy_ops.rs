use alloc::string::ToString;

use aspen_auth_core::Operation;

use super::super::ClientRpcRequest;

/// Deploy operations require cluster admin authorization.
pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        ClientRpcRequest::ClusterDeploy { .. }
        | ClientRpcRequest::ClusterRollback
        | ClientRpcRequest::NodeUpgrade { .. }
        | ClientRpcRequest::NodeRollback { .. }
        | ClientRpcRequest::ClusterDeployStatus => Some(Some(Operation::ClusterAdmin {
            action: "cluster_operation".to_string(),
        })),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deploy_operation(request: &ClientRpcRequest) -> Operation {
        to_operation(request)
            .expect("deploy request should be classified")
            .expect("deploy request should require authorization")
    }

    #[test]
    fn all_deploy_requests_require_cluster_admin_scope() {
        let requests = [
            ClientRpcRequest::ClusterDeploy {
                artifact: "/nix/store/example-aspen-node".to_string(),
                strategy: "rolling".to_string(),
                max_concurrent: 1,
                health_timeout_secs: 120,
                expected_binary: None,
            },
            ClientRpcRequest::ClusterDeployStatus,
            ClientRpcRequest::ClusterRollback,
            ClientRpcRequest::NodeUpgrade {
                deploy_id: "deploy-1".to_string(),
                artifact: "/nix/store/example-aspen-node".to_string(),
                expected_binary: None,
            },
            ClientRpcRequest::NodeRollback {
                deploy_id: "deploy-1".to_string(),
            },
        ];

        assert_eq!(requests.len(), 5);
        for request in &requests {
            assert!(matches!(
                deploy_operation(request),
                Operation::ClusterAdmin { action } if action == "cluster_operation"
            ));
        }
    }
}
