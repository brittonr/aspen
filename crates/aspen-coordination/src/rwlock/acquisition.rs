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

/// Result of a CAS operation on lock state.
enum CasResult<T> {
    /// CAS succeeded with the given result
    Success(T),
    /// CAS failed due to concurrent modification, retry needed
    Retry,
}

/// Type alias for the complex return type of blocking checks.
/// Returns (writer_seq, reader_seq, timeout_ms) when blocking is needed.
type BlockingResult = Result<Option<(u64, u64, u32)>>;

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
        // Tiger Style: argument validation
        debug_assert!(!name.is_empty(), "RWLOCK: name must not be empty");
        debug_assert!(!holder_id.is_empty(), "RWLOCK: holder_id must not be empty");
        debug_assert!(ttl_ms > 0, "RWLOCK: ttl_ms must be positive");

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
                None => match self.try_acquire_read_create(&key, name, holder_id, ttl_ms).await? {
                    CasResult::Success(result) => return Ok(Some(result)),
                    CasResult::Retry => continue,
                },
                Some(mut state) => {
                    state.cleanup_expired_readers();
                    state.cleanup_expired_writer();

                    // Check if already holding read lock - refresh TTL
                    if state.has_read_lock(holder_id) {
                        match self.try_acquire_read_refresh(&key, &state, holder_id, ttl_ms).await? {
                            CasResult::Success(result) => return Ok(Some(result)),
                            CasResult::Retry => continue,
                        }
                    }

                    // Check blocking conditions
                    if let Some(blocking_reason) = self.try_acquire_read_check_blocked(&state, name)? {
                        return blocking_reason;
                    }

                    // Acquire the read lock
                    match self.try_acquire_read_add_reader(&key, &state, holder_id, ttl_ms).await? {
                        CasResult::Success(result) => return Ok(Some(result)),
                        CasResult::Retry => continue,
                    }
                }
            }
        }
    }

    /// Create a new read lock state when none exists.
    async fn try_acquire_read_create(
        &self,
        key: &str,
        name: &str,
        holder_id: &str,
        ttl_ms: u64,
    ) -> Result<CasResult<(u64, u64, u32)>> {
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

        match self.cas_write(key, None, new_json).await? {
            CasResult::Success(()) => {
                debug_assert!(state.writer.is_none(), "RWLOCK: no writer must exist when creating new read lock");
                debug!(name, holder_id, "read lock created");
                Ok(CasResult::Success((0, deadline, 1)))
            }
            CasResult::Retry => Ok(CasResult::Retry),
        }
    }

    /// Refresh TTL for an existing read lock holder.
    async fn try_acquire_read_refresh(
        &self,
        key: &str,
        state: &RWLockState,
        holder_id: &str,
        ttl_ms: u64,
    ) -> Result<CasResult<(u64, u64, u32)>> {
        let mut new_state = state.clone();
        let now = now_unix_ms();
        let deadline = now + ttl_ms;
        if let Some(reader) = new_state.readers.iter_mut().find(|r| r.holder_id == holder_id) {
            reader.deadline_ms = deadline;
        }

        let old_json = serde_json::to_string(state)?;
        let new_json = serde_json::to_string(&new_state)?;

        match self.cas_write(key, Some(old_json), new_json).await? {
            CasResult::Success(()) => {
                let count = new_state.active_reader_count();
                Ok(CasResult::Success((new_state.fencing_token, deadline, count)))
            }
            CasResult::Retry => Ok(CasResult::Retry),
        }
    }

    /// Check if read acquisition is blocked.
    /// Returns Some(result) if blocked, None if acquisition can proceed.
    fn try_acquire_read_check_blocked(&self, state: &RWLockState, name: &str) -> Result<Option<BlockingResult>> {
        // Writer-preference: block if writer is waiting or holding
        if state.mode == RWLockMode::Write || state.writer.as_ref().is_some_and(|w| !w.is_expired()) {
            return Ok(Some(Ok(None)));
        }

        if state.pending_writers > 0 {
            return Ok(Some(Ok(None)));
        }

        // Tiger Style: Enforce reader limit
        let active_readers = state.active_reader_count();
        if active_readers >= MAX_RWLOCK_READERS {
            return Ok(Some(Err(CoordinationError::TooManyReaders {
                name: name.to_string(),
                count: active_readers,
                max: MAX_RWLOCK_READERS,
            }
            .into())));
        }

        Ok(None)
    }

    /// Add a new reader to an existing lock state.
    async fn try_acquire_read_add_reader(
        &self,
        key: &str,
        state: &RWLockState,
        holder_id: &str,
        ttl_ms: u64,
    ) -> Result<CasResult<(u64, u64, u32)>> {
        let mut new_state = state.clone();
        let now = now_unix_ms();
        let deadline = now + ttl_ms;
        new_state.readers.push(ReaderEntry {
            holder_id: holder_id.to_string(),
            deadline_ms: deadline,
        });
        new_state.mode = RWLockMode::Read;

        let old_json = serde_json::to_string(state)?;
        let new_json = serde_json::to_string(&new_state)?;

        match self.cas_write(key, Some(old_json), new_json).await? {
            CasResult::Success(()) => {
                let count = new_state.active_reader_count();
                debug_assert!(
                    new_state.writer.as_ref().is_none_or(|w| w.is_expired()),
                    "RWLOCK: no active writer must be held when read lock is acquired"
                );
                debug_assert!(count > 0, "RWLOCK: reader count must be positive after read acquisition");
                debug!(holder_id, count, "read lock acquired");
                Ok(CasResult::Success((new_state.fencing_token, deadline, count)))
            }
            CasResult::Retry => Ok(CasResult::Retry),
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
        // Tiger Style: argument validation
        debug_assert!(!name.is_empty(), "RWLOCK: name must not be empty for write");
        debug_assert!(!holder_id.is_empty(), "RWLOCK: holder_id must not be empty for write");
        debug_assert!(ttl_ms > 0, "RWLOCK: ttl_ms must be positive for write");

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
                None => match self.try_acquire_write_create(key, name, holder_id, ttl_ms).await? {
                    CasResult::Success(result) => return Ok(Some(result)),
                    CasResult::Retry => continue,
                },
                Some(mut state) => {
                    state.cleanup_expired_readers();
                    state.cleanup_expired_writer();

                    // Check if already holding write lock - refresh TTL
                    if state.has_write_lock(holder_id) {
                        match self.try_acquire_write_refresh(key, &state, ttl_ms).await? {
                            CasResult::Success(result) => return Ok(Some(result)),
                            CasResult::Retry => continue,
                        }
                    }

                    // Check blocking conditions
                    if self.try_acquire_write_is_blocked(&state) {
                        return Ok(None);
                    }

                    // Acquire the write lock
                    match self.try_acquire_write_set_writer(key, &state, holder_id, ttl_ms).await? {
                        CasResult::Success(result) => return Ok(Some(result)),
                        CasResult::Retry => continue,
                    }
                }
            }
        }
    }

    /// Create a new write lock state when none exists.
    async fn try_acquire_write_create(
        &self,
        key: &str,
        name: &str,
        holder_id: &str,
        ttl_ms: u64,
    ) -> Result<CasResult<(u64, u64)>> {
        let now = now_unix_ms();
        let lock_deadline = now + ttl_ms;
        let new_token = 1u64;
        let state = RWLockState {
            name: name.to_string(),
            mode: RWLockMode::Write,
            writer: Some(WriterEntry {
                holder_id: holder_id.to_string(),
                fencing_token: new_token,
                deadline_ms: lock_deadline,
            }),
            readers: Vec::new(),
            pending_writers: 0,
            fencing_token: new_token,
            created_at_ms: now,
        };
        let new_json = serde_json::to_string(&state)?;

        match self.cas_write(key, None, new_json).await? {
            CasResult::Success(()) => {
                debug_assert!(new_token > 0, "RWLOCK: write fencing token must be positive");
                debug!(name, holder_id, fencing_token = new_token, "write lock created");
                Ok(CasResult::Success((new_token, lock_deadline)))
            }
            CasResult::Retry => Ok(CasResult::Retry),
        }
    }

    /// Refresh TTL for an existing write lock holder.
    async fn try_acquire_write_refresh(
        &self,
        key: &str,
        state: &RWLockState,
        ttl_ms: u64,
    ) -> Result<CasResult<(u64, u64)>> {
        let mut new_state = state.clone();
        let now = now_unix_ms();
        let lock_deadline = now + ttl_ms;
        if let Some(ref mut writer) = new_state.writer {
            writer.deadline_ms = lock_deadline;
        }

        let old_json = serde_json::to_string(state)?;
        let new_json = serde_json::to_string(&new_state)?;

        match self.cas_write(key, Some(old_json), new_json).await? {
            CasResult::Success(()) => {
                let token = new_state.writer.as_ref().map(|w| w.fencing_token).unwrap_or(new_state.fencing_token);
                Ok(CasResult::Success((token, lock_deadline)))
            }
            CasResult::Retry => Ok(CasResult::Retry),
        }
    }

    /// Check if write acquisition is blocked.
    fn try_acquire_write_is_blocked(&self, state: &RWLockState) -> bool {
        // Readers holding, cannot acquire write
        if state.mode != RWLockMode::Free && state.active_reader_count() > 0 {
            return true;
        }
        // Writer holding
        state.mode == RWLockMode::Write
    }

    /// Set a new writer on an existing lock state.
    async fn try_acquire_write_set_writer(
        &self,
        key: &str,
        state: &RWLockState,
        holder_id: &str,
        ttl_ms: u64,
    ) -> Result<CasResult<(u64, u64)>> {
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
        new_state.pending_writers = new_state.pending_writers.saturating_sub(1);

        debug_assert!(
            new_state.readers.iter().all(|r| r.is_expired()),
            "RWLOCK: no active readers when acquiring write lock"
        );
        debug_assert!(new_token > state.fencing_token, "RWLOCK: write token must increase");

        let old_json = serde_json::to_string(state)?;
        let new_json = serde_json::to_string(&new_state)?;

        match self.cas_write(key, Some(old_json), new_json).await? {
            CasResult::Success(()) => {
                debug_assert!(
                    new_state.active_reader_count() == 0,
                    "RWLOCK: no active readers must be held when write lock is acquired"
                );
                debug_assert!(
                    new_state.mode == RWLockMode::Write,
                    "RWLOCK: mode must be Write after write acquisition"
                );
                debug!(holder_id, fencing_token = new_token, "write lock acquired");
                Ok(CasResult::Success((new_token, lock_deadline)))
            }
            CasResult::Retry => Ok(CasResult::Retry),
        }
    }

    /// Common CAS write helper that handles retry logic.
    async fn cas_write(&self, key: &str, expected: Option<String>, new_value: String) -> Result<CasResult<()>> {
        match self
            .store
            .write(WriteRequest {
                command: WriteCommand::CompareAndSwap {
                    key: key.to_string(),
                    expected,
                    new_value,
                },
            })
            .await
        {
            Ok(_) => Ok(CasResult::Success(())),
            Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => Ok(CasResult::Retry),
            Err(e) => bail!("rwlock CAS failed: {}", e),
        }
    }
}
