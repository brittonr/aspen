use aspen_auth::Operation;

use super::super::ClientRpcRequest;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        ClientRpcRequest::TraceIngest { .. } => Some(Some(Operation::Write {
            key: "_sys:traces:".to_string(),
            value: vec![],
        })),
        _ => None,
    }
}
