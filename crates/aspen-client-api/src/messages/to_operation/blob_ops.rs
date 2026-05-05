use alloc::format;
use alloc::string::ToString;

use aspen_auth_core::Operation;

use super::super::ClientRpcRequest;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        // Blob write operations
        ClientRpcRequest::AddBlob { .. } => Some(Some(Operation::BlobWrite {
            resource: "blob:".to_string(),
        })),
        ClientRpcRequest::ProtectBlob { hash, .. }
        | ClientRpcRequest::DeleteBlob { hash, .. }
        | ClientRpcRequest::DownloadBlobByHash { hash, .. }
        | ClientRpcRequest::DownloadBlobByProvider { hash, .. } => Some(Some(Operation::BlobWrite {
            resource: format!("blob:{hash}"),
        })),
        ClientRpcRequest::UnprotectBlob { tag } => Some(Some(Operation::BlobWrite {
            resource: format!("tag:{tag}"),
        })),
        ClientRpcRequest::DownloadBlob { .. } => Some(Some(Operation::BlobWrite {
            resource: "download:".to_string(),
        })),

        // Blob read operations
        ClientRpcRequest::GetBlob { hash }
        | ClientRpcRequest::HasBlob { hash }
        | ClientRpcRequest::GetBlobTicket { hash }
        | ClientRpcRequest::GetBlobStatus { hash }
        | ClientRpcRequest::GetBlobReplicationStatus { hash } => Some(Some(Operation::BlobRead {
            resource: format!("blob:{hash}"),
        })),
        ClientRpcRequest::ListBlobs { .. } => Some(Some(Operation::BlobRead {
            resource: "blob:".to_string(),
        })),

        // Blob replication operations (cluster-internal)
        ClientRpcRequest::BlobReplicatePull { hash, .. } | ClientRpcRequest::TriggerBlobReplication { hash, .. } => {
            Some(Some(Operation::BlobWrite {
                resource: format!("replica:{hash}"),
            }))
        }

        // Blob repair cycle is a cluster admin operation
        ClientRpcRequest::RunBlobRepairCycle => Some(Some(Operation::ClusterAdmin {
            action: "blob_repair_cycle".to_string(),
        })),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use aspen_auth_core::Capability;

    use crate::messages::ClientRpcRequest;

    fn op(request: &ClientRpcRequest) -> aspen_auth_core::Operation {
        super::to_operation(request).flatten().expect("protected blob request")
    }

    #[test]
    fn blob_requests_use_blob_capabilities_not_generic_prefixes() {
        let hash = "hash-1".to_string();
        let read = op(&ClientRpcRequest::GetBlob { hash: hash.clone() });
        let write = op(&ClientRpcRequest::DeleteBlob { hash, is_force: false });

        assert!(
            Capability::BlobRead {
                resource_prefix: "blob:".to_string(),
            }
            .authorizes(&read)
        );
        assert!(
            Capability::BlobWrite {
                resource_prefix: "blob:".to_string(),
            }
            .authorizes(&write)
        );
    }

    #[test]
    fn generic_blob_prefixes_do_not_authorize_blob_requests() {
        let generic_read = Capability::Read {
            prefix: "_blob:".to_string(),
        };
        let generic_write = Capability::Write {
            prefix: "_blob:".to_string(),
        };
        let read = op(&ClientRpcRequest::GetBlob {
            hash: "hash-1".to_string(),
        });
        let write = op(&ClientRpcRequest::TriggerBlobReplication {
            hash: "hash-1".to_string(),
            target_nodes: Vec::new(),
            replication_factor: 0,
        });

        assert!(!generic_read.authorizes(&read));
        assert!(!generic_write.authorizes(&write));
    }
}
