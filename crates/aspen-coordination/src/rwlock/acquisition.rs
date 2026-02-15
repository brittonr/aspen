//! Read and write lock acquisition logic.

use std::time::Duration;

use anyhow::Result;
use anyhow::bail;
use aspen_constants::coordination::MAX_RWLOCK_READERS;
use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
use tracing::debug;

use super::RWLockManager;
use super::types::RWLockMode;
use super::types::RWLockState;
use super::types::ReaderEntry;
use super::types::WriterEntry;
use crate::error::CoordinationError;
use crate::types::now_unix_ms;
use crate::verified;

impl<S: KeyValueStore + ?Sized + 'static> RWLockManager<S> {
    /// Acquire a read lock, blocking until available or timeout.
    ///
    /// Returns (fencing_token, deadline_ms, reader_count) on success.
    pub async fn acquire_read(
        &self,
        name: &str,
        holder_id: &str,
        ttl_ms: u64,
        timeout: Option<Duration>,
    ) -> Result<(u64, u64, u32)> {
        let deadline = timeout.map(|t| std::time::Instant::now() + t);

        loop {
            // Check timeout
            if let Some(d) = deadline
                && std::time::Instant::now() >= d
            {
                bail!("read lock acquire timeout");
            }

            match self.try_acquire_read(name, holder_id, ttl_ms).await? {
                Some(result) => return Ok(result),
                None => {
                    // Wait and retry
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            }
        }
    }

    /// Try to acquire a read lock without blocking.
    ///
    /// Returns Some((fencing_token, deadline_ms, reader_count)) on success, None if blocked.
    pub async fn try_acquire_read(&self, name: &str, holder_id: &str, ttl_ms: u64) -> Result<Option<(u64, u64, u32)>> {
        let key = verified::rwlock_key(name);

        loop {
            let current = self.read_state(&key).await?;

            match current {
                None => {
                    // Create new lock with this reader
                    let now = now_unix_ms();
                    let deadline = now + ttl_ms;
                    let state = RWLockState {
                        name: name.to_string(),
                        mode: RWLockMode::Read,
                        writer: None,
                        readers: vec![ReaderEntry {
                            holder_id: holder_id.to_string(),
                            deadline_ms: deadline,
                        }],
                        pending_writers: 0,
                        fencing_token: 0,
                        created_at_ms: now,
                    };
                    let new_json = serde_json::to_string(&state)?;

                    match self
                        .store
                        .write(WriteRequest {
                            command: WriteCommand::CompareAndSwap {
                                key: key.clone(),
                                expected: None,
                                new_value: new_json,
                            },
                        })
                        .await
                    {
                        Ok(_) => {
                            debug!(name, holder_id, "read lock created");
                            return Ok(Some((0, deadline, 1)));
                        }
                        Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => {
                            continue;
                        }
                        Err(e) => bail!("rwlock CAS failed: {}", e),
                    }
                }
                Some(mut state) => {
                    // Cleanup expired entries
                    state.cleanup_expired_readers();
                    state.cleanup_expired_writer();

                    // Check if already holding read lock
                    if state.has_read_lock(holder_id) {
                        // Refresh TTL
                        let mut new_state = state.clone();
                        let now = now_unix_ms();
                        let deadline = now + ttl_ms;
                        if let Some(reader) = new_state.readers.iter_mut().find(|r| r.holder_id == holder_id) {
                            reader.deadline_ms = deadline;
                        }

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
                                return Ok(Some((new_state.fencing_token, deadline, count)));
                            }
                            Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => {
                                continue;
                            }
                            Err(e) => bail!("rwlock CAS failed: {}", e),
                        }
                    }

                    // Check if can acquire read lock
                    // Writer-preference: block if writer is waiting or holding
                    if state.mode == RWLockMode::Write || state.writer.as_ref().is_some_and(|w| !w.is_expired()) {
                        // Write lock held, cannot acquire read
                        return Ok(None);
                    }

                    if state.pending_writers > 0 {
                        // Writers waiting, block new readers (writer-preference)
                        return Ok(None);
                    }

                    // Tiger Style: Enforce reader limit to prevent resource exhaustion
                    let active_readers = state.active_reader_count();
                    if active_readers >= MAX_RWLOCK_READERS {
                        return Err(CoordinationError::TooManyReaders {
                            name: name.to_string(),
                            count: active_readers,
                            max: MAX_RWLOCK_READERS,
                        }
                        .into());
                    }

                    // Can acquire read lock
                    let mut new_state = state.clone();
                    let now = now_unix_ms();
                    let deadline = now + ttl_ms;
                    new_state.readers.push(ReaderEntry {
                        holder_id: holder_id.to_string(),
                        deadline_ms: deadline,
                    });
                    new_state.mode = RWLockMode::Read;

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
                            debug!(name, holder_id, count, "read lock acquired");
                            return Ok(Some((new_state.fencing_token, deadline, count)));
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

    /// Acquire a write lock, blocking until available or timeout.
    ///
    /// Returns (fencing_token, deadline_ms) on success.
    pub async fn acquire_write(
        &self,
        name: &str,
        holder_id: &str,
        ttl_ms: u64,
        timeout: Option<Duration>,
    ) -> Result<(u64, u64)> {
        let key = verified::rwlock_key(name);
        let deadline = timeout.map(|t| std::time::Instant::now() + t);

        // First, register as pending writer (for fairness)
        self.increment_pending_writers(&key, name).await?;

        // Try to acquire, cleaning up on failure
        let result = self.acquire_write_inner(&key, name, holder_id, ttl_ms, deadline).await;

        // Decrement pending writers if we failed
        if result.is_err() {
            let _ = self.decrement_pending_writers(&key).await;
        }

        result
    }

    pub(super) async fn acquire_write_inner(
        &self,
        key: &str,
        name: &str,
        holder_id: &str,
        ttl_ms: u64,
        deadline: Option<std::time::Instant>,
    ) -> Result<(u64, u64)> {
        loop {
            // Check timeout
            if let Some(d) = deadline
                && std::time::Instant::now() >= d
            {
                bail!("write lock acquire timeout");
            }

            match self.try_acquire_write_inner(key, name, holder_id, ttl_ms).await? {
                Some(result) => return Ok(result),
                None => {
                    // Wait and retry
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            }
        }
    }

    /// Try to acquire a write lock without blocking.
    ///
    /// Returns Some((fencing_token, deadline_ms)) on success, None if blocked.
    pub async fn try_acquire_write(&self, name: &str, holder_id: &str, ttl_ms: u64) -> Result<Option<(u64, u64)>> {
        let key = verified::rwlock_key(name);
        self.try_acquire_write_inner(&key, name, holder_id, ttl_ms).await
    }

    pub(super) async fn try_acquire_write_inner(
        &self,
        key: &str,
        name: &str,
        holder_id: &str,
        ttl_ms: u64,
    ) -> Result<Option<(u64, u64)>> {
        loop {
            let current = self.read_state(key).await?;

            match current {
                None => {
                    // Create new lock with this writer
                    let now = now_unix_ms();
                    let lock_deadline = now + ttl_ms;
                    let new_token = 1;
                    let state = RWLockState {
                        name: name.to_string(),
                        mode: RWLockMode::Write,
                        writer: Some(WriterEntry {
                            holder_id: holder_id.to_string(),
                            fencing_token: new_token,
                            deadline_ms: lock_deadline,
                        }),
                        readers: Vec::new(),
                        pending_writers: 0, // We got it, not pending anymore
                        fencing_token: new_token,
                        created_at_ms: now,
                    };
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
                        Ok(_) => {
                            debug!(name, holder_id, fencing_token = new_token, "write lock created");
                            return Ok(Some((new_token, lock_deadline)));
                        }
                        Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => {
                            continue;
                        }
                        Err(e) => bail!("rwlock CAS failed: {}", e),
                    }
                }
                Some(mut state) => {
                    // Cleanup expired entries
                    state.cleanup_expired_readers();
                    state.cleanup_expired_writer();

                    // Check if already holding write lock (reentrant)
                    if state.has_write_lock(holder_id) {
                        // Refresh TTL
                        let mut new_state = state.clone();
                        let now = now_unix_ms();
                        let lock_deadline = now + ttl_ms;
                        if let Some(ref mut writer) = new_state.writer {
                            writer.deadline_ms = lock_deadline;
                        }

                        let old_json = serde_json::to_string(&state)?;
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
                                let token = match new_state.writer.as_ref() {
                                    Some(w) => w.fencing_token,
                                    None => new_state.fencing_token,
                                };
                                return Ok(Some((token, lock_deadline)));
                            }
                            Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => {
                                continue;
                            }
                            Err(e) => bail!("rwlock CAS failed: {}", e),
                        }
                    }

                    // Check if lock is free (no readers, no writer)
                    if state.mode != RWLockMode::Free && state.active_reader_count() > 0 {
                        // Readers holding, cannot acquire write
                        return Ok(None);
                    }

                    if state.mode == RWLockMode::Write {
                        // Writer holding
                        return Ok(None);
                    }

                    // Can acquire write lock
                    let mut new_state = state.clone();
                    let now = now_unix_ms();
                    let lock_deadline = now + ttl_ms;
                    let new_token = crate::verified::compute_next_write_token(new_state.fencing_token);
                    new_state.mode = RWLockMode::Write;
                    new_state.writer = Some(WriterEntry {
                        holder_id: holder_id.to_string(),
                        fencing_token: new_token,
                        deadline_ms: lock_deadline,
                    });
                    new_state.fencing_token = new_token;
                    // Decrement pending_writers since we're acquiring
                    new_state.pending_writers = new_state.pending_writers.saturating_sub(1);

                    let old_json = serde_json::to_string(&state)?;
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
                            debug!(name, holder_id, fencing_token = new_token, "write lock acquired");
                            return Ok(Some((new_token, lock_deadline)));
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
