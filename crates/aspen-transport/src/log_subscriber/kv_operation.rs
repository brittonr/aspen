//! Key-value operations that appear in the Raft log.

use serde::Deserialize;
use serde::Serialize;

/// Key-value operations that appear in the log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KvOperation {
    /// Set a single key-value pair.
    Set {
        /// Key to set.
        key: Vec<u8>,
        /// Value to store.
        value: Vec<u8>,
    },
    /// Set a single key-value pair with expiration.
    SetWithTTL {
        /// Key to set.
        key: Vec<u8>,
        /// Value to store.
        value: Vec<u8>,
        /// Expiration time as Unix timestamp in milliseconds.
        expires_at_ms: u64,
    },
    /// Set multiple key-value pairs atomically.
    SetMulti {
        /// Key-value pairs to set.
        pairs: Vec<(Vec<u8>, Vec<u8>)>,
    },
    /// Set multiple keys with TTL.
    SetMultiWithTTL {
        /// Key-value pairs to set.
        pairs: Vec<(Vec<u8>, Vec<u8>)>,
        /// Expiration time as Unix timestamp in milliseconds.
        expires_at_ms: u64,
    },
    /// Delete a single key.
    Delete {
        /// Key to delete.
        key: Vec<u8>,
    },
    /// Delete multiple keys atomically.
    DeleteMulti {
        /// Keys to delete.
        keys: Vec<Vec<u8>>,
    },
    /// Compare-and-swap: atomically update value if current matches expected.
    CompareAndSwap {
        /// Key to update.
        key: Vec<u8>,
        /// Expected current value (None means key must not exist).
        expected: Option<Vec<u8>>,
        /// New value to set if condition passes.
        new_value: Vec<u8>,
    },
    /// Compare-and-delete: atomically delete if current value matches expected.
    CompareAndDelete {
        /// Key to delete.
        key: Vec<u8>,
        /// Expected current value.
        expected: Vec<u8>,
    },
    /// Batch of mixed Set/Delete operations.
    Batch {
        /// Operations as (is_set, key, value) tuples.
        operations: Vec<(bool, Vec<u8>, Vec<u8>)>,
    },
    /// Conditional batch: operations applied only if all conditions pass.
    ConditionalBatch {
        /// Conditions as (type, key, expected) tuples.
        /// Types: 0=ValueEquals, 1=KeyExists, 2=KeyNotExists.
        conditions: Vec<(u8, Vec<u8>, Vec<u8>)>,
        /// Operations as (is_set, key, value) tuples.
        operations: Vec<(bool, Vec<u8>, Vec<u8>)>,
    },
    /// Set a single key-value pair attached to a lease.
    SetWithLease {
        /// Key to set.
        key: Vec<u8>,
        /// Value to store.
        value: Vec<u8>,
        /// Lease ID this key is attached to.
        lease_id: u64,
    },
    /// Set multiple keys attached to a lease.
    SetMultiWithLease {
        /// Key-value pairs to set.
        pairs: Vec<(Vec<u8>, Vec<u8>)>,
        /// Lease ID these keys are attached to.
        lease_id: u64,
    },
    /// Grant a new lease.
    LeaseGrant {
        /// Lease ID (0 = auto-generated).
        lease_id: u64,
        /// TTL in seconds.
        ttl_seconds: u32,
    },
    /// Revoke a lease and delete all attached keys.
    LeaseRevoke {
        /// Lease ID to revoke.
        lease_id: u64,
    },
    /// Refresh a lease's TTL.
    LeaseKeepalive {
        /// Lease ID to refresh.
        lease_id: u64,
    },
    /// Multi-key transaction with If/Then/Else semantics.
    Transaction {
        /// Comparison conditions as (target, op, key, value).
        compare: Vec<(u8, u8, Vec<u8>, Vec<u8>)>,
        /// Success branch operations as (op_type, key, value).
        success: Vec<(u8, Vec<u8>, Vec<u8>)>,
        /// Failure branch operations.
        failure: Vec<(u8, Vec<u8>, Vec<u8>)>,
    },
    /// Optimistic transaction with read set version validation.
    OptimisticTransaction {
        /// Read set: keys with expected versions for conflict detection.
        read_set: Vec<(Vec<u8>, i64)>,
        /// Write set: operations to apply if no conflicts.
        /// Format: (is_set, key, value). is_set=true for Set, false for Delete.
        write_set: Vec<(bool, Vec<u8>, Vec<u8>)>,
    },
    /// No-op entry (used for leader election).
    Noop,
    /// Membership change entry.
    MembershipChange {
        /// Human-readable description of the change.
        description: String,
    },
}

