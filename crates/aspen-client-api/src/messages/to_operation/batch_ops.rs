use alloc::string::ToString;

use aspen_auth_core::Operation;

use super::super::BatchCondition;
use super::super::BatchWriteOperation;
use super::super::ClientRpcRequest;

fn batch_write_key(operation: &BatchWriteOperation) -> &str {
    match operation {
        BatchWriteOperation::Set { key, .. } | BatchWriteOperation::Delete { key } => key,
    }
}

fn batch_condition_key(condition: &BatchCondition) -> &str {
    match condition {
        BatchCondition::ValueEquals { key, .. }
        | BatchCondition::KeyExists { key }
        | BatchCondition::KeyNotExists { key } => key,
    }
}

fn conditional_batch_write_keys(
    conditions: &[BatchCondition],
    operations: &[BatchWriteOperation],
) -> alloc::vec::Vec<alloc::string::String> {
    conditions
        .iter()
        .map(|condition| batch_condition_key(condition).to_string())
        .chain(operations.iter().map(|operation| batch_write_key(operation).to_string()))
        .collect()
}

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        // Batch operations must authorize every requested key. Authorizing only
        // the first key lets a narrowly-scoped token smuggle out-of-scope reads
        // or writes later in the same atomic batch.
        ClientRpcRequest::BatchRead { keys } => {
            Some((!keys.is_empty()).then(|| Operation::BatchRead { keys: keys.clone() }))
        }
        ClientRpcRequest::BatchWrite { operations } => Some((!operations.is_empty()).then(|| Operation::BatchWrite {
            keys: operations.iter().map(|operation| batch_write_key(operation).to_string()).collect(),
        })),
        ClientRpcRequest::ConditionalBatchWrite { conditions, operations } => {
            let keys = conditional_batch_write_keys(conditions, operations);
            Some((!keys.is_empty()).then(|| Operation::BatchWrite { keys }))
        }

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_read_authorization_covers_every_key() {
        let request = ClientRpcRequest::BatchRead {
            keys: alloc::vec!["allowed/a".to_string(), "denied/b".to_string()],
        };

        let Some(Some(Operation::BatchRead { keys })) = to_operation(&request) else {
            panic!("batch read should require batch-read authorization");
        };

        assert_eq!(keys, alloc::vec!["allowed/a".to_string(), "denied/b".to_string()]);
    }

    #[test]
    fn batch_write_authorization_covers_every_operation_key() {
        let request = ClientRpcRequest::BatchWrite {
            operations: alloc::vec![
                BatchWriteOperation::Set {
                    key: "allowed/a".to_string(),
                    value: alloc::vec![1],
                },
                BatchWriteOperation::Delete {
                    key: "denied/b".to_string(),
                },
            ],
        };

        let Some(Some(Operation::BatchWrite { keys })) = to_operation(&request) else {
            panic!("batch write should require batch-write authorization");
        };

        assert_eq!(keys, alloc::vec!["allowed/a".to_string(), "denied/b".to_string()]);
    }

    #[test]
    fn conditional_batch_write_authorization_covers_condition_and_operation_keys() {
        let request = ClientRpcRequest::ConditionalBatchWrite {
            conditions: alloc::vec![
                BatchCondition::ValueEquals {
                    key: "condition/value".to_string(),
                    expected: alloc::vec![1],
                },
                BatchCondition::KeyExists {
                    key: "condition/exists".to_string(),
                },
                BatchCondition::KeyNotExists {
                    key: "condition/missing".to_string(),
                },
            ],
            operations: alloc::vec![BatchWriteOperation::Delete {
                key: "write/a".to_string(),
            }],
        };

        let Some(Some(Operation::BatchWrite { keys })) = to_operation(&request) else {
            panic!("conditional batch write should require authorization for all condition and write keys");
        };

        assert_eq!(keys, alloc::vec![
            "condition/value".to_string(),
            "condition/exists".to_string(),
            "condition/missing".to_string(),
            "write/a".to_string(),
        ],);
    }

    #[test]
    fn conditional_batch_write_with_only_conditions_is_not_public() {
        let request = ClientRpcRequest::ConditionalBatchWrite {
            conditions: alloc::vec![BatchCondition::KeyNotExists {
                key: "condition/missing".to_string(),
            }],
            operations: alloc::vec![],
        };

        let Some(Some(Operation::BatchWrite { keys })) = to_operation(&request) else {
            panic!("condition-only batch write should still require condition-key authorization");
        };

        assert_eq!(keys, alloc::vec!["condition/missing".to_string()]);
    }
}
