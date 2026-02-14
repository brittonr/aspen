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
                None => {
                    bail!("lock does not exist");
                }
                Some(state) => {
                    // Verify we hold write lock
                    if let Some(ref writer) = state.writer {
                        if writer.holder_id != holder_id {
                            bail!("not holding write lock");
                        }
                        if writer.fencing_token != fencing_token {
                            bail!("fencing token mismatch");
                        }
                    } else {
                        bail!("no write lock held");
                    }

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

                    let old_json = serde_json::to_string(&state)?;
                    let new_json = serde_json::to_string(&new_state)?;

                    match self
                        .store
                        .write(WriteRequest {
                            command: WriteCommand::CompareAndSwap {
                                key: key.clone(),
                                expected: Some(old_json),
                                new_value: new_json,
                            },
                        })
                        .await
                    {
                        Ok(_) => {
                            let count = new_state.active_reader_count();
                            debug!(name, holder_id, "write lock downgraded to read");
                            return Ok((new_state.fencing_token, deadline, count));
                        }
                        Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => {
                            continue;
                        }
                        Err(e) => bail!("rwlock CAS failed: {}", e),
                    }
                }
            }
        }
    }
}
