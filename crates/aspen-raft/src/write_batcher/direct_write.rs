use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteResult;

use super::*;
use crate::types::AppRequest;

impl WriteBatcher {
    /// Write directly to Raft without batching.
    pub(super) async fn write_direct(&self, command: WriteCommand) -> Result<WriteResult, KeyValueStoreError> {
        use aspen_kv_types::BatchCondition;
        use aspen_kv_types::BatchOperation;

        let app_request = match &command {
            WriteCommand::Set { key, value } => AppRequest::Set {
                key: key.clone(),
                value: value.clone(),
            },
            WriteCommand::Delete { key } => AppRequest::Delete { key: key.clone() },
            WriteCommand::SetWithTTL {
                key,
                value,
                ttl_seconds,
            } => {
                let now_ms = aspen_time::current_time_ms();
                let expires_at_ms = now_ms.saturating_add(*ttl_seconds as u64 * 1000);
                AppRequest::SetWithTTL {
                    key: key.clone(),
                    value: value.clone(),
                    expires_at_ms,
                }
            }
            WriteCommand::SetMulti { pairs } => AppRequest::SetMulti { pairs: pairs.clone() },
            WriteCommand::SetMultiWithTTL { pairs, ttl_seconds } => {
                let now_ms = aspen_time::current_time_ms();
                let expires_at_ms = now_ms.saturating_add(*ttl_seconds as u64 * 1000);
                AppRequest::SetMultiWithTTL {
                    pairs: pairs.clone(),
                    expires_at_ms,
                }
            }
            WriteCommand::DeleteMulti { keys } => AppRequest::DeleteMulti { keys: keys.clone() },
            WriteCommand::CompareAndSwap {
                key,
                expected,
                new_value,
            } => AppRequest::CompareAndSwap {
                key: key.clone(),
                expected: expected.clone(),
                new_value: new_value.clone(),
            },
            WriteCommand::CompareAndDelete { key, expected } => AppRequest::CompareAndDelete {
                key: key.clone(),
                expected: expected.clone(),
            },
            WriteCommand::Batch { operations } => {
                let ops: Vec<(bool, String, String)> = operations
                    .iter()
                    .map(|op| match op {
                        BatchOperation::Set { key, value } => (true, key.clone(), value.clone()),
                        BatchOperation::Delete { key } => (false, key.clone(), String::new()),
                    })
                    .collect();
                AppRequest::Batch { operations: ops }
            }
            WriteCommand::ConditionalBatch { conditions, operations } => {
                let conds: Vec<(u8, String, String)> = conditions
                    .iter()
                    .map(|c| match c {
                        BatchCondition::ValueEquals { key, expected } => (0, key.clone(), expected.clone()),
                        BatchCondition::KeyExists { key } => (1, key.clone(), String::new()),
                        BatchCondition::KeyNotExists { key } => (2, key.clone(), String::new()),
                    })
                    .collect();
                let ops: Vec<(bool, String, String)> = operations
                    .iter()
                    .map(|op| match op {
                        BatchOperation::Set { key, value } => (true, key.clone(), value.clone()),
                        BatchOperation::Delete { key } => (false, key.clone(), String::new()),
                    })
                    .collect();
                AppRequest::ConditionalBatch {
                    conditions: conds,
                    operations: ops,
                }
            }
            // Lease operations
            WriteCommand::SetWithLease { key, value, lease_id } => AppRequest::SetWithLease {
                key: key.clone(),
                value: value.clone(),
                lease_id: *lease_id,
            },
            WriteCommand::SetMultiWithLease { pairs, lease_id } => AppRequest::SetMultiWithLease {
                pairs: pairs.clone(),
                lease_id: *lease_id,
            },
            WriteCommand::LeaseGrant { lease_id, ttl_seconds } => AppRequest::LeaseGrant {
                lease_id: *lease_id,
                ttl_seconds: *ttl_seconds,
            },
            WriteCommand::LeaseRevoke { lease_id } => AppRequest::LeaseRevoke { lease_id: *lease_id },
            WriteCommand::LeaseKeepalive { lease_id } => AppRequest::LeaseKeepalive { lease_id: *lease_id },
            WriteCommand::Transaction {
                compare,
                success,
                failure,
            } => {
                use aspen_kv_types::CompareOp;
                use aspen_kv_types::CompareTarget;
                use aspen_kv_types::TxnOp;
                let cmp: Vec<(u8, u8, String, String)> = compare
                    .iter()
                    .map(|c| {
                        let target = match c.target {
                            CompareTarget::Value => 0,
                            CompareTarget::Version => 1,
                            CompareTarget::CreateRevision => 2,
                            CompareTarget::ModRevision => 3,
                        };
                        let op = match c.op {
                            CompareOp::Equal => 0,
                            CompareOp::NotEqual => 1,
                            CompareOp::Greater => 2,
                            CompareOp::Less => 3,
                        };
                        (target, op, c.key.clone(), c.value.clone())
                    })
                    .collect();

                let convert_ops = |ops: &[TxnOp]| -> Vec<(u8, String, String)> {
                    ops.iter()
                        .map(|op| match op {
                            TxnOp::Put { key, value } => (0, key.clone(), value.clone()),
                            TxnOp::Delete { key } => (1, key.clone(), String::new()),
                            TxnOp::Get { key } => (2, key.clone(), String::new()),
                            TxnOp::Range { prefix, limit } => (3, prefix.clone(), limit.to_string()),
                        })
                        .collect()
                };

                AppRequest::Transaction {
                    compare: cmp,
                    success: convert_ops(success),
                    failure: convert_ops(failure),
                }
            }
            WriteCommand::OptimisticTransaction { read_set, write_set } => {
                use aspen_kv_types::WriteOp;
                let write_ops: Vec<(bool, String, String)> = write_set
                    .iter()
                    .map(|op| match op {
                        WriteOp::Set { key, value } => (true, key.clone(), value.clone()),
                        WriteOp::Delete { key } => (false, key.clone(), String::new()),
                    })
                    .collect();
                AppRequest::OptimisticTransaction {
                    read_set: read_set.clone(),
                    write_set: write_ops,
                }
            }
            WriteCommand::ShardSplit {
                source_shard,
                split_key,
                new_shard_id,
                topology_version,
            } => AppRequest::ShardSplit {
                source_shard: *source_shard,
                split_key: split_key.clone(),
                new_shard_id: *new_shard_id,
                topology_version: *topology_version,
            },
            WriteCommand::ShardMerge {
                source_shard,
                target_shard,
                topology_version,
            } => AppRequest::ShardMerge {
                source_shard: *source_shard,
                target_shard: *target_shard,
                topology_version: *topology_version,
            },
            WriteCommand::TopologyUpdate { topology_data } => AppRequest::TopologyUpdate {
                topology_data: topology_data.clone(),
            },
        };

        let result = self.raft.client_write(app_request).await;

        match result {
            Ok(_resp) => Ok(WriteResult {
                command: Some(command),
                batch_applied: None,
                conditions_met: None,
                failed_condition_index: None,
                lease_id: None,
                ttl_seconds: None,
                keys_deleted: None,
                succeeded: None,
                txn_results: None,
                header_revision: None,
                occ_conflict: None,
                conflict_key: None,
                conflict_expected_version: None,
                conflict_actual_version: None,
            }),
            Err(e) => Err(KeyValueStoreError::Failed {
                reason: format!("raft error: {}", e),
            }),
        }
    }
}
