//! KV operations: get, scan, TTL cleanup, and related methods.

use aspen_kv_types::KeyValueWithRevision;
use aspen_storage_types::KvEntry;
use redb::ReadableTable;
use snafu::ResultExt;

use super::BeginReadSnafu;
use super::BeginWriteSnafu;
use super::CommitSnafu;
use super::DeserializeSnafu;
use super::GetSnafu;
use super::OpenTableSnafu;
use super::RangeSnafu;
use super::RedbKvStorage;
use super::RemoveSnafu;
use super::SharedStorageError;
use super::SM_KV_TABLE;

use aspen_constants::raft::MAX_BATCH_SIZE;

#[inline]
fn now_ms() -> u64 {
    aspen_time::current_time_ms()
}

#[inline]
fn bounded_batch_limit_usize(batch_limit: u32) -> usize {
    usize::try_from(batch_limit).unwrap_or(usize::MAX)
}

#[inline]
fn scan_limit_usize(limit: Option<u32>) -> usize {
    let max_results = limit.unwrap_or(MAX_BATCH_SIZE).min(MAX_BATCH_SIZE);
    bounded_batch_limit_usize(max_results)
}

impl RedbKvStorage {
    pub fn get(&self, key: &str) -> Result<Option<KvEntry>, SharedStorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;

        match table.get(key.as_bytes()).context(GetSnafu)? {
            Some(value) => {
                let entry: KvEntry = bincode::deserialize(value.value()).context(DeserializeSnafu)?;

                if let Some(expires_at) = entry.expires_at_ms
                    && now_ms() > expires_at
                {
                    return Ok(None);
                }

                Ok(Some(entry))
            }
            None => Ok(None),
        }
    }

    pub fn get_with_revision(&self, key: &str) -> Result<Option<KeyValueWithRevision>, SharedStorageError> {
        match self.get(key)? {
            Some(entry) => Ok(Some(KeyValueWithRevision {
                key: key.to_string(),
                value: entry.value,
                version: entry.version as u64,
                create_revision: entry.create_revision as u64,
                mod_revision: entry.mod_revision as u64,
            })),
            None => Ok(None),
        }
    }

    pub fn scan(
        &self,
        prefix: &str,
        after_key: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<KeyValueWithRevision>, SharedStorageError> {
        if let Some(after) = after_key {
            debug_assert!(
                prefix.is_empty() || after >= prefix,
                "SCAN: after_key '{}' must be >= prefix '{}'",
                after,
                prefix
            );
        }

        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;

        let current_ms = now_ms();
        let max_results = scan_limit_usize(limit);
        let prefix_bytes = prefix.as_bytes();

        let mut results = Vec::with_capacity(max_results.min(128));

        for item in table.iter().context(RangeSnafu)? {
            let (key_guard, value_guard) = item.context(GetSnafu)?;
            let key_bytes = key_guard.value();

            if !key_bytes.starts_with(prefix_bytes) {
                if key_bytes > prefix_bytes {
                    break;
                }
                continue;
            }

            let key_str = match std::str::from_utf8(key_bytes) {
                Ok(s) => s,
                Err(_) => continue,
            };

            if let Some(after) = after_key
                && key_str <= after
            {
                continue;
            }

            let entry: KvEntry = match bincode::deserialize(value_guard.value()) {
                Ok(e) => e,
                Err(_) => continue,
            };

            if let Some(expires_at) = entry.expires_at_ms
                && current_ms > expires_at
            {
                continue;
            }

            results.push(KeyValueWithRevision {
                key: key_str.to_string(),
                value: entry.value,
                version: entry.version as u64,
                create_revision: entry.create_revision as u64,
                mod_revision: entry.mod_revision as u64,
            });

            if results.len() >= max_results {
                break;
            }
        }

        assert!(
            results.len() <= max_results,
            "SCAN: results {} exceed max_results {}",
            results.len(),
            max_results
        );

        Ok(results)
    }

    pub fn delete_expired_keys(&self, batch_limit: u32) -> Result<u32, SharedStorageError> {
        let current_ms = now_ms();
        let mut deleted: u32 = 0;

        let keys_to_delete: Vec<Vec<u8>> = {
            let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
            let table = read_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;

            let max_keys_to_delete = bounded_batch_limit_usize(batch_limit);
            let mut keys = Vec::with_capacity(max_keys_to_delete.min(128));
            for item in table.iter().context(RangeSnafu)? {
                if keys.len() >= max_keys_to_delete {
                    break;
                }

                let (key, value) = item.context(GetSnafu)?;
                let entry: KvEntry = bincode::deserialize(value.value()).context(DeserializeSnafu)?;

                if let Some(expires_at) = entry.expires_at_ms
                    && expires_at <= current_ms
                {
                    keys.push(key.value().to_vec());
                }
            }
            keys
        };

        if !keys_to_delete.is_empty() {
            let write_txn = self.db.begin_write().context(BeginWriteSnafu)?;
            {
                let mut table = write_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;
                for key in &keys_to_delete {
                    table.remove(key.as_slice()).context(RemoveSnafu)?;
                    deleted += 1;
                }
            }
            write_txn.commit().context(CommitSnafu)?;
        }

        Ok(deleted)
    }

    pub fn count_expired_keys(&self) -> Result<u64, SharedStorageError> {
        let current_ms = now_ms();
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;

        let mut count: u64 = 0;
        for item in table.iter().context(RangeSnafu)? {
            let (_key, value) = item.context(GetSnafu)?;
            let entry: KvEntry = bincode::deserialize(value.value()).context(DeserializeSnafu)?;

            if let Some(expires_at) = entry.expires_at_ms
                && expires_at <= current_ms
            {
                count += 1;
            }
        }

        Ok(count)
    }

    pub fn count_keys_with_ttl(&self) -> Result<u64, SharedStorageError> {
        let current_ms = now_ms();
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;

        let mut count: u64 = 0;
        for item in table.iter().context(RangeSnafu)? {
            let (_key, value) = item.context(GetSnafu)?;
            let entry: KvEntry = bincode::deserialize(value.value()).context(DeserializeSnafu)?;

            if let Some(expires_at) = entry.expires_at_ms
                && expires_at > current_ms
            {
                count += 1;
            }
        }

        Ok(count)
    }

    pub fn get_expired_keys_with_metadata(
        &self,
        batch_limit: u32,
    ) -> Result<Vec<(String, Option<u64>)>, SharedStorageError> {
        let current_ms = now_ms();
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;

        let max_expired_keys = bounded_batch_limit_usize(batch_limit);
        let mut expired_keys = Vec::with_capacity(max_expired_keys.min(128));
        for item in table.iter().context(RangeSnafu)? {
            if expired_keys.len() >= max_expired_keys {
                break;
            }

            let (key, value) = item.context(GetSnafu)?;
            let entry: KvEntry = bincode::deserialize(value.value()).context(DeserializeSnafu)?;

            if let Some(expires_at) = entry.expires_at_ms
                && expires_at <= current_ms
            {
                let key_str = String::from_utf8_lossy(key.value()).to_string();
                expired_keys.push((key_str, None));
            }
        }

        Ok(expired_keys)
    }
}
