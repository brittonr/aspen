use aspen_auth::Operation;

use super::super::ClientRpcRequest;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        // Lease operations
        ClientRpcRequest::LeaseGrant { .. }
        | ClientRpcRequest::LeaseRevoke { .. }
        | ClientRpcRequest::LeaseKeepalive { .. } => Some(Some(Operation::Write {
            key: "_lease:".to_string(),
            value: vec![],
        })),
        ClientRpcRequest::LeaseTimeToLive { .. } | ClientRpcRequest::LeaseList => Some(Some(Operation::Read {
            key: "_lease:".to_string(),
        })),

        _ => None,
    }
}
