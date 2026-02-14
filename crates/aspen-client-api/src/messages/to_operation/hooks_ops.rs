use aspen_auth::Operation;

use super::super::ClientRpcRequest;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        // Hook operations (read-only metadata access)
        ClientRpcRequest::HookList | ClientRpcRequest::HookGetMetrics { .. } => Some(Some(Operation::Read {
            key: "_hooks:".to_string(),
        })),
        ClientRpcRequest::HookTrigger { .. } => Some(Some(Operation::Write {
            key: "_hooks:".to_string(),
            value: vec![],
        })),

        _ => None,
    }
}
