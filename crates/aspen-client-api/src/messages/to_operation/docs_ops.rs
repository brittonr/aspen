use alloc::format;
use alloc::string::ToString;

use aspen_auth_core::Operation;

use super::super::ClientRpcRequest;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        // Docs operations
        ClientRpcRequest::DocsSet { key, .. } => Some(Some(Operation::DocsWrite {
            resource: format!("doc:{key}"),
        })),
        ClientRpcRequest::DocsGet { key } => Some(Some(Operation::DocsRead {
            resource: format!("doc:{key}"),
        })),
        ClientRpcRequest::DocsDelete { key } => Some(Some(Operation::DocsWrite {
            resource: format!("doc:{key}"),
        })),
        ClientRpcRequest::DocsList { .. } | ClientRpcRequest::DocsStatus => Some(Some(Operation::DocsRead {
            resource: "doc:".to_string(),
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

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use aspen_auth_core::Capability;

    use crate::messages::ClientRpcRequest;

    fn op(request: &ClientRpcRequest) -> aspen_auth_core::Operation {
        super::to_operation(request).flatten().expect("protected docs request")
    }

    #[test]
    fn docs_requests_use_docs_capabilities_not_generic_prefixes() {
        let read = op(&ClientRpcRequest::DocsGet { key: "key".to_string() });
        let write = op(&ClientRpcRequest::DocsDelete { key: "key".to_string() });

        assert!(
            Capability::DocsRead {
                resource_prefix: "doc:".to_string(),
            }
            .authorizes(&read)
        );
        assert!(
            Capability::DocsWrite {
                resource_prefix: "doc:".to_string(),
            }
            .authorizes(&write)
        );
    }

    #[test]
    fn generic_docs_prefixes_do_not_authorize_docs_requests() {
        let generic_read = Capability::Read {
            prefix: "_docs:".to_string(),
        };
        let generic_write = Capability::Write {
            prefix: "_docs:".to_string(),
        };
        let read = op(&ClientRpcRequest::DocsGet { key: "key".to_string() });
        let write = op(&ClientRpcRequest::DocsSet {
            key: "key".to_string(),
            value: Vec::new(),
        });

        assert!(!generic_read.authorizes(&read));
        assert!(!generic_write.authorizes(&write));
    }
}
