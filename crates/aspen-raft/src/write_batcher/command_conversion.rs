//! Pure conversion from `WriteCommand` to `AppRequest`.
//!
//! This module extracts the shared logic used by both the `KeyValueStore::write`
//! implementation on `RaftNode` and the `WriteBatcher::write_direct` fallback path.
//! All functions are deterministic data transformations with no I/O.

use aspen_kv_types::BatchCondition;
use aspen_kv_types::BatchOperation;
use aspen_kv_types::CompareOp;
use aspen_kv_types::CompareTarget;
use aspen_kv_types::TxnCompare;
use aspen_kv_types::TxnOp;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteOp;

use crate::constants::MAX_SETMULTI_KEYS;
use crate::types::AppRequest;

struct SetWithTtlRequest<'a> {
    key: &'a str,
    value: &'a str,
    ttl_seconds: u32,
}

enum WriteCommandGroup<'a> {
    Direct(DirectWriteCommand<'a>),
    BatchLease(BatchLeaseWriteCommand<'a>),
    Advanced(AdvancedWriteCommand<'a>),
}

enum DirectWriteCommand<'a> {
    Set {
        key: &'a String,
        value: &'a String,
    },
    Delete {
        key: &'a String,
    },
    SetMulti {
        pairs: &'a [(String, String)],
    },
    DeleteMulti {
        keys: &'a [String],
    },
    SetWithTtl(SetWithTtlRequest<'a>),
    SetMultiWithTtl {
        pairs: &'a [(String, String)],
        ttl_seconds: u32,
    },
    CompareAndSwap {
        key: &'a String,
        expected: &'a Option<String>,
        new_value: &'a String,
    },
    CompareAndDelete {
        key: &'a String,
        expected: &'a String,
    },
}

enum BatchLeaseWriteCommand<'a> {
    Batch {
        operations: &'a [BatchOperation],
    },
    ConditionalBatch {
        conditions: &'a [BatchCondition],
        operations: &'a [BatchOperation],
    },
    SetWithLease {
        key: &'a String,
        value: &'a String,
        lease_id: u64,
    },
    SetMultiWithLease {
        pairs: &'a [(String, String)],
        lease_id: u64,
    },
    LeaseGrant {
        lease_id: u64,
        ttl_seconds: u32,
    },
    LeaseRevoke {
        lease_id: u64,
    },
    LeaseKeepalive {
        lease_id: u64,
    },
}

enum AdvancedWriteCommand<'a> {
    Transaction {
        compare: &'a [TxnCompare],
        success: &'a [TxnOp],
        failure: &'a [TxnOp],
    },
    OptimisticTransaction {
        read_set: &'a [(String, i64)],
        write_set: &'a [WriteOp],
    },
    ShardSplit {
        source_shard: u32,
        split_key: &'a String,
        new_shard_id: u32,
        topology_version: u64,
    },
    ShardMerge {
        source_shard: u32,
        target_shard: u32,
        topology_version: u64,
    },
    TopologyUpdate {
        topology_data: &'a Vec<u8>,
    },
}

#[inline]
fn max_setmulti_keys_usize() -> usize {
    match usize::try_from(MAX_SETMULTI_KEYS) {
        Ok(max_keys) => max_keys,
        Err(_) => usize::MAX,
    }
}

#[inline]
fn ttl_duration_ms(ttl_seconds: u32) -> u64 {
    u64::from(ttl_seconds).saturating_mul(1000)
}

#[inline]
fn validate_key_bearing_command(command: &WriteCommand) {
    debug_assert!(
        match command {
            WriteCommand::Set { key, .. }
            | WriteCommand::Delete { key }
            | WriteCommand::CompareAndSwap { key, .. }
            | WriteCommand::CompareAndDelete { key, .. }
            | WriteCommand::SetWithTTL { key, .. }
            | WriteCommand::SetWithLease { key, .. } => !key.is_empty(),
            _ => true,
        },
        "COMMAND_CONVERT: key-bearing command must have non-empty key"
    );
}

