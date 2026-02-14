use aspen_auth::Operation;

use super::super::ClientRpcRequest;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        // Blob write operations
        ClientRpcRequest::AddBlob { .. }
        | ClientRpcRequest::ProtectBlob { .. }
        | ClientRpcRequest::UnprotectBlob { .. }
        | ClientRpcRequest::DeleteBlob { .. }
        | ClientRpcRequest::DownloadBlob { .. }
        | ClientRpcRequest::DownloadBlobByHash { .. }
        | ClientRpcRequest::DownloadBlobByProvider { .. } => Some(Some(Operation::Write {
            key: "_blob:".to_string(),
            value: vec![],
        })),

        // Blob read operations
        ClientRpcRequest::GetBlob { hash }
        | ClientRpcRequest::HasBlob { hash }
        | ClientRpcRequest::GetBlobTicket { hash }
        | ClientRpcRequest::GetBlobStatus { hash }
        | ClientRpcRequest::GetBlobReplicationStatus { hash } => Some(Some(Operation::Read {
            key: format!("_blob:{hash}"),
        })),
        ClientRpcRequest::ListBlobs { .. } => Some(Some(Operation::Read {
            key: "_blob:".to_string(),
        })),

        // Blob replication operations (cluster-internal)
        ClientRpcRequest::BlobReplicatePull { hash, .. } => Some(Some(Operation::Write {
            key: format!("_blob:replica:{hash}"),
            value: vec![],
        })),
        ClientRpcRequest::TriggerBlobReplication { hash, .. } => Some(Some(Operation::Write {
            key: format!("_blob:replica:{hash}"),
            value: vec![],
        })),

        // Blob repair cycle is a cluster admin operation
        ClientRpcRequest::RunBlobRepairCycle => Some(Some(Operation::ClusterAdmin {
            action: "blob_repair_cycle".to_string(),
        })),

        _ => None,
    }
}
