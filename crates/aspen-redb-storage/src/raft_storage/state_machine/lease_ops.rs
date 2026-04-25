//! Lease grant, revoke, and keepalive apply helpers for the state machine.

use aspen_layer::IndexRegistry;
use aspen_raft_kv_types::RaftKvResponse;
use redb::ReadableTable;
use snafu::ResultExt;

use super::super::DeserializeSnafu;
use super::super::GetSnafu;
use super::super::InsertSnafu;
use super::super::LeaseEntry;
use super::super::RedbKvStorage;
use super::super::RemoveSnafu;
use super::super::SerializeSnafu;
use super::super::SharedStorageError;
use super::set::empty_response;

use crate::compute_lease_refresh;
use crate::create_lease_entry;

#[inline]
fn now_ms() -> u64 {
    aspen_time::current_time_ms()
}

impl RedbKvStorage {
    pub(in crate::raft_storage) fn apply_lease_grant_in_txn(
        leases_table: &mut redb::Table<u64, &[u8]>,
        lease_id: u64,
        ttl_seconds: u32,
    ) -> Result<RaftKvResponse, SharedStorageError> {
        assert!(ttl_seconds > 0, "LEASE GRANT: ttl_seconds must be positive, got 0");

        let actual_lease_id = if lease_id == 0 {
            let now = now_ms();
            let random_component = now % 1_000_000;
            now.saturating_mul(1_000).saturating_add(random_component)
        } else {
            lease_id
        };

        let lease_data = create_lease_entry(ttl_seconds, now_ms());

        let lease_entry = LeaseEntry {
            ttl_seconds: lease_data.ttl_seconds,
            expires_at_ms: lease_data.expires_at_ms,
            keys: lease_data.keys,
        };

        assert!(actual_lease_id > 0, "LEASE GRANT: generated lease_id must be non-zero");

        let lease_bytes = bincode::serialize(&lease_entry).context(SerializeSnafu)?;
        leases_table.insert(actual_lease_id, lease_bytes.as_slice()).context(InsertSnafu)?;

        Ok(RaftKvResponse {
            lease_id: Some(actual_lease_id),
            ttl_seconds: Some(ttl_seconds),
            ..empty_response()
        })
    }

    pub(in crate::raft_storage) fn apply_lease_revoke_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        leases_table: &mut redb::Table<u64, &[u8]>,
        lease_id: u64,
    ) -> Result<RaftKvResponse, SharedStorageError> {
        assert!(lease_id > 0, "LEASE REVOKE: lease_id must be non-zero");

        let keys_to_delete = if let Some(lease_data) = leases_table.get(lease_id).context(GetSnafu)? {
            let lease_entry: LeaseEntry = bincode::deserialize(lease_data.value()).context(DeserializeSnafu)?;
            lease_entry.keys.clone()
        } else {
            Vec::new()
        };

        let mut keys_deleted = 0u32;

        for key in &keys_to_delete {
            let result = Self::apply_delete_in_txn(kv_table, index_table, index_registry, key)?;
            if result.deleted.unwrap_or(false) {
                keys_deleted += 1;
            }
        }

        if !keys_to_delete.is_empty() {
            leases_table.remove(lease_id).context(RemoveSnafu)?;
        }

        Ok(RaftKvResponse {
            keys_deleted: Some(keys_deleted),
            ..empty_response()
        })
    }

    pub(in crate::raft_storage) fn apply_lease_keepalive_in_txn(
        leases_table: &mut redb::Table<u64, &[u8]>,
        lease_id: u64,
    ) -> Result<RaftKvResponse, SharedStorageError> {
        assert!(lease_id > 0, "LEASE KEEPALIVE: lease_id must be non-zero");

        let lease_opt = {
            let lease_bytes = leases_table.get(lease_id).context(GetSnafu)?;
            if let Some(lease_data) = lease_bytes {
                let entry: LeaseEntry = bincode::deserialize(lease_data.value()).context(DeserializeSnafu)?;
                Some(entry)
            } else {
                None
            }
        };

        if let Some(mut lease_entry) = lease_opt {
            let ttl = lease_entry.ttl_seconds;
            lease_entry.expires_at_ms = compute_lease_refresh(ttl, now_ms());

            let updated_bytes = bincode::serialize(&lease_entry).context(SerializeSnafu)?;
            leases_table.insert(lease_id, updated_bytes.as_slice()).context(InsertSnafu)?;

            Ok(RaftKvResponse {
                lease_id: Some(lease_id),
                ttl_seconds: Some(ttl),
                ..empty_response()
            })
        } else {
            Ok(RaftKvResponse {
                lease_id: None,
                ..empty_response()
            })
        }
    }
}