fn group_write_command(command: &WriteCommand) -> WriteCommandGroup<'_> {
    match command {
        WriteCommand::Set { key, value } => WriteCommandGroup::Direct(DirectWriteCommand::Set { key, value }),
        WriteCommand::Delete { key } => WriteCommandGroup::Direct(DirectWriteCommand::Delete { key }),
        WriteCommand::SetMulti { pairs } => WriteCommandGroup::Direct(DirectWriteCommand::SetMulti { pairs }),
        WriteCommand::DeleteMulti { keys } => WriteCommandGroup::Direct(DirectWriteCommand::DeleteMulti { keys }),
        WriteCommand::SetWithTTL {
            key,
            value,
            ttl_seconds,
        } => WriteCommandGroup::Direct(DirectWriteCommand::SetWithTtl(SetWithTtlRequest {
            key,
            value,
            ttl_seconds: *ttl_seconds,
        })),
        WriteCommand::SetMultiWithTTL { pairs, ttl_seconds } => {
            WriteCommandGroup::Direct(DirectWriteCommand::SetMultiWithTtl {
                pairs,
                ttl_seconds: *ttl_seconds,
            })
        }
        WriteCommand::CompareAndSwap {
            key,
            expected,
            new_value,
        } => WriteCommandGroup::Direct(DirectWriteCommand::CompareAndSwap {
            key,
            expected,
            new_value,
        }),
        WriteCommand::CompareAndDelete { key, expected } => {
            WriteCommandGroup::Direct(DirectWriteCommand::CompareAndDelete { key, expected })
        }
        WriteCommand::Batch { operations } => {
            WriteCommandGroup::BatchLease(BatchLeaseWriteCommand::Batch { operations })
        }
        WriteCommand::ConditionalBatch { conditions, operations } => {
            WriteCommandGroup::BatchLease(BatchLeaseWriteCommand::ConditionalBatch { conditions, operations })
        }
        WriteCommand::SetWithLease { key, value, lease_id } => {
            WriteCommandGroup::BatchLease(BatchLeaseWriteCommand::SetWithLease {
                key,
                value,
                lease_id: *lease_id,
            })
        }
        WriteCommand::SetMultiWithLease { pairs, lease_id } => {
            WriteCommandGroup::BatchLease(BatchLeaseWriteCommand::SetMultiWithLease {
                pairs,
                lease_id: *lease_id,
            })
        }
        WriteCommand::LeaseGrant { lease_id, ttl_seconds } => {
            WriteCommandGroup::BatchLease(BatchLeaseWriteCommand::LeaseGrant {
                lease_id: *lease_id,
                ttl_seconds: *ttl_seconds,
            })
        }
        WriteCommand::LeaseRevoke { lease_id } => {
            WriteCommandGroup::BatchLease(BatchLeaseWriteCommand::LeaseRevoke { lease_id: *lease_id })
        }
        WriteCommand::LeaseKeepalive { lease_id } => {
            WriteCommandGroup::BatchLease(BatchLeaseWriteCommand::LeaseKeepalive { lease_id: *lease_id })
        }
        WriteCommand::Transaction {
            compare,
            success,
            failure,
        } => WriteCommandGroup::Advanced(AdvancedWriteCommand::Transaction {
            compare,
            success,
            failure,
        }),
        WriteCommand::OptimisticTransaction { read_set, write_set } => {
            WriteCommandGroup::Advanced(AdvancedWriteCommand::OptimisticTransaction { read_set, write_set })
        }
        WriteCommand::ShardSplit {
            source_shard,
            split_key,
            new_shard_id,
            topology_version,
        } => WriteCommandGroup::Advanced(AdvancedWriteCommand::ShardSplit {
            source_shard: *source_shard,
            split_key,
            new_shard_id: *new_shard_id,
            topology_version: *topology_version,
        }),
        WriteCommand::ShardMerge {
            source_shard,
            target_shard,
            topology_version,
        } => WriteCommandGroup::Advanced(AdvancedWriteCommand::ShardMerge {
            source_shard: *source_shard,
            target_shard: *target_shard,
            topology_version: *topology_version,
        }),
        WriteCommand::TopologyUpdate { topology_data } => {
            WriteCommandGroup::Advanced(AdvancedWriteCommand::TopologyUpdate { topology_data })
        }
    }
}

fn convert_direct_write_command(command: DirectWriteCommand<'_>) -> AppRequest {
    match command {
        DirectWriteCommand::Set { key, value } => AppRequest::Set {
            key: key.clone(),
            value: value.clone(),
        },
        DirectWriteCommand::Delete { key } => AppRequest::Delete { key: key.clone() },
        DirectWriteCommand::SetMulti { pairs } => AppRequest::SetMulti { pairs: pairs.to_vec() },
        DirectWriteCommand::DeleteMulti { keys } => AppRequest::DeleteMulti { keys: keys.to_vec() },
        DirectWriteCommand::SetWithTtl(request) => convert_set_with_ttl(request),
        DirectWriteCommand::SetMultiWithTtl { pairs, ttl_seconds } => convert_set_multi_with_ttl(pairs, ttl_seconds),
        DirectWriteCommand::CompareAndSwap {
            key,
            expected,
            new_value,
        } => AppRequest::CompareAndSwap {
            key: key.clone(),
            expected: expected.clone(),
            new_value: new_value.clone(),
        },
        DirectWriteCommand::CompareAndDelete { key, expected } => AppRequest::CompareAndDelete {
            key: key.clone(),
            expected: expected.clone(),
        },
    }
}

