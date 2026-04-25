//! Lease operations: lease query, cleanup, and management methods.

use redb::ReadableTable;
use snafu::ResultExt;

use super::BeginReadSnafu;
use super::BeginWriteSnafu;
use super::CommitSnafu;
use super::DeserializeSnafu;
use super::GetSnafu;
use super::LeaseEntry;
use super::OpenTableSnafu;
use super::RangeSnafu;
use super::RedbKvStorage;
use super::RemoveSnafu;
use super::SharedStorageError;
use super::SM_KV_TABLE;
use super::SM_LEASES_TABLE;

#[inline]
fn now_ms() -> u64 {
    aspen_time::current_time_ms()
}

#[inline]
fn remaining_ttl_seconds_u32(remaining_ms: u64) -> u32 {
    u32::try_from(remaining_ms / 1000).unwrap_or(u32::MAX)
}

#[inline]
fn batch_limit_usize(batch_limit: u32) -> usize {
    usize::try_from(batch_limit).unwrap_or(usize::MAX)
}

impl RedbKvStorage {
    pub fn get_lease(&self, lease_id: u64) -> Result<Option<(u32, u32)>, SharedStorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SM_LEASES_TABLE).context(OpenTableSnafu)?;

        let current_ms = now_ms();

        match table.get(lease_id).context(GetSnafu)? {
            Some(value) => {
                let entry: LeaseEntry = bincode::deserialize(value.value()).context(DeserializeSnafu)?;

                if current_ms > entry.expires_at_ms {
                    return Ok(None);
                }

                let remaining_ms = entry.expires_at_ms.saturating_sub(current_ms);
                let remaining_seconds = remaining_ttl_seconds_u32(remaining_ms);

                Ok(Some((entry.ttl_seconds, remaining_seconds)))
            }
            None => Ok(None),
        }
    }

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

    pub fn list_leases(&self) -> Result<Vec<(u64, u32, u32)>, SharedStorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SM_LEASES_TABLE).context(OpenTableSnafu)?;

        let current_ms = now_ms();
        let active_count = usize::try_from(self.count_active_leases()?).unwrap_or(usize::MAX);
        let mut leases = Vec::with_capacity(active_count);

        for item in table.iter().context(RangeSnafu)? {
            let (id_guard, value_guard) = item.context(GetSnafu)?;
            let lease_id = id_guard.value();
            let entry: LeaseEntry = bincode::deserialize(value_guard.value()).context(DeserializeSnafu)?;

            if current_ms > entry.expires_at_ms {
                continue;
            }

            let remaining_ms = entry.expires_at_ms.saturating_sub(current_ms);
            let remaining_seconds = remaining_ttl_seconds_u32(remaining_ms);

            leases.push((lease_id, entry.ttl_seconds, remaining_seconds));
        }

        Ok(leases)
    }

    pub fn delete_expired_leases(&self, batch_limit: u32) -> Result<u32, SharedStorageError> {
        let current_ms = now_ms();
        let mut deleted: u32 = 0;

        let expired_leases: Vec<(u64, Vec<String>)> = {
            let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
            let table = read_txn.open_table(SM_LEASES_TABLE).context(OpenTableSnafu)?;

            let mut expired = Vec::with_capacity(batch_limit_usize(batch_limit));
            for item in table.iter().context(RangeSnafu)? {
                if deleted >= batch_limit {
                    break;
                }
                let (id_guard, value_guard) = item.context(GetSnafu)?;
                let lease_id = id_guard.value();
                let entry: LeaseEntry = bincode::deserialize(value_guard.value()).context(DeserializeSnafu)?;

                if current_ms > entry.expires_at_ms {
                    expired.push((lease_id, entry.keys.clone()));
                    deleted += 1;
                }
            }
            expired
        };

        if expired_leases.is_empty() {
            return Ok(0);
        }

        let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
        {
            let mut leases_table = write_txn.open_table(SM_LEASES_TABLE).context(OpenTableSnafu)?;
            let mut kv_table = write_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;

            for (lease_id, attached_keys) in &expired_leases {
                for key in attached_keys {
                    kv_table.remove(key.as_bytes()).context(RemoveSnafu)?;
                }
                leases_table.remove(*lease_id).context(RemoveSnafu)?;
            }
        }
        write_txn.commit().context(CommitSnafu)?;

        Ok(deleted)
    }

    pub fn count_expired_leases(&self) -> Result<u64, SharedStorageError> {
        let current_ms = now_ms();
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SM_LEASES_TABLE).context(OpenTableSnafu)?;

        let mut count: u64 = 0;
        for item in table.iter().context(RangeSnafu)? {
            let (_id_guard, value_guard) = item.context(GetSnafu)?;
            let entry: LeaseEntry = bincode::deserialize(value_guard.value()).context(DeserializeSnafu)?;

            if current_ms > entry.expires_at_ms {
                count += 1;
            }
        }

        Ok(count)
    }

    pub fn count_active_leases(&self) -> Result<u64, SharedStorageError> {
        let current_ms = now_ms();
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SM_LEASES_TABLE).context(OpenTableSnafu)?;

        let mut count: u64 = 0;
        for item in table.iter().context(RangeSnafu)? {
            let (_id_guard, value_guard) = item.context(GetSnafu)?;
            let entry: LeaseEntry = bincode::deserialize(value_guard.value()).context(DeserializeSnafu)?;

            if current_ms <= entry.expires_at_ms {
                count += 1;
            }
        }

        Ok(count)
    }
}
