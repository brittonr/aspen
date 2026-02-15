//! Main dispatch for applying AppRequest entries to the state machine.

use super::super::*;

impl SharedRedbStorage {
    /// Apply a single AppRequest to the state machine tables within a transaction.
    pub(in crate::storage_shared) fn apply_request_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        leases_table: &mut redb::Table<u64, &[u8]>,
        request: &AppRequest,
        log_index: u64,
    ) -> Result<AppResponse, SharedStorageError> {
        match request {
            AppRequest::Set { key, value } => Self::apply_set_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                key,
                value,
                log_index,
                None,
                None,
            ),
            AppRequest::SetWithTTL {
                key,
                value,
                expires_at_ms,
            } => Self::apply_set_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                key,
                value,
                log_index,
                Some(*expires_at_ms),
                None,
            ),
            AppRequest::SetMulti { pairs } => Self::apply_set_multi_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                pairs,
                log_index,
                None,
                None,
            ),
            AppRequest::SetMultiWithTTL { pairs, expires_at_ms } => Self::apply_set_multi_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                pairs,
                log_index,
                Some(*expires_at_ms),
                None,
            ),
            AppRequest::Delete { key } => Self::apply_delete_in_txn(kv_table, index_table, index_registry, key),
            AppRequest::DeleteMulti { keys } => {
                Self::apply_delete_multi_in_txn(kv_table, index_table, index_registry, keys)
            }
            AppRequest::CompareAndSwap {
                key,
                expected,
                new_value,
            } => Self::apply_compare_and_swap_in_txn(
                kv_table,
                index_table,
                index_registry,
                key,
                expected.as_deref(),
                new_value,
                log_index,
            ),
            AppRequest::CompareAndDelete { key, expected } => {
                Self::apply_compare_and_delete_in_txn(kv_table, index_table, index_registry, key, expected)
            }
            AppRequest::Batch { operations } => {
                Self::apply_batch_in_txn(kv_table, index_table, index_registry, leases_table, operations, log_index)
            }
            AppRequest::ConditionalBatch { conditions, operations } => Self::apply_conditional_batch_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                conditions,
                operations,
                log_index,
            ),
            // Lease operations
            AppRequest::SetWithLease { key, value, lease_id } => Self::apply_set_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                key,
                value,
                log_index,
                None,
                Some(*lease_id),
            ),
            AppRequest::SetMultiWithLease { pairs, lease_id } => Self::apply_set_multi_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                pairs,
                log_index,
                None,
                Some(*lease_id),
            ),
            // Lease operations
            AppRequest::LeaseGrant { lease_id, ttl_seconds } => {
                Self::apply_lease_grant_in_txn(leases_table, *lease_id, *ttl_seconds)
            }
            AppRequest::LeaseRevoke { lease_id } => {
                Self::apply_lease_revoke_in_txn(kv_table, index_table, index_registry, leases_table, *lease_id)
            }
            AppRequest::LeaseKeepalive { lease_id } => Self::apply_lease_keepalive_in_txn(leases_table, *lease_id),
            // Transaction operations
            AppRequest::Transaction {
                compare,
                success,
                failure,
            } => Self::apply_transaction_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                compare,
                success,
                failure,
                log_index,
            ),
            AppRequest::OptimisticTransaction { read_set, write_set } => Self::apply_optimistic_transaction_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                read_set,
                write_set,
                log_index,
            ),
            // Shard operations - pass through without state changes
            AppRequest::ShardSplit { .. } | AppRequest::ShardMerge { .. } | AppRequest::TopologyUpdate { .. } => {
                Ok(AppResponse::default())
            }
        }
    }
}
