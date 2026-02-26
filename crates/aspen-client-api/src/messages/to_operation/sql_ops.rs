use aspen_auth::Operation;

use super::super::ClientRpcRequest;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        // SQL queries
        ClientRpcRequest::ExecuteSql { .. } => Some(Some(Operation::Read {
            key: "_sql:".to_string(),
        })),

        _ => None,
    }
}
