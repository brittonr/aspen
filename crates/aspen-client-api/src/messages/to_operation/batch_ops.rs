use alloc::string::ToString;

use aspen_auth_core::Operation;

use super::super::BatchCondition;
use super::super::BatchWriteOperation;
use super::super::ClientRpcRequest;
use super::key_is_reserved_internal;
use super::key_is_reserved_snix;
use super::reserved_internal_admin_operation;
use super::reserved_snix_admin_operation;

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

fn batch_operation_for_read_keys(keys: &[alloc::string::String]) -> Option<Operation> {
    if keys.is_empty() {
        return None;
    }

    if keys.iter().any(|key| key_is_reserved_internal(key)) {
        return Some(reserved_internal_admin_operation("reserved_internal_batch"));
    }

    if keys.iter().any(|key| key_is_reserved_snix(key)) {
        return Some(reserved_snix_admin_operation("reserved_snix_batch"));
    }

    Some(Operation::BatchRead { keys: keys.to_vec() })
}

fn batch_operation_for_write_keys(keys: alloc::vec::Vec<alloc::string::String>) -> Option<Operation> {
    if keys.is_empty() {
        return None;
    }

    if keys.iter().any(|key| key_is_reserved_internal(key)) {
        return Some(reserved_internal_admin_operation("reserved_internal_batch"));
    }

    if keys.iter().any(|key| key_is_reserved_snix(key)) {
        return Some(reserved_snix_admin_operation("reserved_snix_batch"));
    }

    Some(Operation::BatchWrite { keys })
}

pub(crate) fn to_operation(request: &ClientRpcRequest) -> Option<Option<Operation>> {
    match request {
        // Batch operations must authorize every requested key. Authorizing only
        // the first key lets a narrowly-scoped token smuggle out-of-scope reads
        // or writes later in the same atomic batch.
        ClientRpcRequest::BatchRead { keys } => Some(batch_operation_for_read_keys(keys)),
        ClientRpcRequest::BatchWrite { operations } => Some(batch_operation_for_write_keys(
            operations.iter().map(|operation| batch_write_key(operation).to_string()).collect(),
        )),
        ClientRpcRequest::ConditionalBatchWrite { conditions, operations } => {
            Some(batch_operation_for_write_keys(conditional_batch_write_keys(conditions, operations)))
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

        assert_eq!(keys.len(), 4);
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

    #[test]
    fn internal_keys_in_batch_operations_require_admin() {
        let read_request = ClientRpcRequest::BatchRead {
            keys: alloc::vec!["_ci:runs:run-1".to_string(), "user/key".to_string()],
        };
        let Some(Some(read_operation)) = to_operation(&read_request) else {
            panic!("internal batch read should require admin authorization");
        };
        assert!(
            matches!(&read_operation, Operation::ClusterAdmin { action } if action == "reserved_internal_batch"),
            "unexpected read operation: {read_operation:?}"
        );

        let write_request = ClientRpcRequest::BatchWrite {
            operations: alloc::vec![BatchWriteOperation::Set {
                key: "_secrets:kv:prod/db".to_string(),
                value: alloc::vec![1],
            }],
        };
        let Some(Some(write_operation)) = to_operation(&write_request) else {
            panic!("internal batch write should require admin authorization");
        };
        assert!(
            matches!(&write_operation, Operation::ClusterAdmin { action } if action == "reserved_internal_batch"),
            "unexpected write operation: {write_operation:?}"
        );
    }

    #[test]
    fn internal_condition_keys_in_conditional_batch_require_admin() {
        let request = ClientRpcRequest::ConditionalBatchWrite {
            conditions: alloc::vec![BatchCondition::KeyExists {
                key: "__worker:worker-1:jobs".to_string(),
            }],
            operations: alloc::vec![BatchWriteOperation::Delete {
                key: "generic/key".to_string(),
            }],
        };

        let Some(Some(operation)) = to_operation(&request) else {
            panic!("internal condition key should require admin authorization");
        };
        assert!(
            matches!(&operation, Operation::ClusterAdmin { action } if action == "reserved_internal_batch"),
            "unexpected operation: {operation:?}"
        );
    }

    #[test]
    fn snix_keys_in_batch_operations_require_admin() {
        let read_request = ClientRpcRequest::BatchRead {
            keys: alloc::vec!["snix:dir:abc".to_string()],
        };
        let Some(Some(read_operation)) = to_operation(&read_request) else {
            panic!("snix batch read should require admin authorization");
        };
        assert!(
            matches!(&read_operation, Operation::ClusterAdmin { action } if action == "reserved_snix_batch"),
            "unexpected read operation: {read_operation:?}"
        );

        let write_request = ClientRpcRequest::BatchWrite {
            operations: alloc::vec![BatchWriteOperation::Set {
                key: "snix:pathinfo:def".to_string(),
                value: alloc::vec![1],
            }],
        };
        let Some(Some(write_operation)) = to_operation(&write_request) else {
            panic!("snix batch write should require admin authorization");
        };
        assert!(
            matches!(&write_operation, Operation::ClusterAdmin { action } if action == "reserved_snix_batch"),
            "unexpected write operation: {write_operation:?}"
        );
    }

    #[test]
    fn snix_condition_keys_in_conditional_batch_require_admin() {
        let request = ClientRpcRequest::ConditionalBatchWrite {
            conditions: alloc::vec![BatchCondition::KeyExists {
                key: "snix:dir:condition".to_string(),
            }],
            operations: alloc::vec![BatchWriteOperation::Delete {
                key: "generic/key".to_string(),
            }],
        };

        let Some(Some(operation)) = to_operation(&request) else {
            panic!("snix condition key should require admin authorization");
        };
        assert!(
            matches!(&operation, Operation::ClusterAdmin { action } if action == "reserved_snix_batch"),
            "unexpected operation: {operation:?}"
        );
    }
}