impl KvOperation {
    /// Returns true if this operation affects the given key prefix.
    pub fn matches_prefix(&self, prefix: &[u8]) -> bool {
        if prefix.is_empty() {
            return true;
        }

        match self {
            KvOperation::Set { key, .. }
            | KvOperation::SetWithTTL { key, .. }
            | KvOperation::SetWithLease { key, .. }
            | KvOperation::Delete { key }
            | KvOperation::CompareAndSwap { key, .. }
            | KvOperation::CompareAndDelete { key, .. } => key.starts_with(prefix),
            KvOperation::SetMulti { pairs }
            | KvOperation::SetMultiWithTTL { pairs, .. }
            | KvOperation::SetMultiWithLease { pairs, .. } => pairs.iter().any(|(k, _)| k.starts_with(prefix)),
            KvOperation::DeleteMulti { keys } => keys.iter().any(|k| k.starts_with(prefix)),
            KvOperation::Batch { operations } | KvOperation::ConditionalBatch { operations, .. } => {
                operations.iter().any(|(_, k, _)| k.starts_with(prefix))
            }
            KvOperation::Noop
            | KvOperation::MembershipChange { .. }
            | KvOperation::LeaseGrant { .. }
            | KvOperation::LeaseRevoke { .. }
            | KvOperation::LeaseKeepalive { .. } => {
                // Always include control/lease operations
                true
            }
            KvOperation::Transaction { success, failure, .. } => {
                // Check if any operation in success or failure branches affects the prefix
                let check_ops = |ops: &[(u8, Vec<u8>, Vec<u8>)]| {
                    ops.iter().any(|(op_type, key, _)| {
                        // op_type: 0=Put, 1=Delete, 2=Get, 3=Range
                        // Put and Delete modify keys, Get and Range just read
                        (*op_type == 0 || *op_type == 1) && key.starts_with(prefix)
                    })
                };
                check_ops(success) || check_ops(failure)
            }
            KvOperation::OptimisticTransaction { write_set, .. } => {
                // Check if any write operation affects the prefix
                write_set.iter().any(|(_, key, _)| key.starts_with(prefix))
            }
        }
    }

    /// Returns the primary key affected by this operation, if any.
    pub fn primary_key(&self) -> Option<&[u8]> {
        match self {
            KvOperation::Set { key, .. }
            | KvOperation::SetWithTTL { key, .. }
            | KvOperation::SetWithLease { key, .. }
            | KvOperation::Delete { key }
            | KvOperation::CompareAndSwap { key, .. }
            | KvOperation::CompareAndDelete { key, .. } => Some(key),
            KvOperation::SetMulti { pairs }
            | KvOperation::SetMultiWithTTL { pairs, .. }
            | KvOperation::SetMultiWithLease { pairs, .. } => pairs.first().map(|(k, _)| k.as_slice()),
            KvOperation::DeleteMulti { keys } => keys.first().map(|k| k.as_slice()),
            KvOperation::Batch { operations } | KvOperation::ConditionalBatch { operations, .. } => {
                operations.first().map(|(_, k, _)| k.as_slice())
            }
            KvOperation::Noop
            | KvOperation::MembershipChange { .. }
            | KvOperation::LeaseGrant { .. }
            | KvOperation::LeaseRevoke { .. }
            | KvOperation::LeaseKeepalive { .. } => None,
            KvOperation::Transaction { success, .. } => {
                // Return the first key from success branch
                success.first().map(|(_, k, _)| k.as_slice())
            }
            KvOperation::OptimisticTransaction { write_set, .. } => {
                // Return the first key from write set
                write_set.first().map(|(_, k, _)| k.as_slice())
            }
        }
    }

    /// Returns the number of keys affected by this operation.
    pub fn key_count(&self) -> usize {
        match self {
            KvOperation::Set { .. }
            | KvOperation::SetWithTTL { .. }
            | KvOperation::SetWithLease { .. }
            | KvOperation::Delete { .. }
            | KvOperation::CompareAndSwap { .. }
            | KvOperation::CompareAndDelete { .. } => 1,
            KvOperation::SetMulti { pairs }
            | KvOperation::SetMultiWithTTL { pairs, .. }
            | KvOperation::SetMultiWithLease { pairs, .. } => pairs.len(),
            KvOperation::DeleteMulti { keys } => keys.len(),
            KvOperation::Batch { operations } | KvOperation::ConditionalBatch { operations, .. } => operations.len(),
            KvOperation::Noop
            | KvOperation::MembershipChange { .. }
            | KvOperation::LeaseGrant { .. }
            | KvOperation::LeaseRevoke { .. }
            | KvOperation::LeaseKeepalive { .. } => 0,
            KvOperation::Transaction { success, failure, .. } => {
                // Count put/delete operations in both branches (get/range don't modify keys)
                let count_mods = |ops: &[(u8, Vec<u8>, Vec<u8>)]| {
                    ops.iter().filter(|(op_type, _, _)| *op_type == 0 || *op_type == 1).count()
                };
                count_mods(success) + count_mods(failure)
            }
            KvOperation::OptimisticTransaction { write_set, .. } => write_set.len(),
        }
    }
}

