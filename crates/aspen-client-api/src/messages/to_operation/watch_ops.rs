use aspen_auth::Operation;

use super::super::ClientRpcRequest;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        // Watch operations
        ClientRpcRequest::WatchCreate { prefix, .. } => Some(Some(Operation::Read { key: prefix.clone() })),
        ClientRpcRequest::WatchCancel { .. } | ClientRpcRequest::WatchStatus { .. } => Some(None),

        _ => None,
    }
}