fn convert_batch_lease_write_command(command: BatchLeaseWriteCommand<'_>) -> AppRequest {
    match command {
        BatchLeaseWriteCommand::Batch { operations } => AppRequest::Batch {
            operations: convert_batch_ops(operations),
        },
        BatchLeaseWriteCommand::ConditionalBatch { conditions, operations } => {
            convert_conditional_batch(conditions, operations)
        }
        BatchLeaseWriteCommand::SetWithLease { key, value, lease_id } => AppRequest::SetWithLease {
            key: key.clone(),
            value: value.clone(),
            lease_id,
        },
        BatchLeaseWriteCommand::SetMultiWithLease { pairs, lease_id } => AppRequest::SetMultiWithLease {
            pairs: pairs.to_vec(),
            lease_id,
        },
        BatchLeaseWriteCommand::LeaseGrant { lease_id, ttl_seconds } => {
            AppRequest::LeaseGrant { lease_id, ttl_seconds }
        }
        BatchLeaseWriteCommand::LeaseRevoke { lease_id } => AppRequest::LeaseRevoke { lease_id },
        BatchLeaseWriteCommand::LeaseKeepalive { lease_id } => AppRequest::LeaseKeepalive { lease_id },
    }
}

fn convert_advanced_write_command(command: AdvancedWriteCommand<'_>) -> AppRequest {
    match command {
        AdvancedWriteCommand::Transaction {
            compare,
            success,
            failure,
        } => convert_transaction(compare, success, failure),
        AdvancedWriteCommand::OptimisticTransaction { read_set, write_set } => {
            convert_optimistic_txn(read_set, write_set)
        }
        AdvancedWriteCommand::ShardSplit {
            source_shard,
            split_key,
            new_shard_id,
            topology_version,
        } => AppRequest::ShardSplit {
            source_shard,
            split_key: split_key.clone(),
            new_shard_id,
            topology_version,
        },
        AdvancedWriteCommand::ShardMerge {
            source_shard,
            target_shard,
            topology_version,
        } => AppRequest::ShardMerge {
            source_shard,
            target_shard,
            topology_version,
        },
        AdvancedWriteCommand::TopologyUpdate { topology_data } => AppRequest::TopologyUpdate {
            topology_data: topology_data.clone(),
        },
    }
}

/// Convert a `WriteCommand` into the corresponding `AppRequest` for Raft consensus.
///
/// This is a pure data transformation. TTL commands compute an absolute expiration
/// timestamp from the current time via `aspen_time::current_time_ms()`.
pub fn write_command_to_app_request(command: &WriteCommand) -> AppRequest {
    validate_key_bearing_command(command);
    debug_assert!(
        !matches!(
            command,
            WriteCommand::SetMulti { pairs }
                | WriteCommand::SetMultiWithTTL { pairs, .. }
                | WriteCommand::SetMultiWithLease { pairs, .. }
                if pairs.len() > max_setmulti_keys_usize()
        ),
        "COMMAND_CONVERT: multi-key command exceeds MAX_SETMULTI_KEYS"
    );

    match group_write_command(command) {
        WriteCommandGroup::Direct(command) => convert_direct_write_command(command),
        WriteCommandGroup::BatchLease(command) => convert_batch_lease_write_command(command),
        WriteCommandGroup::Advanced(command) => convert_advanced_write_command(command),
    }
}

/// Convert a TTL-based set into an `AppRequest` with absolute expiration timestamp.
fn convert_set_with_ttl(request: SetWithTtlRequest<'_>) -> AppRequest {
    // Tiger Style: TTL must be positive
    assert!(request.ttl_seconds > 0, "SET_WITH_TTL: ttl_seconds must be positive, got 0");
    // Tiger Style: key must not be empty
    assert!(!request.key.is_empty(), "SET_WITH_TTL: key must not be empty");

    let now_ms = aspen_time::current_time_ms();
    let expires_at_ms = now_ms.saturating_add(ttl_duration_ms(request.ttl_seconds));

    // Tiger Style: computed expiration must be in the future
    debug_assert!(expires_at_ms > now_ms, "SET_WITH_TTL: expires_at_ms {expires_at_ms} must be > now_ms {now_ms}");

    AppRequest::SetWithTTL {
        key: request.key.to_owned(),
        value: request.value.to_owned(),
        expires_at_ms,
    }
}

