use aspen_auth::Operation;

use super::super::ClientRpcRequest;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        // Key-value read operations
        ClientRpcRequest::ReadKey { key }
        | ClientRpcRequest::ScanKeys { prefix: key, .. }
        | ClientRpcRequest::GetVaultKeys { vault_name: key } => Some(Some(Operation::Read { key: key.clone() })),

        // Key-value write operations
        ClientRpcRequest::WriteKey { key, value } | ClientRpcRequest::WriteKeyWithLease { key, value, .. } => {
            Some(Some(Operation::Write {
                key: key.clone(),
                value: value.clone(),
            }))
        }
        ClientRpcRequest::DeleteKey { key }
        | ClientRpcRequest::CompareAndSwapKey { key, .. }
        | ClientRpcRequest::CompareAndDeleteKey { key, .. } => Some(Some(Operation::Write {
            key: key.clone(),
            value: vec![],
        })),

        _ => None,
    }
}
