//! Distributed read-write lock with fencing tokens.
//!
//! Provides shared read access or exclusive write access with:
//! - Multiple concurrent readers OR a single exclusive writer
//! - Writer-preference fairness to prevent writer starvation
//! - Monotonically increasing fencing tokens for split-brain prevention
//! - TTL-based automatic expiration for crash recovery
//!
//! ## Fairness Policy
//!
//! This implementation uses writer-preference:
//! - When a writer is waiting, new readers are blocked
//! - This prevents writer starvation in read-heavy workloads
//! - Readers already holding the lock can continue until they release
//!
//! ## Lock State
//!
//! The lock state is stored as JSON in the key-value store:
//! ```json
//! {
//!   "mode": "Read" | "Write" | "Free",
//!   "writer": {"holder_id": "...", "fencing_token": 42, "deadline_ms": ...},
//!   "readers": [{"holder_id": "...", "deadline_ms": ...}, ...],
//!   "pending_writers": 0,  // Count of waiting writers
//!   "fencing_token": 42    // Global token, incremented on write acquisition
//! }
//! ```

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::api::{KeyValueStore, KeyValueStoreError, ReadRequest, WriteCommand, WriteRequest};
use crate::coordination::types::now_unix_ms;

/// RWLock key prefix.
const RWLOCK_PREFIX: &str = "__rwlock:";

/// Lock mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RWLockMode {
    /// No lock held.
    Free,
    /// One or more readers holding shared lock.
    Read,
    /// Exclusive writer holding the lock.
    Write,
}

impl RWLockMode {
    /// Convert the mode to a string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            RWLockMode::Free => "free",
            RWLockMode::Read => "read",
            RWLockMode::Write => "write",
        }
    }
}

/// A reader holding a shared lock.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReaderEntry {
    /// Unique holder identifier.
    pub holder_id: String,
    /// Deadline for automatic release (ms since epoch).
    pub deadline_ms: u64,
}

impl ReaderEntry {
    /// Check if this reader entry has expired.
    pub fn is_expired(&self) -> bool {
        self.deadline_ms == 0 || now_unix_ms() > self.deadline_ms
    }
}

/// The writer holding an exclusive lock.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WriterEntry {
    /// Unique holder identifier.
    pub holder_id: String,
    /// Fencing token for this write acquisition.
    pub fencing_token: u64,
    /// Deadline for automatic release (ms since epoch).
    pub deadline_ms: u64,
}

impl WriterEntry {
    /// Check if this writer entry has expired.
    pub fn is_expired(&self) -> bool {
        self.deadline_ms == 0 || now_unix_ms() > self.deadline_ms
    }
}

/// RWLock state stored in the key-value store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RWLockState {
    /// Lock name.
    pub name: String,
    /// Current lock mode.
    pub mode: RWLockMode,
    /// Writer holding exclusive lock (if mode == Write).
    pub writer: Option<WriterEntry>,
    /// Readers holding shared lock (if mode == Read).
    pub readers: Vec<ReaderEntry>,
    /// Count of pending writers (for writer-preference fairness).
    pub pending_writers: u32,
    /// Global fencing token (incremented on each write acquisition).
    pub fencing_token: u64,
    /// Creation timestamp.
    pub created_at_ms: u64,
}

