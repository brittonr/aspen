//! Read and write lock release logic.

use anyhow::Result;
use anyhow::bail;
use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
use tracing::debug;

use super::RWLockManager;
use super::types::RWLockMode;
use crate::verified;

impl<S: KeyValueStore + ?Sized + 'static> RWLockManager<S> {
    /// Release a read lock.
    pub async fn release_read(&self, name: &str, holder_id: &str) -> Result<()> {
        let key = verified::rwlock_key(name);

        loop {
            let current = self.read_state(&key).await?;

            match current {
                None => {
                    // Lock doesn't exist, nothing to release
                    return Ok(());
                }
                Some(state) => {
                    // Serialize original state for CAS before any modifications
                    let old_json = serde_json::to_string(&state)?;

                    let mut new_state = state;
                    // Cleanup expired
                    new_state.cleanup_expired_readers();

                    // Find and remove this reader
                    let original_count = new_state.readers.len();
                    new_state.readers.retain(|r| r.holder_id != holder_id);

                    if new_state.readers.len() == original_count {
                        // Not holding read lock
                        return Ok(());
                    }

                    // Update mode if no readers left
                    if new_state.readers.is_empty() && new_state.mode == RWLockMode::Read {
                        new_state.mode = RWLockMode::Free;
                    }

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
                            debug!(name, holder_id, "read lock released");
                            return Ok(());
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

    /// Release a write lock.
    pub async fn release_write(&self, name: &str, holder_id: &str, fencing_token: u64) -> Result<()> {
        let key = verified::rwlock_key(name);

        loop {
            let current = self.read_state(&key).await?;

            match current {
                None => {
                    // Lock doesn't exist, nothing to release
                    return Ok(());
                }
                Some(state) => {
                    // Check if we hold the write lock
                    if let Some(ref writer) = state.writer {
                        if writer.holder_id != holder_id {
                            bail!("not holding write lock");
                        }
                        if writer.fencing_token != fencing_token {
                            bail!("fencing token mismatch");
                        }
                    } else {
                        // No writer, nothing to release
                        return Ok(());
                    }

                    // Serialize original state for CAS before any modifications
                    let old_json = serde_json::to_string(&state)?;

                    // Release the write lock
                    let mut new_state = state;
                    new_state.writer = None;
                    new_state.mode = RWLockMode::Free;

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
                            debug!(name, holder_id, fencing_token, "write lock released");
                            return Ok(());
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
