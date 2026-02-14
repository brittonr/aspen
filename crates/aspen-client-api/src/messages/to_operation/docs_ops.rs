use aspen_auth::Operation;

use super::super::ClientRpcRequest;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        // Docs operations
        ClientRpcRequest::DocsSet { key, value } => Some(Some(Operation::Write {
            key: format!("_docs:{key}"),
            value: value.clone(),
        })),
        ClientRpcRequest::DocsGet { key } | ClientRpcRequest::DocsDelete { key } => Some(Some(Operation::Read {
            key: format!("_docs:{key}"),
        })),
        ClientRpcRequest::DocsList { .. } | ClientRpcRequest::DocsStatus => Some(Some(Operation::Read {
            key: "_docs:".to_string(),
        })),

        // Peer cluster operations
        ClientRpcRequest::AddPeerCluster { .. }
        | ClientRpcRequest::RemovePeerCluster { .. }
        | ClientRpcRequest::UpdatePeerClusterFilter { .. }
        | ClientRpcRequest::UpdatePeerClusterPriority { .. }
        | ClientRpcRequest::SetPeerClusterEnabled { .. } => Some(Some(Operation::ClusterAdmin {
            action: "peer_cluster_operation".to_string(),
        })),
        ClientRpcRequest::ListPeerClusters
        | ClientRpcRequest::GetPeerClusterStatus { .. }
        | ClientRpcRequest::GetKeyOrigin { .. }
        | ClientRpcRequest::GetClientTicket { .. }
        | ClientRpcRequest::GetDocsTicket { .. } => Some(None),

        _ => None,
    }
}
