//! Writer-preference fairness: pending writer count management.

use anyhow::Result;
use anyhow::bail;
use aspen_constants::coordination::MAX_RWLOCK_PENDING_WRITERS;
use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;

use super::RWLockManager;
use super::types::RWLockState;
use crate::error::CoordinationError;

impl<S: KeyValueStore + ?Sized + 'static> RWLockManager<S> {
    /// Increment pending writers count.
    pub(super) async fn increment_pending_writers(&self, key: &str, name: &str) -> Result<()> {
        loop {
            let current = self.read_state(key).await?;

            match current {
                None => {
                    // Create new state with pending writer
                    let mut state = RWLockState::new(name);
                    state.pending_writers = 1;
                    let new_json = serde_json::to_string(&state)?;

                    match self
                        .store
                        .write(WriteRequest {
                            command: WriteCommand::CompareAndSwap {
                                key: key.to_string(),
                                expected: None,
                                new_value: new_json,
                            },
                        })
                        .await
                    {
                        Ok(_) => return Ok(()),
                        Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => continue,
                        Err(e) => bail!("rwlock CAS failed: {}", e),
                    }
                }
                Some(state) => {
                    // Tiger Style: Enforce pending writer limit to prevent resource exhaustion
                    if state.pending_writers >= MAX_RWLOCK_PENDING_WRITERS {
                        return Err(CoordinationError::TooManyPendingWriters {
                            name: name.to_string(),
                            count: state.pending_writers,
                            max: MAX_RWLOCK_PENDING_WRITERS,
                        }
                        .into());
                    }

                    // Clone state before modification for CAS (avoids double-read panic risk)
                    let old_json = serde_json::to_string(&state)?;
                    let mut new_state = state;
                    new_state.pending_writers += 1;
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
                        Ok(_) => return Ok(()),
                        Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => continue,
                        Err(e) => bail!("rwlock CAS failed: {}", e),
                    }
                }
            }
        }
    }

    /// Decrement pending writers count.
    pub(super) async fn decrement_pending_writers(&self, key: &str) -> Result<()> {
        loop {
            let current = self.read_state(key).await?;

            if let Some(state) = current {
                // Clone state before modification for CAS (avoids double-read panic risk)
                let old_json = serde_json::to_string(&state)?;
                let mut new_state = state;
                new_state.pending_writers = new_state.pending_writers.saturating_sub(1);
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
                    Ok(_) => return Ok(()),
                    Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => continue,
                    Err(e) => bail!("rwlock CAS failed: {}", e),
                }
            } else {
                return Ok(());
            }
        }
    }
}
