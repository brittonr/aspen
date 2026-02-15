//! Lease grant, revoke, and keepalive apply helpers for the state machine.

use redb::ReadableTable;
use snafu::ResultExt;

use super::super::*;

impl SharedRedbStorage {
    /// Apply a lease grant operation within a transaction.
    pub(in crate::storage_shared) fn apply_lease_grant_in_txn(
        leases_table: &mut redb::Table<u64, &[u8]>,
        lease_id: u64,
        ttl_seconds: u32,
    ) -> Result<AppResponse, SharedStorageError> {
        // Tiger Style: TTL must be positive
        assert!(ttl_seconds > 0, "LEASE GRANT: ttl_seconds must be positive, got 0");

        // Generate lease_id if not provided (0 means auto-generate)
        let actual_lease_id = if lease_id == 0 {
            // Simple ID generation using timestamp + random component
            let now = now_unix_ms();
            let random_component = now % 1000000;
            now * 1000 + random_component
        } else {
            lease_id
        };

        // Use pure function to create lease entry data
        let lease_data = create_lease_entry(ttl_seconds, now_unix_ms());

        // Create lease entry from pure computed data
        let lease_entry = LeaseEntry {
            ttl_seconds: lease_data.ttl_seconds,
            expires_at_ms: lease_data.expires_at_ms,
            keys: lease_data.keys, // Keys will be added when SetWithLease is called
        };

        // Tiger Style: actual_lease_id must be non-zero after generation
        assert!(actual_lease_id > 0, "LEASE GRANT: generated lease_id must be non-zero");
        // Tiger Style: expires_at_ms must be in the future relative to computed data
        debug_assert!(lease_entry.expires_at_ms > 0, "LEASE GRANT: expires_at_ms must be positive");

        // Store lease
        let lease_bytes = bincode::serialize(&lease_entry).context(SerializeSnafu)?;
        leases_table.insert(actual_lease_id, lease_bytes.as_slice()).context(InsertSnafu)?;

        Ok(AppResponse {
            lease_id: Some(actual_lease_id),
            ttl_seconds: Some(ttl_seconds),
            ..Default::default()
        })
    }

    /// Apply a lease revoke operation within a transaction.
    pub(in crate::storage_shared) fn apply_lease_revoke_in_txn(
        kv_table: &mut redb::Table<&[u8], &[u8]>,
        index_table: &mut redb::Table<&[u8], &[u8]>,
        index_registry: &IndexRegistry,
        leases_table: &mut redb::Table<u64, &[u8]>,
        lease_id: u64,
    ) -> Result<AppResponse, SharedStorageError> {
        // Tiger Style: lease_id must be non-zero
        assert!(lease_id > 0, "LEASE REVOKE: lease_id must be non-zero");

        // Get the lease entry and extract the keys
        let keys_to_delete = if let Some(lease_data) = leases_table.get(lease_id).context(GetSnafu)? {
            let lease_entry: LeaseEntry = bincode::deserialize(lease_data.value()).context(DeserializeSnafu)?;
            lease_entry.keys.clone()
        } else {
            Vec::new()
        };

        let mut keys_deleted = 0u32;

        // Delete all keys attached to this lease
        for key in &keys_to_delete {
            let result = Self::apply_delete_in_txn(kv_table, index_table, index_registry, key)?;
            if result.deleted.unwrap_or(false) {
                keys_deleted += 1;
            }
        }

        // Delete the lease itself if it exists
        if !keys_to_delete.is_empty() {
            leases_table.remove(lease_id).context(RemoveSnafu)?;
        }

        Ok(AppResponse {
            keys_deleted: Some(keys_deleted),
            ..Default::default()
        })
    }

    /// Apply a lease keepalive operation within a transaction.
    pub(in crate::storage_shared) fn apply_lease_keepalive_in_txn(
        leases_table: &mut redb::Table<u64, &[u8]>,
        lease_id: u64,
    ) -> Result<AppResponse, SharedStorageError> {
        // Tiger Style: lease_id must be non-zero
        assert!(lease_id > 0, "LEASE KEEPALIVE: lease_id must be non-zero");

        // Get the existing lease and extract data to avoid borrow conflicts
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
            // Use pure function to compute new expiration time
            let ttl = lease_entry.ttl_seconds;
            lease_entry.expires_at_ms = compute_lease_refresh(ttl, now_unix_ms());

            // Update the lease
            let updated_bytes = bincode::serialize(&lease_entry).context(SerializeSnafu)?;
            leases_table.insert(lease_id, updated_bytes.as_slice()).context(InsertSnafu)?;

            Ok(AppResponse {
                lease_id: Some(lease_id),
                ttl_seconds: Some(ttl),
                ..Default::default()
            })
        } else {
            // Lease not found - return None for lease_id to indicate not found
            Ok(AppResponse {
                lease_id: None,
                ..Default::default()
            })
        }
    }
}
