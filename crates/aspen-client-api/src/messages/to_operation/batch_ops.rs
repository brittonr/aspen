use aspen_auth::Operation;

use super::super::BatchWriteOperation;
use super::super::ClientRpcRequest;

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        // Batch operations
        ClientRpcRequest::BatchRead { keys } => Some(keys.first().map(|key| Operation::Read { key: key.clone() })),
        ClientRpcRequest::BatchWrite { operations } | ClientRpcRequest::ConditionalBatchWrite { operations, .. } => {
            Some(operations.first().map(|op| match op {
                BatchWriteOperation::Set { key, value } => Operation::Write {
                    key: key.clone(),
                    value: value.clone(),
                },
                BatchWriteOperation::Delete { key } => Operation::Write {
                    key: key.clone(),
                    value: vec![],
                },
            }))
        }

        _ => None,
    }
}
