//! KV operations: get, scan, TTL cleanup, and related methods.

use redb::ReadableTable;
use snafu::ResultExt;

use super::*;

impl SharedRedbStorage {
    /// Get a key-value entry from the state machine.
    pub fn get(&self, key: &str) -> Result<Option<KvEntry>, SharedStorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;

        match table.get(key.as_bytes()).context(GetSnafu)? {
            Some(value) => {
                let entry: KvEntry = bincode::deserialize(value.value()).context(DeserializeSnafu)?;

                // Check expiration
                if let Some(expires_at) = entry.expires_at_ms
                    && now_unix_ms() > expires_at
                {
                    return Ok(None); // Expired
                }

                Ok(Some(entry))
            }
            None => Ok(None),
        }
    }

    /// Get a key-value with revision metadata.
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

    /// Scan keys matching a prefix.
    pub fn scan(
        &self,
        prefix: &str,
        after_key: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<KeyValueWithRevision>, SharedStorageError> {
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;

        let now_ms = now_unix_ms();
        let bounded_limit = limit.unwrap_or(MAX_BATCH_SIZE as usize).min(MAX_BATCH_SIZE as usize);
        let prefix_bytes = prefix.as_bytes();

        let mut results = Vec::with_capacity(bounded_limit.min(128));

        for item in table.iter().context(RangeSnafu)? {
            let (key_guard, value_guard) = item.context(GetSnafu)?;
            let key_bytes = key_guard.value();

            // Check prefix match
            if !key_bytes.starts_with(prefix_bytes) {
                if key_bytes > prefix_bytes {
                    // Past the prefix range
                    break;
                }
                continue;
            }

            let key_str = match std::str::from_utf8(key_bytes) {
                Ok(s) => s,
                Err(_) => continue,
            };

            // Check continuation token
            if let Some(after) = after_key
                && key_str <= after
            {
                continue;
            }

            let entry: KvEntry = match bincode::deserialize(value_guard.value()) {
                Ok(e) => e,
                Err(_) => continue,
            };

            // Check expiration
            if let Some(expires_at) = entry.expires_at_ms
                && now_ms > expires_at
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

            if results.len() >= bounded_limit {
                break;
            }
        }

        Ok(results)
    }

    // =========================================================================
    // TTL Cleanup Methods
    // =========================================================================

    /// Delete expired keys in a batch.
    ///
    /// Returns the number of keys deleted.
    /// This is used by the background TTL cleanup task.
    ///
    /// # Tiger Style
    /// - Fixed batch limit prevents unbounded work per call
    /// - Iterates over all keys (no index for TTL in Redb)
    /// - Idempotent: safe to call concurrently or repeatedly
    pub fn delete_expired_keys(&self, batch_limit: u32) -> Result<u32, SharedStorageError> {
        let now_ms = now_unix_ms();
        let mut deleted: u32 = 0;

        // Collect keys to delete (scan + filter)
        let keys_to_delete: Vec<Vec<u8>> = {
            let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
            let table = read_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;

            let mut keys = Vec::new();
            for item in table.iter().context(RangeSnafu)? {
                if keys.len() >= batch_limit as usize {
                    break;
                }

                let (key, value) = item.context(GetSnafu)?;
                let entry: KvEntry = bincode::deserialize(value.value()).context(DeserializeSnafu)?;

                if let Some(expires_at) = entry.expires_at_ms
                    && expires_at <= now_ms
                {
                    keys.push(key.value().to_vec());
                }
            }
            keys
        };

        // Delete in a write transaction
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

    /// Count the number of expired keys in the state machine.
    ///
    /// Useful for metrics and monitoring.
    pub fn count_expired_keys(&self) -> Result<u64, SharedStorageError> {
        let now_ms = now_unix_ms();
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;

        let mut count: u64 = 0;
        for item in table.iter().context(RangeSnafu)? {
            let (_key, value) = item.context(GetSnafu)?;
            let entry: KvEntry = bincode::deserialize(value.value()).context(DeserializeSnafu)?;

            if let Some(expires_at) = entry.expires_at_ms
                && expires_at <= now_ms
            {
                count += 1;
            }
        }

        Ok(count)
    }

    /// Count the number of keys with TTL set (not yet expired).
    ///
    /// Useful for metrics and monitoring.
    pub fn count_keys_with_ttl(&self) -> Result<u64, SharedStorageError> {
        let now_ms = now_unix_ms();
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;

        let mut count: u64 = 0;
        for item in table.iter().context(RangeSnafu)? {
            let (_key, value) = item.context(GetSnafu)?;
            let entry: KvEntry = bincode::deserialize(value.value()).context(DeserializeSnafu)?;

            if let Some(expires_at) = entry.expires_at_ms
                && expires_at > now_ms
            {
                count += 1;
            }
        }

        Ok(count)
    }

    /// Get expired keys with their metadata for hook event emission.
    ///
    /// Returns a vector of (key, ttl_set_at_ms) pairs for expired keys.
    /// The ttl_set_at_ms is the timestamp when the TTL was originally set,
    /// which can be computed from (expires_at_ms - ttl_ms) if we store that,
    /// or approximated from the created_at timestamp.
    ///
    /// # Tiger Style
    /// - Fixed batch limit prevents unbounded work per call
    /// - Read-only operation (doesn't delete keys)
    pub fn get_expired_keys_with_metadata(
        &self,
        batch_limit: u32,
    ) -> Result<Vec<(String, Option<u64>)>, SharedStorageError> {
        let now_ms = now_unix_ms();
        let read_txn = self.db.begin_read().context(BeginReadSnafu)?;
        let table = read_txn.open_table(SM_KV_TABLE).context(OpenTableSnafu)?;

        let mut expired_keys = Vec::new();
        for item in table.iter().context(RangeSnafu)? {
            if expired_keys.len() >= batch_limit as usize {
                break;
            }

            let (key, value) = item.context(GetSnafu)?;
            let entry: KvEntry = bincode::deserialize(value.value()).context(DeserializeSnafu)?;

            if let Some(expires_at) = entry.expires_at_ms
                && expires_at <= now_ms
            {
                let key_str = String::from_utf8_lossy(key.value()).to_string();
                // We don't store ttl_set_at explicitly; pass None
                // The expires_at_ms is available but when the TTL was originally set is not tracked
                expired_keys.push((key_str, None));
            }
        }

        Ok(expired_keys)
    }
}