/// Convert a TTL-based multi-set into an `AppRequest` with absolute expiration timestamp.
fn convert_set_multi_with_ttl(pairs: &[(String, String)], ttl_seconds: u32) -> AppRequest {
    // Tiger Style: TTL must be positive
    assert!(ttl_seconds > 0, "SET_MULTI_WITH_TTL: ttl_seconds must be positive, got 0");
    // Tiger Style: pairs must be bounded
    assert!(
        pairs.len() <= max_setmulti_keys_usize(),
        "SET_MULTI_WITH_TTL: {} pairs exceeds MAX_SETMULTI_KEYS {}",
        pairs.len(),
        MAX_SETMULTI_KEYS
    );

    let now_ms = aspen_time::current_time_ms();
    let expires_at_ms = now_ms.saturating_add(ttl_duration_ms(ttl_seconds));
    AppRequest::SetMultiWithTTL {
        pairs: pairs.to_vec(),
        expires_at_ms,
    }
}

/// Convert `BatchOperation` items into the compact tuple format used by `AppRequest::Batch`.
///
/// Format: `(is_set, key, value)` where `is_set=true` for Set, `false` for Delete.
fn convert_batch_ops(operations: &[BatchOperation]) -> Vec<(bool, String, String)> {
    operations
        .iter()
        .map(|op| match op {
            BatchOperation::Set { key, value } => (true, key.clone(), value.clone()),
            BatchOperation::Delete { key } => (false, key.clone(), String::new()),
        })
        .collect()
}

/// Convert `BatchCondition` items into the compact tuple format.
///
/// Format: `(type, key, expected)` where type: 0=ValueEquals, 1=KeyExists, 2=KeyNotExists.
fn convert_batch_conditions(conditions: &[BatchCondition]) -> Vec<(u8, String, String)> {
    conditions
        .iter()
        .map(|c| match c {
            BatchCondition::ValueEquals { key, expected } => (0, key.clone(), expected.clone()),
            BatchCondition::KeyExists { key } => (1, key.clone(), String::new()),
            BatchCondition::KeyNotExists { key } => (2, key.clone(), String::new()),
        })
        .collect()
}

/// Convert a conditional batch into an `AppRequest`.
fn convert_conditional_batch(conditions: &[BatchCondition], operations: &[BatchOperation]) -> AppRequest {
    AppRequest::ConditionalBatch {
        conditions: convert_batch_conditions(conditions),
        operations: convert_batch_ops(operations),
    }
}

/// Convert `TxnOp` items into the compact tuple format.
///
/// Format: `(op_type, key, value)` where op_type: 0=Put, 1=Delete, 2=Get, 3=Range.
fn convert_txn_ops(ops: &[TxnOp]) -> Vec<(u8, String, String)> {
    ops.iter()
        .map(|op| match op {
            TxnOp::Put { key, value } => (0, key.clone(), value.clone()),
            TxnOp::Delete { key } => (1, key.clone(), String::new()),
            TxnOp::Get { key } => (2, key.clone(), String::new()),
            TxnOp::Range { prefix, limit } => (3, prefix.clone(), limit.to_string()),
        })
        .collect()
}

/// Convert compare conditions into the compact tuple format.
///
/// Format: `(target, op, key, value)` where:
/// - target: 0=Value, 1=Version, 2=CreateRevision, 3=ModRevision
/// - op: 0=Equal, 1=NotEqual, 2=Greater, 3=Less
fn convert_compare_conditions(compare: &[TxnCompare]) -> Vec<(u8, u8, String, String)> {
    compare
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
        .collect()
}

/// Convert a Transaction command into an `AppRequest`.
fn convert_transaction(compare: &[TxnCompare], success: &[TxnOp], failure: &[TxnOp]) -> AppRequest {
    AppRequest::Transaction {
        compare: convert_compare_conditions(compare),
        success: convert_txn_ops(success),
        failure: convert_txn_ops(failure),
    }
}

/// Convert `WriteOp` items into the compact tuple format.
///
/// Format: `(is_set, key, value)` where `is_set=true` for Set, `false` for Delete.
fn convert_write_ops(write_set: &[WriteOp]) -> Vec<(bool, String, String)> {
    write_set
        .iter()
        .map(|op| match op {
            WriteOp::Set { key, value } => (true, key.clone(), value.clone()),
            WriteOp::Delete { key } => (false, key.clone(), String::new()),
        })
        .collect()
}

/// Convert an OptimisticTransaction command into an `AppRequest`.
fn convert_optimistic_txn(read_set: &[(String, i64)], write_set: &[WriteOp]) -> AppRequest {
    AppRequest::OptimisticTransaction {
        read_set: read_set.to_vec(),
        write_set: convert_write_ops(write_set),
    }
}
