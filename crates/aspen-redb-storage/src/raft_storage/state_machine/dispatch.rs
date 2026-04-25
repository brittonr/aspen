//! Main dispatch for applying RaftKvRequest entries to the state machine.

use aspen_layer::IndexRegistry;
use aspen_raft_kv_types::RaftKvRequest;
use aspen_raft_kv_types::RaftKvResponse;

use super::super::RedbKvStorage;
use super::super::SharedStorageError;

impl RedbKvStorage {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::raft_storage) fn apply_request_in_txn(
        &self,
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        leases_table: &mut redb::Table<u64, &[u8]>,
        _sm_meta_table: &mut redb::Table<&str, &[u8]>,
        request: &RaftKvRequest,
        log_index: u64,
    ) -> Result<RaftKvResponse, SharedStorageError> {
        assert!(log_index > 0, "DISPATCH: log_index must be positive, got 0");

        match request {
            RaftKvRequest::Set { key, value } => Self::apply_set_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                super::set::SetOperationInput {
                    key,
                    value,
                    log_index,
                    expires_at_ms: None,
                    lease_id: None,
                },
            ),
            RaftKvRequest::SetWithTtl {
                key,
                value,
                expires_at_ms,
            } => Self::apply_set_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                super::set::SetOperationInput {
                    key,
                    value,
                    log_index,
                    expires_at_ms: Some(*expires_at_ms),
                    lease_id: None,
                },
            ),
            RaftKvRequest::SetMulti { pairs } => Self::apply_set_multi_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                super::set::SetMultiOperationInput {
                    pairs,
                    log_index,
                    expires_at_ms: None,
                    lease_id: None,
                },
            ),
            RaftKvRequest::SetMultiWithTtl { pairs, expires_at_ms } => Self::apply_set_multi_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                super::set::SetMultiOperationInput {
                    pairs,
                    log_index,
                    expires_at_ms: Some(*expires_at_ms),
                    lease_id: None,
                },
            ),
            RaftKvRequest::Delete { key } => {
                Self::apply_delete_in_txn(kv_table, index_table, index_registry, key)
            }
            RaftKvRequest::DeleteMulti { keys } => {
                Self::apply_delete_multi_in_txn(kv_table, index_table, index_registry, keys)
            }
            RaftKvRequest::CompareAndSwap {
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
            RaftKvRequest::CompareAndDelete { key, expected } => {
                Self::apply_compare_and_delete_in_txn(kv_table, index_table, index_registry, key, expected)
            }
            RaftKvRequest::Batch { operations } => Self::apply_batch_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                operations,
                log_index,
            ),
            RaftKvRequest::ConditionalBatch { conditions, operations } => Self::apply_conditional_batch_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                conditions,
                operations,
                log_index,
            ),
            RaftKvRequest::SetWithLease { key, value, lease_id } => Self::apply_set_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                super::set::SetOperationInput {
                    key,
                    value,
                    log_index,
                    expires_at_ms: None,
                    lease_id: Some(*lease_id),
                },
            ),
            RaftKvRequest::SetMultiWithLease { pairs, lease_id } => Self::apply_set_multi_in_txn(
                kv_table,
                index_table,
                index_registry,
                leases_table,
                super::set::SetMultiOperationInput {
                    pairs,
                    log_index,
                    expires_at_ms: None,
                    lease_id: Some(*lease_id),
                },
            ),
            RaftKvRequest::LeaseGrant { lease_id, ttl_seconds } => {
                Self::apply_lease_grant_in_txn(leases_table, *lease_id, *ttl_seconds)
            }
            RaftKvRequest::LeaseRevoke { lease_id } => {
                Self::apply_lease_revoke_in_txn(kv_table, index_table, index_registry, leases_table, *lease_id)
            }
            RaftKvRequest::LeaseKeepalive { lease_id } => {
                Self::apply_lease_keepalive_in_txn(leases_table, *lease_id)
            }
            RaftKvRequest::Transaction {
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
            RaftKvRequest::OptimisticTransaction { read_set, write_set } => {
                Self::apply_optimistic_transaction_in_txn(
                    kv_table,
                    index_table,
                    index_registry,
                    leases_table,
                    read_set,
                    write_set,
                    log_index,
                )
            }
        }
    }
}