impl RWLockState {
    /// Create a new free lock state.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            mode: RWLockMode::Free,
            writer: None,
            readers: Vec::new(),
            pending_writers: 0,
            fencing_token: 0,
            created_at_ms: now_unix_ms(),
        }
    }

    /// Remove expired readers.
    pub fn cleanup_expired_readers(&mut self) {
        self.readers.retain(|r| !r.is_expired());
        if self.readers.is_empty() && self.mode == RWLockMode::Read {
            self.mode = RWLockMode::Free;
        }
    }

    /// Check if writer has expired and cleanup if so.
    pub fn cleanup_expired_writer(&mut self) {
        if let Some(ref writer) = self.writer
            && writer.is_expired()
        {
            self.writer = None;
            if self.mode == RWLockMode::Write {
                self.mode = RWLockMode::Free;
            }
        }
    }

    /// Find a reader by holder ID.
    pub fn find_reader(&self, holder_id: &str) -> Option<&ReaderEntry> {
        self.readers.iter().find(|r| r.holder_id == holder_id)
    }

    /// Check if this holder already has a read lock.
    pub fn has_read_lock(&self, holder_id: &str) -> bool {
        self.readers
            .iter()
            .any(|r| r.holder_id == holder_id && !r.is_expired())
    }

    /// Check if this holder has the write lock.
    pub fn has_write_lock(&self, holder_id: &str) -> bool {
        self.writer
            .as_ref()
            .map(|w| w.holder_id == holder_id && !w.is_expired())
            .unwrap_or(false)
    }

    /// Get the number of active (non-expired) readers.
    pub fn active_reader_count(&self) -> u32 {
        self.readers.iter().filter(|r| !r.is_expired()).count() as u32
    }
}

/// Manager for distributed read-write lock operations.
pub struct RWLockManager<S: KeyValueStore + ?Sized> {
    store: Arc<S>,
}