impl From<aspen_raft_types::AppRequest> for KvOperation {
    fn from(req: aspen_raft_types::AppRequest) -> Self {
        use aspen_raft_types::AppRequest;
        match req {
            AppRequest::Set { key, value } => KvOperation::Set {
                key: key.into_bytes(),
                value: value.into_bytes(),
            },
            AppRequest::SetWithTTL {
                key,
                value,
                expires_at_ms,
            } => KvOperation::SetWithTTL {
                key: key.into_bytes(),
                value: value.into_bytes(),
                expires_at_ms,
            },
            AppRequest::SetMulti { pairs } => KvOperation::SetMulti {
                pairs: pairs.into_iter().map(|(k, v)| (k.into_bytes(), v.into_bytes())).collect(),
            },
            AppRequest::SetMultiWithTTL { pairs, expires_at_ms } => KvOperation::SetMultiWithTTL {
                pairs: pairs.into_iter().map(|(k, v)| (k.into_bytes(), v.into_bytes())).collect(),
                expires_at_ms,
            },
            AppRequest::Delete { key } => KvOperation::Delete { key: key.into_bytes() },
            AppRequest::DeleteMulti { keys } => KvOperation::DeleteMulti {
                keys: keys.into_iter().map(String::into_bytes).collect(),
            },
            AppRequest::CompareAndSwap {
                key,
                expected,
                new_value,
            } => KvOperation::CompareAndSwap {
                key: key.into_bytes(),
                expected: expected.map(String::into_bytes),
                new_value: new_value.into_bytes(),
            },
            AppRequest::CompareAndDelete { key, expected } => KvOperation::CompareAndDelete {
                key: key.into_bytes(),
                expected: expected.into_bytes(),
            },
            AppRequest::Batch { operations } => KvOperation::Batch {
                operations: operations
                    .into_iter()
                    .map(|(is_set, key, value)| (is_set, key.into_bytes(), value.into_bytes()))
                    .collect(),
            },
            AppRequest::ConditionalBatch { conditions, operations } => KvOperation::ConditionalBatch {
                conditions: conditions
                    .into_iter()
                    .map(|(cond_type, key, expected)| (cond_type, key.into_bytes(), expected.into_bytes()))
                    .collect(),
                operations: operations
                    .into_iter()
                    .map(|(is_set, key, value)| (is_set, key.into_bytes(), value.into_bytes()))
                    .collect(),
            },
            AppRequest::SetWithLease { key, value, lease_id } => KvOperation::SetWithLease {
                key: key.into_bytes(),
                value: value.into_bytes(),
                lease_id,
            },
            AppRequest::SetMultiWithLease { pairs, lease_id } => KvOperation::SetMultiWithLease {
                pairs: pairs.into_iter().map(|(k, v)| (k.into_bytes(), v.into_bytes())).collect(),
                lease_id,
            },
            AppRequest::LeaseGrant { lease_id, ttl_seconds } => KvOperation::LeaseGrant { lease_id, ttl_seconds },
            AppRequest::LeaseRevoke { lease_id } => KvOperation::LeaseRevoke { lease_id },
            AppRequest::LeaseKeepalive { lease_id } => KvOperation::LeaseKeepalive { lease_id },
            AppRequest::Transaction {
                compare,
                success,
                failure,
            } => {
                // Convert String-based tuples to Vec<u8>-based tuples
                let cmp = compare.into_iter().map(|(t, o, k, v)| (t, o, k.into_bytes(), v.into_bytes())).collect();
                let succ = success.into_iter().map(|(t, k, v)| (t, k.into_bytes(), v.into_bytes())).collect();
                let fail = failure.into_iter().map(|(t, k, v)| (t, k.into_bytes(), v.into_bytes())).collect();
                KvOperation::Transaction {
                    compare: cmp,
                    success: succ,
                    failure: fail,
                }
            }
            AppRequest::OptimisticTransaction { read_set, write_set } => {
                let reads = read_set.into_iter().map(|(k, v)| (k.into_bytes(), v)).collect();
                let writes =
                    write_set.into_iter().map(|(is_set, k, v)| (is_set, k.into_bytes(), v.into_bytes())).collect();
                KvOperation::OptimisticTransaction {
                    read_set: reads,
                    write_set: writes,
                }
            }
            // Shard topology operations are converted to Noop for log subscribers
            // (subscribers don't need to track topology changes)
            AppRequest::ShardSplit { .. } | AppRequest::ShardMerge { .. } | AppRequest::TopologyUpdate { .. } => {
                KvOperation::Noop
            }
        }
    }
}
