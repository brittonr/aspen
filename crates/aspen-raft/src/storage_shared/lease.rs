//! Lease operations: lease query, cleanup, and management methods.

use redb::ReadableTable;
use snafu::ResultExt;

use super::*;

impl SharedRedbStorage {
    /// Get lease information by ID.
    ///
    /// Returns (granted_ttl_seconds, remaining_ttl_seconds) if the lease exists and hasn't expired.
    pub fn get_lease(&self, lease_id: u64) -> Result<Option<(u32, u32)>, SharedStorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SM_LEASES_TABLE).context(OpenTableSnafu)?;

        let now_ms = now_unix_ms();

        match table.get(lease_id).context(GetSnafu)? {
            Some(value) => {
                let entry: LeaseEntry = bincode::deserialize(value.value()).context(DeserializeSnafu)?;

                // Check if lease has expired
                if now_ms > entry.expires_at_ms {
                    return Ok(None);
                }

                let remaining_ms = entry.expires_at_ms.saturating_sub(now_ms);
                let remaining_seconds = (remaining_ms / 1000) as u32;

                Ok(Some((entry.ttl_seconds, remaining_seconds)))
            }
            None => Ok(None),
        }
    }

    /// Get all keys attached to a lease.
    pub fn get_lease_keys(&self, lease_id: u64) -> Result<Vec<String>, SharedStorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SM_LEASES_TABLE).context(OpenTableSnafu)?;

        match table.get(lease_id).context(GetSnafu)? {
            Some(value) => {
                let entry: LeaseEntry = bincode::deserialize(value.value()).context(DeserializeSnafu)?;
                Ok(entry.keys)
            }
            None => Ok(vec![]),
        }
    }

    /// List all active (non-expired) leases.
    ///
    /// Returns a list of (lease_id, granted_ttl_seconds, remaining_ttl_seconds).
    pub fn list_leases(&self) -> Result<Vec<(u64, u32, u32)>, SharedStorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SM_LEASES_TABLE).context(OpenTableSnafu)?;

        let now_ms = now_unix_ms();
        let mut leases = Vec::new();

        for item in table.iter().context(RangeSnafu)? {
            let (id_guard, value_guard) = item.context(GetSnafu)?;
            let lease_id = id_guard.value();
            let entry: LeaseEntry = bincode::deserialize(value_guard.value()).context(DeserializeSnafu)?;

            // Skip expired leases
            if now_ms > entry.expires_at_ms {
                continue;
            }

            let remaining_ms = entry.expires_at_ms.saturating_sub(now_ms);
            let remaining_seconds = (remaining_ms / 1000) as u32;

            leases.push((lease_id, entry.ttl_seconds, remaining_seconds));
        }

        Ok(leases)
    }

    /// Delete expired leases and their attached keys in a batch.
    ///
    /// Returns the number of leases deleted.
    ///
    /// # Tiger Style
    ///
    /// - Fixed batch limit prevents unbounded work per call
    /// - Deletes both the lease and all keys attached to it
    /// - Idempotent: safe to call concurrently or repeatedly
    pub fn delete_expired_leases(&self, batch_limit: u32) -> Result<u32, SharedStorageError> {
        let now_ms = now_unix_ms();
        let mut deleted: u32 = 0;

        // First, collect expired lease IDs and their attached keys
        let expired_leases: Vec<(u64, Vec<String>)> = {
            let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
            let table = read_txn.open_table(SM_LEASES_TABLE).context(OpenTableSnafu)?;

            let mut expired = Vec::new();
            for item in table.iter().context(RangeSnafu)? {
                if deleted >= batch_limit {
                    break;
                }
                let (id_guard, value_guard) = item.context(GetSnafu)?;
                let lease_id = id_guard.value();
                let entry: LeaseEntry = bincode::deserialize(value_guard.value()).context(DeserializeSnafu)?;

                if now_ms > entry.expires_at_ms {
                    expired.push((lease_id, entry.keys.clone()));
                    deleted += 1;
                }
            }
            expired
        };

        if expired_leases.is_empty() {
            return Ok(0);
        }

        // Delete leases and their keys in a single transaction
        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut leases_table = write_txn.open_table(SM_LEASES_TABLE).context(OpenTableSnafu)?;
            let mut kv_table = write_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;

            for (lease_id, attached_keys) in &expired_leases {
                // Delete all keys attached to this lease
                for key in attached_keys {
                    let _ = kv_table.remove(key.as_bytes()).context(RemoveSnafu)?;
                }
                // Delete the lease itself
                let _ = leases_table.remove(*lease_id).context(RemoveSnafu)?;
            }
        }
        write_txn.commit().context(CommitSnafu)?;

        Ok(deleted)
    }

    /// Count the number of expired leases.
    ///
    /// Useful for metrics and monitoring.
    pub fn count_expired_leases(&self) -> Result<u64, SharedStorageError> {
        let now_ms = now_unix_ms();
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SM_LEASES_TABLE).context(OpenTableSnafu)?;

        let mut count: u64 = 0;
        for item in table.iter().context(RangeSnafu)? {
            let (_id_guard, value_guard) = item.context(GetSnafu)?;
            let entry: LeaseEntry = bincode::deserialize(value_guard.value()).context(DeserializeSnafu)?;

            if now_ms > entry.expires_at_ms {
                count += 1;
            }
        }

        Ok(count)
    }

    /// Count the number of active (non-expired) leases.
    ///
    /// Useful for metrics and monitoring.
    pub fn count_active_leases(&self) -> Result<u64, SharedStorageError> {
        let now_ms = now_unix_ms();
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SM_LEASES_TABLE).context(OpenTableSnafu)?;

        let mut count: u64 = 0;
        for item in table.iter().context(RangeSnafu)? {
            let (_id_guard, value_guard) = item.context(GetSnafu)?;
            let entry: LeaseEntry = bincode::deserialize(value_guard.value()).context(DeserializeSnafu)?;

            if now_ms <= entry.expires_at_ms {
                count += 1;
            }
        }

        Ok(count)
    }
}