impl<S: KeyValueStore + ?Sized + 'static> RWLockManager<S> {
    /// Create a new RWLock manager.
    pub fn new(store: Arc<S>) -> Self {
        Self { store }
    }

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
    pub async fn try_acquire_read(
        &self,
        name: &str,
        holder_id: &str,
        ttl_ms: u64,
    ) -> Result<Option<(u64, u64, u32)>> {
        let key = format!("{}{}", RWLOCK_PREFIX, name);

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
                        if let Some(reader) = new_state
                            .readers
                            .iter_mut()
                            .find(|r| r.holder_id == holder_id)
                        {
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
                    if state.mode == RWLockMode::Write
                        || (state.writer.is_some() && !state.writer.as_ref().unwrap().is_expired())
                    {
                        // Write lock held, cannot acquire read
                        return Ok(None);
                    }

                    if state.pending_writers > 0 {
                        // Writers waiting, block new readers (writer-preference)
                        return Ok(None);
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
        let key = format!("{}{}", RWLOCK_PREFIX, name);
        let deadline = timeout.map(|t| std::time::Instant::now() + t);

        // First, register as pending writer (for fairness)
        self.increment_pending_writers(&key, name).await?;

        // Try to acquire, cleaning up on failure
        let result = self
            .acquire_write_inner(&key, name, holder_id, ttl_ms, deadline)
            .await;

        // Decrement pending writers if we failed
        if result.is_err() {
            let _ = self.decrement_pending_writers(&key).await;
        }

        result
    }

    async fn acquire_write_inner(
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

            match self
                .try_acquire_write_inner(key, name, holder_id, ttl_ms)
                .await?
            {
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
    pub async fn try_acquire_write(
        &self,
        name: &str,
        holder_id: &str,
        ttl_ms: u64,
    ) -> Result<Option<(u64, u64)>> {
        let key = format!("{}{}", RWLOCK_PREFIX, name);
        self.try_acquire_write_inner(&key, name, holder_id, ttl_ms)
            .await
    }

    async fn try_acquire_write_inner(
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
                            debug!(
                                name,
                                holder_id,
                                fencing_token = new_token,
                                "write lock created"
                            );
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
                                let token = new_state.writer.as_ref().unwrap().fencing_token;
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
                    let new_token = new_state.fencing_token + 1;
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
                            debug!(
                                name,
                                holder_id,
                                fencing_token = new_token,
                                "write lock acquired"
                            );
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

    /// Release a read lock.
    pub async fn release_read(&self, name: &str, holder_id: &str) -> Result<()> {
        let key = format!("{}{}", RWLOCK_PREFIX, name);

        loop {
            let current = self.read_state(&key).await?;

            match current {
                None => {
                    // Lock doesn't exist, nothing to release
                    return Ok(());
                }
                Some(mut state) => {
                    // Cleanup expired
                    state.cleanup_expired_readers();

                    // Find and remove this reader
                    let original_count = state.readers.len();
                    state.readers.retain(|r| r.holder_id != holder_id);

                    if state.readers.len() == original_count {
                        // Not holding read lock
                        return Ok(());
                    }

                    // Update mode if no readers left
                    if state.readers.is_empty() && state.mode == RWLockMode::Read {
                        state.mode = RWLockMode::Free;
                    }

                    let old_json = serde_json::to_string(&self.read_state(&key).await?.unwrap())?;
                    let new_json = serde_json::to_string(&state)?;

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
    pub async fn release_write(
        &self,
        name: &str,
        holder_id: &str,
        fencing_token: u64,
    ) -> Result<()> {
        let key = format!("{}{}", RWLOCK_PREFIX, name);

        loop {
            let current = self.read_state(&key).await?;

            match current {
                None => {
                    // Lock doesn't exist, nothing to release
                    return Ok(());
                }
                Some(mut state) => {
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

                    // Release the write lock
                    state.writer = None;
                    state.mode = RWLockMode::Free;

                    let old_json = serde_json::to_string(&self.read_state(&key).await?.unwrap())?;
                    let new_json = serde_json::to_string(&state)?;

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
        let key = format!("{}{}", RWLOCK_PREFIX, name);

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

    /// Get lock status.
    ///
    /// Returns (mode, reader_count, writer_holder, fencing_token).
    pub async fn status(&self, name: &str) -> Result<(String, u32, Option<String>, u64)> {
        let key = format!("{}{}", RWLOCK_PREFIX, name);

        match self.read_state(&key).await? {
            Some(mut state) => {
                state.cleanup_expired_readers();
                state.cleanup_expired_writer();
                let reader_count = state.active_reader_count();
                let writer_holder = state.writer.as_ref().map(|w| w.holder_id.clone());
                Ok((
                    state.mode.as_str().to_string(),
                    reader_count,
                    writer_holder,
                    state.fencing_token,
                ))
            }
            None => Ok(("free".to_string(), 0, None, 0)),
        }
    }

    /// Increment pending writers count.
    async fn increment_pending_writers(&self, key: &str, name: &str) -> Result<()> {
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
                Some(mut state) => {
                    state.pending_writers += 1;
                    let old_json = serde_json::to_string(&self.read_state(key).await?.unwrap())?;
                    let new_json = serde_json::to_string(&state)?;

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
    async fn decrement_pending_writers(&self, key: &str) -> Result<()> {
        loop {
            let current = self.read_state(key).await?;

            if let Some(mut state) = current {
                state.pending_writers = state.pending_writers.saturating_sub(1);
                let old_json = serde_json::to_string(&self.read_state(key).await?.unwrap())?;
                let new_json = serde_json::to_string(&state)?;

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

    /// Read lock state from the store.
    async fn read_state(&self, key: &str) -> Result<Option<RWLockState>> {
        match self
            .store
            .read(ReadRequest {
                key: key.to_string(),
            })
            .await
        {
            Ok(result) => {
                let value = result.kv.map(|kv| kv.value).unwrap_or_default();
                if value.is_empty() {
                    Ok(None)
                } else {
                    let state: RWLockState = serde_json::from_str(&value)?;
                    Ok(Some(state))
                }
            }
            Err(KeyValueStoreError::NotFound { .. }) => Ok(None),
            Err(e) => bail!("rwlock read failed: {}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::inmemory::DeterministicKeyValueStore;

    #[tokio::test]
    async fn test_rwlock_read_acquire_release() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = RWLockManager::new(store);

        // Acquire read lock
        let (token, deadline, count) = manager
            .try_acquire_read("test", "reader1", 60000)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(token, 0); // No writes yet
        assert!(deadline > 0);
        assert_eq!(count, 1);

        // Release
        manager.release_read("test", "reader1").await.unwrap();

        // Check status
        let (mode, reader_count, writer, _) = manager.status("test").await.unwrap();
        assert_eq!(mode, "free");
        assert_eq!(reader_count, 0);
        assert!(writer.is_none());
    }

    #[tokio::test]
    async fn test_rwlock_write_acquire_release() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = RWLockManager::new(store);

        // Acquire write lock
        let (token, deadline) = manager
            .try_acquire_write("test", "writer1", 60000)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(token, 1); // First write
        assert!(deadline > 0);

        // Release
        manager
            .release_write("test", "writer1", token)
            .await
            .unwrap();

        // Check status
        let (mode, _, _, _) = manager.status("test").await.unwrap();
        assert_eq!(mode, "free");
    }

    #[tokio::test]
    async fn test_rwlock_multiple_readers() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = RWLockManager::new(store);

        // First reader
        let (_, _, count1) = manager
            .try_acquire_read("test", "reader1", 60000)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(count1, 1);

        // Second reader (should succeed)
        let (_, _, count2) = manager
            .try_acquire_read("test", "reader2", 60000)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(count2, 2);

        // Third reader
        let (_, _, count3) = manager
            .try_acquire_read("test", "reader3", 60000)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(count3, 3);

        // Check status
        let (mode, count, _, _) = manager.status("test").await.unwrap();
        assert_eq!(mode, "read");
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_rwlock_write_blocks_readers() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = RWLockManager::new(store);

        // Acquire write lock
        let (token, _) = manager
            .try_acquire_write("test", "writer1", 60000)
            .await
            .unwrap()
            .unwrap();

        // Try to acquire read (should fail)
        let result = manager
            .try_acquire_read("test", "reader1", 60000)
            .await
            .unwrap();
        assert!(result.is_none());

        // Release write lock
        manager
            .release_write("test", "writer1", token)
            .await
            .unwrap();

        // Now read should succeed
        let result = manager
            .try_acquire_read("test", "reader1", 60000)
            .await
            .unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_rwlock_readers_block_write() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = RWLockManager::new(store);

        // Acquire read locks
        manager
            .try_acquire_read("test", "reader1", 60000)
            .await
            .unwrap()
            .unwrap();
        manager
            .try_acquire_read("test", "reader2", 60000)
            .await
            .unwrap()
            .unwrap();

        // Try to acquire write (should fail)
        let result = manager
            .try_acquire_write("test", "writer1", 60000)
            .await
            .unwrap();
        assert!(result.is_none());

        // Release all readers
        manager.release_read("test", "reader1").await.unwrap();
        manager.release_read("test", "reader2").await.unwrap();

        // Now write should succeed
        let result = manager
            .try_acquire_write("test", "writer1", 60000)
            .await
            .unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_rwlock_downgrade() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = RWLockManager::new(store);

        // Acquire write lock
        let (token, _) = manager
            .try_acquire_write("test", "holder1", 60000)
            .await
            .unwrap()
            .unwrap();

        // Downgrade to read
        let (new_token, _, count) = manager
            .downgrade("test", "holder1", token, 60000)
            .await
            .unwrap();

        assert_eq!(new_token, token); // Token preserved
        assert_eq!(count, 1);

        // Check status
        let (mode, reader_count, writer, _) = manager.status("test").await.unwrap();
        assert_eq!(mode, "read");
        assert_eq!(reader_count, 1);
        assert!(writer.is_none());
    }

    #[tokio::test]
    async fn test_rwlock_fencing_token_increments() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = RWLockManager::new(store);

        // First write
        let (token1, _) = manager
            .try_acquire_write("test", "writer1", 60000)
            .await
            .unwrap()
            .unwrap();
        manager
            .release_write("test", "writer1", token1)
            .await
            .unwrap();

        // Second write
        let (token2, _) = manager
            .try_acquire_write("test", "writer2", 60000)
            .await
            .unwrap()
            .unwrap();

        assert!(token2 > token1, "fencing token should increment");
    }
}
