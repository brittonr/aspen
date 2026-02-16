//! Write-to-read lock downgrade logic.

use anyhow::Result;
use anyhow::bail;
use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
use tracing::debug;

use super::RWLockManager;
use super::types::RWLockMode;
use super::types::ReaderEntry;
use crate::types::now_unix_ms;
use crate::verified;

/// Internal result type for downgrade operations.
enum DowngradeResult {
    /// Successfully downgraded (fencing_token, deadline_ms, reader_count).
    Success(u64, u64, u32),
    /// CAS conflict, retry required.
    Retry,
}

impl<S: KeyValueStore + ?Sized + 'static> RWLockManager<S> {
    /// Downgrade a write lock to a read lock.
    ///
    /// Returns (fencing_token, deadline_ms, reader_count) on success.
    pub async fn downgrade(
        &self,
        name: &str,
        holder_id: &str,
        fencing_token: u64,
        ttl_ms: u64,
    ) -> Result<(u64, u64, u32)> {
        let key = verified::rwlock_key(name);

        loop {
            let current = self.read_state(&key).await?;

            match current {
                None => bail!("lock does not exist"),
                Some(state) => {
                    match self.downgrade_execute(name, holder_id, fencing_token, ttl_ms, &key, &state).await? {
                        DowngradeResult::Success(token, deadline, count) => return Ok((token, deadline, count)),
                        DowngradeResult::Retry => continue,
                    }
                }
            }
        }
    }

    /// Execute the downgrade operation on the lock state.
    async fn downgrade_execute(
        &self,
        name: &str,
        holder_id: &str,
        fencing_token: u64,
        ttl_ms: u64,
        key: &str,
        state: &super::types::RWLockState,
    ) -> Result<DowngradeResult> {
        // Verify we hold write lock
        self.downgrade_verify_writer(holder_id, fencing_token, state)?;

        // Downgrade to read lock
        let mut new_state = state.clone();
        let now = now_unix_ms();
        let deadline = now + ttl_ms;
        new_state.mode = RWLockMode::Read;
        new_state.writer = None;
        new_state.readers.push(ReaderEntry {
            holder_id: holder_id.to_string(),
            deadline_ms: deadline,
        });

        let old_json = serde_json::to_string(state)?;
        let new_json = serde_json::to_string(&new_state)?;

        match self
            .store
            .write(WriteRequest {
                command: WriteCommand::CompareAndSwap {
                    key: key.to_string(),
                    expected: Some(old_json),
                    new_value: new_json,
                },
            })
            .await
        {
            Ok(_) => {
                let count = new_state.active_reader_count();
                debug_assert!(new_state.writer.is_none(), "RWLOCK: writer must be None after downgrade");
                debug_assert!(count >= 1, "RWLOCK: must have at least 1 reader after downgrade, got {count}");
                debug_assert!(
                    new_state.fencing_token == fencing_token,
                    "RWLOCK: fencing token must be preserved after downgrade: {} != {fencing_token}",
                    new_state.fencing_token
                );
                debug!(name, holder_id, "write lock downgraded to read");
                Ok(DowngradeResult::Success(new_state.fencing_token, deadline, count))
            }
            Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => Ok(DowngradeResult::Retry),
            Err(e) => bail!("rwlock CAS failed: {}", e),
        }
    }

    /// Verify that the holder has the write lock with the correct fencing token.
    fn downgrade_verify_writer(
        &self,
        holder_id: &str,
        fencing_token: u64,
        state: &super::types::RWLockState,
    ) -> Result<()> {
        if let Some(ref writer) = state.writer {
            if writer.holder_id != holder_id {
                bail!("not holding write lock");
            }
            if writer.fencing_token != fencing_token {
                bail!("fencing token mismatch");
            }
            Ok(())
        } else {
            bail!("no write lock held")
        }
    }
}
