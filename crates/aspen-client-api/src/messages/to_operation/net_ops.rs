use aspen_auth::Operation;

use super::super::ClientRpcRequest;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        ClientRpcRequest::NetPublish { name, .. } => Some(Some(Operation::Write {
            key: format!("/_sys/net/svc/{}", name),
            value: vec![],
        })),
        ClientRpcRequest::NetUnpublish { name, .. } => Some(Some(Operation::Write {
            key: format!("/_sys/net/svc/{}", name),
            value: vec![],
        })),
        ClientRpcRequest::NetLookup { name, .. } => Some(Some(Operation::Read {
            key: format!("/_sys/net/svc/{}", name),
        })),
        ClientRpcRequest::NetList { .. } => Some(Some(Operation::Read {
            key: "/_sys/net/svc/".to_string(),
        })),
        _ => None,
    }
}
